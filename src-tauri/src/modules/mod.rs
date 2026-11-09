//! Reading Bible modules: read-only SQLite files made by bible-nav-modules.
//!
//! - `db`: opens and checks module files.
//! - `queries`: the queries the app runs, with bound parameters.
//! - `commands`: the Tauri commands the web page calls.
//! - `store`: the visible modules folder on disk.
//! - `folder`: connects the folder to the app, with its own commands.
//! - `error`: the error type they share.

pub mod commands;
pub mod db;
pub mod error;
pub mod folder;
pub mod queries;
pub mod store;

#[cfg(test)]
mod test_support;
