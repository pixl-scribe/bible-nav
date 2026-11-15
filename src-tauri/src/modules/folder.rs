//! Connects the modules folder (`store.rs`) to the app.
//!
//! At startup it picks the folder, installs the bundled module, and opens
//! every module it finds into the registry. It also gives the page three
//! commands: see the folder, dismiss the first-run notice, and change the
//! folder. The page never sees or sends file paths except to display the
//! folder's location; changing it goes through the system folder picker.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::json;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

use super::commands::ModuleRegistry;
use super::error::{ModuleError, Result};
use super::problem::Problem;
use super::store::{self, FolderChoice, InstallOutcome};
use super::trust;

/// Settings kept in the app's own (hidden) data folder. Only the modules
/// themselves live in the visible folder.
const SETTINGS_FILE: &str = "modules.json";
const FOLDER_KEY: &str = "folder";
const NOTICE_KEY: &str = "noticeAcknowledged";
pub(crate) const CATALOG_CHECKED_KEY: &str = "catalogCheckedAt";
/// The app's bundled modules (`<id>-<version>.db.gz`) are in this resource folder.
const BUNDLED_FOLDER: &str = "resources";
/// Key statements are kept in this folder inside the modules folder.
const KEYS_FOLDER: &str = "keys";

/// A module the page can use, for display.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub file: String,
    /// From the module itself, so covered by its signed file hash.
    pub description: String,
    pub language: String,
    pub license: String,
    /// The app carries this module (`resources`).
    pub built_in: bool,
    /// Its text matched a pinned edition: exactly the text this app knows.
    pub text_verified: bool,
    /// Older versions of it in the folder, newest first.
    pub older: Vec<OlderVersion>,
}

/// An older version of a module, kept beside the newest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlderVersion {
    pub version: String,
    pub file: String,
}

/// What the page needs to know about the modules folder.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatus {
    /// Where modules are kept, for display.
    pub folder: Option<String>,
    /// Modules are next to the app (a portable or thumb-drive copy).
    pub portable: bool,
    /// Show the first-run notice about where modules are stored.
    pub show_notice: bool,
    pub modules: Vec<ModuleSummary>,
    /// Problems a person should know about, such as files that aren't valid
    /// modules, as translatable messages.
    pub problems: Vec<Problem>,
    /// When the module list was last checked, in seconds since 1970, if
    /// ever. Portable copies don't record it.
    pub catalog_checked_at: Option<u64>,
}

/// The current folder and status, shared across commands.
#[derive(Default)]
pub struct FolderState {
    current: Mutex<(Option<FolderChoice>, FolderStatus)>,
    /// A folder picked but not yet switched to.
    picked: Mutex<Option<PathBuf>>,
}

impl FolderState {
    fn set_picked(&self, folder: Option<PathBuf>) -> Result<()> {
        *self
            .picked
            .lock()
            .map_err(|_| ModuleError::Task("the folder state is unusable".into()))? = folder;
        Ok(())
    }

    fn take_picked(&self) -> Result<Option<PathBuf>> {
        Ok(self
            .picked
            .lock()
            .map_err(|_| ModuleError::Task("the folder state is unusable".into()))?
            .take())
    }

    pub(crate) fn get(&self) -> Result<(Option<FolderChoice>, FolderStatus)> {
        self.current.lock().map(|guard| guard.clone()).map_err(|_| {
            ModuleError::Task("the folder state is unusable after an earlier failure".into())
        })
    }

    pub(crate) fn set(&self, choice: Option<FolderChoice>, status: FolderStatus) -> Result<()> {
        let mut guard = self.current.lock().map_err(|_| {
            ModuleError::Task("the folder state is unusable after an earlier failure".into())
        })?;
        *guard = (choice, status);
        Ok(())
    }
}

/// Runs at startup. Never fails: anything that goes wrong is reported in
/// the status for the page to show, so the app still opens.
pub fn initialize<R: Runtime>(app: &AppHandle<R>) {
    let saved = read_setting(app, FOLDER_KEY).and_then(|value| value.as_str().map(PathBuf::from));
    let app_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let choice = store::choose_folder(
        app_dir.as_deref(),
        saved.clone(),
        app.path().document_dir().ok(),
        app.path().home_dir().ok(),
    );

    let status = match &choice {
        Some(choice) => {
            // Remember the default, so the folder doesn't change if the
            // Documents folder moves or is renamed later.
            if !choice.portable && saved.is_none() {
                write_setting(app, FOLDER_KEY, json!(choice.path.to_string_lossy()));
            }
            load_folder(app, choice, Install::Bundled)
        }
        None => FolderStatus {
            problems: vec![Problem::new("problem.noFolder")],
            ..FolderStatus::default()
        },
    };
    if let Err(error) = app.state::<FolderState>().set(choice, status) {
        eprintln!("{error}");
    }
}

/// Whether loading a folder also installs the bundled modules.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Install {
    /// At startup and after changing folders.
    Bundled,
    /// When rescanning for modules copied in while the app is open.
    Nothing,
}

/// Installs the bundled modules if asked, then opens every module in the
/// folder into the registry, replacing any opened before.
pub(crate) fn load_folder<R: Runtime>(
    app: &AppHandle<R>,
    choice: &FolderChoice,
    install: Install,
) -> FolderStatus {
    let mut status = FolderStatus {
        folder: Some(choice.path.to_string_lossy().into_owned()),
        portable: choice.portable,
        show_notice: !choice.portable && read_setting(app, NOTICE_KEY) != Some(json!(true)),
        catalog_checked_at: read_setting(app, CATALOG_CHECKED_KEY).and_then(|value| value.as_u64()),
        ..FolderStatus::default()
    };

    // Key statements first, since they decide which keys the module checks
    // below trust. The app's own copies live in its data folder.
    let saved = app.path().app_data_dir().ok().map(|dir| dir.join("keys"));
    let (trust, problems) = trust::load(
        saved.as_deref(),
        &choice.path.join(KEYS_FOLDER),
        !choice.portable,
    );
    trust::set_current(trust);
    status.problems.extend(problems);

    // Portable folders may be read-only, so nothing is written to them.
    if !choice.portable && install == Install::Bundled {
        let installed = app
            .path()
            .resolve(BUNDLED_FOLDER, BaseDirectory::Resource)
            .map_err(|error| error.to_string())
            .and_then(|resources| {
                store::install_all_bundled(&resources, &choice.path).map_err(|e| e.to_string())
            });
        match installed {
            Ok(results) => {
                for (bundled, result) in results {
                    match result {
                        Ok(InstallOutcome::ReplacedInvalid { set_aside, .. }) => {
                            status.problems.push(
                                Problem::new("problem.damagedReplaced")
                                    .with("file", file_name(&set_aside)),
                            )
                        }
                        Ok(_) => {}
                        Err(error) => status.problems.push(
                            Problem::new("problem.builtInNotInstalled")
                                .with("file", file_name(&bundled))
                                .detail(error),
                        ),
                    }
                }
            }
            Err(error) => status
                .problems
                .push(Problem::new("problem.builtInNotFound").detail(error)),
        }
        if let Err(error) = store::write_readme(&choice.path) {
            status
                .problems
                .push(Problem::new("problem.readmeNotWritten").detail(error));
        }
    }

    let registry = app.state::<ModuleRegistry>();
    if let Err(error) = registry.clear() {
        status
            .problems
            .push(Problem::new("problem.internal").detail(error));
    }
    let built_in = bundled_file_names(app);
    match store::scan_all(&choice.path) {
        Ok(store::Scanned {
            newest,
            older,
            skipped,
        }) => {
            for module in newest {
                match registry.open(&module.id, &module.path) {
                    Ok(()) => status.modules.push(ModuleSummary {
                        file: file_name(&module.path),
                        built_in: built_in.contains(&file_name(&module.path)),
                        older: older
                            .iter()
                            .filter(|other| other.id == module.id)
                            .map(|other| OlderVersion {
                                version: other.version.clone(),
                                file: file_name(&other.path),
                            })
                            .collect(),
                        id: module.id,
                        name: module.name,
                        version: module.version,
                        description: module.description,
                        language: module.language,
                        license: module.license,
                        text_verified: module.text_verified,
                    }),
                    Err(error) => status.problems.push(
                        Problem::new("problem.notOpened")
                            .with("file", file_name(&module.path))
                            .detail(error),
                    ),
                }
            }
            for skip in skipped {
                status.problems.push(
                    Problem::new("problem.notAModule")
                        .with("file", file_name(&skip.path))
                        .detail(skip.reason),
                );
            }
        }
        Err(error) => status
            .problems
            .push(Problem::new("problem.folderUnreadable").detail(error)),
    }
    status
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The module files the app carries: `eng_kjv-1.1.0.db` for a bundled
/// `eng_kjv-1.1.0.db.gz`.
fn bundled_file_names<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    app.path()
        .resolve(BUNDLED_FOLDER, BaseDirectory::Resource)
        .ok()
        .and_then(|folder| std::fs::read_dir(folder).ok())
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .and_then(|name| name.strip_suffix(".gz"))
                        .filter(|name| name.ends_with(".db"))
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn read_setting<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<serde_json::Value> {
    app.store(SETTINGS_FILE).ok()?.get(key)
}

pub(crate) fn write_setting<R: Runtime>(app: &AppHandle<R>, key: &str, value: serde_json::Value) {
    match app.store(SETTINGS_FILE) {
        Ok(settings) => {
            settings.set(key, value);
            if let Err(error) = settings.save() {
                eprintln!("couldn't save {SETTINGS_FILE}: {error}");
            }
        }
        Err(error) => eprintln!("couldn't open {SETTINGS_FILE}: {error}"),
    }
}

/// Where modules are kept, which modules were found, and any problems.
#[tauri::command]
pub fn modules_folder_status(state: State<'_, FolderState>) -> Result<FolderStatus> {
    Ok(state.get()?.1)
}

/// Hides the first-run notice for good.
#[tauri::command]
pub fn dismiss_modules_folder_notice<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FolderState>,
) -> Result<FolderStatus> {
    write_setting(&app, NOTICE_KEY, json!(true));
    let (choice, mut status) = state.get()?;
    status.show_notice = false;
    state.set(choice, status.clone())?;
    Ok(status)
}

/// Looks for modules again, so ones copied into the folder while the app is
/// open become available. Installs nothing, so it's quick enough to run
/// whenever the window regains focus.
#[tauri::command]
pub async fn rescan_modules<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FolderState>,
) -> Result<FolderStatus> {
    let (choice, current) = state.get()?;
    let Some(choice) = choice else {
        return Ok(current);
    };
    let worker_app = app.clone();
    let worker_choice = choice.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        load_folder(&worker_app, &worker_choice, Install::Nothing)
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?;
    state.set(Some(choice), status.clone())?;
    Ok(status)
}

/// Lets the person pick a new modules folder with the system folder
/// picker, whose `title` the page gives (translated). The folder is kept
/// here, never sent by the page, until `change_modules_folder` uses it.
/// Returns the folder for display, or `None` if they cancel.
#[tauri::command]
pub async fn pick_modules_folder<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FolderState>,
    title: String,
) -> Result<Option<String>> {
    let (current, _) = state.get()?;
    if current.as_ref().is_some_and(|choice| choice.portable) {
        return Err(ModuleError::Unsupported(
            "a portable copy keeps its modules next to the app".into(),
        ));
    }

    // The picker waits for the person, so it runs on a background thread
    // rather than freezing the window.
    let picker_app = app.clone();
    let start = current.as_ref().map(|choice| choice.path.clone());
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = picker_app.dialog().file().set_title(title);
        if let Some(start) = start {
            dialog = dialog.set_directory(start);
        }
        dialog.blocking_pick_folder()
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?;

    let Some(picked) = picked else {
        state.set_picked(None)?;
        return Ok(None);
    };
    let folder = picked
        .into_path()
        .map_err(|error| ModuleError::Unsupported(format!("that folder can't be used: {error}")))?;
    let display = folder.to_string_lossy().into_owned();
    state.set_picked(Some(folder))?;
    Ok(Some(display))
}

/// Switches to the folder last picked with `pick_modules_folder`. With
/// `move_modules`, the current modules move there first, each checked in
/// its new place before the original is removed; otherwise they stay where
/// they are. Either way the built-in modules and saved key statements are
/// written to the new folder, and its modules are opened.
#[tauri::command]
pub async fn change_modules_folder<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FolderState>,
    move_modules: bool,
) -> Result<FolderStatus> {
    let (current, _) = state.get()?;
    let new_folder = state
        .take_picked()?
        .ok_or_else(|| ModuleError::Unsupported("no new folder was picked".into()))?;

    let worker_app = app.clone();
    let old_folder = current.map(|choice| choice.path);
    let (choice, status) = tauri::async_runtime::spawn_blocking(move || -> Result<_> {
        let report = match (&old_folder, move_modules) {
            (Some(old), true) => store::move_modules(old, &new_folder)?,
            _ => store::MoveReport::default(),
        };
        write_setting(&worker_app, FOLDER_KEY, json!(new_folder.to_string_lossy()));
        // Choosing a folder counts as having seen the notice.
        write_setting(&worker_app, NOTICE_KEY, json!(true));
        let choice = FolderChoice {
            path: new_folder,
            portable: false,
        };
        let mut status = load_folder(&worker_app, &choice, Install::Bundled);
        if !report.left_behind.is_empty() {
            status.problems.push(
                Problem::new("problem.leftBehind").with("files", report.left_behind.join(", ")),
            );
        }
        Ok((choice, status))
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))??;

    state.set(Some(choice), status.clone())?;
    Ok(status)
}
