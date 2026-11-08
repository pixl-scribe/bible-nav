//! Errors from opening and reading Bible modules.

use std::fmt;

/// Everything that can go wrong when opening or reading a module.
///
/// Rust has no exceptions: functions that can fail return `Result<T>`, which
/// is either `Ok(value)` or `Err(ModuleError)`. The `?` operator returns early
/// with the error, a bit like rethrowing in TypeScript.
#[derive(Debug)]
pub enum ModuleError {
    /// SQLite reported an error, such as an unreadable or corrupt file.
    Sqlite(rusqlite::Error),
    /// The file isn't a module this app can read. Holds the reason.
    Unsupported(String),
    /// A verse reference wasn't found in the module.
    NotFound(String),
    /// No module with this id is open.
    UnknownModule(String),
    /// A background query task failed to finish.
    Task(String),
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleError::Sqlite(error) => write!(f, "database error: {error}"),
            ModuleError::Unsupported(reason) => write!(f, "unsupported module: {reason}"),
            ModuleError::NotFound(what) => write!(f, "not found: {what}"),
            ModuleError::UnknownModule(id) => write!(f, "no open module with id \"{id}\""),
            ModuleError::Task(reason) => write!(f, "query task failed: {reason}"),
        }
    }
}

impl std::error::Error for ModuleError {}

/// Lets `?` turn a SQLite error into a `ModuleError` automatically.
impl From<rusqlite::Error> for ModuleError {
    fn from(error: rusqlite::Error) -> Self {
        ModuleError::Sqlite(error)
    }
}

/// Tauri sends command errors to the web page as JSON, so they must be
/// serializable. The page receives the error's message as a string.
impl serde::Serialize for ModuleError {
    // The standard `Result` is spelled out here, because the `Result` alias
    // below has a fixed error type and serde needs its own.
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Shorthand used throughout the modules code.
pub type Result<T> = std::result::Result<T, ModuleError>;
