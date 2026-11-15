//! Word search, over the module's full-text index (`verses_fts`, FTS5 with
//! the porter stemmer, so "covenant" also finds "covenants").
//!
//! What the reader types is never passed to FTS5 as query syntax. It's
//! turned into quoted strings by `fts_query`: each bare word becomes its
//! own quoted string, and text in quotes stays together as a phrase, so
//! `faith hope` finds verses with both words (in any order) and
//! `"in the beginning"` finds that phrase. Words such as OR, NOT, or NEAR,
//! and characters such as `*` or `(`, are then just words to look for, and
//! the result is still bound as a query parameter.

use std::collections::HashMap;

use rusqlite::{params, params_from_iter, Connection};
use serde::Serialize;

use super::error::Result;
use super::queries::{
    assign_note_starts, attach_headings, verse_from_row, IndexedVerse, VerseRow, VERSE_COLUMNS,
};

/// The FTS5 query for what the reader typed, or None if it holds no words.
pub fn fts_query(search: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    // A string is only worth searching for if it holds a letter or digit.
    let mut finish = |text: &mut String| {
        if text.chars().any(char::is_alphanumeric) {
            // Inside an FTS5 string a double quote is written twice. None
            // can be here (they end the string), but this keeps it safe.
            parts.push(format!("\"{}\"", text.trim().replace('"', "\"\"")));
        }
        text.clear();
    };
    for ch in search.chars() {
        match ch {
            '"' | '\u{201C}' | '\u{201D}' => {
                finish(&mut current);
                in_quotes = !in_quotes;
            }
            ch if ch.is_whitespace() && !in_quotes => finish(&mut current),
            ch => current.push(ch),
        }
    }
    finish(&mut current);
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// The sid of every verse matching the search, in Bible order. Even a very
/// common word ("the", about 28,000 verses) is only a list of references.
pub fn text_sids(conn: &Connection, search: &str) -> Result<Vec<String>> {
    let Some(query) = fts_query(search) else {
        return Ok(Vec::new());
    };
    let mut statement = conn.prepare_cached(
        "SELECT v.sid FROM verses_fts JOIN verses v ON v.id = verses_fts.rowid
         WHERE verses_fts MATCH ?1 ORDER BY v.id",
    )?;
    let sids = statement
        .query_map([query], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(sids)
}

/// One page of the verses matching the search: `limit` verses from result
/// number `offset`, each with its result number (`idx`) and the words that
/// matched in it (`matches`).
pub fn text_verses(
    conn: &Connection,
    search: &str,
    offset: u32,
    limit: u32,
) -> Result<Vec<IndexedVerse>> {
    let Some(query) = fts_query(search) else {
        return Ok(Vec::new());
    };
    let columns = VERSE_COLUMNS
        .split(", ")
        .map(|column| format!("v.{column}"))
        .collect::<Vec<_>>()
        .join(", ");
    // highlight() marks each matched word with \u{1} before and \u{2} after.
    let mut statement = conn.prepare(&format!(
        "SELECT {columns}, highlight(verses_fts, 0, char(1), char(2))
         FROM verses_fts JOIN verses v ON v.id = verses_fts.rowid
         WHERE verses_fts MATCH ?1 ORDER BY v.id LIMIT ?2 OFFSET ?3"
    ))?;
    let mut verses = statement
        .query_map(params![query, limit, offset], |row| {
            let mut verse = verse_from_row(row)?;
            let marked: String = row.get(7)?;
            verse.matches = marked_words(&marked);
            Ok(verse)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assign_note_starts(conn, verses.iter_mut())?;
    attach_headings(conn, &mut verses)?;
    Ok(verses
        .into_iter()
        .zip(i64::from(offset)..)
        .map(|(verse, idx)| IndexedVerse { idx, verse })
        .collect())
}

/// The words between highlight()'s markers, each once, in order.
fn marked_words(marked: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for piece in marked.split('\u{1}').skip(1) {
        if let Some((word, _)) = piece.split_once('\u{2}') {
            if !words.iter().any(|known| known.eq_ignore_ascii_case(word)) {
                words.push(word.to_string());
            }
        }
    }
    words
}

/// The verses with these sids, in the order given, leaving out any this
/// module doesn't have: a parallel text showing the main text's search
/// results in another Bible. Meant for a page at a time (a hundred or so).
pub fn verses_for_sids(conn: &Connection, sids: &[String]) -> Result<Vec<VerseRow>> {
    if sids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; sids.len()].join(", ");
    let mut statement = conn.prepare(&format!(
        "SELECT {VERSE_COLUMNS} FROM verses WHERE sid IN ({placeholders})"
    ))?;
    let mut by_sid: HashMap<String, VerseRow> = statement
        .query_map(params_from_iter(sids), verse_from_row)?
        .map(|verse| verse.map(|verse| (verse.sid.clone(), verse)))
        .collect::<rusqlite::Result<_>>()?;
    let mut verses: Vec<VerseRow> = sids.iter().filter_map(|sid| by_sid.remove(sid)).collect();
    assign_note_starts(conn, verses.iter_mut())?;
    attach_headings(conn, &mut verses)?;
    Ok(verses)
}

/// A word offered while the reader types, and how many verses have it.
#[derive(Debug, Serialize)]
pub struct WordSuggestion {
    pub word: String,
    pub verses: u32,
}

/// Words in the module beginning with `prefix` (as written, unstemmed,
/// from `verses_vocab`), the exact word first, then in order.
pub fn word_suggestions(
    conn: &Connection,
    prefix: &str,
    limit: u32,
) -> Result<Vec<WordSuggestion>> {
    let prefix = prefix.trim().to_lowercase();
    if prefix.is_empty() {
        return Ok(Vec::new());
    }
    // Every word starting with the prefix sorts between the prefix and the
    // prefix followed by the highest character.
    let end = format!("{prefix}\u{10FFFF}");
    let mut statement = conn.prepare_cached(
        "SELECT term, doc FROM verses_vocab
         WHERE term >= ?1 AND term < ?2
         ORDER BY term <> ?1, term LIMIT ?3",
    )?;
    let words = statement
        .query_map(params![prefix, end, limit], |row| {
            Ok(WordSuggestion {
                word: row.get(0)?,
                verses: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::kjv;

    #[test]
    fn typed_words_become_quoted_strings() {
        assert_eq!(fts_query("covenant").as_deref(), Some("\"covenant\""));
        assert_eq!(
            fts_query("  faith   hope ").as_deref(),
            Some("\"faith\" \"hope\"")
        );
        assert_eq!(
            fts_query("\"in the beginning\" God").as_deref(),
            Some("\"in the beginning\" \"God\"")
        );
        assert_eq!(fts_query("“job”").as_deref(), Some("\"job\""));
        assert_eq!(fts_query("").as_deref(), None);
        assert_eq!(fts_query(" -- ** ").as_deref(), None);
    }

    #[test]
    fn query_syntax_is_only_words() {
        // Operators and stray characters can't change or break the query.
        assert_eq!(
            fts_query("covenant OR NEAR(x").as_deref(),
            Some("\"covenant\" \"OR\" \"NEAR(x\"")
        );
        let db = kjv();
        for search in ["covenant OR NEAR(x", "a\"b", "^(*", "\"unclosed phrase"] {
            text_sids(db.conn(), search).expect("never a syntax error");
        }
    }

    #[test]
    fn finds_every_verse_with_a_word_or_its_forms() {
        let sids = text_sids(kjv().conn(), "covenant").unwrap();
        assert_eq!(sids.len(), 279);
        assert_eq!(sids[0], "GEN 6:18");
        assert!(sids.contains(&"HEB 13:20".to_string()));
    }

    #[test]
    fn all_words_must_be_there_and_quotes_keep_a_phrase() {
        let db = kjv();
        let both = text_sids(db.conn(), "faith hope").unwrap();
        assert!(both.contains(&"1CO 13:13".to_string()));
        assert!(both.len() < text_sids(db.conn(), "faith").unwrap().len());
        let phrase = text_sids(db.conn(), "\"in the beginning\"").unwrap();
        assert!(phrase.contains(&"GEN 1:1".to_string()));
        assert!(phrase.contains(&"JHN 1:1".to_string()));
    }

    #[test]
    fn pages_carry_their_position_and_matched_words() {
        let page = text_verses(kjv().conn(), "covenant", 1, 2).unwrap();
        let found: Vec<(i64, &str)> = page.iter().map(|v| (v.idx, v.verse.sid.as_str())).collect();
        assert_eq!(found, [(1, "GEN 9:9"), (2, "GEN 9:11")]);
        assert_eq!(page[0].verse.matches, ["covenant"]);
        let plural = text_verses(kjv().conn(), "covenant", 0, 300).unwrap();
        assert!(plural
            .iter()
            .any(|v| v.verse.matches.iter().any(|w| w == "covenants")));
    }

    #[test]
    fn verses_come_by_sid_in_the_order_given() {
        let sids = ["JHN 3:16", "GEN 1:1", "XYZ 1:1"].map(String::from);
        let verses = verses_for_sids(kjv().conn(), &sids).unwrap();
        let found: Vec<&str> = verses.iter().map(|v| v.sid.as_str()).collect();
        assert_eq!(found, ["JHN 3:16", "GEN 1:1"]);
    }

    #[test]
    fn suggests_words_by_prefix_with_their_verse_counts() {
        let words = word_suggestions(kjv().conn(), "Job", 5).unwrap();
        let found: Vec<(&str, u32)> = words.iter().map(|w| (w.word.as_str(), w.verses)).collect();
        assert_eq!(found, [("job", 54), ("jobab", 9)]);
        assert!(word_suggestions(kjv().conn(), " ", 5).unwrap().is_empty());
    }
}
