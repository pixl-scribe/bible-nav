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
    /// The short name for tabs and menus, such as "KJV": the module's own,
    /// or for modules made before it had one, guessed from the name.
    pub abbreviation: String,
    pub description: String,
    pub module_type: String,
    /// The module's id, such as "eng_kjv".
    pub fetch_id: Option<String>,
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
    /// How many footnotes come before this verse in its chapter, so the
    /// page can letter them (a, b, ... restarting each chapter) even when
    /// the loaded text starts partway through a chapter.
    pub note_start: u32,
    /// Headings shown just before this verse, such as a Psalm title before
    /// verse 1. Filled in for verse-by-verse text (verse windows and
    /// reference results); paragraph windows carry headings as paragraphs.
    pub headings_before: Vec<HeadingRow>,
    /// Headings shown just after this verse, such as an epistle's closing
    /// note after its last verse.
    pub headings_after: Vec<HeadingRow>,
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

/// One whole chapter, for infinite scrolling: its paragraphs in reading
/// order, each with its heading or verses. Paragraphs never cross chapters,
/// so both groupings (paragraph and verse) draw from this.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterText {
    /// The chapter's id, such as "GEN 1".
    pub sid: String,
    pub book_code: String,
    pub chapter: i64,
    pub paragraphs: Vec<ParagraphRow>,
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
        note_start: 0,
        headings_before: Vec::new(),
        headings_after: Vec::new(),
    })
}

/// Counts the footnotes in stored verse content (a JSON array).
pub fn count_notes(children: &str) -> u32 {
    fn count(value: &serde_json::Value) -> u32 {
        match value {
            serde_json::Value::Array(items) => items.iter().map(count).sum(),
            serde_json::Value::Object(item) => {
                if item.get("style").and_then(|style| style.as_str()) == Some("f") {
                    1
                } else {
                    item.get("children").map(count).unwrap_or(0)
                }
            }
            _ => 0,
        }
    }
    serde_json::from_str(children)
        .map(|value| count(&value))
        .unwrap_or(0)
}

/// Fills in each verse's `note_start`, for verses given in reading order.
///
/// Where a verse doesn't directly follow the previous one (a new window, a
/// new chapter, or a gap between reference ranges), the footnotes earlier in
/// its chapter are counted from the module, so letters never depend on which
/// part of the chapter happens to be loaded.
fn assign_note_starts<'a>(
    conn: &Connection,
    verses: impl IntoIterator<Item = &'a mut VerseRow>,
) -> Result<()> {
    let mut previous: Option<(i64, i64, i64)> = None;
    let mut count = 0;
    for verse in verses {
        let follows = previous.is_some_and(|(book, chapter, id)| {
            book == verse.book_id && chapter == verse.chapter && id + 1 == verse.id
        });
        if !follows {
            count = notes_before(conn, verse)?;
        }
        verse.note_start = count;
        count += count_notes(&verse.children);
        previous = Some((verse.book_id, verse.chapter, verse.id));
    }
    Ok(())
}

/// The number of footnotes in the verses before `verse` in its chapter.
fn notes_before(conn: &Connection, verse: &VerseRow) -> Result<u32> {
    let mut statement = conn.prepare_cached(
        "SELECT children FROM verses WHERE bookId = ?1 AND chapter = ?2 AND id < ?3",
    )?;
    let mut total = 0;
    for children in statement.query_map(params![verse.book_id, verse.chapter, verse.id], |row| {
        row.get::<_, String>(0)
    })? {
        total += count_notes(&children?);
    }
    Ok(total)
}

/// Gives each verse the headings shown just before or after it.
///
/// A heading's `afterVerseId` is the verse it follows in the module, which
/// may be in an earlier chapter: a Psalm title follows the previous psalm's
/// last verse. So a heading goes before the next verse if that verse is in
/// the heading's own chapter (a Psalm title before verse 1), and otherwise
/// after the verse it follows (an epistle's closing note after its last
/// verse). Verses may be any selection, such as reference results with gaps.
fn attach_headings(conn: &Connection, verses: &mut [VerseRow]) -> Result<()> {
    let (Some(first), Some(last)) = (
        verses.iter().map(|verse| verse.id).min(),
        verses.iter().map(|verse| verse.id).max(),
    ) else {
        return Ok(());
    };
    // Where each verse is in `verses`, by verse id.
    let index: std::collections::HashMap<i64, usize> = verses
        .iter()
        .enumerate()
        .map(|(position, verse)| (verse.id, position))
        .collect();
    let in_chapter = |verse: &VerseRow, heading: &HeadingRow| {
        verse.book_id == heading.book_id && verse.chapter == heading.chapter
    };

    let mut statement = conn.prepare(&format!(
        "SELECT {HEADING_COLUMNS} FROM headings WHERE afterVerseId BETWEEN ?1 AND ?2 ORDER BY id"
    ))?;
    let headings = statement
        .query_map(params![first - 1, last], heading_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for heading in headings {
        let next = index.get(&(heading.after_verse_id + 1)).copied();
        let previous = index.get(&heading.after_verse_id).copied();
        if let Some(position) = next.filter(|&p| in_chapter(&verses[p], &heading)) {
            verses[position].headings_before.push(heading);
        } else if let Some(position) = previous.filter(|&p| in_chapter(&verses[p], &heading)) {
            verses[position].headings_after.push(heading);
        }
    }
    Ok(())
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
    // Modules made before the abbreviation existed (such as eng_kjv 1.1.0)
    // don't have the column.
    let has_abbreviation: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_table_info('module') WHERE name = 'abbreviation')",
        [],
        |row| row.get(0),
    )?;
    let abbreviation = if has_abbreviation {
        "abbreviation"
    } else {
        "NULL"
    };
    let info = conn.query_row(
        &format!(
            "SELECT name, description, moduleType, fetchId, language, languageCode, license, version, formatVersion, {abbreviation}
             FROM module"
        ),
        [],
        |row| {
            let name: String = row.get(0)?;
            let abbreviation: Option<String> = row.get(9)?;
            Ok(ModuleInfo {
                abbreviation: abbreviation.unwrap_or_else(|| guess_abbreviation(&name)),
                name,
                description: row.get(1)?,
                module_type: row.get(2)?,
                fetch_id: row.get(3)?,
                language: row.get(4)?,
                language_code: row.get(5)?,
                license: row.get(6)?,
                version: row.get(7)?,
                format_version: row.get(8)?,
            })
        },
    )?;
    Ok(info)
}

/// An abbreviation for a module that has none: the capital initials of
/// its name, ignoring anything in brackets ("King James Version (1769)" is
/// "KJV"), or the whole name if that gives fewer than two.
pub fn guess_abbreviation(name: &str) -> String {
    let mut without_brackets = String::new();
    let mut depth = 0;
    for character in name.chars() {
        match character {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => without_brackets.push(character),
            _ => {}
        }
    }
    let initials: String = without_brackets
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .filter(|letter| letter.is_uppercase())
        .collect();
    if initials.chars().count() >= 2 {
        initials
    } else {
        name.trim().to_string()
    }
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
#[cfg(test)]
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

    assign_note_starts(
        conn,
        paragraphs
            .iter_mut()
            .flat_map(|paragraph| paragraph.verses.iter_mut()),
    )?;
    Ok(ParagraphWindow { center, paragraphs })
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

/// Builds the `WHERE` and `ORDER BY` parts for verses in the ranges, with
/// one pair of placeholders per range in each, and the matching parameter
/// values in the order the placeholders appear. Only placeholders go into
/// the SQL text; the values are bound separately.
///
/// The verses come in the order the reader typed the references ("Romans 8;
/// John 1" lists Romans first), and in canonical order within a range. A
/// verse in more than one range comes once, with the first range naming it.
/// So in `(id BETWEEN ? AND ?) OR (...)` the `CASE` below gives each verse
/// the number of its first range, and `id` orders within it.
fn range_filter(id_ranges: &[(i64, i64)]) -> (String, String, Vec<i64>) {
    let clause = vec!["(id BETWEEN ? AND ?)"; id_ranges.len()].join(" OR ");
    let cases: String = (0..id_ranges.len())
        .map(|position| format!(" WHEN id BETWEEN ? AND ? THEN {position}"))
        .collect();
    let order = format!("CASE{cases} END, id");
    let pairs = id_ranges.iter().flat_map(|&(from, to)| [from, to]);
    // Once for the WHERE clause, then again for the ORDER BY.
    let values = pairs.clone().chain(pairs).collect();
    (clause, order, values)
}

/// The sids of every verse in the ranges, in the order the ranges were given
/// (see `range_filter`).
pub fn ref_sids(conn: &Connection, ranges: &[RefRange]) -> Result<Vec<String>> {
    let ids = id_ranges(conn, ranges)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let (clause, order, values) = range_filter(&ids);
    let mut statement = conn.prepare(&format!(
        "SELECT sid FROM verses WHERE {clause} ORDER BY {order}"
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
    let (clause, order, mut values) = range_filter(&ids);
    values.push(i64::from(limit));
    values.push(i64::from(offset));
    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE {clause} ORDER BY {order} LIMIT ? OFFSET ?"
    ))?;
    let mut verses = statement
        .query_map(params_from_iter(values), verse_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assign_note_starts(conn, verses.iter_mut())?;
    attach_headings(conn, &mut verses)?;
    Ok(verses
        .into_iter()
        .zip(i64::from(offset)..)
        .map(|(verse, idx)| IndexedVerse { idx, verse })
        .collect())
}

/// The verses with footnotes in the chapter that holds verse `sid`, in
/// reading order, each with its `note_start` so the page letters the notes
/// exactly as the reading text does. This is what the Notes tab lists.
/// The verse to show for `sid` in this module: `sid` itself, or, if the
/// module lacks it, the nearest verse in the same chapter, else the
/// nearest chapter of the same book. `None` if the book isn't there. Used
/// when a reader switches Bibles, which may number verses differently.
pub fn nearest_sid(conn: &Connection, sid: &str) -> Result<Option<String>> {
    let Some((book, place)) = sid.split_once(' ') else {
        return Ok(None);
    };
    let (chapter, verse) = place.split_once(':').unwrap_or((place, "1"));
    let (Ok(chapter), Ok(verse)) = (chapter.parse::<i64>(), verse.parse::<i64>()) else {
        return Ok(None);
    };
    let found = conn
        .query_row(
            "SELECT v.sid FROM verses v JOIN books b ON b.id = v.bookId
             WHERE b.code = ?1
             ORDER BY abs(v.chapter - ?2), abs(v.nbr - ?3), v.nbr
             LIMIT 1",
            rusqlite::params![book, chapter, verse],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found)
}

/// The chapter holding verse `sid` (or the nearest verse this module has),
/// or the chapter `offset` chapters after it (negative: before it), in the
/// module's book order. `None` past the first or last chapter, or if the
/// book isn't in the module.
pub fn chapter_at(conn: &Connection, sid: &str, offset: i32) -> Result<Option<ChapterText>> {
    let Some(start) = nearest_sid(conn, sid)? else {
        return Ok(None);
    };
    let (mut book_id, mut chapter): (i64, i64) = conn.query_row(
        "SELECT bookId, chapter FROM verses WHERE sid = ?1",
        [&start],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    // Step one chapter at a time; a reader asks for neighbours, so offsets
    // are small.
    let step = if offset < 0 {
        "SELECT bookId, nbr FROM chapters
         WHERE bookId < ?1 OR (bookId = ?1 AND nbr < ?2)
         ORDER BY bookId DESC, nbr DESC LIMIT 1"
    } else {
        "SELECT bookId, nbr FROM chapters
         WHERE bookId > ?1 OR (bookId = ?1 AND nbr > ?2)
         ORDER BY bookId, nbr LIMIT 1"
    };
    for _ in 0..offset.unsigned_abs() {
        let next: Option<(i64, i64)> = conn
            .query_row(step, params![book_id, chapter], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        let Some(next) = next else {
            return Ok(None);
        };
        (book_id, chapter) = next;
    }
    chapter_text(conn, book_id, chapter).map(Some)
}

fn chapter_text(conn: &Connection, book_id: i64, chapter: i64) -> Result<ChapterText> {
    let (sid, book_code, first, last): (String, String, i64, i64) = conn.query_row(
        "SELECT c.sid, b.code, min(p.id), max(p.id)
         FROM chapters c JOIN books b ON b.id = c.bookId
         JOIN paragraphs p ON p.bookId = c.bookId AND p.chapter = c.nbr
         WHERE c.bookId = ?1 AND c.nbr = ?2",
        params![book_id, chapter],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let after = u32::try_from(last - first).unwrap_or(0);
    let window = paragraph_window(conn, first, 0, after)?;
    Ok(ChapterText {
        sid,
        book_code,
        chapter,
        paragraphs: window.paragraphs,
    })
}

/// Whether any verse has a footnote, for listing the module's Notes tab.
pub fn has_notes(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM verses WHERE instr(children, '\"style\":\"f\"') > 0)",
        [],
        |row| row.get(0),
    )?)
}

pub fn chapter_notes(conn: &Connection, sid: &str) -> Result<Vec<VerseRow>> {
    let (book_id, chapter): (i64, i64) = conn
        .query_row(
            "SELECT bookId, chapter FROM verses WHERE sid = ?1",
            [sid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| ModuleError::NotFound(format!("verse {sid}")))?;
    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE bookId = ?1 AND chapter = ?2 ORDER BY id"
    ))?;
    let mut verses = statement
        .query_map(params![book_id, chapter], verse_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assign_note_starts(conn, verses.iter_mut())?;
    // `retain` keeps only the verses for which the closure returns true.
    verses.retain(|verse| count_notes(&verse.children) > 0);
    Ok(verses)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verse_sids(chapter: &ChapterText) -> Vec<String> {
        chapter
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.verses.iter().map(|verse| verse.sid.clone()))
            .collect()
    }

    #[test]
    fn a_chapter_comes_whole_with_its_headings() {
        let db = crate::modules::test_support::kjv();
        let genesis = chapter_at(db.conn(), "GEN 1:5", 0).unwrap().unwrap();
        assert_eq!(genesis.sid, "GEN 1");
        assert_eq!(genesis.book_code, "GEN");
        let sids = verse_sids(&genesis);
        assert_eq!(sids.len(), 31);
        assert_eq!(sids.first().map(String::as_str), Some("GEN 1:1"));
        assert_eq!(sids.last().map(String::as_str), Some("GEN 1:31"));
        // Psalm 3's title is a heading paragraph before verse 1.
        let psalm = chapter_at(db.conn(), "PSA 3:1", 0).unwrap().unwrap();
        assert_eq!(
            psalm.paragraphs[0]
                .heading
                .as_ref()
                .map(|h| h.style.as_str()),
            Some("d")
        );
    }

    #[test]
    fn neighbouring_chapters_cross_books_and_stop_at_the_ends() {
        let db = crate::modules::test_support::kjv();
        let conn = db.conn();
        let sid = |sid: &str, offset: i32| chapter_at(conn, sid, offset).unwrap().map(|c| c.sid);
        assert_eq!(sid("MAL 4:6", 1).as_deref(), Some("MAT 1"));
        assert_eq!(sid("MAT 1:1", -1).as_deref(), Some("MAL 4"));
        assert_eq!(sid("GEN 1:1", -1), None);
        assert_eq!(sid("REV 22:21", 1), None);
        // Psalm 117 has two verses; its neighbours are found all the same.
        assert_eq!(sid("PSA 116:1", 1).as_deref(), Some("PSA 117"));
        assert_eq!(sid("PSA 116:1", 2).as_deref(), Some("PSA 118"));
        let short = chapter_at(conn, "PSA 117:1", 0).unwrap().unwrap();
        assert_eq!(verse_sids(&short), ["PSA 117:1", "PSA 117:2"]);
        assert_eq!(sid("XYZ 1:1", 0), None);
    }

    #[test]
    fn the_nearest_verse_stands_in_for_a_missing_one() {
        let db = crate::modules::test_support::kjv();
        let conn = db.conn();
        assert_eq!(
            nearest_sid(conn, "JHN 3:16").unwrap().as_deref(),
            Some("JHN 3:16")
        );
        // 3 John has 14 verses in the KJV.
        assert_eq!(
            nearest_sid(conn, "3JN 1:15").unwrap().as_deref(),
            Some("3JN 1:14")
        );
        // Malachi has 4 chapters; a Hebrew "MAL 5" is nearest MAL 4.
        assert_eq!(
            nearest_sid(conn, "MAL 5:1").unwrap().as_deref(),
            Some("MAL 4:1")
        );
        assert_eq!(nearest_sid(conn, "XYZ 1:1").unwrap(), None);
        assert_eq!(nearest_sid(conn, "nonsense").unwrap(), None);
    }

    #[test]
    fn the_kjv_has_notes() {
        let db = crate::modules::test_support::kjv();
        assert!(has_notes(db.conn()).unwrap());
    }

    #[test]
    fn guesses_an_abbreviation_from_capital_initials() {
        assert_eq!(guess_abbreviation("King James Version (1769)"), "KJV");
        assert_eq!(guess_abbreviation("Geneva Bible (1599)"), "GB");
        assert_eq!(guess_abbreviation("Literal Standard Version"), "LSV");
        assert_eq!(guess_abbreviation("textus receptus"), "textus receptus");
    }

    #[test]
    fn the_bundled_kjv_is_abbreviated_kjv() {
        let db = crate::modules::test_support::kjv();
        assert_eq!(module_info(db.conn()).unwrap().abbreviation, "KJV");
    }
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
        assert_eq!(
            info.format_version,
            crate::modules::db::SUPPORTED_FORMAT_VERSION
        );
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

    /// The styles of the headings before and after each verse, by sid.
    fn heading_places<'a>(
        verses: impl IntoIterator<Item = &'a VerseRow>,
    ) -> Vec<(String, Vec<&'a str>, Vec<&'a str>)> {
        verses
            .into_iter()
            .map(|verse| {
                (
                    verse.sid.clone(),
                    verse
                        .headings_before
                        .iter()
                        .map(|h| h.style.as_str())
                        .collect(),
                    verse
                        .headings_after
                        .iter()
                        .map(|h| h.style.as_str())
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn reference_results_carry_their_headings() {
        let db = kjv();
        let page = ref_verses(db.conn(), &[range("PSA 3:1", Some("PSA 3:2"))], 0, 10).unwrap();
        assert_eq!(
            heading_places(page.iter().map(|v| &v.verse)),
            [
                ("PSA 3:1".into(), vec!["d"], vec![]),
                ("PSA 3:2".into(), vec![], vec![])
            ]
        );
        // A search that starts after verse 1 leaves the title out.
        let page = ref_verses(db.conn(), &[range("PSA 3:2", None)], 0, 10).unwrap();
        assert!(page[0].verse.headings_before.is_empty());
        // A closing note shows after the last verse of its epistle.
        let page = ref_verses(db.conn(), &[range("ROM 16:27", None)], 0, 10).unwrap();
        assert_eq!(page[0].verse.headings_after.len(), 1);
    }

    #[test]
    fn reference_ranges_return_verses_in_the_order_typed() {
        let db = kjv();
        let ranges = [
            range("ROM 8:1", Some("ROM 8:2")),
            range("JHN 3:16", Some("JHN 3:18")),
        ];
        let sids = ref_sids(db.conn(), &ranges).unwrap();
        assert_eq!(
            sids,
            ["ROM 8:1", "ROM 8:2", "JHN 3:16", "JHN 3:17", "JHN 3:18"]
        );
        // Pages follow the same order, across the ranges.
        let page = ref_verses(db.conn(), &ranges, 1, 2).unwrap();
        let found: Vec<(i64, &str)> = page.iter().map(|v| (v.idx, v.verse.sid.as_str())).collect();
        assert_eq!(found, [(1, "ROM 8:2"), (2, "JHN 3:16")]);
    }

    #[test]
    fn a_verse_in_two_ranges_comes_once_where_first_named() {
        let ranges = [
            range("ROM 8:3", Some("ROM 8:5")),
            range("ROM 8:1", Some("ROM 8:4")),
        ];
        let sids = ref_sids(kjv().conn(), &ranges).unwrap();
        assert_eq!(
            sids,
            ["ROM 8:3", "ROM 8:4", "ROM 8:5", "ROM 8:1", "ROM 8:2"]
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

    /// Psalm 12's footnotes, lettered per chapter: a and b in verse 1, then
    /// c, d, e, f in verses 2 to 5, none in 6, g in 7, and h in 8.
    fn psalm_12_note_starts(verses: &[&VerseRow]) -> Vec<(i64, u32)> {
        verses
            .iter()
            .filter(|verse| verse.sid.starts_with("PSA 12:"))
            .map(|verse| (verse.nbr, verse.note_start))
            .collect()
    }

    #[test]
    fn counts_footnotes_in_stored_content() {
        assert_eq!(
            count_notes(r#"["a",{"style":"f","children":[{"style":"fr","children":["1.1"]}]}]"#),
            1
        );
        assert_eq!(
            count_notes(
                r#"[{"style":"wj","children":[{"style":"f","children":[]}]},{"style":"f","children":[]}]"#
            ),
            2
        );
        assert_eq!(count_notes(r#"["no notes"]"#), 0);
        assert_eq!(count_notes("not json"), 0);
    }

    #[test]
    fn note_starts_count_from_the_start_of_the_chapter() {
        let db = kjv();
        // A window that starts partway through Psalm 12, at verse 4.
        let center = paragraph_for_sid(db.conn(), "PSA 12:5").unwrap();
        let window = paragraph_window(db.conn(), center, 1, 3).unwrap();
        let verses: Vec<&VerseRow> = window
            .paragraphs
            .iter()
            .flat_map(|p| p.verses.iter())
            .collect();
        assert_eq!(
            psalm_12_note_starts(&verses),
            [(4, 4), (5, 5), (6, 6), (7, 6), (8, 7)]
        );
    }

    #[test]
    fn note_starts_survive_gaps_between_reference_ranges() {
        let db = kjv();
        let ranges = [range("PSA 12:1", None), range("PSA 12:5", Some("PSA 12:6"))];
        let page = ref_verses(db.conn(), &ranges, 0, 10).unwrap();
        let verses: Vec<&VerseRow> = page.iter().map(|v| &v.verse).collect();
        assert_eq!(psalm_12_note_starts(&verses), [(1, 0), (5, 5), (6, 6)]);
    }

    #[test]
    fn lists_a_chapters_notes_from_any_verse_in_it() {
        let db = kjv();
        let verses = chapter_notes(db.conn(), "PSA 12:6").unwrap();
        let verses: Vec<&VerseRow> = verses.iter().collect();
        // Verse 6 has no notes, so it isn't listed.
        assert_eq!(
            psalm_12_note_starts(&verses),
            [(1, 0), (2, 2), (3, 3), (4, 4), (5, 5), (7, 6), (8, 7)]
        );
        // The New Testament has no notes in this module.
        assert!(chapter_notes(db.conn(), "JHN 3:16").unwrap().is_empty());
        assert!(matches!(
            chapter_notes(db.conn(), "PSA 999:1"),
            Err(ModuleError::NotFound(_))
        ));
    }

    #[test]
    fn reference_text_cannot_change_the_query() {
        let db = kjv();
        let sneaky = range("JHN 3:16') OR 1=1 --", None);
        assert!(ref_sids(db.conn(), &[sneaky]).is_err());
    }
}
