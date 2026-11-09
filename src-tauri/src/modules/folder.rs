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
use super::store::{self, FolderChoice, InstallOutcome};

/// Settings kept in the app's own (hidden) data folder. Only the modules
/// themselves live in the visible folder.
const SETTINGS_FILE: &str = "modules.json";
const FOLDER_KEY: &str = "folder";
const NOTICE_KEY: &str = "noticeAcknowledged";
/// The app's bundled modules (`<id>-<version>.db.gz`) are in this resource folder.
const BUNDLED_FOLDER: &str = "resources";

/// A module the page can use, for display.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSummary {
    pub id: String,
    pub name: String,
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
    /// modules, in plain words.
    pub problems: Vec<String>,
}

/// The current folder and status, shared across commands.
#[derive(Default)]
pub struct FolderState {
    current: Mutex<(Option<FolderChoice>, FolderStatus)>,
}

impl FolderState {
    fn get(&self) -> Result<(Option<FolderChoice>, FolderStatus)> {
        self.current.lock().map(|guard| guard.clone()).map_err(|_| {
            ModuleError::Task("the folder state is unusable after an earlier failure".into())
        })
    }

    fn set(&self, choice: Option<FolderChoice>, status: FolderStatus) -> Result<()> {
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
            problems: vec![
                "Bible Nav couldn't find a Documents or home folder for its modules.".into(),
            ],
            ..FolderStatus::default()
        },
    };
    if let Err(error) = app.state::<FolderState>().set(choice, status) {
        eprintln!("{error}");
    }
}

/// Whether loading a folder also installs the bundled modules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Install {
    /// At startup and after changing folders.
    Bundled,
    /// When rescanning for modules copied in while the app is open.
    Nothing,
}

/// Installs the bundled modules if asked, then opens every module in the
/// folder into the registry, replacing any opened before.
fn load_folder<R: Runtime>(
    app: &AppHandle<R>,
    choice: &FolderChoice,
    install: Install,
) -> FolderStatus {
    let mut status = FolderStatus {
        folder: Some(choice.path.to_string_lossy().into_owned()),
        portable: choice.portable,
        show_notice: !choice.portable && read_setting(app, NOTICE_KEY) != Some(json!(true)),
        ..FolderStatus::default()
    };

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
                            status.problems.push(format!(
                                "A damaged module was set aside as {}, and a fresh copy installed.",
                                set_aside.display()
                            ))
                        }
                        Ok(_) => {}
                        Err(error) => status.problems.push(format!(
                            "The built-in module {} couldn't be installed: {error}",
                            bundled.display()
                        )),
                    }
                }
            }
            Err(error) => status
                .problems
                .push(format!("The built-in modules couldn't be found: {error}")),
        }
        if let Err(error) = store::write_readme(&choice.path) {
            status
                .problems
                .push(format!("The folder's README couldn't be written: {error}"));
        }
    }

    let registry = app.state::<ModuleRegistry>();
    if let Err(error) = registry.clear() {
        status.problems.push(error.to_string());
    }
    match store::scan(&choice.path) {
        Ok((modules, skipped)) => {
            for module in modules {
                match registry.open(&module.id, &module.path) {
                    Ok(()) => status.modules.push(ModuleSummary {
                        file: module
                            .path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        id: module.id,
                        name: module.name,
                        version: module.version,
                    }),
                    Err(error) => status
                        .problems
                        .push(format!("{}: {error}", module.path.display())),
                }
            }
            for skip in skipped {
                status.problems.push(format!(
                    "{} isn't a usable module ({})",
                    skip.path.display(),
                    skip.reason
                ));
            }
        }
        Err(error) => status
            .problems
            .push(format!("The modules folder couldn't be read: {error}")),
    }
    status
}

fn read_setting<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<serde_json::Value> {
    app.store(SETTINGS_FILE).ok()?.get(key)
}

fn write_setting<R: Runtime>(app: &AppHandle<R>, key: &str, value: serde_json::Value) {
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

/// Lets the person pick a new folder with the system folder picker, moves
/// the modules there, and reopens them. Returns `None` if they cancel.
#[tauri::command]
pub async fn change_modules_folder<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FolderState>,
) -> Result<Option<FolderStatus>> {
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
        let mut dialog = picker_app
            .dialog()
            .file()
            .set_title("Choose a folder for Bible modules");
        if let Some(start) = start {
            dialog = dialog.set_directory(start);
        }
        dialog.blocking_pick_folder()
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))?;

    let Some(picked) = picked else {
        return Ok(None);
    };
    let new_folder = picked
        .into_path()
        .map_err(|error| ModuleError::Unsupported(format!("that folder can't be used: {error}")))?;

    let worker_app = app.clone();
    let old_folder = current.map(|choice| choice.path);
    let (choice, status) = tauri::async_runtime::spawn_blocking(move || -> Result<_> {
        let report = match &old_folder {
            Some(old) => store::move_modules(old, &new_folder)?,
            None => store::MoveReport::default(),
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
            status.problems.push(format!(
                "These modules stayed in the old folder because the new folder already had files with the same names: {}",
                report.left_behind.join(", ")
            ));
        }
        Ok((choice, status))
    })
    .await
    .map_err(|error| ModuleError::Task(error.to_string()))??;

    state.set(Some(choice), status.clone())?;
    Ok(Some(status))
}
