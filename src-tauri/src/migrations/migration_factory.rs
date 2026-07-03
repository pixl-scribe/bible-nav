use tauri_plugin_sql::{Migration, MigrationKind};

pub struct MigrationFactory;

impl MigrationFactory {
  pub fn get_migrations() -> Vec<Migration> {
    vec![
      Migration {
        version: 1,
        description: "Initial Schema",
        sql: include_str!("./scripts/initial_schema.sql"),
        kind: MigrationKind::Up
      }
    ]
  }
}
