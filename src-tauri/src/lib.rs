// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// mod migrations;
// use crate::migrations::migration_factory::MigrationFactory;
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
        let resource_path = app_handle.path().resolve("resources/eng_kjv.db.gz", tauri::path::BaseDirectory::Resource).unwrap();

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
        .setup(|app| {
            let app_handle = app.handle();
            // Trigger extraction
            init_database(&app_handle).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
