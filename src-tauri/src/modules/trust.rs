//! Which keys this app trusts to sign modules right now: the keys built in
//! (`keys.rs`), changed by every key statement it has accepted
//! (`statements.rs`).
//!
//! - A key any accepted statement revokes is never trusted again, whatever
//!   order statements arrive in: revocation always wins.
//! - A key a statement adds is trusted, unless revoked.
//! - A revoked key's signature still counts on a manifest whose SHA-256 a
//!   statement lists in `keepModules`, and nowhere else.

use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;

use ed25519_dalek::VerifyingKey;

use super::integrity::key_id;
use super::keys;
use super::problem::Problem;
use super::statements::{self, Statement};

#[derive(Debug, Clone, Default)]
pub struct Trust {
    /// Keys whose signature on any manifest is accepted.
    pub trusted: Vec<VerifyingKey>,
    /// Revoked keys, whose signature counts only on the manifests in `keep`.
    pub revoked: Vec<VerifyingKey>,
    /// SHA-256 (lowercase hex) of manifests kept despite a revoked signer.
    pub keep: HashSet<String>,
}

impl Trust {
    /// Trusts exactly these keys, with no statements.
    #[cfg(test)]
    pub fn only(keys: &[VerifyingKey]) -> Self {
        Self::build(keys, &[])
    }

    /// The built-in module keys, changed by these accepted statements.
    pub fn build(built_in: &[VerifyingKey], statements: &[Statement]) -> Self {
        let revoked_ids: HashSet<&str> = statements
            .iter()
            .flat_map(|statement| statement.revoke.iter().map(String::as_str))
            .collect();
        let mut trust = Trust::default();
        let mut seen = HashSet::new();
        let candidates = built_in
            .iter()
            .chain(statements.iter().flat_map(|statement| statement.add.iter()));
        for key in candidates {
            let id = key_id(key);
            if !seen.insert(id.clone()) {
                continue;
            }
            if revoked_ids.contains(id.as_str()) {
                trust.revoked.push(*key);
            } else {
                trust.trusted.push(*key);
            }
        }
        trust.keep = statements
            .iter()
            .flat_map(|statement| statement.keep_modules.iter().cloned())
            .collect();
        trust
    }

    /// Whether a manifest with these bytes may carry a signature by `key`.
    pub fn allows(&self, key: &VerifyingKey, manifest: &[u8]) -> bool {
        self.trusted.contains(key)
            || (self.revoked.contains(key) && self.keep.contains(&statements::sha256_hex(manifest)))
    }
}

fn current_trust() -> &'static RwLock<Option<Trust>> {
    static CURRENT: RwLock<Option<Trust>> = RwLock::new(None);
    &CURRENT
}

/// The trust the app uses now. Until statements have been loaded from the
/// modules folder (`set_current`), it's the built-in keys changed by the
/// built-in statements.
pub fn current() -> Trust {
    if let Some(trust) = current_trust().read().ok().and_then(|guard| guard.clone()) {
        return trust;
    }
    let checked = statements::check_all(built_in_statements(), &keys::backup_keys());
    let accepted: Vec<Statement> = checked
        .accepted
        .into_iter()
        .map(|(statement, _)| statement)
        .collect();
    Trust::build(&keys::built_in_module_keys(), &accepted)
}

/// Reads key statements from those built in, those saved earlier in
/// `saved` (the app's own data folder), and `modules_keys` (the modules
/// folder's `keys` folder), and returns the trust they give, with messages
/// about statements that were ignored or couldn't be saved.
///
/// With `write`, every accepted statement is saved in both folders, so the
/// app never forgets it and anyone who copies the modules folder gets it.
/// Portable copies pass `write: false`: they leave no trace on the computer
/// and may run from read-only media.
pub fn load(saved: Option<&Path>, modules_keys: &Path, write: bool) -> (Trust, Vec<Problem>) {
    load_with(
        saved,
        modules_keys,
        write,
        &keys::backup_keys(),
        &keys::built_in_module_keys(),
    )
}

fn load_with(
    saved: Option<&Path>,
    modules_keys: &Path,
    write: bool,
    backup_keys: &[VerifyingKey],
    built_in: &[VerifyingKey],
) -> (Trust, Vec<Problem>) {
    let mut found = built_in_statements();
    if let Some(saved) = saved {
        found.extend(statements::read_folder(saved));
    }
    found.extend(statements::read_folder(modules_keys));
    let checked = statements::check_all(found, backup_keys);
    let mut problems = checked.problems;
    if write {
        for (statement, found) in &checked.accepted {
            for folder in saved.into_iter().chain([modules_keys]) {
                if let Err(error) = statements::save(folder, statement, found) {
                    problems.push(
                        Problem::new("problem.statementNotSaved")
                            .with("origin", found.origin.clone())
                            .with("folder", folder.display().to_string())
                            .detail(error),
                    );
                }
            }
        }
    }
    let accepted: Vec<Statement> = checked
        .accepted
        .into_iter()
        .map(|(statement, _)| statement)
        .collect();
    (Trust::build(built_in, &accepted), problems)
}

fn built_in_statements() -> Vec<statements::Found> {
    keys::BUILT_IN_STATEMENTS
        .iter()
        .map(|statement| statements::Found {
            origin: format!("built in ({})", statement.name),
            bytes: statement.statement.to_vec(),
            signatures: String::from_utf8_lossy(statement.signatures).into_owned(),
        })
        .collect()
}

/// Replaces the trust the app uses, after loading statements.
pub fn set_current(trust: Trust) {
    if let Ok(mut guard) = current_trust().write() {
        *guard = Some(trust);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::integrity::key_id;
    use ed25519_dalek::SigningKey;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn statement(revoke: &[&SigningKey], add: &[&SigningKey], keep: &[&[u8]]) -> Statement {
        Statement {
            sequence: 1,
            revoke: revoke.iter().map(|k| key_id(&k.verifying_key())).collect(),
            add: add.iter().map(|k| k.verifying_key()).collect(),
            keep_modules: keep.iter().map(|m| statements::sha256_hex(m)).collect(),
        }
    }

    #[test]
    fn a_statement_revokes_a_key_and_adds_its_replacement() {
        let (old, new) = (key(1), key(2));
        let trust = Trust::build(&[old.verifying_key()], &[statement(&[&old], &[&new], &[])]);
        assert!(!trust.allows(&old.verifying_key(), b"manifest"));
        assert!(trust.allows(&new.verifying_key(), b"manifest"));
    }

    #[test]
    fn revocation_wins_whatever_order_statements_arrive_in() {
        let (old, new) = (key(1), key(2));
        let adds = statement(&[&old], &[&new], &[]);
        let revokes_new = statement(&[&new], &[], &[]);
        for order in [
            vec![adds.clone(), revokes_new.clone()],
            vec![revokes_new, adds],
        ] {
            let trust = Trust::build(&[old.verifying_key()], &order);
            assert!(!trust.allows(&new.verifying_key(), b"manifest"));
            assert!(!trust.allows(&old.verifying_key(), b"manifest"));
        }
    }

    #[test]
    fn a_revoked_key_still_counts_on_kept_manifests_only() {
        let old = key(1);
        let kept: &[u8] = b"kept manifest\n";
        let trust = Trust::build(&[old.verifying_key()], &[statement(&[&old], &[], &[kept])]);
        assert!(trust.allows(&old.verifying_key(), kept));
        assert!(!trust.allows(&old.verifying_key(), b"another manifest\n"));
    }

    #[test]
    fn a_manifest_signed_by_a_revoked_key_says_so() {
        use crate::modules::test_support::{sign_copy, test_key};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tst-1.0.0.db");
        std::fs::write(&path, b"module bytes").unwrap();
        sign_copy(&path, "tst", "1.0.0", 2, &test_key());
        let check = |trust: &Trust| {
            crate::modules::integrity::verify(
                &path,
                &crate::modules::integrity::sidecars(&path),
                "tst-1.0.0.db",
                trust,
                2,
            )
        };
        let revoked = Trust::build(
            &[test_key().verifying_key()],
            &[statement(&[&test_key()], &[], &[])],
        );
        let error = check(&revoked).unwrap_err();
        assert!(error.to_string().contains("revoked"), "{error}");

        let manifest = std::fs::read(crate::modules::integrity::sidecars(&path).manifest).unwrap();
        let kept = Trust::build(
            &[test_key().verifying_key()],
            &[statement(&[&test_key()], &[], &[&manifest])],
        );
        assert!(check(&kept).is_ok());
    }

    fn load_test(saved: &Path, modules_keys: &Path, write: bool) -> (Trust, Vec<Problem>) {
        load_with(
            Some(saved),
            modules_keys,
            write,
            &[keys::development_backup_key()],
            &[],
        )
    }

    #[test]
    fn loading_reads_saves_and_reports_statements() {
        let vectors = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-vectors/key-statements");
        let saved = tempfile::tempdir().unwrap();
        let modules = tempfile::tempdir().unwrap();
        let keys_folder = modules.path().join("keys");
        std::fs::create_dir_all(&keys_folder).unwrap();
        for (from, to) in [
            ("valid", "bible-nav-keys-1"),
            ("tampered", "bible-nav-keys-2"),
        ] {
            for extension in ["json", "sig"] {
                std::fs::copy(
                    vectors.join(format!("{from}.{extension}")),
                    keys_folder.join(format!("{to}.{extension}")),
                )
                .unwrap();
            }
        }

        // A portable copy reads them but writes nothing.
        let (_, problems) = load_test(saved.path(), &keys_folder, false);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(statements::read_folder(saved.path()).is_empty());

        let (trust, problems) = load_test(saved.path(), &keys_folder, true);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].key, "problem.statementIgnored");
        assert!(problems[0].values["origin"].ends_with("bible-nav-keys-2.json"));
        let trusted: Vec<String> = trust.trusted.iter().map(key_id).collect();
        assert!(trusted.contains(&"6a3803d5f059902a".to_string()));
        assert_eq!(statements::read_folder(saved.path()).len(), 1);

        // Once saved, the statement still applies with the folder emptied.
        std::fs::remove_dir_all(&keys_folder).unwrap();
        let (trust, _) = load_test(saved.path(), &keys_folder, true);
        let trusted: Vec<String> = trust.trusted.iter().map(key_id).collect();
        assert!(trusted.contains(&"6a3803d5f059902a".to_string()));
        // And it's copied back into the modules folder for others.
        assert_eq!(statements::read_folder(&keys_folder).len(), 1);
    }

    #[test]
    fn a_statement_in_the_modules_folder_stops_the_bundled_kjv_opening() {
        use crate::modules::db::ModuleDb;
        use crate::modules::integrity::sidecars;
        use crate::modules::test_support::{copy_module, fixture_path};
        use base64::Engine;

        let modules = tempfile::tempdir().unwrap();
        let kjv = copy_module(&fixture_path(), modules.path());
        let name = kjv.file_name().unwrap().to_string_lossy().into_owned();
        let primary = keys::built_in_module_keys()
            .into_iter()
            .find(|key| key_id(key) == "447d4fa16f501391")
            .expect("the maintainer's primary key is built in");
        let backup = key(42);
        let keys_folder = modules.path().join("keys");
        std::fs::create_dir_all(&keys_folder).unwrap();

        let write_statement = |sequence: u32, keep: &str| {
            let json = format!(
                "{{\n  \"type\": \"bible-nav key statement\",\n  \"statementVersion\": 1,\n  \"sequence\": {sequence},\n  \"issuedAt\": \"2027-01-01\",\n  \"revoke\": [{{ \"keyId\": \"447d4fa16f501391\", \"reason\": \"test\" }}],\n  \"add\": [],\n  \"keepModules\": [{keep}]\n}}\n"
            );
            let signature = ed25519_dalek::Signer::sign(&backup, json.as_bytes());
            let base = keys_folder.join(format!("bible-nav-keys-{sequence}"));
            std::fs::write(base.with_extension("json"), &json).unwrap();
            std::fs::write(
                base.with_extension("sig"),
                format!(
                    "ed25519 {} {}\n",
                    key_id(&backup.verifying_key()),
                    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
                ),
            )
            .unwrap();
        };
        let open =
            |trust: &Trust| ModuleDb::open_checked(&kjv, &sidecars(&kjv), &name, trust).map(|_| ());
        let load_here = || {
            load_with(
                None,
                &keys_folder,
                false,
                &[backup.verifying_key()],
                &[primary],
            )
        };

        assert!(open(&load_here().0).is_ok());

        write_statement(1, "");
        let (trust, problems) = load_here();
        assert!(problems.is_empty(), "{problems:?}");
        let error = open(&trust).unwrap_err();
        assert!(error.to_string().contains("revoked"), "{error}");

        // A later statement keeping this exact manifest lets it open again.
        let manifest = std::fs::read(sidecars(&kjv).manifest).unwrap();
        write_statement(2, &format!("\"{}\"", statements::sha256_hex(&manifest)));
        let (trust, problems) = load_here();
        assert!(problems.is_empty(), "{problems:?}");
        assert!(open(&trust).is_ok());
    }
}
