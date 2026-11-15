import { invoke } from '@tauri-apps/api/core';
import type { BookOutline } from './chapter-steps';
import type { Book } from '$lib/model/book';
import type {
  ChapterNote,
  Heading,
  HeadingRow,
  Inline,
  Paragraph,
  ParagraphRow,
  Verse,
  VerseRow,
} from '$lib/model/verse';
import { osisToRanges, type RefRange } from '$lib/services/osis-to-ranges';
import type { Chapter } from '$lib/services/chapter-text';
import {
  letterNotes,
  notesIn,
  parseChildren,
} from '$lib/services/verse-content';

interface ModuleInfo {
  name: string;
  /** The short name for tabs and menus, such as "KJV". */
  abbreviation: string;
  version: string;
  formatVersion: number;
}

interface ChapterRow {
  sid: string;
  bookCode: string;
  chapter: number;
  paragraphs: ParagraphRow[];
}

function toHeading(row: HeadingRow): Heading {
  return { ...row, children: parseChildren<Inline>(row.children) };
}

function toVerse(row: VerseRow): Verse {
  return {
    ...row,
    children: letterNotes(parseChildren(row.children), row.noteStart),
    headingsBefore: row.headingsBefore.map(toHeading),
    headingsAfter: row.headingsAfter.map(toHeading),
  };
}

function toParagraph(row: ParagraphRow): Paragraph {
  return {
    ...row,
    heading: row.heading ? toHeading(row.heading) : undefined,
    verses: row.verses.map((verse) => toVerse(verse)),
  };
}

/**
 * Reads one module through the Rust commands in src-tauri/src/modules. The
 * page never opens database files or builds SQL itself.
 */
export default class ModuleDbService {
  constructor(private moduleId: string) {}

  /** The module's display name, abbreviation, and version, such as "1.1.0". */
  public async getModuleInfo(): Promise<{
    name: string;
    abbreviation: string;
    version: string;
  }> {
    const info = await invoke<ModuleInfo>('module_info', {
      moduleId: this.moduleId,
    });
    return {
      name: info.name,
      abbreviation: info.abbreviation,
      version: info.version,
    };
  }

  /**
   * The verse to show for `sid` in this module: `sid`, or the nearest
   * verse the module has, or undefined if it lacks the book.
   */
  public async nearestSid(sid: string): Promise<string | undefined> {
    return (
      (await invoke<string | null>('nearest_sid', {
        moduleId: this.moduleId,
        sid,
      })) ?? undefined
    );
  }

  /** Every book's chapters and their verse counts, in order. */
  public async getOutline(): Promise<BookOutline[]> {
    return invoke<BookOutline[]>('book_outline', { moduleId: this.moduleId });
  }

  public async getBooks(): Promise<Record<string, Book>> {
    const books = await invoke<Book[]>('books', { moduleId: this.moduleId });
    return Object.fromEntries(books.map((book) => [book.code, book]));
  }

  /**
   * One whole chapter: the chapter holding verse `sid`, or `offset`
   * chapters after (positive) or before (negative) it. Undefined past
   * either end of the module.
   */
  public async getChapter(
    sid: string,
    offset = 0
  ): Promise<Chapter | undefined> {
    const row = await invoke<ChapterRow | null>('chapter_at', {
      moduleId: this.moduleId,
      sid,
      offset,
    });
    if (!row) return undefined;
    return {
      sid: row.sid,
      bookCode: row.bookCode,
      chapter: row.chapter,
      paragraphs: row.paragraphs.map(toParagraph),
    };
  }

  /**
   * A page of the verses in an OSIS reference: `limit` of them from result
   * number `offset`, each with any headings around it.
   */
  public async getRefVerses(
    osisRef: string,
    offset: number,
    limit: number
  ): Promise<Verse[]> {
    const ranges: RefRange[] = osisToRanges(osisRef);
    if (ranges.length === 0) return [];
    const rows = await invoke<VerseRow[]>('ref_verses', {
      moduleId: this.moduleId,
      ranges,
      offset,
      limit,
    });
    return rows.map((row) => toVerse(row));
  }

  /** Every verse matching a word search, in Bible order. */
  public async getTextSids(search: string): Promise<string[]> {
    return invoke<string[]>('text_sids', { moduleId: this.moduleId, search });
  }

  /**
   * A page of a word search's verses: `limit` of them from result number
   * `offset`, each with the words that matched in it.
   */
  public async getTextVerses(
    search: string,
    offset: number,
    limit: number
  ): Promise<Verse[]> {
    const rows = await invoke<VerseRow[]>('text_verses', {
      moduleId: this.moduleId,
      search,
      offset,
      limit,
    });
    return rows.map((row) => toVerse(row));
  }

  /**
   * The verses with these sids, in the order given, leaving out any this
   * module doesn't have: a parallel text showing the main text's results.
   */
  public async getVersesForSids(sids: string[]): Promise<Verse[]> {
    if (sids.length === 0) return [];
    const rows = await invoke<VerseRow[]>('verses_for_sids', {
      moduleId: this.moduleId,
      sids,
    });
    return rows.map((row) => toVerse(row));
  }

  /** Words beginning with `prefix`, with how many verses have each. */
  public async getWordSuggestions(
    prefix: string,
    limit: number
  ): Promise<{ word: string; verses: number }[]> {
    return invoke('word_suggestions', {
      moduleId: this.moduleId,
      prefix,
      limit,
    });
  }

  /** Every verse sid in an OSIS reference such as 'John.3.16-John.3.18'. */
  public async getSidsByRef(osisRef: string): Promise<string[]> {
    const ranges = osisToRanges(osisRef);
    if (ranges.length === 0) return [];
    return invoke<string[]>('ref_sids', { moduleId: this.moduleId, ranges });
  }

  /**
   * Every footnote in the chapter holding verse `sid`, in reading order and
   * lettered as in the reading text.
   */
  public async getChapterNotes(sid: string): Promise<ChapterNote[]> {
    const rows = await invoke<VerseRow[]>('chapter_notes', {
      moduleId: this.moduleId,
      sid,
    });
    return rows.flatMap((row) =>
      notesIn(toVerse(row).children).map((note) => ({
        sid: row.sid,
        letter: note.letter ?? '',
        note,
      }))
    );
  }
}
