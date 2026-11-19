//! A lexicon's keys root and content root, computed from the module itself
//! (bible-nav-modules/docs/lexicon-format.md, "The lexicon tree"), and the
//! check that they are the ones its manifest gives.
//!
//! Written from the spec alone, like `text_tree.rs`: it shares no code with
//! the generator or with `tools/text-roots.py`, so all three agreeing shows
//! the spec is followed. A manifest's roots are only what its signer wrote,
//! so the app never relies on them without computing them.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::error::{ModuleError, Result};
use super::integrity::Manifest;

/// The spec's tree version, hashed into both roots.
const TREE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconRoots {
    pub keys: String,
    pub content: String,
    /// How many entries it has, which the manifest also gives.
    pub count: u64,
}

fn u32_bytes(n: u32) -> [u8; 4] {
    n.to_be_bytes()
}

/// `str(s)` in the spec: the UTF-8 byte length, then the bytes.
fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&u32_bytes(s.len() as u32));
    out.extend_from_slice(s.as_bytes());
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn bad(what: &str) -> ModuleError {
    ModuleError::Untrusted(format!("an entry's body {what}"))
}

/// `enc(item)` from the integrity spec: text is `t` and the string; an
/// element is `e`, its style, its attributes sorted by name, and its
/// children. In a stored body every key but `style` and `children` is an
/// attribute, and must be text.
fn encode_item(out: &mut Vec<u8>, item: &Value) -> Result<()> {
    match item {
        Value::String(text) => {
            out.push(b't');
            push_str(out, text);
        }
        Value::Object(fields) => {
            let style = fields
                .get("style")
                .and_then(Value::as_str)
                .ok_or_else(|| bad("has an element without a style"))?;
            let children = fields
                .get("children")
                .and_then(Value::as_array)
                .ok_or_else(|| bad("has an element without children"))?;
            let mut attributes: Vec<(&str, &str)> = Vec::new();
            for (name, value) in fields {
                if name == "style" || name == "children" {
                    continue;
                }
                let value = value
                    .as_str()
                    .ok_or_else(|| bad("has an attribute that isn't text"))?;
                attributes.push((name, value));
            }
            // Sorted by name in UTF-8 byte order, which is how Rust
            // compares `&str`.
            attributes.sort_by(|a, b| a.0.cmp(b.0));
            out.push(b'e');
            push_str(out, style);
            out.extend_from_slice(&u32_bytes(attributes.len() as u32));
            for (name, value) in attributes {
                push_str(out, name);
                push_str(out, value);
            }
            encode_list(out, children)?;
        }
        _ => return Err(bad("has an item that is neither text nor an element")),
    }
    Ok(())
}

fn encode_list(out: &mut Vec<u8>, items: &[Value]) -> Result<()> {
    out.extend_from_slice(&u32_bytes(items.len() as u32));
    for item in items {
        encode_item(out, item)?;
    }
    Ok(())
}

/// Both roots, read from the module's `entries` and `lookups` tables.
pub fn roots(conn: &Connection) -> Result<LexiconRoots> {
    let mut statement = conn.prepare(
        "SELECT key, headword, headwordLang, homograph, translit, pronunciation, allCited, body
         FROM entries ORDER BY id",
    )?;
    let mut keys: Vec<u8> = Vec::new();
    let mut leaves: Vec<u8> = Vec::new();
    let mut count = 0u32;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let all_cited: i64 = row.get(6)?;
        let body: String = row.get(7)?;
        let body: Vec<Value> = serde_json::from_str(&body).map_err(|_| bad("isn't a JSON list"))?;
        let mut leaf = vec![0x00];
        push_str(&mut leaf, "entry");
        push_str(&mut leaf, &key);
        for column in 1..=5 {
            let text: String = row.get(column)?;
            push_str(&mut leaf, &text);
        }
        leaf.extend_from_slice(&u32_bytes(all_cited as u32));
        encode_list(&mut leaf, &body)?;
        leaves.extend_from_slice(&sha256(&leaf));
        push_str(&mut keys, &key);
        count += 1;
    }

    let mut lookups: Vec<(String, String, String)> = conn
        .prepare("SELECT l.kind, l.value, e.key FROM lookups l JOIN entries e ON e.id = l.entryId")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;
    // By kind, then value, then entry key, in UTF-8 byte order.
    lookups.sort();
    let mut lookup_part = u32_bytes(lookups.len() as u32).to_vec();
    for (kind, value, key) in &lookups {
        let mut leaf = vec![0x00];
        push_str(&mut leaf, "lookup");
        push_str(&mut leaf, kind);
        push_str(&mut leaf, value);
        push_str(&mut leaf, key);
        lookup_part.extend_from_slice(&sha256(&leaf));
    }

    let root = |view: &str, entries: &[u8]| {
        let mut bytes = vec![0x01];
        push_str(&mut bytes, "bible-nav lexicon tree");
        bytes.extend_from_slice(&u32_bytes(TREE_VERSION));
        push_str(&mut bytes, view);
        bytes.extend_from_slice(&u32_bytes(count));
        bytes.extend_from_slice(entries);
        bytes.extend_from_slice(&lookup_part);
        hex(&sha256(&bytes))
    };
    Ok(LexiconRoots {
        keys: root("keys", &keys),
        content: root("content", &leaves),
        count: u64::from(count),
    })
}

/// Checks a lexicon's computed roots against its manifest. Roots are
/// remembered for the rest of the run by the file's SHA-256 (already
/// checked against the manifest), never on disk, where they could be
/// tampered with.
pub fn check_roots(conn: &Connection, manifest: &Manifest) -> Result<()> {
    let entries = manifest
        .entries
        .as_ref()
        .ok_or_else(|| ModuleError::Untrusted("its manifest has no entries section".into()))?;
    if entries.tree_version != TREE_VERSION {
        return Err(ModuleError::Unsupported(format!(
            "its lexicon tree version {} isn't supported (this app reads version {TREE_VERSION})",
            entries.tree_version
        )));
    }
    static COMPUTED: OnceLock<Mutex<HashMap<String, LexiconRoots>>> = OnceLock::new();
    let cache = COMPUTED.get_or_init(|| Mutex::new(HashMap::new()));
    let sha256 = manifest.file.sha256.to_ascii_lowercase();
    let cached = cache.lock().ok().and_then(|map| map.get(&sha256).cloned());
    let roots = match cached {
        Some(roots) => roots,
        None => {
            let roots = roots(conn)?;
            if let Ok(mut map) = cache.lock() {
                map.insert(sha256, roots.clone());
            }
            roots
        }
    };
    if roots.keys != entries.keys_root
        || roots.content != entries.content_root
        || roots.count != entries.count
    {
        return Err(ModuleError::Untrusted(
            "its entries don't match the roots in its manifest, so it may have been altered".into(),
        ));
    }
    Ok(())
}
