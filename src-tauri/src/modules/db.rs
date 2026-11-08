//! Opens module files safely: read-only, immutable, and only if they're in a
//! format this app understands.
//!
//! This is the only place that opens modules, so every module the app reads
//! has passed these checks. Verifying a module's integrity (hashes and
//! signatures) will be added here too.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::error::{ModuleError, Result};

/// The module format version this app reads (`module.formatVersion`).
pub const SUPPORTED_FORMAT_VERSION: i64 = 1;

/// Tables every format-1 module has.
const REQUIRED_TABLES: [&str; 6] = [
    "module",
    "books",
    "chapters",
    "paragraphs",
    "verses",
    "headings",
];

/// An open, checked module.
///
/// The connection is private, so other code can only reach it through
/// `conn()`, which hands out a shared reference (`&Connection`). That lets
/// callers run queries but not close or replace the connection.
pub struct ModuleDb {
    conn: Connection,
}

impl ModuleDb {
    /// Opens a module read-only and immutable, then checks its format.
    ///
    /// `immutable=1` tells SQLite the file never changes, so it never creates
    /// journal or lock files next to it. That keeps modules usable on
    /// read-only media such as a thumb drive.
    pub fn open(path: &Path) -> Result<Self> {
        let uri = format!("{}?immutable=1", file_uri(path));
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(uri, flags)?;
        check_format(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Rejects files that aren't format-1 modules, with a reason a person can read.
fn check_format(conn: &Connection) -> Result<()> {
    for table in REQUIRED_TABLES {
        let exists: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(ModuleError::Unsupported(format!(
                "the {table} table is missing"
            )));
        }
    }

    let has_version: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_table_info('module') WHERE name = 'formatVersion')",
        [],
        |row| row.get(0),
    )?;
    if !has_version {
        return Err(ModuleError::Unsupported(
            "it has no format version, so it predates format 1".into(),
        ));
    }

    let version: i64 = conn.query_row("SELECT formatVersion FROM module", [], |row| row.get(0))?;
    if version != SUPPORTED_FORMAT_VERSION {
        return Err(ModuleError::Unsupported(format!(
            "format version {version} isn't supported (this app reads version {SUPPORTED_FORMAT_VERSION})"
        )));
    }
    Ok(())
}

/// Turns a file path into a SQLite URI, so that options like `immutable` can
/// be added. Anything with a special meaning in a URI, such as a space, `#`,
/// or `?`, is percent-encoded so it stays part of the path.
fn file_uri(path: &Path) -> String {
    let bytes = path.as_os_str().as_encoded_bytes();
    let mut uri = String::from("file:");
    let mut rest = bytes;

    // Windows drive paths like C:\Bible become file:/C:/Bible.
    if cfg!(windows) && bytes.len() >= 2 && bytes[1] == b':' {
        uri.push('/');
        uri.push(bytes[0] as char);
        uri.push(':');
        rest = &bytes[2..];
    }

    for &byte in rest {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            b'\\' if cfg!(windows) => uri.push('/'),
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::{fixture_path, kjv};

    #[test]
    fn opens_the_corrected_kjv() {
        let db = kjv();
        let version: i64 = db
            .conn()
            .query_row("SELECT formatVersion FROM module", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SUPPORTED_FORMAT_VERSION);
    }

    #[test]
    fn is_read_only() {
        let db = kjv();
        let result = db.conn().execute("DELETE FROM verses", []);
        assert!(result.is_err(), "writing to a module should fail");
    }

    #[test]
    fn opens_paths_with_spaces_and_uri_characters() {
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join("my Bible #1 ?copy%.db");
        std::fs::copy(fixture_path(), &odd).unwrap();
        ModuleDb::open(&odd).expect("odd file names should open");
    }

    #[test]
    fn rejects_old_format_modules() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE module (name TEXT);
             CREATE TABLE books (id INTEGER);
             CREATE TABLE chapters (id INTEGER);
             CREATE TABLE verses (id INTEGER);",
        )
        .unwrap();
        drop(conn);

        let error = ModuleDb::open(&path)
            .err()
            .expect("old format should be rejected");
        assert!(
            error.to_string().contains("paragraphs table is missing"),
            "{error}"
        );
    }

    #[test]
    fn rejects_unknown_format_versions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE module (formatVersion INTEGER);
             INSERT INTO module VALUES (2);
             CREATE TABLE books (id INTEGER);
             CREATE TABLE chapters (id INTEGER);
             CREATE TABLE paragraphs (id INTEGER);
             CREATE TABLE verses (id INTEGER);
             CREATE TABLE headings (id INTEGER);",
        )
        .unwrap();
        drop(conn);

        let error = ModuleDb::open(&path)
            .err()
            .expect("version 2 should be rejected");
        assert!(
            error
                .to_string()
                .contains("format version 2 isn't supported"),
            "{error}"
        );
    }

    #[test]
    fn rejects_files_that_are_not_databases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.db");
        std::fs::write(&path, "not a database, just some text that is long enough").unwrap();
        assert!(ModuleDb::open(&path).is_err());
    }

    #[test]
    fn encodes_special_characters_in_uris() {
        let uri = file_uri(Path::new("/tmp/a b#c?d%e.db"));
        assert_eq!(uri, "file:/tmp/a%20b%23c%3Fd%25e.db");
    }
}
