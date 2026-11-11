/**
 * Verse content in module format version 1. Matches
 * bible-nav-modules/src/model/book.ts; see the module format contract.
 */

/**
 * A word with a Strong's number. See https://ubsicap.github.io/usx/charstyles.html#w
 */
export interface GlossaryWord {
  style: 'w';
  strong: string;
  txt: string;
}

/**
 * Styled text that can nest other inline content. add: translator's
 * addition, nd: name of deity, tl: transliterated word, wj: words of Jesus.
 */
export interface StyledText {
  style: 'add' | 'nd' | 'tl' | 'wj';
  children: Inline[];
}

export type Inline = string | GlossaryWord | StyledText;

export type NoteSegmentStyle = 'fr' | 'ft' | 'fk' | 'fq' | 'fqa' | 'fl' | 'fw';

export interface NoteSegment {
  style: NoteSegmentStyle;
  children: Inline[];
}

/** A footnote. Only verses have them. */
export interface Note {
  style: 'f';
  children: NoteSegment[];
  /** Its letter within the chapter (a, b, ... z, aa, ...), added when loaded. */
  letter?: string;
}

export type VerseChild = Inline | Note;

/** Text between verses: a Psalm title (d) or a section heading or note (s1). */
export interface Heading {
  id: number;
  bookId: number;
  chapter: number;
  paragraph: number;
  /** Shown before the verse whose id is afterVerseId + 1. */
  afterVerseId: number;
  style: string;
  children: Inline[];
}

export interface Verse {
  /** Position within reference search results. */
  idx?: number;
  id: number;
  bookId: number;
  chapter: number;
  nbr: number;
  paragraph: number;
  sid: string;
  children: VerseChild[];
  /** How many footnotes come before this verse in its chapter. */
  noteStart: number;
  /**
   * Headings shown just before this verse, such as a Psalm title before
   * verse 1, and just after it, such as an epistle's closing note. Rust
   * places them by the heading's own chapter. Verse-by-verse text only;
   * paragraph text carries headings as paragraphs.
   */
  headingsBefore: Heading[];
  headingsAfter: Heading[];
}

/**
 * A paragraph with its layout style: p (prose), q1 (poetic line), b (blank
 * line between stanzas), or d / s1 (holds a heading).
 */
export interface Paragraph {
  id: number;
  bookId: number;
  chapter: number;
  style: string;
  heading?: Heading;
  verses: Verse[];
}

/** A footnote as listed in the Notes tab: its verse, letter, and note. */
export interface ChapterNote {
  sid: string;
  letter: string;
  note: Note;
}

// Rows as the Rust commands return them, with children still as JSON text.

export interface VerseRow {
  idx?: number;
  id: number;
  bookId: number;
  chapter: number;
  nbr: number;
  paragraph: number;
  sid: string;
  children: string;
  noteStart: number;
  headingsBefore: HeadingRow[];
  headingsAfter: HeadingRow[];
}

export interface HeadingRow {
  id: number;
  bookId: number;
  chapter: number;
  paragraph: number;
  afterVerseId: number;
  style: string;
  children: string;
}

export interface ParagraphRow {
  id: number;
  bookId: number;
  chapter: number;
  style: string;
  heading: HeadingRow | null;
  verses: VerseRow[];
}
