//! The public keys this app trusts to sign Bible modules.
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
/// isn't trusted for them. It signs only key statements (revoking a primary
/// key, adding its replacement), which the app doesn't read yet. It's built
/// in now so that copies released today can trust those statements later.
#[allow(dead_code)]
pub const BACKUP_KEY: [u8; 32] = [
    0x19, 0x43, 0xc1, 0x02, 0x47, 0x3c, 0x5c, 0xa7, 0xbb, 0x13, 0x21, 0xd3, 0xf2, 0xa2, 0x03, 0x84,
    0xe4, 0xc9, 0x83, 0x84, 0x8b, 0x22, 0x96, 0xe6, 0x90, 0x97, 0xd5, 0x5c, 0xb4, 0x31, 0xdd, 0x9e,
];

/// The development key from bible-nav-modules/keys/development. Its private
/// key is public, so only development builds trust it.
const DEVELOPMENT_KEY: [u8; 32] = [
    0x30, 0x5a, 0xed, 0x1c, 0x55, 0xe5, 0x04, 0x98, 0x40, 0x3f, 0x6a, 0xe5, 0x6b, 0x1b, 0x92, 0x05,
    0x4c, 0x24, 0x61, 0x66, 0x7e, 0xde, 0x16, 0xcd, 0x1c, 0xbc, 0x8c, 0xb3, 0x6f, 0xde, 0x22, 0xd0,
];

/// The keys whose signature on a module's manifest this build accepts:
/// the primary keys, plus the development key in development builds.
///
/// `cfg!(debug_assertions)` is true in development builds (`tauri dev`,
/// `cargo test`) and false in release builds.
pub fn trusted_module_keys() -> Vec<VerifyingKey> {
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
        let ids: Vec<String> = trusted_module_keys().iter().map(key_id).collect();
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
        let ids: Vec<String> = trusted_module_keys().iter().map(key_id).collect();
        assert!(ids.contains(&"447d4fa16f501391".to_string()));
        assert!(!ids.contains(&"7659eff6260cd7f5".to_string()));
    }
}
