//! Computes a module's text root and content root, following "The text
//! tree" in bible-nav-modules/docs/integrity-spec.md.
//!
//! This is written from the spec alone, independently of the generator
//! (TypeScript) and `tools/text-roots.py` (Python), so a mistake in one
//! can't hide in the others: all three must give the same roots.
//!
//! The roots are computed from the module's own text, never taken from its
//! manifest, because a manifest only says what its signer wrote.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::error::{ModuleError, Result};

/// The two roots of a module, as lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub text: String,
    pub content: String,
}

/// An inline item: text, or an element with a style, attributes, and
/// children. Stored `children` JSON maps to these (spec, "Inline content").
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Text(String),
    Element {
        style: String,
        attributes: Vec<(String, String)>,
        children: Vec<Item>,
    },
}

/// Reads one stored item straight from its JSON, without building a
/// generic JSON tree first: parsing a whole Bible that way took most of the
/// time. A string is text; an object is an element (`w` with `strong` and
/// `txt`, anything else with `children`). Other fields are ignored.
impl<'de> Deserialize<'de> for Item {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(ItemVisitor)
    }
}

struct ItemVisitor;

impl<'de> Visitor<'de> for ItemVisitor {
    type Value = Item;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("text or an element")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> std::result::Result<Item, E> {
        Ok(Item::Text(text.to_string()))
    }

    fn visit_string<E: de::Error>(self, text: String) -> std::result::Result<Item, E> {
        Ok(Item::Text(text))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Item, A::Error> {
        let (mut style, mut strong, mut txt, mut children) = (None, None, None, None);
        while let Some(key) = map.next_key::<std::borrow::Cow<'de, str>>()? {
            match key.as_ref() {
                "style" => style = Some(map.next_value::<String>()?),
                "strong" => strong = Some(map.next_value::<String>()?),
                "txt" => txt = Some(map.next_value::<String>()?),
                "children" => children = Some(map.next_value::<Vec<Item>>()?),
                _ => {
                    map.next_value::<de::IgnoredAny>()?;
                }
            }
        }
        let style = style.ok_or_else(|| de::Error::missing_field("style"))?;
        if style == "w" {
            let strong = strong.ok_or_else(|| de::Error::missing_field("strong"))?;
            let txt = txt.ok_or_else(|| de::Error::missing_field("txt"))?;
            return Ok(Item::Element {
                style,
                attributes: vec![("strong".into(), strong)],
                children: vec![Item::Text(txt)],
            });
        }
        let children = children.ok_or_else(|| de::Error::missing_field("children"))?;
        Ok(Item::Element {
            style,
            attributes: Vec::new(),
            children,
        })
    }
}

/// The text view (spec, steps 1 to 5): footnotes removed, `w` and `wj`
/// replaced by their children, attributes removed, adjacent text joined,
/// and empty text removed, at every level.
fn text_view(items: &[Item]) -> Vec<Item> {
    let mut changed = Vec::new();
    for item in items {
        match item {
            Item::Text(text) => changed.push(Item::Text(text.clone())),
            Item::Element { style, .. } if style == "f" => {}
            Item::Element {
                style, children, ..
            } if style == "w" || style == "wj" => changed.extend(text_view(children)),
            Item::Element {
                style, children, ..
            } => changed.push(Item::Element {
                style: style.clone(),
                attributes: Vec::new(),
                children: text_view(children),
            }),
        }
    }
    let mut joined: Vec<Item> = Vec::new();
    for item in changed {
        match (joined.last_mut(), item) {
            (Some(Item::Text(previous)), Item::Text(text)) => previous.push_str(&text),
            (_, item) => joined.push(item),
        }
    }
    joined.retain(|item| !matches!(item, Item::Text(text) if text.is_empty()));
    joined
}

fn put_u32(out: &mut Vec<u8>, n: usize) {
    // Every count and length in a module fits in 32 bits.
    out.extend_from_slice(&(n as u32).to_be_bytes());
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    put_u32(out, text.len());
    out.extend_from_slice(text.as_bytes());
}

fn encode(out: &mut Vec<u8>, item: &Item) {
    match item {
        Item::Text(text) => {
            out.push(b't');
            put_str(out, text);
        }
        Item::Element {
            style,
            attributes,
            children,
        } => {
            out.push(b'e');
            put_str(out, style);
            let mut sorted: Vec<&(String, String)> = attributes.iter().collect();
            // Rust compares strings by their UTF-8 bytes, as the spec asks.
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            put_u32(out, sorted.len());
            for (name, value) in sorted {
                put_str(out, name);
                put_str(out, value);
            }
            encode_list(out, children);
        }
    }
}

fn encode_list(out: &mut Vec<u8>, items: &[Item]) {
    put_u32(out, items.len());
    for item in items {
        encode(out, item);
    }
}

type Hash = [u8; 32];

fn leaf(kind: &str, label: &str, items: &[Item]) -> Hash {
    let mut bytes = vec![0x00];
    put_str(&mut bytes, kind);
    put_str(&mut bytes, label);
    encode_list(&mut bytes, items);
    Sha256::digest(&bytes).into()
}

fn node(kind: &str, label: &str, children: &[Hash]) -> Hash {
    let mut bytes = vec![0x01];
    put_str(&mut bytes, kind);
    put_str(&mut bytes, label);
    put_u32(&mut bytes, children.len());
    for child in children {
        bytes.extend_from_slice(child);
    }
    Sha256::digest(&bytes).into()
}

/// One leaf in a chapter, with its place in reading order: a verse at twice
/// its id, a heading at twice its `afterVerseId` plus one (the spec's "plus
/// one half"), ties kept in id order.
struct Placed {
    position: i64,
    id: i64,
    text: Option<Hash>,
    content: Hash,
}

/// Computes both roots of an open module.
pub fn roots(conn: &Connection) -> Result<Roots> {
    // Every verse and heading becomes two leaf hashes, filed under its
    // chapter. Only hashes are kept, so this needs little memory.
    let mut chapters: HashMap<(i64, i64), Vec<Placed>> = HashMap::new();

    let mut verses = conn.prepare("SELECT bookId, chapter, id, nbr, children FROM verses")?;
    let mut rows = verses.query([])?;
    while let Some(row) = rows.next()? {
        let (book, chapter, id, number): (i64, i64, i64, i64) =
            (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
        let items = parse(column_text(row, 4)?)?;
        let label = number.to_string();
        chapters.entry((book, chapter)).or_default().push(Placed {
            position: id * 2,
            id,
            text: Some(leaf("verse", &label, &text_view(&items))),
            content: leaf("verse", &label, &items),
        });
    }

    let mut headings =
        conn.prepare("SELECT bookId, chapter, id, afterVerseId, style, children FROM headings")?;
    let mut rows = headings.query([])?;
    while let Some(row) = rows.next()? {
        let (book, chapter, id, after): (i64, i64, i64, i64) =
            (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
        let style: String = row.get(4)?;
        let items = parse(column_text(row, 5)?)?;
        chapters.entry((book, chapter)).or_default().push(Placed {
            position: after * 2 + 1,
            id,
            // The text view keeps only canonical headings.
            text: (style == "d").then(|| leaf("heading", &style, &text_view(&items))),
            content: leaf("heading", &style, &items),
        });
    }
    for placed in chapters.values_mut() {
        placed.sort_by_key(|item| (item.position, item.id));
    }

    let mut text_testaments = Vec::new();
    let mut content_testaments = Vec::new();
    let present: Vec<String> = conn
        .prepare("SELECT code FROM testaments")?
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    for testament in ["OT", "NT"] {
        if !present.iter().any(|code| code == testament) {
            continue;
        }
        let mut text_books = Vec::new();
        let mut content_books = Vec::new();
        let books: Vec<(i64, String)> = conn
            .prepare("SELECT id, code FROM books WHERE testamentCode = ?1 ORDER BY id")?
            .query_map([testament], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        for (book, code) in books {
            let mut text_chapters = Vec::new();
            let mut content_chapters = Vec::new();
            let numbers: Vec<i64> = conn
                .prepare("SELECT nbr FROM chapters WHERE bookId = ?1 ORDER BY nbr")?
                .query_map([book], |row| row.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            for number in numbers {
                let placed = chapters.remove(&(book, number)).unwrap_or_default();
                let text: Vec<Hash> = placed.iter().filter_map(|item| item.text).collect();
                let content: Vec<Hash> = placed.iter().map(|item| item.content).collect();
                let label = number.to_string();
                text_chapters.push(node("chapter", &label, &text));
                content_chapters.push(node("chapter", &label, &content));
            }
            text_books.push(node("book", &code, &text_chapters));
            content_books.push(node("book", &code, &content_chapters));
        }
        text_testaments.push(node("testament", testament, &text_books));
        content_testaments.push(node("testament", testament, &content_books));
    }
    if !chapters.is_empty() {
        return Err(unreadable("some verses or headings belong to no chapter"));
    }

    Ok(Roots {
        text: hex(&root("text", &text_testaments)),
        content: hex(&root("content", &content_testaments)),
    })
}

fn root(view: &str, testaments: &[Hash]) -> Hash {
    let mut bytes = vec![0x01];
    put_str(&mut bytes, "bible-nav text tree");
    put_u32(&mut bytes, 1);
    put_str(&mut bytes, view);
    put_u32(&mut bytes, testaments.len());
    for testament in testaments {
        bytes.extend_from_slice(testament);
    }
    Sha256::digest(&bytes).into()
}

/// A text column, borrowed from SQLite's row rather than copied.
fn column_text<'row>(row: &'row rusqlite::Row, index: usize) -> Result<&'row str> {
    row.get_ref(index)?
        .as_str()
        .map_err(|_| unreadable("content isn't text"))
}

fn parse(json: &str) -> Result<Vec<Item>> {
    serde_json::from_str(json)
        .map_err(|error| unreadable(&format!("its content can't be read ({error})")))
}

fn unreadable(why: &str) -> ModuleError {
    ModuleError::Unsupported(format!("its text can't be checked: {why}"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Item {
        Item::Text(s.into())
    }

    fn element(style: &str, children: Vec<Item>) -> Item {
        Item::Element {
            style: style.into(),
            attributes: Vec::new(),
            children,
        }
    }

    #[test]
    fn the_bundled_kjv_has_the_roots_in_its_manifest() {
        // The same roots as the generator and tools/text-roots.py.
        let db = crate::modules::test_support::kjv();
        let roots = roots(db.conn()).unwrap();
        assert_eq!(
            roots.text,
            "0337278626457c9d994ffe85b0b941affd9cc5fce1d5be90618c46ed381e0b07"
        );
        assert_eq!(
            roots.content,
            "f693db9b1a0f0739ad08682511cef50ec040b0a04c31834fabf95291a9e0a482"
        );
    }

    #[test]
    fn the_text_view_keeps_only_the_words_and_edition_markup() {
        let stored: Vec<Item> = serde_json::from_value(serde_json::json!([
            "In the ",
            {"style": "w", "strong": "H7225", "txt": "beginning"},
            {"style": "f", "children": [{"style": "ft", "children": ["a note"]}]},
            " ",
            {"style": "wj", "children": ["God ", {"style": "nd", "children": [
                {"style": "w", "strong": "H3068", "txt": "LORD"}]}]},
            ""
        ]))
        .unwrap();
        assert_eq!(
            text_view(&stored),
            vec![
                text("In the beginning God "),
                element("nd", vec![text("LORD")]),
            ]
        );
    }

    #[test]
    fn how_the_text_was_split_does_not_change_the_text_root() {
        let one = leaf("verse", "1", &text_view(&[text("In the beginning")]));
        let split = leaf(
            "verse",
            "1",
            &text_view(&[text("In the "), text(""), text("beginning")]),
        );
        assert_eq!(one, split);
    }

    #[test]
    fn a_text_item_encodes_as_in_the_spec() {
        let mut out = Vec::new();
        encode(&mut out, &text("Hi"));
        assert_eq!(out, [b't', 0, 0, 0, 2, b'H', b'i']);
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore]
    fn time_kjv_roots() {
        let db = crate::modules::test_support::kjv();
        for _ in 0..3 {
            let start = std::time::Instant::now();
            super::roots(db.conn()).unwrap();
            eprintln!("roots: {:?}", start.elapsed());
        }
    }
}
