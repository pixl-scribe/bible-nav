//! Key statements: signed by a backup key, they revoke primary keys and add
//! their replacements. See "Key statements" in
//! bible-nav-modules/docs/integrity-spec.md, which this follows rule by rule.
//!
//! A statement is `bible-nav-keys-<sequence>.json`, whose exact bytes are
//! signed, and `bible-nav-keys-<sequence>.sig`, with the same lines as a
//! manifest's `.sig` file. Statements are read from three places: those
//! built into the app, those the app saved earlier, and the `keys` folder in
//! the modules folder. Every one accepted is saved, so it's never forgotten
//! and travels on with copies of the modules folder.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use ed25519_dalek::VerifyingKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::integrity::{key_id, signer};

/// The first field of every statement, so its signed bytes say what they are.
pub const STATEMENT_TYPE: &str = "bible-nav key statement";

/// An accepted statement: what it changes, after every check has passed.
#[derive(Debug, Clone)]
pub struct Statement {
    pub sequence: u64,
    /// Ids of the primary keys it revokes.
    pub revoke: Vec<String>,
    /// The primary keys it adds.
    pub add: Vec<VerifyingKey>,
    /// SHA-256 (lowercase hex) of manifests that stay trusted although the
    /// key that signed them is revoked.
    pub keep_modules: Vec<String>,
}

// The statement as written. `deny_unknown_fields` and the absence of
// defaults make serde reject any field that's missing or extra, as the spec
// requires: a statement is accepted only if it has exactly these fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Written {
    #[serde(rename = "type")]
    kind: String,
    statement_version: u64,
    sequence: u64,
    issued_at: String,
    revoke: Vec<WrittenRevoke>,
    add: Vec<WrittenAdd>,
    keep_modules: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WrittenRevoke {
    key_id: String,
    // Read only to check it's text: the app treats every reason alike.
    #[allow(dead_code)]
    reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WrittenAdd {
    key_id: String,
    role: String,
    public_key: String,
}

/// Checks a statement completely and returns what it changes, or the reason
/// it's ignored. `backup_keys` are the backup keys built into the app: the
/// only keys that may sign a statement, and keys no statement may touch.
pub fn check(
    bytes: &[u8],
    signatures: &str,
    backup_keys: &[VerifyingKey],
) -> std::result::Result<Statement, String> {
    let written: Written =
        serde_json::from_slice(bytes).map_err(|error| format!("it can't be read: {error}"))?;
    if written.kind != STATEMENT_TYPE || written.statement_version != 1 {
        return Err("it's a kind of statement this app doesn't know".into());
    }
    if written.sequence < 1 {
        return Err("its sequence must be a whole number from 1".into());
    }
    if !is_date(&written.issued_at) {
        return Err("its issuedAt must be a date written YYYY-MM-DD".into());
    }
    if written.revoke.is_empty() && written.add.is_empty() {
        return Err("it neither revokes nor adds a key".into());
    }

    let mut revoke = Vec::new();
    for entry in &written.revoke {
        if !is_hex(&entry.key_id, 16) {
            return Err("a revoked keyId isn't 16 lowercase hex digits".into());
        }
        revoke.push(entry.key_id.clone());
    }
    let mut add = Vec::new();
    for entry in &written.add {
        if entry.role != "primary" {
            return Err("it may only add primary keys".into());
        }
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&entry.public_key)
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or("an added publicKey isn't 32 bytes in Base64")?;
        let key = VerifyingKey::from_bytes(&raw)
            .map_err(|_| "an added publicKey isn't a valid Ed25519 key")?;
        if key_id(&key) != entry.key_id {
            return Err(format!(
                "an added keyId ({}) doesn't match its publicKey ({})",
                entry.key_id,
                key_id(&key)
            ));
        }
        add.push(key);
    }
    for hash in &written.keep_modules {
        if !is_hex(hash, 64) {
            return Err("a keepModules entry isn't 64 lowercase hex digits".into());
        }
    }

    let backup_ids: HashSet<String> = backup_keys.iter().map(key_id).collect();
    let added_ids: HashSet<String> = add.iter().map(key_id).collect();
    for id in revoke.iter().chain(added_ids.iter()) {
        if backup_ids.contains(id) {
            return Err(format!("it revokes or adds a backup key ({id})"));
        }
    }
    if let Some(id) = revoke.iter().find(|id| added_ids.contains(*id)) {
        return Err(format!("it both revokes and adds key {id}"));
    }

    // The signature is checked last, over the exact bytes, so a statement
    // that breaks a rule is reported as such even when it's well signed.
    if signer(bytes, signatures, backup_keys).is_none() {
        return Err("it isn't signed by a backup key this app trusts".into());
    }
    Ok(Statement {
        sequence: written.sequence,
        revoke,
        add,
        keep_modules: written.keep_modules,
    })
}

/// A statement found somewhere, not yet checked.
#[derive(Debug, Clone)]
pub struct Found {
    /// Where it came from, for messages.
    pub origin: String,
    pub bytes: Vec<u8>,
    pub signatures: String,
}

/// Every `bible-nav-keys-*.json` in `folder`, with its `.sig` file (empty
/// if missing, so the statement is reported as unsigned). A folder that
/// doesn't exist holds none.
pub fn read_folder(folder: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("bible-nav-keys-") && name.ends_with(".json"))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let bytes = fs::read(&path).ok()?;
            let signatures = fs::read_to_string(path.with_extension("sig")).unwrap_or_default();
            Some(Found {
                origin: path.display().to_string(),
                bytes,
                signatures,
            })
        })
        .collect()
}

/// What checking a set of found statements gave.
#[derive(Debug, Default)]
pub struct Checked {
    /// Accepted statements with their files, each once however many copies
    /// were found.
    pub accepted: Vec<(Statement, Found)>,
    /// Statements ignored, with the reason, for the reader to see: one that
    /// fails to check may be a forgery.
    pub problems: Vec<String>,
}

/// Checks every found statement, keeping one copy of each.
pub fn check_all(found: Vec<Found>, backup_keys: &[VerifyingKey]) -> Checked {
    let mut checked = Checked::default();
    let mut seen = HashSet::new();
    for statement in found {
        let hash = sha256_hex(&statement.bytes);
        if seen.contains(&hash) {
            continue;
        }
        match check(&statement.bytes, &statement.signatures, backup_keys) {
            Ok(accepted) => {
                seen.insert(hash);
                checked.accepted.push((accepted, statement));
            }
            Err(reason) => checked.problems.push(format!(
                "The key statement {} was ignored: {reason}.",
                statement.origin
            )),
        }
    }
    checked
}

/// Saves an accepted statement in `folder` unless an identical copy is
/// already there, as `bible-nav-keys-<sequence>.json` and `.sig`. If that
/// name holds a different statement, the name gets the start of its SHA-256.
pub fn save(folder: &Path, statement: &Statement, found: &Found) -> std::io::Result<()> {
    let hash = sha256_hex(&found.bytes);
    if read_folder(folder)
        .iter()
        .any(|existing| sha256_hex(&existing.bytes) == hash)
    {
        return Ok(());
    }
    fs::create_dir_all(folder)?;
    let mut base = folder.join(format!("bible-nav-keys-{}", statement.sequence));
    if base.with_extension("json").exists() {
        base = folder.join(format!(
            "bible-nav-keys-{}-{}",
            statement.sequence,
            &hash[..8]
        ));
    }
    // The signatures go first, so a statement never appears without them.
    fs::write(base.with_extension("sig"), &found.signatures)?;
    fs::write(base.with_extension("json"), &found.bytes)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `YYYY-MM-DD`, digits only; the spec asks for the form, not a real date.
fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keys::{backup_keys, development_backup_key};

    /// The vectors are signed with the development backup key, which only
    /// development builds trust, so tests name it explicitly.
    fn test_backup_keys() -> Vec<VerifyingKey> {
        vec![development_backup_key()]
    }

    /// The shared test vectors from bible-nav-modules, copied into
    /// src-tauri/test-vectors/key-statements. Both implementations must
    /// accept and reject exactly the statements expected.json says.
    fn vectors() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-vectors/key-statements")
    }

    fn vector(name: &str) -> Found {
        let base = vectors().join(name);
        Found {
            origin: name.into(),
            bytes: fs::read(base.with_extension("json")).unwrap(),
            signatures: fs::read_to_string(base.with_extension("sig")).unwrap(),
        }
    }

    #[test]
    fn the_shared_test_vectors_are_accepted_and_rejected_as_expected() {
        let expected: serde_json::Value =
            serde_json::from_slice(&fs::read(vectors().join("expected.json")).unwrap()).unwrap();
        let cases = expected["cases"].as_object().unwrap();
        assert_eq!(cases.len(), 15);
        for (name, outcome) in cases {
            let found = vector(name);
            let result = check(&found.bytes, &found.signatures, &test_backup_keys());
            assert_eq!(
                result.is_ok(),
                outcome["accept"].as_bool().unwrap(),
                "{name}: {result:?}"
            );
        }
    }

    #[test]
    fn only_development_builds_accept_the_development_backup_key() {
        let found = vector("valid");
        assert_eq!(
            check(&found.bytes, &found.signatures, &backup_keys()).is_ok(),
            cfg!(debug_assertions)
        );
    }

    #[test]
    fn the_test_vectors_match_bible_nav_modules() {
        // When the generator's repository is beside this one, its vectors
        // must be the same files, so the two can't drift apart.
        let theirs = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bible-nav-modules/test-vectors/key-statements");
        let Ok(entries) = fs::read_dir(&theirs) else {
            return;
        };
        let mut compared = 0;
        for entry in entries {
            let path = entry.unwrap().path();
            let ours = vectors().join(path.file_name().unwrap());
            assert_eq!(
                fs::read(&path).unwrap(),
                fs::read(&ours).unwrap(),
                "{ours:?}"
            );
            compared += 1;
        }
        assert_eq!(compared, fs::read_dir(vectors()).unwrap().count());
    }

    #[test]
    fn a_valid_statement_says_what_it_changes() {
        let found = vector("valid");
        let statement = check(&found.bytes, &found.signatures, &test_backup_keys()).unwrap();
        assert_eq!(statement.sequence, 1);
        assert_eq!(statement.revoke, ["34750f98bd59fcfc"]);
        let added: Vec<String> = statement.add.iter().map(key_id).collect();
        assert_eq!(added, ["6a3803d5f059902a"]);
        assert_eq!(statement.keep_modules.len(), 1);
    }

    #[test]
    fn duplicates_are_checked_once_and_failures_are_reported() {
        let checked = check_all(
            vec![vector("valid"), vector("valid"), vector("tampered")],
            &test_backup_keys(),
        );
        assert_eq!(checked.accepted.len(), 1);
        assert_eq!(checked.problems.len(), 1);
        assert!(
            checked.problems[0].contains("tampered was ignored"),
            "{:?}",
            checked.problems
        );
    }

    #[test]
    fn saving_keeps_one_copy_and_never_overwrites_another_statement() {
        let dir = tempfile::tempdir().unwrap();
        let valid = vector("valid");
        let statement = check(&valid.bytes, &valid.signatures, &test_backup_keys()).unwrap();
        save(dir.path(), &statement, &valid).unwrap();
        save(dir.path(), &statement, &valid).unwrap();
        assert_eq!(read_folder(dir.path()).len(), 1);
        assert_eq!(
            fs::read(dir.path().join("bible-nav-keys-1.sig")).unwrap(),
            valid.signatures.as_bytes()
        );

        // A different statement with the same sequence gets its own name.
        let mut other = vector("valid-revoke-only");
        let other_statement = check(&other.bytes, &other.signatures, &test_backup_keys()).unwrap();
        let other_with_sequence_1 = Statement {
            sequence: 1,
            ..other_statement
        };
        other.origin = "other".into();
        save(dir.path(), &other_with_sequence_1, &other).unwrap();
        let names: Vec<String> = read_folder(dir.path())
            .iter()
            .map(|found| found.origin.rsplit('/').next().unwrap().to_string())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"bible-nav-keys-1.json".to_string()));
        assert_eq!(
            fs::read(dir.path().join("bible-nav-keys-1.json")).unwrap(),
            valid.bytes
        );
    }
}
