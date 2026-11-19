//! Queries on lexicon modules (bible-nav-modules/docs/lexicon-format.md):
//! an entry by its key, or the entries a Strong's number or lemma leads
//! to. Every value is a bound parameter.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::Serialize;

use super::error::Result;

/// One lexicon entry, as the page shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LexiconEntry {
    /// Unique and stable between versions: "H7225", "G26", "BDB09078".
    pub key: String,
    pub headword: String,
    /// hbo, arc, or grc: sets the font and text direction.
    pub headword_lang: String,
    /// BDB's "I.", "II." for words spelled the same, or empty.
    pub homograph: String,
    pub translit: String,
    pub pronunciation: String,
    /// BDB's dagger: every occurrence is cited.
    pub all_cited: bool,
    /// The body as stored, JSON for the page to parse, like a verse's
    /// `children`.
    pub body: String,
}

const COLUMNS: &str = "e.key, e.headword, e.headwordLang, e.homograph, e.translit, e.pronunciation, e.allCited, e.body";

fn entry(row: &Row) -> rusqlite::Result<LexiconEntry> {
    Ok(LexiconEntry {
        key: row.get(0)?,
        headword: row.get(1)?,
        headword_lang: row.get(2)?,
        homograph: row.get(3)?,
        translit: row.get(4)?,
        pronunciation: row.get(5)?,
        all_cited: row.get::<_, i64>(6)? != 0,
        body: row.get(7)?,
    })
}

/// The entry with this key, if there is one.
pub fn entry_by_key(conn: &Connection, key: &str) -> Result<Option<LexiconEntry>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM entries e WHERE e.key = ?1"),
            [key],
            entry,
        )
        .optional()?)
}

/// The entries a lookup leads to, in the lexicon's order: one for a
/// Strong's number in Strong's dictionaries, sometimes several in BDB.
/// `value` must already be in normal form (a Strong's number such as
/// "H430", not "H0430"); `normalize_strong` gives it.
pub fn lookup(conn: &Connection, kind: &str, value: &str) -> Result<Vec<LexiconEntry>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM lookups l JOIN entries e ON e.id = l.entryId
         WHERE l.kind = ?1 AND l.value = ?2 ORDER BY e.id"
    ))?;
    let entries = statement
        .query_map([kind, value], entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

/// A Strong's number in normal form: H or G, the number without leading
/// zeros, and a letter suffix, if any, in lower case. Bible modules keep
/// numbers as their source wrote them ("H0430" in the KJV), so the page's
/// numbers pass through this before a lookup.
pub fn normalize_strong(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let mut chars = raw.chars();
    let prefix = chars.next()?.to_ascii_uppercase();
    if prefix != 'H' && prefix != 'G' {
        return None;
    }
    let rest = chars.as_str();
    let (digits, suffix) = match rest.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((index, _)) => rest.split_at(index),
        None => (rest, ""),
    };
    let number = digits.trim_start_matches('0');
    let suffix_ok = suffix.is_empty()
        || (suffix.chars().count() == 1 && suffix.chars().all(|c| c.is_ascii_alphabetic()));
    if number.is_empty() || !suffix_ok {
        return None;
    }
    Some(format!("{prefix}{number}{}", suffix.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db::ModuleDb;
    use crate::modules::error::ModuleError;
    use crate::modules::integrity::{sidecars, ModuleKind};
    use crate::modules::test_support::{sign_lexicon, test_key, write_lexicon};
    use crate::modules::trust::Trust;
    use std::path::{Path, PathBuf};

    /// Two entries; H1 links to H2. The headwords are alef-bet and bet,
    /// written as escapes.
    fn small_lexicon(dir: &Path) -> PathBuf {
        write_lexicon(
            dir,
            "tst_lex",
            "1.0.0",
            &[
                (
                    "H1",
                    "\u{5d0}\u{5d1}",
                    r#"[{"style":"deriv","children":["from ",{"style":"ref","strong":"H2","children":["\u05d1"]},";"]},{"style":"def","children":[{"style":"em","children":["first"]},", rank, etc.)"]}]"#,
                ),
                (
                    "H2",
                    "\u{5d1}",
                    r#"[{"style":"def","children":["or ",{"style":"word","lang":"hbo","translit":"b","children":[]}]}]"#,
                ),
            ],
        )
    }

    fn open(path: &Path) -> crate::modules::error::Result<ModuleDb> {
        ModuleDb::open_checked(
            path,
            &sidecars(path),
            &path.file_name().unwrap().to_string_lossy(),
            &Trust::only(&[test_key().verifying_key()]),
        )
    }

    #[test]
    fn a_signed_lexicon_opens_and_finds_its_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = small_lexicon(dir.path());
        sign_lexicon(&path, &test_key(), |_| {});
        let db = open(&path).unwrap();
        assert_eq!(db.kind(), ModuleKind::Lexicon);
        assert_eq!(db.lookups(), ["strong:H"]);
        assert!(db.pin().is_none());

        let found = lookup(db.conn(), "strong", "H1").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "H1");
        assert_eq!(found[0].headword, "\u{5d0}\u{5d1}");
        assert!(found[0].body.contains("rank, etc.)"));
        assert!(lookup(db.conn(), "strong", "H3").unwrap().is_empty());
        assert_eq!(entry_by_key(db.conn(), "H2").unwrap().unwrap().key, "H2");
        assert!(entry_by_key(db.conn(), "H9").unwrap().is_none());
    }

    #[test]
    fn roots_that_do_not_match_the_entries_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = small_lexicon(dir.path());
        // Signed, with the right file hash, but the wrong content root:
        // what a signer who altered an entry would have to write.
        sign_lexicon(&path, &test_key(), |m| {
            m["entries"]["contentRoot"] = "0".repeat(64).into();
        });
        assert!(
            matches!(open(&path), Err(ModuleError::Untrusted(reason)) if reason.contains("may have been altered"))
        );
        sign_lexicon(&path, &test_key(), |m| m["entries"]["count"] = 3.into());
        assert!(matches!(open(&path), Err(ModuleError::Untrusted(_))));
    }

    #[test]
    fn a_lexicon_manifest_must_have_the_right_sections_and_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = small_lexicon(dir.path());
        sign_lexicon(&path, &test_key(), |m| {
            m["text"] = serde_json::json!({ "treeVersion": 1, "textRoot": "", "contentRoot": "" });
        });
        assert!(matches!(open(&path), Err(ModuleError::Untrusted(_))));
        sign_lexicon(&path, &test_key(), |m| {
            m.as_object_mut().unwrap().remove("entries");
        });
        assert!(matches!(open(&path), Err(ModuleError::Untrusted(_))));
        sign_lexicon(&path, &test_key(), |m| {
            m["module"]["formatVersion"] = 2.into()
        });
        assert!(matches!(open(&path), Err(ModuleError::Unsupported(_))));
        sign_lexicon(&path, &test_key(), |m| {
            m["module"]["moduleType"] = "atlas".into()
        });
        assert!(
            matches!(open(&path), Err(ModuleError::Unsupported(reason)) if reason.contains("atlas"))
        );
    }

    #[test]
    fn a_lexicon_signed_as_a_bible_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = small_lexicon(dir.path());
        sign_lexicon(&path, &test_key(), |m| {
            m["module"].as_object_mut().unwrap().remove("moduleType");
            m["module"]["formatVersion"] = 2.into();
            m.as_object_mut().unwrap().remove("entries");
            m["text"] = serde_json::json!({ "treeVersion": 1, "textRoot": "", "contentRoot": "" });
        });
        assert!(open(&path).is_err());
    }

    #[test]
    fn the_independent_python_checker_gives_the_same_roots() {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bible-nav-modules/tools/text-roots.py");
        if !script.exists()
            || std::process::Command::new("python3")
                .arg("--version")
                .output()
                .is_err()
        {
            eprintln!("skipped: needs python3 and bible-nav-modules beside bible-nav");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = small_lexicon(dir.path());
        let output = std::process::Command::new("python3")
            .arg("-I")
            .arg(&script)
            .arg(&path)
            .output()
            .unwrap();
        let found: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        let roots = crate::modules::lexicon_tree::roots(&conn).unwrap();
        assert_eq!(found["keys"], roots.keys.as_str());
        assert_eq!(found["content"], roots.content.as_str());
    }

    /// The real Strong's Hebrew module, if bible-nav-modules has generated
    /// it: the app's own roots must match its manifest, and H7218 must carry
    /// its correction. Skipped when the export isn't there.
    #[test]
    fn opens_the_generated_strongs_hebrew() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bible-nav-modules/exports/eng_strongs_heb-1.0.0.db");
        if !path.exists() {
            eprintln!("skipped: no eng_strongs_heb export");
            return;
        }
        let db = ModuleDb::open_checked(
            &path,
            &sidecars(&path),
            "eng_strongs_heb-1.0.0.db",
            // Until the maintainer signs it, it carries only the generator's
            // development signature.
            &Trust::only(
                &[
                    crate::modules::keys::built_in_module_keys(),
                    vec![crate::modules::keys::development_key()],
                ]
                .concat(),
            ),
        )
        .unwrap();
        assert_eq!(db.kind(), ModuleKind::Lexicon);
        let rosh = lookup(db.conn(), "strong", &normalize_strong("H07218").unwrap()).unwrap();
        assert_eq!(rosh.len(), 1);
        assert!(rosh[0].body.contains("rank, etc.)"), "{}", rosh[0].body);
        assert_eq!(rosh[0].translit, "rôʼsh");
    }

    #[test]
    fn strong_numbers_have_one_normal_form() {
        // The same cases as the generator's strong-number.test.ts.
        for (raw, normal) in [
            ("H0430", "H430"),
            ("H430", "H430"),
            ("G0026", "G26"),
            ("g26", "G26"),
            ("H1234A", "H1234a"),
            (" H7225 ", "H7225"),
        ] {
            assert_eq!(normalize_strong(raw).as_deref(), Some(normal), "{raw}");
        }
        for raw in ["", "H", "H0", "X12", "7225", "H12ab", "H-1", "BDB09078"] {
            assert_eq!(normalize_strong(raw), None, "{raw}");
        }
    }
}
