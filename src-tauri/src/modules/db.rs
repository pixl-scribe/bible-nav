//! Opens module files safely: only if a trusted key signed their manifest
//! and the file matches it, read-only, immutable, and only if they're in a
//! format this app understands.
//!
//! This is the only place that opens modules, so every module the app reads
//! has passed these checks.

use std::path::Path;

use ed25519_dalek::VerifyingKey;
use rusqlite::{Connection, OpenFlags};

use super::error::{ModuleError, Result};
use super::integrity::{self, Manifest, Sidecars};
use super::keys;

/// The module format version this app reads (`module.formatVersion`).
pub const SUPPORTED_FORMAT_VERSION: i64 = 2;

/// Tables every format-2 module has.
const REQUIRED_TABLES: [&str; 7] = [
    "module",
    "testaments",
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
    /// Opens a module, after checking it against the signed manifest beside
    /// it with the keys this build trusts.
    pub fn open(path: &Path) -> Result<Self> {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::open_checked(
            path,
            &integrity::sidecars(path),
            &file_name,
            &keys::trusted_module_keys(),
        )
    }

    /// Opens a module whose manifest may be elsewhere, such as a bundled
    /// module being unpacked under a temporary name. `file_name` is the name
    /// its manifest must give.
    ///
    /// The manifest, its signature, and the file's SHA-256 are checked
    /// before SQLite reads the file, so a damaged or altered file never
    /// reaches the database engine. Then it opens read-only and immutable:
    /// `immutable=1` tells SQLite the file never changes, so it never creates
    /// journal or lock files next to it. That keeps modules usable on
    /// read-only media such as a thumb drive.
    pub fn open_checked(
        path: &Path,
        sidecars: &Sidecars,
        file_name: &str,
        keys: &[VerifyingKey],
    ) -> Result<Self> {
        let manifest =
            integrity::verify(path, sidecars, file_name, keys, SUPPORTED_FORMAT_VERSION)?;
        let uri = format!("{}?immutable=1", file_uri(path));
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(uri, flags)?;
        check_format(&conn)?;
        check_identity(&conn, &manifest)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Rejects files that aren't format-2 modules, with a reason a person can read.
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

/// Checks the module says it's the module its manifest names.
fn check_identity(conn: &Connection, manifest: &Manifest) -> Result<()> {
    let (id, version): (Option<String>, String) =
        conn.query_row("SELECT fetchId, version FROM module", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
    if id.as_deref() != Some(manifest.module.id.as_str()) || version != manifest.module.version {
        return Err(ModuleError::Untrusted(format!(
            "it says it's {} {version}, but its manifest is for {} {}",
            id.unwrap_or_default(),
            manifest.module.id,
            manifest.module.version
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
    use crate::modules::test_support::{copy_module, fixture_path, kjv, sign_copy, test_key};

    /// Signs `path` with the test key as `id` `version`, then opens it.
    fn open_signed(path: &Path, id: &str, version: &str) -> Result<ModuleDb> {
        sign_copy(path, id, version, SUPPORTED_FORMAT_VERSION, &test_key());
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        ModuleDb::open_checked(
            path,
            &integrity::sidecars(path),
            &name,
            &[test_key().verifying_key()],
        )
    }

    fn fake_module(sql: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tst-1.0.0.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(sql).unwrap();
        drop(conn);
        (dir, path)
    }

    #[test]
    fn opens_the_bundled_kjv() {
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
    fn refuses_a_copy_with_one_byte_changed() {
        let dir = tempfile::tempdir().unwrap();
        let copy = copy_module(&fixture_path(), dir.path());
        let mut bytes = std::fs::read(&copy).unwrap();
        let middle = bytes.len() / 2;
        bytes[middle] ^= 1;
        std::fs::write(&copy, bytes).unwrap();
        let error = ModuleDb::open(&copy).err().expect("should be refused");
        assert!(error.to_string().contains("damaged or altered"), "{error}");
    }

    #[test]
    fn refuses_a_module_without_its_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join(fixture_path().file_name().unwrap());
        std::fs::copy(fixture_path(), &copy).unwrap();
        let error = ModuleDb::open(&copy).err().expect("should be refused");
        assert!(error.to_string().contains("is missing"), "{error}");
    }

    #[test]
    fn refuses_a_module_that_is_not_what_its_manifest_names() {
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("grc_tr-1.0.0.db");
        std::fs::copy(fixture_path(), &copy).unwrap();
        let error = open_signed(&copy, "grc_tr", "1.0.0")
            .err()
            .expect("should be refused");
        assert!(
            error
                .to_string()
                .contains("but its manifest is for grc_tr 1.0.0"),
            "{error}"
        );
    }

    #[test]
    fn opens_paths_with_spaces_and_uri_characters() {
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join("my Bible #1 ?copy%.db");
        std::fs::copy(fixture_path(), &odd).unwrap();
        let version: String = kjv()
            .conn()
            .query_row("SELECT version FROM module", [], |row| row.get(0))
            .unwrap();
        open_signed(&odd, "eng_kjv", &version).expect("odd file names should open");
    }

    #[test]
    fn rejects_old_format_modules() {
        let (_dir, path) = fake_module(
            "CREATE TABLE module (name TEXT);
             CREATE TABLE books (id INTEGER);
             CREATE TABLE chapters (id INTEGER);
             CREATE TABLE verses (id INTEGER);",
        );
        let error = open_signed(&path, "tst", "1.0.0")
            .err()
            .expect("old format should be rejected");
        assert!(error.to_string().contains("table is missing"), "{error}");
    }

    #[test]
    fn rejects_unknown_format_versions() {
        let (_dir, path) = fake_module(
            "CREATE TABLE module (formatVersion INTEGER, fetchId TEXT, version TEXT);
             INSERT INTO module VALUES (3, 'tst', '1.0.0');
             CREATE TABLE testaments (id INTEGER);
             CREATE TABLE books (id INTEGER);
             CREATE TABLE chapters (id INTEGER);
             CREATE TABLE paragraphs (id INTEGER);
             CREATE TABLE verses (id INTEGER);
             CREATE TABLE headings (id INTEGER);",
        );
        let error = open_signed(&path, "tst", "1.0.0")
            .err()
            .expect("version 3 should be rejected");
        assert!(
            error
                .to_string()
                .contains("format version 3 isn't supported"),
            "{error}"
        );
    }

    #[test]
    fn rejects_files_that_are_not_databases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes-1.0.0.db");
        std::fs::write(&path, "not a database, just some text that is long enough").unwrap();
        assert!(open_signed(&path, "notes", "1.0.0").is_err());
    }

    #[test]
    fn encodes_special_characters_in_uris() {
        let uri = file_uri(Path::new("/tmp/a b#c?d%e.db"));
        assert_eq!(uri, "file:/tmp/a%20b%23c%3Fd%25e.db");
    }
}
