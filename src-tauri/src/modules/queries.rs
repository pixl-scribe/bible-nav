//! The queries the app runs against a module.
//!
//! Every value from the page is passed as a bound parameter (`?1`, `?2`, ...),
//! never pasted into the SQL text, so nothing the page sends can change what
//! a query does.
//!
//! Verse text (`children`) is returned as the JSON string stored in the
//! module. The page parses it, so this code doesn't need to know its shape.

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::error::{ModuleError, Result};

// `#[derive(Serialize)]` generates the code that turns these structs into
// JSON for the page. `rename_all = "camelCase"` makes `book_id` arrive as
// `bookId`, matching TypeScript naming.

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    pub name: String,
    pub description: String,
    pub module_type: String,
    pub language: String,
    pub language_code: String,
    pub license: String,
    pub version: String,
    pub format_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: i64,
    pub testament_code: Option<String>,
    pub name: String,
    pub code: String,
    pub header: String,
    pub toc1: String,
    pub toc2: String,
    pub toc3: String,
    pub mt1: String,
    pub mt2: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerseRow {
    pub id: i64,
    pub book_id: i64,
    pub chapter: i64,
    pub nbr: i64,
    pub paragraph: i64,
    pub sid: String,
    /// The verse's inline content as stored: a JSON array.
    pub children: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingRow {
    pub id: i64,
    pub book_id: i64,
    pub chapter: i64,
    pub paragraph: i64,
    /// Show this heading before the verse whose id is `after_verse_id + 1`.
    pub after_verse_id: i64,
    pub style: String,
    /// The heading's inline content as stored: a JSON array.
    pub children: String,
}

/// One paragraph, with whatever it holds: verses, a heading, or nothing
/// (a blank `b` line between stanzas).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRow {
    pub id: i64,
    pub book_id: i64,
    pub chapter: i64,
    pub style: String,
    pub heading: Option<HeadingRow>,
    pub verses: Vec<VerseRow>,
}

/// The paragraphs around `center`, in reading order.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphWindow {
    pub center: i64,
    pub paragraphs: Vec<ParagraphRow>,
}

/// The verses around `center`, plus the headings that fall among them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerseWindow {
    pub center: i64,
    pub verses: Vec<VerseRow>,
    pub headings: Vec<HeadingRow>,
}

/// A verse or a range of verses, given by sids such as "JHN 3:16".
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefRange {
    pub from: String,
    pub to: Option<String>,
}

/// A verse within a reference search, with its position in the results.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedVerse {
    pub idx: i64,
    #[serde(flatten)]
    pub verse: VerseRow,
}

const VERSE_COLUMNS: &str = "id, bookId, chapter, nbr, paragraph, sid, children";
const HEADING_COLUMNS: &str = "id, bookId, chapter, paragraph, afterVerseId, style, children";

fn verse_from_row(row: &Row) -> rusqlite::Result<VerseRow> {
    Ok(VerseRow {
        id: row.get(0)?,
        book_id: row.get(1)?,
        chapter: row.get(2)?,
        nbr: row.get(3)?,
        paragraph: row.get(4)?,
        sid: row.get(5)?,
        children: row.get(6)?,
    })
}

fn heading_from_row(row: &Row) -> rusqlite::Result<HeadingRow> {
    Ok(HeadingRow {
        id: row.get(0)?,
        book_id: row.get(1)?,
        chapter: row.get(2)?,
        paragraph: row.get(3)?,
        after_verse_id: row.get(4)?,
        style: row.get(5)?,
        children: row.get(6)?,
    })
}

pub fn module_info(conn: &Connection) -> Result<ModuleInfo> {
    let info = conn.query_row(
        "SELECT name, description, moduleType, language, languageCode, license, version, formatVersion
         FROM module",
        [],
        |row| {
            Ok(ModuleInfo {
                name: row.get(0)?,
                description: row.get(1)?,
                module_type: row.get(2)?,
                language: row.get(3)?,
                language_code: row.get(4)?,
                license: row.get(5)?,
                version: row.get(6)?,
                format_version: row.get(7)?,
            })
        },
    )?;
    Ok(info)
}

pub fn books(conn: &Connection) -> Result<Vec<Book>> {
    let mut statement = conn.prepare(
        "SELECT id, testamentCode, name, code, header, toc1, toc2, toc3, mt1, mt2
         FROM books ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Book {
            id: row.get(0)?,
            testament_code: row.get(1)?,
            name: row.get(2)?,
            code: row.get(3)?,
            header: row.get(4)?,
            toc1: row.get(5)?,
            toc2: row.get(6)?,
            toc3: row.get(7)?,
            mt1: row.get(8)?,
            mt2: row.get(9)?,
        })
    })?;
    // `collect` gathers the rows into a Vec, stopping at the first error.
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// The id of the verse with this sid, such as "JHN 3:16".
pub fn verse_id_for_sid(conn: &Connection, sid: &str) -> Result<i64> {
    conn.query_row("SELECT id FROM verses WHERE sid = ?1", [sid], |row| {
        row.get(0)
    })
    .optional()?
    .ok_or_else(|| ModuleError::NotFound(format!("verse {sid}")))
}

/// The id of the paragraph that holds the verse with this sid.
pub fn paragraph_for_sid(conn: &Connection, sid: &str) -> Result<i64> {
    conn.query_row(
        "SELECT paragraph FROM verses WHERE sid = ?1",
        [sid],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| ModuleError::NotFound(format!("verse {sid}")))
}

/// The paragraphs from `before` paragraphs ahead of `center` to `after`
/// paragraphs past it, with their verses and headings.
///
/// Paragraph ids have no gaps, so the window is exactly that range, clipped
/// at the start and end of the module.
pub fn paragraph_window(
    conn: &Connection,
    center: i64,
    before: u32,
    after: u32,
) -> Result<ParagraphWindow> {
    let from = (center - i64::from(before)).max(0);
    let to = center + i64::from(after);

    let mut paragraphs = Vec::new();
    {
        let mut statement = conn.prepare(
            "SELECT id, bookId, chapter, style FROM paragraphs
             WHERE id BETWEEN ?1 AND ?2 ORDER BY id",
        )?;
        let rows = statement.query_map(params![from, to], |row| {
            Ok(ParagraphRow {
                id: row.get(0)?,
                book_id: row.get(1)?,
                chapter: row.get(2)?,
                style: row.get(3)?,
                heading: None,
                verses: Vec::new(),
            })
        })?;
        for row in rows {
            paragraphs.push(row?);
        }
    }

    // Paragraph ids run in order from `from`, so a paragraph's position in
    // the list is its id minus `from`.
    let position = |paragraph: i64, paragraphs: &[ParagraphRow]| -> Option<usize> {
        let index = usize::try_from(paragraph - from).ok()?;
        (index < paragraphs.len()).then_some(index)
    };

    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE paragraph BETWEEN ?1 AND ?2 ORDER BY id"
    ))?;
    for verse in statement.query_map(params![from, to], verse_from_row)? {
        let verse = verse?;
        if let Some(index) = position(verse.paragraph, &paragraphs) {
            paragraphs[index].verses.push(verse);
        }
    }

    let mut statement = conn.prepare(&format!(
        "SELECT {HEADING_COLUMNS} FROM headings WHERE paragraph BETWEEN ?1 AND ?2 ORDER BY id"
    ))?;
    for heading in statement.query_map(params![from, to], heading_from_row)? {
        let heading = heading?;
        if let Some(index) = position(heading.paragraph, &paragraphs) {
            paragraphs[index].heading = Some(heading);
        }
    }

    Ok(ParagraphWindow { center, paragraphs })
}

/// The verses from `before` verses ahead of `center` to `after` verses past
/// it, plus any headings that appear before one of them.
pub fn verse_window(
    conn: &Connection,
    center: i64,
    before: u32,
    after: u32,
) -> Result<VerseWindow> {
    let from = (center - i64::from(before)).max(1);
    let to = center + i64::from(after);

    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE id BETWEEN ?1 AND ?2 ORDER BY id"
    ))?;
    let verses = statement
        .query_map(params![from, to], verse_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // A heading shown before verse n has afterVerseId = n - 1.
    let mut statement = conn.prepare(&format!(
        "SELECT {HEADING_COLUMNS} FROM headings WHERE afterVerseId BETWEEN ?1 AND ?2 ORDER BY id"
    ))?;
    let headings = statement
        .query_map(params![from - 1, to - 1], heading_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(VerseWindow {
        center,
        verses,
        headings,
    })
}

/// Turns reference ranges into verse-id ranges, failing if a sid is unknown.
fn id_ranges(conn: &Connection, ranges: &[RefRange]) -> Result<Vec<(i64, i64)>> {
    ranges
        .iter()
        .map(|range| {
            let from = verse_id_for_sid(conn, &range.from)?;
            let to = match &range.to {
                Some(to) => verse_id_for_sid(conn, to)?,
                None => from,
            };
            Ok((from.min(to), from.max(to)))
        })
        .collect()
}

/// Builds `(id BETWEEN ? AND ?) OR (...)` with one pair of placeholders per
/// range, and the matching parameter values. Only placeholders go into the
/// SQL text; the values are bound separately.
fn range_filter(id_ranges: &[(i64, i64)]) -> (String, Vec<i64>) {
    let clause = vec!["(id BETWEEN ? AND ?)"; id_ranges.len()].join(" OR ");
    let values = id_ranges
        .iter()
        .flat_map(|&(from, to)| [from, to])
        .collect();
    (clause, values)
}

/// The sids of every verse in the ranges, in canonical order.
pub fn ref_sids(conn: &Connection, ranges: &[RefRange]) -> Result<Vec<String>> {
    let ids = id_ranges(conn, ranges)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let (clause, values) = range_filter(&ids);
    let mut statement = conn.prepare(&format!(
        "SELECT sid FROM verses WHERE {clause} ORDER BY id"
    ))?;
    let sids = statement
        .query_map(params_from_iter(values), |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(sids)
}

/// One page of the verses in the ranges: `limit` verses starting at result
/// number `offset`. Each verse carries its result number as `idx`.
pub fn ref_verses(
    conn: &Connection,
    ranges: &[RefRange],
    offset: u32,
    limit: u32,
) -> Result<Vec<IndexedVerse>> {
    let ids = id_ranges(conn, ranges)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let (clause, mut values) = range_filter(&ids);
    values.push(i64::from(limit));
    values.push(i64::from(offset));
    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE {clause} ORDER BY id LIMIT ? OFFSET ?"
    ))?;
    let verses = statement
        .query_map(params_from_iter(values), verse_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(verses
        .into_iter()
        .zip(i64::from(offset)..)
        .map(|(verse, idx)| IndexedVerse { idx, verse })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::kjv;

    fn range(from: &str, to: Option<&str>) -> RefRange {
        RefRange {
            from: from.into(),
            to: to.map(Into::into),
        }
    }

    #[test]
    fn reads_module_info() {
        let info = module_info(kjv().conn()).unwrap();
        assert_eq!(info.name, "King James Version (1769)");
        assert_eq!(info.format_version, 1);
    }

    #[test]
    fn reads_books_in_order() {
        let books = books(kjv().conn()).unwrap();
        assert_eq!(books.len(), 66);
        assert_eq!(books[0].code, "GEN");
        let john = books.iter().find(|book| book.code == "JHN").unwrap();
        assert_eq!((john.toc2.as_str(), john.toc3.as_str()), ("John", "Jhn"));
        let first_kings = books.iter().find(|book| book.code == "1KI").unwrap();
        assert_eq!(
            first_kings.mt2.as_deref(),
            Some("Commonly Called the Third Book of the Kings")
        );
    }

    #[test]
    fn finds_verses_and_paragraphs_by_sid() {
        let db = kjv();
        assert_eq!(verse_id_for_sid(db.conn(), "JHN 3:16").unwrap(), 26137);
        assert_eq!(paragraph_for_sid(db.conn(), "PSA 23:1").unwrap(), 2898);
        let missing = verse_id_for_sid(db.conn(), "XYZ 1:1").unwrap_err();
        assert!(missing.to_string().contains("XYZ 1:1"));
    }

    #[test]
    fn paragraph_window_includes_styles_and_headings() {
        let db = kjv();
        // Psalm 23: a title (d), then one poetic line (q1) per verse.
        let window = paragraph_window(db.conn(), 2898, 1, 5).unwrap();
        let styles: Vec<&str> = window.paragraphs.iter().map(|p| p.style.as_str()).collect();
        assert_eq!(styles, ["d", "q1", "q1", "q1", "q1", "q1", "q1"]);

        let title = window.paragraphs[0]
            .heading
            .as_ref()
            .expect("Psalm 23 has a title");
        assert!(title.children.contains("Psalm"), "{}", title.children);
        assert!(window.paragraphs[0].verses.is_empty());

        let first_line = &window.paragraphs[1];
        assert_eq!(first_line.verses.len(), 1);
        assert_eq!(first_line.verses[0].sid, "PSA 23:1");
        assert!(first_line.verses[0].children.contains("LORD"));
    }

    #[test]
    fn paragraph_window_includes_blank_stanza_breaks() {
        let db = kjv();
        let center = paragraph_for_sid(db.conn(), "PSA 1:3").unwrap();
        let window = paragraph_window(db.conn(), center, 0, 2).unwrap();
        let styles: Vec<&str> = window.paragraphs.iter().map(|p| p.style.as_str()).collect();
        assert_eq!(styles, ["q1", "b", "q1"]);
        assert!(window.paragraphs[1].verses.is_empty() && window.paragraphs[1].heading.is_none());
    }

    #[test]
    fn paragraph_window_stops_at_the_start_of_the_module() {
        let window = paragraph_window(kjv().conn(), 1, 20, 1).unwrap();
        assert_eq!(window.paragraphs[0].id, 0);
        assert_eq!(window.paragraphs[0].verses[0].sid, "GEN 1:1");
    }

    #[test]
    fn verse_window_includes_headings_before_its_verses() {
        let db = kjv();
        let psalm_3_1 = verse_id_for_sid(db.conn(), "PSA 3:1").unwrap();
        let window = verse_window(db.conn(), psalm_3_1, 1, 1).unwrap();
        let sids: Vec<&str> = window.verses.iter().map(|v| v.sid.as_str()).collect();
        assert_eq!(sids, ["PSA 2:12", "PSA 3:1", "PSA 3:2"]);
        assert_eq!(window.headings.len(), 1);
        assert_eq!(window.headings[0].after_verse_id, psalm_3_1 - 1);
        assert_eq!(window.headings[0].style, "d");
    }

    #[test]
    fn verse_window_stops_at_the_start_of_the_module() {
        let window = verse_window(kjv().conn(), 1, 40, 1).unwrap();
        assert_eq!(window.verses[0].sid, "GEN 1:1");
    }

    #[test]
    fn reference_ranges_return_verses_in_order() {
        let db = kjv();
        let ranges = [
            range("ROM 8:1", Some("ROM 8:2")),
            range("JHN 3:16", Some("JHN 3:18")),
        ];
        let sids = ref_sids(db.conn(), &ranges).unwrap();
        assert_eq!(
            sids,
            ["JHN 3:16", "JHN 3:17", "JHN 3:18", "ROM 8:1", "ROM 8:2"]
        );
    }

    #[test]
    fn reference_ranges_cross_books() {
        let sids = ref_sids(kjv().conn(), &[range("JHN 21:25", Some("ACT 1:1"))]).unwrap();
        assert_eq!(sids, ["JHN 21:25", "ACT 1:1"]);
    }

    #[test]
    fn reference_pages_carry_their_position() {
        let db = kjv();
        let ranges = [range("JHN 3:16", Some("JHN 3:21"))];
        let page = ref_verses(db.conn(), &ranges, 2, 3).unwrap();
        let found: Vec<(i64, &str)> = page.iter().map(|v| (v.idx, v.verse.sid.as_str())).collect();
        assert_eq!(found, [(2, "JHN 3:18"), (3, "JHN 3:19"), (4, "JHN 3:20")]);
    }

    #[test]
    fn reference_text_cannot_change_the_query() {
        let db = kjv();
        let sneaky = range("JHN 3:16') OR 1=1 --", None);
        assert!(ref_sids(db.conn(), &[sneaky]).is_err());
    }
}
