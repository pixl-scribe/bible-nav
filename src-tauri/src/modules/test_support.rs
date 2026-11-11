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
