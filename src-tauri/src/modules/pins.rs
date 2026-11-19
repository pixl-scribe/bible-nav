//! Pinned editions: the texts this app ships with, known exactly by their
//! text root for each major version. A module with a pinned id must have a
//! pinned major version and exactly that text, whatever key signed it, so
//! even a stolen key can't change a text the app already knows. A new major
//! version (new text) needs a release of the app that pins it. See "Module
//! versions" and "Pinned editions" in bible-nav-modules/docs/integrity-spec.md.
//!
//! The list is here, in one place beside `keys.rs`, so a fork can change it.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

use super::error::{ModuleError, Result};
use super::integrity::Manifest;
use super::text_tree::{self, Roots};

/// A text this app knows by its text root: one major version of a module.
#[derive(Debug)]
pub struct Pin {
    pub module_id: &'static str,
    /// Every version with this major number has this text.
    pub major: u64,
    /// The edition's name, as in the manifest's `text.edition`, for
    /// messages. It isn't matched: the id and major version are.
    pub edition: &'static str,
    /// The edition's text root.
    pub text_root: &'static str,
    /// SHA-256 of module files already known to hold this text, so the
    /// app needn't compute the root of a file it already knows.
    pub known_files: &'static [&'static str],
}

/// The pinned editions: only the locked texts bundled with the app (the
/// KJV, and the Textus Receptus and Masoretic text once they're added).
/// Add a pin for each new major version when it's locked and signed; never
/// change or remove one, or that version would be refused.
pub const PINNED_EDITIONS: &[Pin] = &[Pin {
    module_id: "eng_kjv",
    major: 1,
    edition: "eBible.org eng-kjv2006, 2026-11-10 release (CrossWire KJV: an electronic edition of Benjamin Blayney's 1769 Oxford text, with Strong's numbers)",
    text_root: "0337278626457c9d994ffe85b0b941affd9cc5fce1d5be90618c46ed381e0b07",
    known_files: &[
        // eng_kjv-1.1.0.db
        "a97a27f54eb62d1eac3edf7ec8facf447351ff9dd2e2e647f1060fbb1ebfa0a9",
        // eng_kjv-1.2.0.db, which added the abbreviation
        "1aecfa81874ba91f4a9f76fae1ee84470e9b6f8455cf3e2587eff90bdabfe7e5",
    ],
}];

/// Checks a module's text against its manifest and the pinned editions,
/// after its file hash has matched the manifest:
///
/// 1. its text and content roots, computed from the module, are the ones
///    in its manifest;
/// 2. if its module id is pinned, its major version is pinned and its text
///    root is that pin's.
///
/// Returns the pinned edition it matched, if any.
pub fn check_text(
    conn: &Connection,
    manifest: &Manifest,
    pins: &'static [Pin],
) -> Result<Option<&'static Pin>> {
    let text = manifest
        .text
        .as_ref()
        .ok_or_else(|| ModuleError::Untrusted("its manifest has no text section".into()))?;
    if text.tree_version != 1 {
        return Err(ModuleError::Unsupported(format!(
            "its text tree version {} isn't supported (this app reads version 1)",
            text.tree_version
        )));
    }

    let sha256 = manifest.file.sha256.to_ascii_lowercase();
    let known = pins
        .iter()
        .find(|pin| pin.known_files.contains(&sha256.as_str()));
    let text_root = match known {
        // The file was hashed against the manifest already, and this hash
        // is known to hold the pinned text.
        Some(pin) => {
            if text.text_root != pin.text_root {
                return Err(ModuleError::Untrusted(
                    "its manifest gives the wrong text root for its text".into(),
                ));
            }
            pin.text_root.to_string()
        }
        None => {
            let roots = computed_roots(conn, &sha256)?;
            if roots.text != text.text_root || roots.content != text.content_root {
                return Err(ModuleError::Untrusted(
                    "its text doesn't match the text roots in its manifest, so it may have been altered"
                        .into(),
                ));
            }
            roots.text
        }
    };

    check_pinned(manifest, &text_root, pins)
}

/// The pin rules alone, for a module whose text root is `text_root`: if
/// its id is pinned, its major version must be pinned and the root must be
/// that pin's. Used on computed roots when a module opens, and on a
/// catalog entry's signed manifest before it's offered for download.
pub fn check_pinned(
    manifest: &Manifest,
    text_root: &str,
    pins: &'static [Pin],
) -> Result<Option<&'static Pin>> {
    let for_id: Vec<&'static Pin> = pins
        .iter()
        .filter(|pin| pin.module_id == manifest.module.id)
        .collect();
    if for_id.is_empty() {
        return Ok(None);
    }
    let module = &manifest.module;
    let major = major_version(&module.version).ok_or_else(|| {
        ModuleError::Unsupported(format!(
            "its version {} isn't written major.minor.patch",
            module.version
        ))
    })?;
    let Some(pin) = for_id.into_iter().find(|pin| pin.major == major) else {
        return Err(ModuleError::Unsupported(format!(
            "{} {} is a new text of a module this version of Bible Nav pins, so it needs a newer Bible Nav",
            module.id, module.version
        )));
    };
    if pin.text_root != text_root {
        return Err(ModuleError::Untrusted(format!(
            "its text differs from {} {}.x ({}), which this app knows, so it may have been altered",
            module.id, pin.major, pin.edition
        )));
    }
    Ok(Some(pin))
}

/// The major version: the number before the first dot of `1.2.3`.
fn major_version(version: &str) -> Option<u64> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let rest: Vec<&str> = parts.collect();
    (rest.len() == 2 && rest.iter().all(|part| part.parse::<u64>().is_ok())).then_some(major)
}

/// Roots already computed during this run, by the SHA-256 of the file they
/// came from. The roots depend only on the file's bytes, and the file was
/// just hashed, so a match can be reused safely. Nothing is kept between
/// runs: a cache on disk could be changed by anyone who can write there.
fn computed_roots(conn: &Connection, sha256: &str) -> Result<Roots> {
    static COMPUTED: OnceLock<Mutex<HashMap<String, Roots>>> = OnceLock::new();
    let cache = COMPUTED.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(roots) = cache.lock().ok().and_then(|map| map.get(sha256).cloned()) {
        return Ok(roots);
    }
    let roots = text_tree::roots(conn)?;
    if let Ok(mut map) = cache.lock() {
        map.insert(sha256.to_string(), roots.clone());
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db::ModuleDb;
    use crate::modules::integrity::sidecars;
    use crate::modules::test_support::{fixture_path, sign_copy_as, sign_with_roots, test_key};
    use crate::modules::trust::Trust;
    use std::path::{Path, PathBuf};

    const KJV_EDITION: &str = "eBible.org eng-kjv2006, 2026-11-10 release (CrossWire KJV: an electronic edition of Benjamin Blayney's 1769 Oxford text, with Strong's numbers)";

    /// A copy of the bundled KJV as module `id` `version`, with "beginning"
    /// in Genesis 1:1 changed to "beginnings" if `alter` is set.
    fn kjv_copy(dir: &Path, id: &str, version: &str, alter: bool) -> PathBuf {
        let path = dir.join(format!("{id}-{version}.db"));
        std::fs::copy(fixture_path(), &path).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE module SET fetchId = ?1, version = ?2",
            [id, version],
        )
        .unwrap();
        if alter {
            let changed = conn
                .execute(
                    "UPDATE verses SET children = replace(children, '\"beginning\"', '\"beginnings\"') WHERE sid = 'GEN 1:1'",
                    [],
                )
                .unwrap();
            assert_eq!(changed, 1);
        }
        drop(conn);
        path
    }

    fn open(path: &Path) -> crate::modules::error::Result<ModuleDb> {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        ModuleDb::open_checked(
            path,
            &sidecars(path),
            &name,
            &Trust::only(&[test_key().verifying_key()]),
        )
    }

    #[test]
    fn the_bundled_kjv_is_the_pinned_edition() {
        let db = crate::modules::test_support::kjv();
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(sidecars(&fixture_path()).manifest).unwrap())
                .unwrap();
        let pin = check_text(db.conn(), &manifest, PINNED_EDITIONS).unwrap();
        assert_eq!(pin.map(|pin| pin.module_id), Some("eng_kjv"));
        let raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(sidecars(&fixture_path()).manifest).unwrap())
                .unwrap();
        assert_eq!(raw["text"]["edition"], KJV_EDITION);
        assert_eq!(PINNED_EDITIONS[0].edition, KJV_EDITION);
        assert!(PINNED_EDITIONS[0]
            .known_files
            .contains(&manifest.file.sha256.as_str()));
    }

    #[test]
    fn a_new_minor_version_with_the_same_text_opens() {
        let dir = tempfile::tempdir().unwrap();
        let path = kjv_copy(dir.path(), "eng_kjv", "1.9.0", false);
        sign_copy_as(&path, "eng_kjv", "1.9.0", 2, &test_key(), KJV_EDITION);
        open(&path).expect("same text, same major version");
    }

    #[test]
    fn altered_text_in_a_pinned_major_version_is_refused_whatever_key_signed_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = kjv_copy(dir.path(), "eng_kjv", "1.9.0", true);
        // Honest roots for the altered text, signed by a trusted key.
        sign_copy_as(&path, "eng_kjv", "1.9.0", 2, &test_key(), KJV_EDITION);
        let error = open(&path).err().expect("refused");
        assert!(
            error.to_string().contains("text differs from eng_kjv 1.x"),
            "{error}"
        );
    }

    #[test]
    fn a_new_major_version_of_a_pinned_module_needs_a_newer_app() {
        let dir = tempfile::tempdir().unwrap();
        for alter in [true, false] {
            let path = kjv_copy(dir.path(), "eng_kjv", "2.0.0", alter);
            sign_copy_as(
                &path,
                "eng_kjv",
                "2.0.0",
                2,
                &test_key(),
                "A corrected edition",
            );
            let error = open(&path).err().expect("refused");
            assert!(
                error.to_string().contains("needs a newer Bible Nav"),
                "{error}"
            );
        }
    }

    #[test]
    fn a_manifest_claiming_the_pinned_root_for_altered_text_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = kjv_copy(dir.path(), "eng_kjv", "1.9.0", true);
        let lie = Roots {
            text: PINNED_EDITIONS[0].text_root.into(),
            content: "f693db9b1a0f0739ad08682511cef50ec040b0a04c31834fabf95291a9e0a482".into(),
        };
        sign_with_roots(&path, "eng_kjv", "1.9.0", 2, &test_key(), KJV_EDITION, &lie);
        let error = open(&path).err().expect("refused");
        assert!(
            error.to_string().contains("doesn't match the text roots"),
            "{error}"
        );
    }

    #[test]
    fn an_unpinned_module_opens_at_any_version_whatever_its_edition_name() {
        let dir = tempfile::tempdir().unwrap();
        // Altered KJV text under another id, even naming the KJV's edition:
        // only the module id and major version are pinned.
        let path = kjv_copy(dir.path(), "eng_gnv", "7.0.0", true);
        sign_copy_as(&path, "eng_gnv", "7.0.0", 2, &test_key(), KJV_EDITION);
        let db = open(&path).expect("not a pinned module, so it opens");
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(sidecars(&path).manifest).unwrap()).unwrap();
        assert!(check_text(db.conn(), &manifest, PINNED_EDITIONS)
            .unwrap()
            .is_none());
    }

    #[test]
    fn the_known_kjv_file_needs_its_pinned_root_in_the_manifest() {
        // The bundled file itself, untouched, so its hash is a known one.
        let dir = tempfile::tempdir().unwrap();
        let version = crate::modules::test_support::kjv_version();
        let path = dir
            .path()
            .join(crate::modules::test_support::kjv_file_name());
        std::fs::copy(fixture_path(), &path).unwrap();
        let wrong = Roots {
            text: "0".repeat(64),
            content: "0".repeat(64),
        };
        sign_with_roots(
            &path,
            "eng_kjv",
            &version,
            2,
            &test_key(),
            KJV_EDITION,
            &wrong,
        );
        let error = open(&path).err().expect("refused");
        assert!(error.to_string().contains("wrong text root"), "{error}");
    }

    #[test]
    fn reads_major_versions() {
        assert_eq!(major_version("1.1.0"), Some(1));
        assert_eq!(major_version("12.0.3"), Some(12));
        assert_eq!(major_version("1.1"), None);
        assert_eq!(major_version("v1.0.0"), None);
        assert_eq!(major_version("1.x.0"), None);
    }
}
