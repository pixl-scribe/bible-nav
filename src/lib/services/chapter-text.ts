/**
 * Chapters, the unit of infinite scrolling. A chapter comes whole from Rust
 * (`chapter_at`) as paragraphs, each holding a heading or verses; both
 * groupings are drawn from the same chapter, so switching grouping needs
 * no reload.
 */
import type { Heading, Paragraph, Verse } from '$lib/model/verse';

export interface Chapter {
  /** The chapter's id, such as 'GEN 1'. */
  sid: string;
  bookCode: string;
  chapter: number;
  paragraphs: Paragraph[];
}

/**
 * A chapter's verses for grouping by verse: each verse carries the headings
 * shown before it (a Psalm title before verse 1), and the last verse those
 * after it (an epistle's closing note).
 */
export function chapterVerses(chapter: Chapter): Verse[] {
  const verses: Verse[] = [];
  let pending: Heading[] = [];
  for (const paragraph of chapter.paragraphs) {
    if (paragraph.heading) {
      pending.push(paragraph.heading);
      continue;
    }
    for (const verse of paragraph.verses) {
      verses.push({ ...verse, headingsBefore: pending, headingsAfter: [] });
      pending = [];
    }
  }
  const last = verses[verses.length - 1];
  if (last && pending.length > 0) {
    verses[verses.length - 1] = { ...last, headingsAfter: pending };
  }
  return verses;
}

/** The chapter of a verse: 'PSA 12:3' is in 'PSA 12'. */
export function chapterOfSid(sid: string): string {
  return sid.split(':')[0];
}

/** The first verse of a chapter, such as 'GEN 1:1' for 'GEN 1'. */
export function firstVerseOf(chapterSid: string): string {
  return `${chapterSid}:1`;
}
