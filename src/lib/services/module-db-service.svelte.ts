import { invoke } from '@tauri-apps/api/core';
import type { Book } from '$lib/model/book';
import type {
  Heading,
  HeadingRow,
  Inline,
  Paragraph,
  ParagraphRow,
  Verse,
  VerseRow,
} from '$lib/model/verse';
import { calcLimitAndOffsetWithButter } from '$lib/services/calc-limit-and-offset';
import { osisToRanges, type RefRange } from '$lib/services/osis-to-ranges';
import { letterNotes, parseChildren } from '$lib/services/verse-content';

/** Paragraphs kept on each side of the active one. */
export const paraBuffer = 20;
/** Verses kept on each side of the active one. */
export const verseBuffer = 40;

/** The active item, with the items before and after it, in reading order. */
export interface TextWindow<T> {
  prev: T[];
  active: T | undefined;
  next: T[];
}

interface ModuleInfo {
  name: string;
  version: string;
  formatVersion: number;
}

interface ParagraphWindowRow {
  center: number;
  paragraphs: ParagraphRow[];
}

interface VerseWindowRow {
  center: number;
  verses: VerseRow[];
  headings: HeadingRow[];
}

function toHeading(row: HeadingRow): Heading {
  return { ...row, children: parseChildren<Inline>(row.children) };
}

function toVerse(row: VerseRow, headings: Heading[] = []): Verse {
  return {
    ...row,
    children: letterNotes(parseChildren(row.children), row.noteStart),
    headings,
  };
}

function toParagraph(row: ParagraphRow): Paragraph {
  return {
    ...row,
    heading: row.heading ? toHeading(row.heading) : undefined,
    verses: row.verses.map((verse) => toVerse(verse)),
  };
}

function split<T>(items: T[], position: (item: T) => number): TextWindow<T> {
  return {
    prev: items.filter((item) => position(item) < 0),
    active: items.find((item) => position(item) === 0),
    next: items.filter((item) => position(item) > 0),
  };
}

/**
 * Reads one module through the Rust commands in src-tauri/src/modules. The
 * page never opens database files or builds SQL itself.
 */
export default class ModuleDbService {
  constructor(private moduleId: string) {}

  public async getModuleName(): Promise<string> {
    const info = await invoke<ModuleInfo>('module_info', {
      moduleId: this.moduleId,
    });
    return info.name;
  }

  public async getBooks(): Promise<Record<string, Book>> {
    const books = await invoke<Book[]>('books', { moduleId: this.moduleId });
    return Object.fromEntries(books.map((book) => [book.code, book]));
  }

  /** The paragraphs around the one holding verse `sid`. */
  public async getParagraphsAtSid(sid: string): Promise<TextWindow<Paragraph>> {
    const window = await invoke<ParagraphWindowRow>('paragraph_window_at_sid', {
      moduleId: this.moduleId,
      sid,
      before: paraBuffer,
      after: paraBuffer,
    });
    return this.paragraphWindow(window);
  }

  /** The paragraphs around paragraph `paragraphId`. */
  public async getParagraphs(
    paragraphId: number
  ): Promise<TextWindow<Paragraph>> {
    const window = await invoke<ParagraphWindowRow>('paragraph_window', {
      moduleId: this.moduleId,
      center: paragraphId,
      before: paraBuffer,
      after: paraBuffer,
    });
    return this.paragraphWindow(window);
  }

  /** The verses around verse `sid`, each with any headings shown before it. */
  public async getVersesAtSid(sid: string): Promise<TextWindow<Verse>> {
    const window = await invoke<VerseWindowRow>('verse_window_at_sid', {
      moduleId: this.moduleId,
      sid,
      before: verseBuffer,
      after: verseBuffer,
    });
    return this.verseWindow(window);
  }

  /** The verses around verse `verseId`. */
  public async getVerses(verseId: number): Promise<TextWindow<Verse>> {
    const window = await invoke<VerseWindowRow>('verse_window', {
      moduleId: this.moduleId,
      center: verseId,
      before: verseBuffer,
      after: verseBuffer,
    });
    return this.verseWindow(window);
  }

  /** Every verse sid in an OSIS reference such as 'John.3.16-John.3.18'. */
  public async getSidsByRef(osisRef: string): Promise<string[]> {
    const ranges = osisToRanges(osisRef);
    if (ranges.length === 0) return [];
    return invoke<string[]>('ref_sids', { moduleId: this.moduleId, ranges });
  }

  /**
   * The verses around result number `index` within an OSIS reference, as
   * `prev`, `active`, and `next` by their position in the results.
   */
  public async getVersesByRef(
    osisRef: string,
    index: number = 0
  ): Promise<TextWindow<Verse>> {
    const ranges: RefRange[] = osisToRanges(osisRef);
    if (ranges.length === 0) return { prev: [], active: undefined, next: [] };
    const { limit, offset } = calcLimitAndOffsetWithButter(verseBuffer, index);
    const rows = await invoke<VerseRow[]>('ref_verses', {
      moduleId: this.moduleId,
      ranges,
      offset,
      limit,
    });
    return split(
      rows.map((row) => toVerse(row)),
      (verse) => (verse.idx ?? 0) - index
    );
  }

  private paragraphWindow(window: ParagraphWindowRow): TextWindow<Paragraph> {
    return split(
      window.paragraphs.map(toParagraph),
      (paragraph) => paragraph.id - window.center
    );
  }

  private verseWindow(window: VerseWindowRow): TextWindow<Verse> {
    // A heading shown before verse n has afterVerseId = n - 1.
    const headingsBefore: Record<number, Heading[]> = {};
    for (const heading of window.headings.map(toHeading)) {
      const verseId = heading.afterVerseId + 1;
      headingsBefore[verseId] = [...(headingsBefore[verseId] ?? []), heading];
    }
    return split(
      window.verses.map((row) => toVerse(row, headingsBefore[row.id])),
      (verse) => verse.id - window.center
    );
  }
}
