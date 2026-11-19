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
use super::integrity::ModuleKind;
use super::lexicon::{self, LexiconEntry};
use super::queries::{self, Book, BookOutline, IndexedVerse, ModuleInfo, RefRange, VerseRow};
use super::search::{self, WordSuggestion};

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
///
/// `run` is for Bible queries: a lexicon has none of a Bible's tables, so
/// asking one for verses is refused plainly instead of failing in SQL.
async fn run<T, F>(registry: &ModuleRegistry, module_id: &str, query: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&Connection) -> Result<T> + Send + 'static,
{
    run_kind(registry, module_id, Some(ModuleKind::Bible), query).await
}

/// Runs a query on a module of the kind `kind`, or of any kind if `None`.
async fn run_kind<T, F>(
    registry: &ModuleRegistry,
    module_id: &str,
    kind: Option<ModuleKind>,
    query: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&Connection) -> Result<T> + Send + 'static,
{
    let module = registry.get(module_id)?;
    let module_id = module_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let db = module.lock().map_err(|_| {
            ModuleError::Task("the module is unusable after an earlier failure".into())
        })?;
        if let Some(kind) = kind {
            if db.kind() != kind {
                return Err(ModuleError::Unsupported(format!(
                    "{module_id} is a {}, not a {}",
                    kind_name(db.kind()),
                    kind_name(kind)
                )));
            }
        }
        query(db.conn())
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?
}

fn kind_name(kind: ModuleKind) -> &'static str {
    match kind {
        ModuleKind::Bible => "Bible",
        ModuleKind::Lexicon => "lexicon",
    }
}

#[tauri::command]
pub async fn module_info(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
) -> Result<ModuleInfo> {
    run_kind(&registry, &module_id, None, queries::module_info).await
}

/// The entries of a lexicon that a Strong's number or lemma leads to.
/// `kind` is "strong" or "lemma". A Strong's number may be written as the
/// Bible writes it ("H0430"); it's put in normal form first.
#[tauri::command]
pub async fn lexicon_lookup(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    kind: String,
    value: String,
) -> Result<Vec<LexiconEntry>> {
    let value = match kind.as_str() {
        "strong" => match lexicon::normalize_strong(&value) {
            Some(normal) => normal,
            None => return Ok(Vec::new()),
        },
        "lemma" => value,
        _ => {
            return Err(ModuleError::Unsupported(format!(
                "{kind} isn't a kind of lookup"
            )))
        }
    };
    run_kind(
        &registry,
        &module_id,
        Some(ModuleKind::Lexicon),
        move |conn| lexicon::lookup(conn, &kind, &value),
    )
    .await
}

/// One lexicon entry by its key, such as "H7218", for a link inside an
/// entry or the Back button.
#[tauri::command]
pub async fn lexicon_entry(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    key: String,
) -> Result<Option<LexiconEntry>> {
    run_kind(
        &registry,
        &module_id,
        Some(ModuleKind::Lexicon),
        move |conn| lexicon::entry_by_key(conn, &key),
    )
    .await
}

#[tauri::command]
pub async fn books(registry: State<'_, ModuleRegistry>, module_id: String) -> Result<Vec<Book>> {
    run(&registry, &module_id, queries::books).await
}

/// Every book's chapters and their verse counts.
#[tauri::command]
pub async fn book_outline(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
) -> Result<Vec<BookOutline>> {
    run(&registry, &module_id, queries::book_outline).await
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

/// Every verse matching a word search, in Bible order (see `search`).
#[tauri::command]
pub async fn text_sids(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    search: String,
) -> Result<Vec<String>> {
    run(&registry, &module_id, move |conn| {
        search::text_sids(conn, &search)
    })
    .await
}

/// One page of a word search's verses, with the words that matched.
#[tauri::command]
pub async fn text_verses(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    search: String,
    offset: u32,
    limit: u32,
) -> Result<Vec<IndexedVerse>> {
    run(&registry, &module_id, move |conn| {
        search::text_verses(conn, &search, offset, limit)
    })
    .await
}

/// The verses with these sids, in order, skipping any this module lacks:
/// a parallel text showing another Bible's search results.
#[tauri::command]
pub async fn verses_for_sids(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sids: Vec<String>,
) -> Result<Vec<VerseRow>> {
    run(&registry, &module_id, move |conn| {
        search::verses_for_sids(conn, &sids)
    })
    .await
}

/// Words beginning with what's typed, with how many verses have each.
#[tauri::command]
pub async fn word_suggestions(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    prefix: String,
    limit: u32,
) -> Result<Vec<WordSuggestion>> {
    run(&registry, &module_id, move |conn| {
        search::word_suggestions(conn, &prefix, limit)
    })
    .await
}

/// One whole chapter for infinite scrolling: the chapter holding `sid`, or
/// `offset` chapters after (positive) or before (negative) it. `None` past
/// either end of the module.
#[tauri::command]
pub async fn chapter_at(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sid: String,
    offset: i32,
) -> Result<Option<queries::ChapterText>> {
    run(&registry, &module_id, move |conn| {
        queries::chapter_at(conn, &sid, offset)
    })
    .await
}

/// The verse to show for `sid` in this module, when the reader switches
/// Bibles: `sid`, or the nearest verse the module has.
#[tauri::command]
pub async fn nearest_sid(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sid: String,
) -> Result<Option<String>> {
    run(&registry, &module_id, move |conn| {
        queries::nearest_sid(conn, &sid)
    })
    .await
}

/// The verses with footnotes in the chapter holding verse `sid`.
#[tauri::command]
pub async fn chapter_notes(
    registry: State<'_, ModuleRegistry>,
    module_id: String,
    sid: String,
) -> Result<Vec<VerseRow>> {
    run(&registry, &module_id, move |conn| {
        queries::chapter_notes(conn, &sid)
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
