//! The Modules dialog's commands for the module catalog: checking it,
//! downloading and cancelling, deleting older versions, and opening the
//! folder. Nothing goes online except when the reader asks.
//!
//! Portable copies may download too, when asked, but only into their own
//! modules folder on the drive; they record nothing on the computer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_opener::OpenerExt;

use super::catalog::{self, Availability, Offer};
use super::download::{self, Web};
use super::error::{ModuleError, Result};
use super::folder::{
    load_folder, write_setting, FolderState, FolderStatus, Install, ModuleSummary,
    CATALOG_CHECKED_KEY,
};
use super::integrity::sidecars;
use super::store::{compare_versions, FolderChoice};
use super::{keys, statements, trust};

/// The event that reports a download's progress to the page.
const PROGRESS_EVENT: &str = "module-download-progress";

/// The offers from the last check, and the downloads in progress.
#[derive(Default)]
pub struct CatalogState {
    offers: Mutex<Vec<Offer>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// One module the catalog offers, for the Modules dialog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferSummary {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub license: String,
    /// The compressed download's size in bytes.
    pub size: u64,
    /// A new text of a pinned module, which only a newer Bible Nav can use.
    pub needs_newer_app: bool,
    /// The newest installed version of this module, if any.
    pub installed_version: Option<String>,
    /// "new" (not installed), "installed" (this version), "update" (newer
    /// than the installed one), or "older".
    pub relation: &'static str,
    /// An update to a new major version: the text itself changed.
    pub text_changes: bool,
    /// A pinned text whose signed manifest gives the pinned root; the file
    /// is checked again after downloading.
    pub text_verified: bool,
}

/// What a check of the catalog found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    pub offers: Vec<OfferSummary>,
    /// Entries or key statements that were skipped, and why.
    pub problems: Vec<String>,
    pub checked_at: u64,
    /// The modules folder, reloaded in case a key statement changed what's
    /// trusted.
    pub folder: FolderStatus,
}

/// Progress of one download, sent as `module-download-progress`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    id: String,
    version: String,
    done: u64,
    total: u64,
}

fn current_folder(state: &FolderState) -> Result<FolderChoice> {
    state
        .get()?
        .0
        .ok_or_else(|| ModuleError::Unsupported("there's no modules folder".into()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// Reads the catalog from the web, applies its key statements, and checks
/// every entry. Goes online; the page calls it only when the reader asks.
#[tauri::command]
pub async fn check_for_modules<R: Runtime>(
    app: AppHandle<R>,
    folder_state: State<'_, FolderState>,
    catalog_state: State<'_, CatalogState>,
) -> Result<CatalogStatus> {
    let choice = current_folder(&folder_state)?;
    let worker_app = app.clone();
    let worker_choice = choice.clone();
    let (offers, problems, folder) =
        tauri::async_runtime::spawn_blocking(move || check_catalog(&worker_app, &worker_choice))
            .await
            .map_err(|error| ModuleError::Task(error.to_string()))??;

    let checked_at = now();
    if !choice.portable {
        write_setting(&app, CATALOG_CHECKED_KEY, json!(checked_at));
    }
    let mut folder = folder;
    folder.catalog_checked_at = Some(checked_at);
    folder_state.set(Some(choice), folder.clone())?;
    let summaries = summarize(&offers, &folder.modules);
    *catalog_state
        .offers
        .lock()
        .map_err(|_| ModuleError::Task("the module list is unusable".into()))? = offers;
    Ok(CatalogStatus {
        offers: summaries,
        problems,
        checked_at,
        folder,
    })
}

fn check_catalog<R: Runtime>(
    app: &AppHandle<R>,
    choice: &FolderChoice,
) -> Result<(Vec<Offer>, Vec<String>, FolderStatus)> {
    let (url, bytes) = download::fetch_catalog(&Web::new(), catalog::CATALOG_URLS)?;
    let parsed = catalog::parse(&bytes, &url)?;

    // Key statements first, so the entries are checked with the keys they
    // establish. Accepted ones are saved like any others: in the modules
    // folder, and the app's own folder unless this copy is portable.
    let checked = statements::check_all(parsed.statements, &keys::backup_keys());
    let mut problems = checked.problems;
    let saved = (!choice.portable)
        .then(|| app.path().app_data_dir().ok().map(|dir| dir.join("keys")))
        .flatten();
    let keys_folder = choice.path.join("keys");
    for (statement, found) in &checked.accepted {
        for folder in saved
            .iter()
            .map(PathBuf::as_path)
            .chain([keys_folder.as_path()])
        {
            if let Err(error) = statements::save(folder, statement, found) {
                problems.push(format!(
                    "A key statement from the module list couldn't be saved in {}: {error}",
                    folder.display()
                ));
            }
        }
    }
    // Reloading the folder applies them to the installed modules too.
    let folder = load_folder(app, choice, Install::Nothing);
    let (offers, skipped) = catalog::check_entries(parsed.entries, &trust::current());
    problems.extend(
        skipped
            .into_iter()
            .map(|reason| format!("The module list's {reason}.")),
    );
    Ok((offers, problems, folder))
}

/// How each offer relates to what's installed.
fn summarize(offers: &[Offer], installed: &[ModuleSummary]) -> Vec<OfferSummary> {
    offers
        .iter()
        .map(|offer| {
            let module = &offer.manifest.module;
            let current = installed.iter().find(|summary| summary.id == module.id);
            let relation = match current {
                None => "new",
                Some(summary) => match compare_versions(&module.version, &summary.version) {
                    std::cmp::Ordering::Greater => "update",
                    std::cmp::Ordering::Equal => "installed",
                    std::cmp::Ordering::Less => "older",
                },
            };
            let major = |version: &str| version.split('.').next().map(str::to_string);
            OfferSummary {
                id: module.id.clone(),
                version: module.version.clone(),
                name: module.name.clone().unwrap_or_else(|| module.id.clone()),
                description: offer.entry.about.description.clone(),
                language: offer.entry.about.language.clone(),
                license: offer.entry.about.license.clone(),
                size: offer.entry.download.size,
                needs_newer_app: offer.availability == Availability::NeedsNewerApp,
                installed_version: current.map(|summary| summary.version.clone()),
                relation,
                text_verified: offer.pin.is_some(),
                text_changes: relation == "update"
                    && current
                        .is_some_and(|summary| major(&summary.version) != major(&module.version)),
            }
        })
        .collect()
}

fn download_key(id: &str, version: &str) -> String {
    format!("{id}-{version}")
}

/// Downloads an offered module into the modules folder, reporting progress
/// as `module-download-progress` events, and reloads the folder.
#[tauri::command]
pub async fn download_module<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    version: String,
    folder_state: State<'_, FolderState>,
    catalog_state: State<'_, CatalogState>,
) -> Result<FolderStatus> {
    let choice = current_folder(&folder_state)?;
    let offer = catalog_state
        .offers
        .lock()
        .map_err(|_| ModuleError::Task("the module list is unusable".into()))?
        .iter()
        .find(|offer| offer.manifest.module.id == id && offer.manifest.module.version == version)
        .cloned()
        .ok_or_else(|| ModuleError::UnknownModule(download_key(&id, &version)))?;
    if offer.availability == Availability::NeedsNewerApp {
        return Err(ModuleError::Unsupported(format!(
            "{id} {version} needs a newer version of Bible Nav"
        )));
    }
    let key = download_key(&id, &version);
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    catalog_state
        .cancels
        .lock()
        .map_err(|_| ModuleError::Task("the downloads are unusable".into()))?
        .insert(key.clone(), cancel.clone());

    let worker_app = app.clone();
    let folder = choice.path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut last = Instant::now();
        let mut report = |done: u64, total: u64| {
            // Often enough for a smooth bar, rarely enough not to flood.
            if done == total || last.elapsed().as_millis() >= 100 {
                last = Instant::now();
                let _ = worker_app.emit(
                    PROGRESS_EVENT,
                    Progress {
                        id: id.clone(),
                        version: version.clone(),
                        done,
                        total,
                    },
                );
            }
        };
        download::install(
            &offer,
            &folder,
            &Web::new(),
            &trust::current(),
            &mut report,
            &cancel,
        )
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()));
    // This download is over: forget its cancel flag (unless a new download
    // of the same module has already replaced it).
    if let Ok(mut cancels) = catalog_state.cancels.lock() {
        if cancels
            .get(&key)
            .is_some_and(|current| Arc::ptr_eq(current, &flag))
        {
            cancels.remove(&key);
        }
    }
    result??;
    reload(&app, &folder_state, choice).await
}

/// Stops a download in progress. It ends with a "cancelled" error.
#[tauri::command]
pub fn cancel_module_download(
    id: String,
    version: String,
    catalog_state: State<'_, CatalogState>,
) -> Result<()> {
    if let Some(flag) = catalog_state
        .cancels
        .lock()
        .map_err(|_| ModuleError::Task("the downloads are unusable".into()))?
        .get(&download_key(&id, &version))
    {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

/// Deletes an older version of a module, with its manifest and signatures.
/// Only files the folder status lists as older versions can be deleted, so
/// the version in use, and anything that isn't a module, never can be.
#[tauri::command]
pub async fn delete_module_version<R: Runtime>(
    app: AppHandle<R>,
    file: String,
    folder_state: State<'_, FolderState>,
) -> Result<FolderStatus> {
    let (choice, status) = folder_state.get()?;
    let choice =
        choice.ok_or_else(|| ModuleError::Unsupported("there's no modules folder".into()))?;
    let listed = status
        .modules
        .iter()
        .any(|module| module.older.iter().any(|older| older.file == file));
    if !listed || file.contains(std::path::is_separator) {
        return Err(ModuleError::UnknownModule(file));
    }
    let path = choice.path.join(&file);
    remove_module_files(&path)?;
    reload(&app, &folder_state, choice).await
}

fn remove_module_files(path: &Path) -> Result<()> {
    let files = sidecars(path);
    std::fs::remove_file(path)?;
    for sidecar in [files.manifest, files.signatures] {
        if sidecar.exists() {
            std::fs::remove_file(sidecar)?;
        }
    }
    Ok(())
}

/// Opens the modules folder in the system's file manager.
#[tauri::command]
pub fn open_modules_folder<R: Runtime>(
    app: AppHandle<R>,
    folder_state: State<'_, FolderState>,
) -> Result<()> {
    let choice = current_folder(&folder_state)?;
    app.opener()
        .open_path(choice.path.to_string_lossy(), None::<&str>)
        .map_err(|error| ModuleError::Unsupported(error.to_string()))
}

async fn reload<R: Runtime>(
    app: &AppHandle<R>,
    folder_state: &State<'_, FolderState>,
    choice: FolderChoice,
) -> Result<FolderStatus> {
    let worker_app = app.clone();
    let worker_choice = choice.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        load_folder(&worker_app, &worker_choice, Install::Nothing)
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?;
    folder_state.set(Some(choice), status.clone())?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::integrity::sidecars;
    use crate::modules::test_support::fixture_path;

    fn kjv_offer() -> Offer {
        let files = sidecars(&fixture_path());
        let catalog = serde_json::json!({
            "type": "bible-nav module catalog",
            "catalogVersion": 1,
            "modules": [{
                "manifest": std::fs::read_to_string(files.manifest).unwrap(),
                "signatures": std::fs::read_to_string(files.signatures).unwrap(),
                "download": {
                    "name": "eng_kjv-1.1.0.db.gz",
                    "size": 9305192,
                    "sha256": "0beb8bfe9eb6b76399b79a3624adf6bf181e6b6c33abebc2e6ac57ddc1a0fb36",
                    "urls": ["https://example.org/eng_kjv-1.1.0.db.gz"]
                },
                "about": { "license": "Public Domain" }
            }]
        });
        let parsed = catalog::parse(&serde_json::to_vec(&catalog).unwrap(), "test").unwrap();
        catalog::check_entries(parsed.entries, &trust::current())
            .0
            .remove(0)
    }

    fn installed(version: &str) -> ModuleSummary {
        ModuleSummary {
            id: "eng_kjv".into(),
            name: "King James Version (1769)".into(),
            version: version.into(),
            file: format!("eng_kjv-{version}.db"),
            description: String::new(),
            language: String::new(),
            license: String::new(),
            built_in: true,
            text_verified: true,
            older: Vec::new(),
        }
    }

    #[test]
    fn offers_say_how_they_relate_to_what_is_installed() {
        let offers = [kjv_offer()];
        let relation = |version: Option<&str>| {
            let modules: Vec<ModuleSummary> = version.into_iter().map(installed).collect();
            let summary = summarize(&offers, &modules).remove(0);
            (summary.relation, summary.text_changes)
        };
        assert_eq!(relation(None), ("new", false));
        assert_eq!(relation(Some("1.1.0")), ("installed", false));
        assert_eq!(relation(Some("1.0.0")), ("update", false));
        assert_eq!(relation(Some("0.9.0")), ("update", true));
        assert_eq!(relation(Some("2.0.0")), ("older", false));
        let summary = summarize(&offers, &[]).remove(0);
        assert_eq!(summary.name, "King James Version (1769)");
        assert!(summary.text_verified);
        assert_eq!(summary.size, 9305192);
    }

    #[test]
    fn deleting_removes_the_module_and_its_manifest_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let module = crate::modules::test_support::copy_module(&fixture_path(), dir.path());
        std::fs::write(dir.path().join("notes.txt"), "keep").unwrap();
        remove_module_files(&module).unwrap();
        let left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["notes.txt"]);
    }
}
