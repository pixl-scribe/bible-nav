//! Tauri commands: the only way the web page can read modules.
//!
//! The page calls these with `invoke`, for example:
//! `invoke('books', { moduleId: 'eng_kjv' })`. Tauri converts the argument
//! names from camelCase, and turns the return value into JSON.
//!
//! The page can only ask for modules by id. It can't open files or choose
//! paths: Rust code decides which files are opened, through
//! `ModuleRegistry::open`, and every one passes the checks in `db.rs`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::State;

use super::db::ModuleDb;
use super::error::{ModuleError, Result};
use super::queries::{
    self, Book, IndexedVerse, ModuleInfo, ParagraphWindow, RefRange, VerseWindow,
};

/// The modules currently open, by id (such as "eng_kjv").
///
/// Tauri shares one registry across all commands (`app.manage(...)`), and
/// commands may run on several threads at once, so the map sits behind a
/// `Mutex`. Each module also has its own `Mutex`, because a SQLite
/// connection must only be used by one thread at a time. `Arc` is a shared,
/// reference-counted pointer: it lets a query keep using a module even while
/// the map itself is unlocked.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: Mutex<HashMap<String, Arc<Mutex<ModuleDb>>>>,
}

impl ModuleRegistry {
    /// Opens and checks a module, then makes it available under `id`.
    /// Only Rust code calls this; the page has no command that opens files.
    pub fn open(&self, id: &str, path: &Path) -> Result<()> {
        let db = ModuleDb::open(path)?;
        let mut modules = self.lock_map()?;
        modules.insert(id.to_string(), Arc::new(Mutex::new(db)));
        Ok(())
    }

    /// Closes every module, before reopening them from a new folder.
    pub fn clear(&self) -> Result<()> {
        self.lock_map()?.clear();
        Ok(())
    }

    fn lock_map(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, Arc<Mutex<ModuleDb>>>>> {
        self.modules.lock().map_err(|_| {
            ModuleError::Task("the module registry is unusable after an earlier failure".into())
        })
    }

    fn get(&self, id: &str) -> Result<Arc<Mutex<ModuleDb>>> {
        self.lock_map()?
            .get(id)
            .cloned()
            .ok_or_else(|| ModuleError::UnknownModule(id.to_string()))
    }
}

/// Runs a query on a background thread, so a slow query never freezes the
/// window. `query` is a closure: a function value, like an arrow function in
/// TypeScript. `move` makes it take ownership of what it uses, which Rust
/// requires before handing it to another thread.
async fn run<T, F>(registry: &ModuleRegistry, module_id: &str, query: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&Connection) -> Result<T> + Send + 'static,
{
    let module = registry.get(module_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let db = module.lock().map_err(|_| {
            ModuleError::Task("the module is unusable after an earlier failure".into())
        })?;
        query(db.conn())
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?
}

#[tauri::command]
pub async fn module_info(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
) -> Result<ModuleInfo> {
    run(&registry, &module_id, queries::module_info).await
}

#[tauri::command]
pub async fn books(registry: State<'_, ModuleRegistry>, module_id: String) -> Result<Vec<Book>> {
    run(&registry, &module_id, queries::books).await
}

/// The paragraphs around paragraph `center`.
#[tauri::command]
pub async fn paragraph_window(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    center: i64,
    before: u32,
    after: u32,
) -> Result<ParagraphWindow> {
    run(&registry, &module_id, move |conn| {
        queries::paragraph_window(conn, center, before, after)
    })
    .await
}

/// The paragraphs around the one holding verse `sid`, in a single call.
#[tauri::command]
pub async fn paragraph_window_at_sid(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sid: String,
    before: u32,
    after: u32,
) -> Result<ParagraphWindow> {
    run(&registry, &module_id, move |conn| {
        let center = queries::paragraph_for_sid(conn, &sid)?;
        queries::paragraph_window(conn, center, before, after)
    })
    .await
}

/// The verses around verse id `center`.
#[tauri::command]
pub async fn verse_window(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    center: i64,
    before: u32,
    after: u32,
) -> Result<VerseWindow> {
    run(&registry, &module_id, move |conn| {
        queries::verse_window(conn, center, before, after)
    })
    .await
}

/// The verses around verse `sid`, in a single call.
#[tauri::command]
pub async fn verse_window_at_sid(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sid: String,
    before: u32,
    after: u32,
) -> Result<VerseWindow> {
    run(&registry, &module_id, move |conn| {
        let center = queries::verse_id_for_sid(conn, &sid)?;
        queries::verse_window(conn, center, before, after)
    })
    .await
}

/// The sids of every verse in the reference ranges.
#[tauri::command]
pub async fn ref_sids(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    ranges: Vec<RefRange>,
) -> Result<Vec<String>> {
    run(&registry, &module_id, move |conn| {
        queries::ref_sids(conn, &ranges)
    })
    .await
}

/// One page of the verses in the reference ranges.
#[tauri::command]
pub async fn ref_verses(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    ranges: Vec<RefRange>,
    offset: u32,
    limit: u32,
) -> Result<Vec<IndexedVerse>> {
    run(&registry, &module_id, move |conn| {
        queries::ref_verses(conn, &ranges, offset, limit)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::fixture_path;

    #[test]
    fn registry_serves_opened_modules_only() {
        let registry = ModuleRegistry::default();
        assert!(matches!(
            registry.get("eng_kjv"),
            Err(ModuleError::UnknownModule(_))
        ));
        registry.open("eng_kjv", &fixture_path()).unwrap();
        assert!(registry.get("eng_kjv").is_ok());
    }

    #[test]
    fn queries_run_through_the_registry() {
        let registry = ModuleRegistry::default();
        registry.open("eng_kjv", &fixture_path()).unwrap();
        let books =
            tauri::async_runtime::block_on(run(&registry, "eng_kjv", queries::books)).unwrap();
        assert_eq!(books.len(), 66);
        let unknown = tauri::async_runtime::block_on(run(&registry, "grc_tr", queries::books));
        assert!(matches!(unknown, Err(ModuleError::UnknownModule(_))));
    }
}
