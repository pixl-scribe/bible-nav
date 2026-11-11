//! Checks a module's signed manifest, before SQLite opens the module.
//!
//! Each module `<id>-<version>.db` comes with `<id>-<version>.manifest.json`,
//! which records its identity and the SHA-256 of the whole file, and
//! `<id>-<version>.manifest.sig`, which holds Ed25519 signatures of the
//! manifest's exact bytes. See bible-nav-modules/docs/integrity-spec.md.
//!
//! A module is trusted only if a key this app trusts signed its manifest
//! (`keys.rs`), and the file is byte for byte the one the manifest describes.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::error::{ModuleError, Result};

/// The first field of every manifest, so its signed bytes say what they are.
pub const MANIFEST_TYPE: &str = "bible-nav module manifest";

/// The parts of a manifest the app uses. Other fields are allowed and
/// ignored here; they're still covered by the signature.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "type")]
    pub kind: String,
    pub manifest_version: u32,
    pub module: ManifestModule,
    pub file: ManifestFile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestModule {
    pub id: String,
    pub version: String,
    pub format_version: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

/// Where a module's manifest and signatures are, next to the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecars {
    pub manifest: PathBuf,
    pub signatures: PathBuf,
}

/// The manifest and signature files beside a module:
/// `eng_kjv-1.1.0.db` has `eng_kjv-1.1.0.manifest.json` and
/// `eng_kjv-1.1.0.manifest.sig`.
pub fn sidecars(module: &Path) -> Sidecars {
    let stem = module
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    Sidecars {
        manifest: module.with_file_name(format!("{stem}.manifest.json")),
        signatures: module.with_file_name(format!("{stem}.manifest.sig")),
    }
}

/// A key's id: the first 16 hexadecimal digits of the SHA-256 of its 32 raw
/// bytes, as in the signature file.
pub fn key_id(key: &VerifyingKey) -> String {
    hex(&Sha256::digest(key.as_bytes()))[..16].to_string()
}

/// Checks a module file against its signed manifest:
///
/// 1. a key in `keys` signed the manifest;
/// 2. the manifest is one this app understands, for a format it reads;
/// 3. the file's name, size, and SHA-256 are those in the manifest.
///
/// `file_name` is the name the module has (or will have once installed),
/// which may differ from `module` while it's being unpacked.
pub fn verify(
    module: &Path,
    sidecars: &Sidecars,
    file_name: &str,
    keys: &[VerifyingKey],
    format_version: i64,
) -> Result<Manifest> {
    let manifest_bytes = fs::read(&sidecars.manifest).map_err(|_| {
        untrusted(format!(
            "its manifest ({}) is missing",
            display_name(&sidecars.manifest)
        ))
    })?;
    let signatures = fs::read_to_string(&sidecars.signatures).map_err(|_| {
        untrusted(format!(
            "its signature file ({}) is missing",
            display_name(&sidecars.signatures)
        ))
    })?;
    check_signatures(&manifest_bytes, &signatures, keys)?;

    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| untrusted(format!("its manifest can't be read: {error}")))?;
    if manifest.kind != MANIFEST_TYPE || manifest.manifest_version != 1 {
        return Err(untrusted(
            "its manifest is of a kind this app doesn't know".into(),
        ));
    }
    if manifest.module.format_version != format_version {
        return Err(ModuleError::Unsupported(format!(
            "format version {} isn't supported (this app reads version {format_version})",
            manifest.module.format_version
        )));
    }
    if manifest.file.name != file_name {
        return Err(untrusted(format!(
            "its manifest is for {}, not {file_name}",
            manifest.file.name
        )));
    }
    check_file(module, manifest.file.size, &manifest.file.sha256)?;
    Ok(manifest)
}

/// Succeeds if any line of the signature file is a valid signature of the
/// manifest by one of `keys`. Lines by other keys, or in a form this app
/// doesn't know, are ignored: a manifest may carry several signatures.
fn check_signatures(manifest: &[u8], signatures: &str, keys: &[VerifyingKey]) -> Result<()> {
    for line in signatures.lines() {
        let mut parts = line.split(' ');
        let (Some("ed25519"), Some(id), Some(encoded), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
            continue;
        };
        let Ok(signature) = Signature::from_slice(&bytes) else {
            continue;
        };
        let valid = keys
            .iter()
            .filter(|key| key_id(key) == id)
            .any(|key| key.verify_strict(manifest, &signature).is_ok());
        if valid {
            return Ok(());
        }
    }
    Err(untrusted("it isn't signed by a key this app trusts".into()))
}

/// Files already found to match their manifest during this run, with the
/// size and modification time they had then.
type CheckedFiles = HashMap<PathBuf, (u64, Option<SystemTime>, String)>;

fn checked_files() -> &'static Mutex<CheckedFiles> {
    static CHECKED: OnceLock<Mutex<CheckedFiles>> = OnceLock::new();
    CHECKED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Checks a file's size and SHA-256.
///
/// Hashing a whole Bible takes a moment, and the folder is scanned again
/// whenever the window regains focus. So a file that matched earlier in
/// this run, and still has the same size and modification time, isn't
/// hashed again. Anything that can change a file without changing those
/// could also change the app itself, so this doesn't weaken the check.
fn check_file(module: &Path, size: u64, sha256: &str) -> Result<()> {
    let metadata = fs::metadata(module)?;
    if metadata.len() != size {
        return Err(damaged());
    }
    let modified = metadata.modified().ok();
    let key = module
        .canonicalize()
        .unwrap_or_else(|_| module.to_path_buf());
    let wanted = sha256.to_ascii_lowercase();
    if let Ok(checked) = checked_files().lock() {
        if checked.get(&key) == Some(&(size, modified, wanted.clone())) {
            return Ok(());
        }
    }

    let mut hasher = Sha256::new();
    let mut file = File::open(module)?;
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if hex(&hasher.finalize()) != wanted {
        return Err(damaged());
    }
    if let Ok(mut checked) = checked_files().lock() {
        checked.insert(key, (size, modified, wanted));
    }
    Ok(())
}

fn damaged() -> ModuleError {
    untrusted("the file doesn't match its signed manifest, so it may be damaged or altered".into())
}

fn untrusted(reason: String) -> ModuleError {
    ModuleError::Untrusted(reason)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::{sign_copy, test_key};
    use ed25519_dalek::Signer;

    const FORMAT: i64 = 2;

    fn module_with(contents: &[u8], name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        fs::write(&path, contents).unwrap();
        (dir, path)
    }

    #[test]
    fn finds_the_manifest_and_signatures_beside_a_module() {
        let found = sidecars(Path::new("/m/eng_kjv-1.1.0.db"));
        assert_eq!(
            found.manifest,
            PathBuf::from("/m/eng_kjv-1.1.0.manifest.json")
        );
        assert_eq!(
            found.signatures,
            PathBuf::from("/m/eng_kjv-1.1.0.manifest.sig")
        );
    }

    #[test]
    fn accepts_a_module_signed_by_a_trusted_key() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", FORMAT, &test_key());
        let manifest = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap();
        assert_eq!(manifest.module.id, "tst");
    }

    #[test]
    fn rejects_a_module_signed_by_another_key() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", FORMAT, &test_key());
        let other = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[other.verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("isn't signed by a key"),
            "{error}"
        );
    }

    #[test]
    fn rejects_a_changed_byte_in_the_module() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", FORMAT, &test_key());
        fs::write(&path, b"module bytez").unwrap();
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(error.to_string().contains("damaged or altered"), "{error}");
    }

    #[test]
    fn rejects_a_changed_manifest() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", FORMAT, &test_key());
        let manifest = sidecars(&path).manifest;
        let changed = fs::read_to_string(&manifest)
            .unwrap()
            .replace("\"1.0.0\"", "\"9.0.0\"");
        fs::write(&manifest, changed).unwrap();
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("isn't signed by a key"),
            "{error}"
        );
    }

    #[test]
    fn rejects_a_renamed_module() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", FORMAT, &test_key());
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-2.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(error.to_string().contains("manifest is for"), "{error}");
    }

    #[test]
    fn rejects_a_module_without_its_manifest() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("manifest (tst-1.0.0.manifest.json) is missing"),
            "{error}"
        );
    }

    #[test]
    fn rejects_a_format_this_app_does_not_read() {
        let (_dir, path) = module_with(b"module bytes", "tst-1.0.0.db");
        sign_copy(&path, "tst", "1.0.0", 3, &test_key());
        let error = verify(
            &path,
            &sidecars(&path),
            "tst-1.0.0.db",
            &[test_key().verifying_key()],
            FORMAT,
        )
        .unwrap_err();
        assert!(error.to_string().contains("format version 3"), "{error}");
    }

    #[test]
    fn ignores_signature_lines_it_cannot_use() {
        let manifest = b"{\"type\": \"bible-nav module manifest\"}\n";
        let key = test_key();
        let good = format!(
            "ed25519 {} {}\n",
            key_id(&key.verifying_key()),
            base64::engine::general_purpose::STANDARD.encode(key.sign(manifest).to_bytes())
        );
        let lines =
            format!("rsa 0123 AAAA\ned25519 nonsense\ned25519 0123456789abcdef !!!\n{good}");
        assert!(check_signatures(manifest, &lines, &[key.verifying_key()]).is_ok());
        assert!(check_signatures(manifest, "rsa 0123 AAAA\n", &[key.verifying_key()]).is_err());
    }

    #[test]
    fn the_bundled_module_is_signed_with_the_development_key() {
        let path = crate::modules::test_support::fixture_path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        verify(
            &path,
            &sidecars(&path),
            &name,
            &crate::modules::keys::trusted_module_keys(),
            FORMAT,
        )
        .expect("the bundled KJV should match its manifest");
    }
}
