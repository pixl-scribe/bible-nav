//! Shared test fixtures: the bundled KJV module, decompressed once into a
//! temporary folder that's deleted when the tests finish, and a test key for
//! signing made-up modules.

use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::db::ModuleDb;
use super::integrity::{key_id, sidecars, MANIFEST_TYPE};

static FIXTURE: OnceLock<(TempDir, PathBuf)> = OnceLock::new();

/// The bundled KJV module, `resources/eng_kjv-<version>.db.gz`.
pub fn bundled_kjv_gz() -> PathBuf {
    let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    fs::read_dir(&resources)
        .expect("resources folder")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("eng_kjv-") && name.ends_with(".db.gz"))
        })
        .expect("a bundled eng_kjv-<version>.db.gz")
}

/// The path to a decompressed copy of the bundled KJV module, named as it
/// is installed (`eng_kjv-<version>.db`), with its manifest and signatures.
pub fn fixture_path() -> PathBuf {
    FIXTURE
        .get_or_init(|| {
            let source = bundled_kjv_gz();
            let name = source.file_stem().expect("a file name").to_owned();
            let dir = tempfile::tempdir().expect("temporary folder");
            let path = dir.path().join(name);
            let mut decoder = GzDecoder::new(File::open(&source).expect("bundled module"));
            copy(
                &mut decoder,
                &mut File::create(&path).expect("fixture file"),
            )
            .expect("decompress");
            let bundled = sidecars(&source.with_extension(""));
            let fixture = sidecars(&path);
            fs::copy(bundled.manifest, fixture.manifest).expect("bundled manifest");
            fs::copy(bundled.signatures, fixture.signatures).expect("bundled signatures");
            (dir, path)
        })
        .1
        .clone()
}

/// The bundled KJV's file name, such as `eng_kjv-1.2.0.db`, so tests
/// don't depend on which version is bundled.
pub fn kjv_file_name() -> String {
    fixture_path()
        .file_name()
        .expect("a file name")
        .to_string_lossy()
        .into_owned()
}

/// The bundled KJV's version, such as `1.2.0`.
pub fn kjv_version() -> String {
    kjv_file_name()
        .trim_start_matches("eng_kjv-")
        .trim_end_matches(".db")
        .to_string()
}

/// The fixture, opened through the same checks the app uses.
pub fn kjv() -> ModuleDb {
    ModuleDb::open(&fixture_path()).expect("the bundled KJV should open")
}

/// Copies a module with its manifest and signatures into `folder`, keeping
/// their names. Returns the copy's path.
pub fn copy_module(module: &Path, folder: &Path) -> PathBuf {
    let target = folder.join(module.file_name().expect("a file name"));
    fs::copy(module, &target).expect("copy module");
    let (from, to) = (sidecars(module), sidecars(&target));
    fs::copy(from.manifest, to.manifest).expect("copy manifest");
    fs::copy(from.signatures, to.signatures).expect("copy signatures");
    target
}

/// A fixed key for signing made-up modules in tests. No build trusts it.
pub fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

/// Writes a manifest for the file at `module`, under its own name, and a
/// signature file signed with `key`.
pub fn sign_copy(module: &Path, id: &str, version: &str, format_version: i64, key: &SigningKey) {
    sign_copy_as(module, id, version, format_version, key, "Test edition");
}

/// Like `sign_copy`, naming the edition. The manifest's roots are computed
/// from the file, so it describes the text honestly; files that aren't
/// modules get roots of zeros.
pub fn sign_copy_as(
    module: &Path,
    id: &str,
    version: &str,
    format_version: i64,
    key: &SigningKey,
    edition: &str,
) {
    let roots =
        rusqlite::Connection::open_with_flags(module, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()
            .and_then(|conn| super::text_tree::roots(&conn).ok())
            .unwrap_or(super::text_tree::Roots {
                text: "0".repeat(64),
                content: "0".repeat(64),
            });
    sign_with_roots(module, id, version, format_version, key, edition, &roots);
}

/// Like `sign_copy_as`, with the roots given, true or not.
pub fn sign_with_roots(
    module: &Path,
    id: &str,
    version: &str,
    format_version: i64,
    key: &SigningKey,
    edition: &str,
    roots: &super::text_tree::Roots,
) {
    let bytes = fs::read(module).expect("module");
    let sha256: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let manifest = serde_json::to_string_pretty(&serde_json::json!({
        "type": MANIFEST_TYPE,
        "manifestVersion": 1,
        "module": { "id": id, "version": version, "formatVersion": format_version },
        "file": {
            "name": module.file_name().unwrap().to_string_lossy(),
            "size": bytes.len(),
            "sha256": sha256,
        },
        "text": {
            "treeVersion": 1,
            "edition": edition,
            "textRoot": roots.text,
            "contentRoot": roots.content,
        },
    }))
    .unwrap()
        + "\n";
    let signature = key.sign(manifest.as_bytes());
    let files = sidecars(module);
    fs::write(&files.manifest, &manifest).expect("manifest");
    fs::write(
        &files.signatures,
        format!(
            "ed25519 {} {}\n",
            key_id(&key.verifying_key()),
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
        ),
    )
    .expect("signatures");
}

/// Writes a small lexicon module, `<id>-<version>.db` in `dir`, with the
/// generator's tables (bible-nav-modules/docs/lexicon-format.md). Each
/// entry is (key, headword, body JSON); keys that are Strong's numbers get
/// a `strong` lookup, as the generator gives them.
pub fn write_lexicon(
    dir: &Path,
    id: &str,
    version: &str,
    entries: &[(&str, &str, &str)],
) -> PathBuf {
    let path = dir.join(format!("{id}-{version}.db"));
    let conn = rusqlite::Connection::open(&path).expect("lexicon");
    conn.execute_batch(
        "CREATE TABLE module (formatVersion INTEGER NOT NULL, name TEXT NOT NULL,
           abbreviation TEXT NOT NULL, description TEXT NOT NULL, moduleType TEXT NOT NULL,
           year INTEGER, fetchId TEXT, language TEXT NOT NULL, languageCode TEXT NOT NULL,
           license TEXT NOT NULL, version TEXT NOT NULL);
         CREATE TABLE entries (id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE,
           headword TEXT NOT NULL, headwordLang TEXT NOT NULL, homograph TEXT NOT NULL,
           translit TEXT NOT NULL, pronunciation TEXT NOT NULL, allCited INTEGER NOT NULL,
           body TEXT NOT NULL);
         CREATE TABLE lookups (kind TEXT NOT NULL, value TEXT NOT NULL,
           entryId INTEGER NOT NULL, PRIMARY KEY (kind, value, entryId)) WITHOUT ROWID;",
    )
    .expect("tables");
    conn.execute(
        "INSERT INTO module VALUES (1, 'Test Lexicon', 'TL', 'A test.', 'lexicon', NULL, ?1,
           'English', 'eng', 'Public domain.', ?2)",
        [id, version],
    )
    .expect("module row");
    for (index, (key, headword, body)) in entries.iter().enumerate() {
        let id = index as i64 + 1;
        conn.execute(
            "INSERT INTO entries VALUES (?1, ?2, ?3, 'hbo', '', 'tr', 'pron', 0, ?4)",
            rusqlite::params![id, key, headword, body],
        )
        .expect("entry");
        if super::lexicon::normalize_strong(key).as_deref() == Some(*key) {
            conn.execute(
                "INSERT INTO lookups VALUES ('strong', ?1, ?2)",
                rusqlite::params![key, id],
            )
            .expect("lookup");
        }
    }
    drop(conn);
    path
}

/// Signs a lexicon with a lexicon manifest whose roots are computed from
/// the file. `change` may alter the manifest first, to test what the app
/// refuses.
pub fn sign_lexicon(module: &Path, key: &SigningKey, change: impl FnOnce(&mut serde_json::Value)) {
    let conn =
        rusqlite::Connection::open_with_flags(module, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("lexicon");
    let roots = super::lexicon_tree::roots(&conn).expect("roots");
    let (id, version): (String, String) = conn
        .query_row("SELECT fetchId, version FROM module", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("module row");
    drop(conn);
    let bytes = fs::read(module).expect("module");
    let sha256: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut manifest = serde_json::json!({
        "type": MANIFEST_TYPE,
        "manifestVersion": 1,
        "module": { "id": id, "version": version, "moduleType": "lexicon", "formatVersion": 1 },
        "file": {
            "name": module.file_name().unwrap().to_string_lossy(),
            "size": bytes.len(),
            "sha256": sha256,
        },
        "entries": {
            "treeVersion": 1,
            "edition": "Test edition",
            "source": "Test source",
            "lookups": ["strong:H"],
            "count": roots.count,
            "keysRoot": roots.keys,
            "contentRoot": roots.content,
        },
    });
    change(&mut manifest);
    let manifest = serde_json::to_string_pretty(&manifest).unwrap() + "\n";
    let signature = key.sign(manifest.as_bytes());
    let files = sidecars(module);
    fs::write(&files.manifest, &manifest).expect("manifest");
    fs::write(
        &files.signatures,
        format!(
            "ed25519 {} {}\n",
            key_id(&key.verifying_key()),
            base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
        ),
    )
    .expect("signatures");
}
