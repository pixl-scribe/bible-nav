//! Reading modules (Bibles and lexicons): read-only SQLite files made by
//! bible-nav-modules.
//!
//! - `db`: opens and checks module files.
//! - `integrity`: checks a module's signed manifest before it's opened.
//! - `keys`: the public keys and key statements built into the app.
//! - `statements`: key statements, which revoke and add primary keys.
//! - `trust`: which keys sign modules now, from the keys and statements.
//! - `pins`: pinned editions, whose text the app knows by its text root.
//! - `text_tree`: computes a module's text and content roots.
//! - `lexicon_tree`: computes a lexicon's keys and content roots.
//! - `lexicon`: queries on lexicon modules.
//! - `problem`: problems for the reader, as translatable message keys.
//! - `queries`: the queries the app runs, with bound parameters.
//! - `search`: word search over the full-text index.
//! - `catalog`: the list of modules to download, checked entry by entry.
//! - `download`: fetching the catalog and installing downloaded modules.
//! - `downloads`: the Modules dialog's commands for the catalog.
//! - `commands`: the Tauri commands the web page calls.
//! - `store`: the visible modules folder on disk.
//! - `folder`: connects the folder to the app, with its own commands.
//! - `error`: the error type they share.

pub mod catalog;
pub mod commands;
pub mod db;
pub mod download;
pub mod downloads;
pub mod error;
pub mod folder;
pub mod integrity;
pub mod keys;
pub mod lexicon;
pub mod lexicon_tree;
pub mod pins;
pub mod problem;
pub mod queries;
pub mod search;
pub mod statements;
pub mod store;
pub mod text_tree;
pub mod trust;

#[cfg(test)]
mod test_support;
