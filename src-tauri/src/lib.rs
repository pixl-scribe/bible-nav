// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// mod migrations;
// use crate::migrations::migration_factory::MigrationFactory;
mod modules;

use tauri::Manager;
use std::fs::{File, create_dir_all};
use std::io::copy;
use flate2::read::GzDecoder;

/**
 * Uncompresses and copies the default KJV module to the
 */
#[tauri::command]
fn init_database(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let app_data = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;

    let modules_dir = app_data.join("modules");
    create_dir_all(&modules_dir).unwrap();

    let db_path = modules_dir.join("eng_kjv.db");
    println!("The db path is: {}", db_path.display());

    if !db_path.exists() {
        // Transitional (removed in Phase 3): the bundled file is now named
        // with its version, eng_kjv-<version>.db.gz.
        let resources = app_handle.path().resolve("resources", tauri::path::BaseDirectory::Resource).map_err(|e| e.to_string())?;
        let resource_path = std::fs::read_dir(&resources)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .find(|path| path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("eng_kjv-") && n.ends_with(".db.gz")))
            .ok_or("no bundled eng_kjv module")?;

        println!("The resource path is: {}", resource_path.display());

        let gz_file = File::open(resource_path).unwrap();
        let mut decoder = GzDecoder::new(gz_file);
        let mut outfile = File::create(&db_path).unwrap();

        copy(&mut decoder, &mut outfile).unwrap();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // let migrations = MigrationFactory::get_migrations();
    // println!("{:?}", migrations);

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default()
          // .add_migrations("sqlite:biblenav.db", migrations)
          .build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
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
            modules::folder::modules_folder_status,
            modules::folder::dismiss_modules_folder_notice,
            modules::folder::change_modules_folder,
            modules::folder::rescan_modules,
        ])
        .setup(|app| {
            let app_handle = app.handle();
            // Choose the visible modules folder, install the bundled module
            // there, and open its modules for the page.
            modules::folder::initialize(app_handle);
            // The page still reads its text from the old hidden copy until it
            // switches to the module commands. Remove this then.
            init_database(app_handle).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
