// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod migrations;
use crate::migrations::migration_factory::MigrationFactory;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = MigrationFactory::get_migrations();
    // println!("{:?}", migrations);

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default()
          .add_migrations("sqlite:biblenav.db", migrations)
          .build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
