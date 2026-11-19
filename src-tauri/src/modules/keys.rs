//! The public keys this app trusts, and the key statements built into it.
//!
//! They're all here, in one place, so a fork can replace them with its own.
//! See "Keys and revocation" in bible-nav-modules/docs/integrity-spec.md.

use ed25519_dalek::VerifyingKey;

/// The maintainer's primary keys, which sign modules for release. From
/// bible-nav-modules/keys/primary/public.pem.
const PRIMARY_KEYS: &[[u8; 32]] = &[
    // Key id 447d4fa16f501391.
    [
        0x53, 0x09, 0x52, 0x61, 0x02, 0x89, 0x3b, 0x25, 0xf1, 0x6d, 0x09, 0x41, 0x56, 0x60, 0x45,
        0x66, 0x50, 0x35, 0x3e, 0xf6, 0x08, 0xfb, 0xf5, 0x5f, 0x9f, 0xfd, 0x58, 0x39, 0x15, 0xb7,
        0xf2, 0x88,
    ],
];

/// The maintainer's backup key (key id 7659eff6260cd7f5), from
/// bible-nav-modules/keys/backup/public.pem. It never signs modules, so it
/// isn't trusted for them. It signs only key statements, which revoke
/// primary keys and add their replacements (`statements.rs`).
const BACKUP_KEY: [u8; 32] = [
    0x19, 0x43, 0xc1, 0x02, 0x47, 0x3c, 0x5c, 0xa7, 0xbb, 0x13, 0x21, 0xd3, 0xf2, 0xa2, 0x03, 0x84,
    0xe4, 0xc9, 0x83, 0x84, 0x8b, 0x22, 0x96, 0xe6, 0x90, 0x97, 0xd5, 0x5c, 0xb4, 0x31, 0xdd, 0x9e,
];

/// The development key from bible-nav-modules/keys/development. Its private
/// key is public, so only development builds trust it.
const DEVELOPMENT_KEY: [u8; 32] = [
    0x30, 0x5a, 0xed, 0x1c, 0x55, 0xe5, 0x04, 0x98, 0x40, 0x3f, 0x6a, 0xe5, 0x6b, 0x1b, 0x92, 0x05,
    0x4c, 0x24, 0x61, 0x66, 0x7e, 0xde, 0x16, 0xcd, 0x1c, 0xbc, 0x8c, 0xb3, 0x6f, 0xde, 0x22, 0xd0,
];

/// The development backup key from bible-nav-modules/keys/development-backup,
/// for testing key statements. Its private key is public, so only
/// development builds trust it.
const DEVELOPMENT_BACKUP_KEY: [u8; 32] = [
    0x26, 0x54, 0xbd, 0xee, 0x0a, 0x53, 0xd6, 0x94, 0x78, 0x95, 0xb3, 0xb2, 0x2d, 0x02, 0x0a, 0x40,
    0xbe, 0xff, 0xc0, 0xef, 0x30, 0xb5, 0x2b, 0x28, 0x9d, 0x6a, 0x43, 0x24, 0xeb, 0x58, 0xee, 0xd5,
];

/// Key statements built into this release, from
/// bible-nav-modules/keys/statements: each one's JSON and `.sig` file. None
/// exist yet. Add every statement the maintainer publishes, so even a copy
/// that never meets one in a modules folder knows about it.
pub const BUILT_IN_STATEMENTS: &[BuiltInStatement] = &[];

/// A key statement compiled into the app with `include_bytes!`.
pub struct BuiltInStatement {
    pub name: &'static str,
    pub statement: &'static [u8],
    pub signatures: &'static [u8],
}

/// The keys allowed to sign key statements: the backup key, plus the
/// development backup key in development builds. Never trusted for modules.
pub fn backup_keys() -> Vec<VerifyingKey> {
    let mut raw = vec![BACKUP_KEY];
    if cfg!(debug_assertions) {
        raw.push(DEVELOPMENT_BACKUP_KEY);
    }
    raw.iter()
        .filter_map(|bytes| VerifyingKey::from_bytes(bytes).ok())
        .collect()
}

/// The development backup key, which signs the shared test vectors.
#[cfg(test)]
pub fn development_backup_key() -> VerifyingKey {
    VerifyingKey::from_bytes(&DEVELOPMENT_BACKUP_KEY).unwrap()
}

/// The keys built in to sign modules: the primary keys, plus the
/// development key in development builds. Key statements can revoke them or
/// add others (`trust.rs` combines them).
///
/// `cfg!(debug_assertions)` is true in development builds (`tauri dev`,
/// `cargo test`) and false in release builds.
pub fn built_in_module_keys() -> Vec<VerifyingKey> {
    let mut raw: Vec<[u8; 32]> = PRIMARY_KEYS.to_vec();
    if cfg!(debug_assertions) {
        raw.push(DEVELOPMENT_KEY);
    }
    raw.iter()
        // A list entry that isn't a valid key is a mistake in this file, so
        // it's skipped rather than trusted.
        .filter_map(|bytes| VerifyingKey::from_bytes(bytes).ok())
        .collect()
}

/// The development key, for tests that check a module the generator signed
/// with it, in release test builds too (which don't trust it otherwise).
#[cfg(test)]
pub fn development_key() -> VerifyingKey {
    VerifyingKey::from_bytes(&DEVELOPMENT_KEY).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::integrity::key_id;

    #[test]
    fn the_development_key_matches_the_generator() {
        // The key id bible-nav-modules prints for its development key.
        let key = VerifyingKey::from_bytes(&DEVELOPMENT_KEY).unwrap();
        assert_eq!(key_id(&key), "c7703a6f42bedeee");
    }

    #[test]
    fn only_development_builds_trust_the_development_key() {
        let ids: Vec<String> = built_in_module_keys().iter().map(key_id).collect();
        // `cargo test` is a development build; `cargo test --release` isn't.
        assert_eq!(
            ids.contains(&"c7703a6f42bedeee".to_string()),
            cfg!(debug_assertions)
        );
    }

    #[test]
    fn the_maintainer_keys_match_the_generator() {
        // The key ids bible-nav-modules prints for keys/primary and keys/backup.
        let primary: Vec<String> = PRIMARY_KEYS
            .iter()
            .map(|bytes| key_id(&VerifyingKey::from_bytes(bytes).unwrap()))
            .collect();
        assert_eq!(primary, ["447d4fa16f501391"]);
        let backup = VerifyingKey::from_bytes(&BACKUP_KEY).unwrap();
        assert_eq!(key_id(&backup), "7659eff6260cd7f5");
    }

    #[test]
    fn the_backup_key_never_signs_modules() {
        let ids: Vec<String> = built_in_module_keys().iter().map(key_id).collect();
        assert!(ids.contains(&"447d4fa16f501391".to_string()));
        assert!(!ids.contains(&"7659eff6260cd7f5".to_string()));
    }

    #[test]
    fn the_development_backup_key_matches_the_generator() {
        let key = VerifyingKey::from_bytes(&DEVELOPMENT_BACKUP_KEY).unwrap();
        assert_eq!(key_id(&key), "aecf25562bdc80f3");
        let backup: Vec<String> = backup_keys().iter().map(key_id).collect();
        assert_eq!(
            backup.contains(&"aecf25562bdc80f3".to_string()),
            cfg!(debug_assertions)
        );
        assert!(backup.contains(&"7659eff6260cd7f5".to_string()));
    }

    #[test]
    fn every_statement_in_the_keys_folder_is_built_in() {
        // src-tauri/keys holds the published statements; each must be listed
        // in BUILT_IN_STATEMENTS, or a release would leave it out.
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("keys");
        let mut on_disk: Vec<String> = std::fs::read_dir(&folder)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".json"))
                    .collect()
            })
            .unwrap_or_default();
        on_disk.sort();
        let mut built_in: Vec<String> = BUILT_IN_STATEMENTS
            .iter()
            .map(|statement| statement.name.to_string())
            .collect();
        built_in.sort();
        assert_eq!(on_disk, built_in);
    }
}
