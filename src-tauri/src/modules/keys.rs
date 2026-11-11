//! The public keys this app trusts to sign Bible modules.
//!
//! They're all here, in one place, so a fork can replace them with its own.
//! See "Keys and revocation" in bible-nav-modules/docs/integrity-spec.md.

use ed25519_dalek::VerifyingKey;

/// The maintainer's primary keys, which sign modules for release. None
/// exist yet: until one is added, release builds load no modules at all.
///
/// The backup key isn't listed: it never signs modules, only key
/// statements, which the app doesn't read yet.
const PRIMARY_KEYS: &[[u8; 32]] = &[];

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
    fn development_builds_trust_the_development_key() {
        let ids: Vec<String> = trusted_module_keys().iter().map(key_id).collect();
        assert!(ids.contains(&"c7703a6f42bedeee".to_string()));
    }
}
