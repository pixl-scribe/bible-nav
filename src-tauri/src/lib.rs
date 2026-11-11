// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod modules;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        // Modules are opened into this registry by Rust code only. The page
        // reads them through the commands below. See src/modules/.
        .manage(modules::commands::ModuleRegistry::default())
        .manage(modules::folder::FolderState::default())
        .invoke_handler(tauri::generate_handler![
            modules::commands::module_info,
            modules::commands::books,
            modules::commands::paragraph_window,
            modules::commands::paragraph_window_at_sid,
            modules::commands::verse_window,
            modules::commands::verse_window_at_sid,
            modules::commands::ref_sids,
            modules::commands::ref_verses,
            modules::commands::chapter_notes,
            modules::folder::modules_folder_status,
            modules::folder::dismiss_modules_folder_notice,
            modules::folder::change_modules_folder,
            modules::folder::rescan_modules,
        ])
        .setup(|app| {
            // Choose the visible modules folder, install the bundled modules
            // there, and open its modules for the page.
            modules::folder::initialize(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
