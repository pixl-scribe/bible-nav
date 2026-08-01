import Database from '@tauri-apps/plugin-sql'; // requires svelte service
import { appDataDir } from '@tauri-apps/api/path';
import verseCounts from '../assets/bible-verse-counts.yaml';
import type { TestamentVerseCounts } from '$lib/model/bible-verse-counts';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';
import type { Book } from '$lib/model/book';

export type SearchType = 'ref' | 'ref-point' | 'text';
const defaultSearch = 'GEN 1:1';
const defaultSearchType: SearchType = 'ref-point';

function getTestamentChapterSids(
  testamentVerseCounts: TestamentVerseCounts
): string[] {
  return Object.keys(testamentVerseCounts).flatMap((bookCode: string) => {
    return Array.from(
      { length: testamentVerseCounts[bookCode].length },
      (_, index) => `${bookCode} ${index + 1}`
    );
  });
}
const allChapterSids = [
  ...getTestamentChapterSids(verseCounts.OT),
  ...getTestamentChapterSids(verseCounts.NT),
];

export default class ModuleService {
  private db: Database | undefined;
  public currentSearch = $state<string | undefined>(defaultSearch);
  public currentSearchType = $state<SearchType | undefined>(defaultSearchType);
  public verses = $state<Verse[]>([]);
  public books = $state<Record<string, Book>>({});

  constructor(private moduleId: string) {
    this.getBooks().then();
    $effect(() => {
      const currentSearch = this.currentSearch ?? defaultSearch;
      const currentSearchType = this.currentSearchType ?? defaultSearchType;
      this.getText(currentSearch, currentSearchType).then();
    });
  }

  private async getBooks() {
    const db = await this.getDb();
    const books = await db.select<Book[]>('SELECT * FROM books');
    this.books = Object.fromEntries(books.map((book) => [book.code, book]));
  }

  private async getText(currentSearch: string, currentSearchType: SearchType) {
    if (currentSearchType === 'ref-point') {
      const currentChapterSid = ModuleService.getChapterSid(currentSearch);
      const prior = ModuleService.getPriorChapterSids(currentChapterSid, 3);
      const next = ModuleService.getNextChapterSids(currentChapterSid, 3);
      this.verses = await this.getChapters([
        ...prior,
        currentChapterSid,
        ...next,
      ]);
      // console.log({ prior, currentChapterSid, next, verses: this.verses });
    }
  }

  private async getChapters(chapterSids: string[]): Promise<Verse[]> {
    const db = await this.getDb();

    const placeholders = chapterSids
      .map((_, index) => `$${index + 1}`)
      .join(', ');

    const start = Date.now();
    const verseResults = await db.select<VerseRow[]>(
      `
      SELECT v.sid, v.paragraph, v.children
      FROM chapters c JOIN main.verses v ON c.bookId = v.bookId AND c.nbr = v.chapter
      WHERE c.sid in (${placeholders})`,
      chapterSids
    );
    console.log(`sel ran in ${Date.now() - start}ms.`);
    return verseResults.map(({ sid, paragraph, children }) => ({
      sid,
      paragraph,
      children: JSON.parse(children) as VerseChild[],
    }));
  }

  private async getDb(): Promise<Database> {
    if (this.db !== undefined) {
      return this.db;
    }
    const dbPath = await this.getDbPath();
    try {
      this.db = await Database.load(dbPath);
      await this.db.execute('PRAGMA query_only = 1');
      await this.db.execute('PRAGMA journal_mode = OFF');
      await this.db.execute('PRAGMA synchronous = OFF');
      return this.db;
    } catch (err) {
      console.error(`Error connecting to ${dbPath}`, err);
      throw err;
    }
  }

  private async getDbPath(): Promise<string> {
    const appData = await appDataDir();
    return `sqlite:${appData}/modules/${this.moduleId}.db?immutable=1&mode=ro`;
  }

  /**
   * Converts verse refs (e.g. 'GEN 1:1') to chapter refs (e.g. 'GEN 1')
   */
  private static getChapterSid(refPoint: string) {
    return refPoint.split(':')?.[0];
  }

  private static getPriorChapterSids(
    currentSid: string,
    count: number
  ): string[] {
    const index = allChapterSids.indexOf(currentSid);
    if (index < 0) return [];
    const start = Math.max(0, index - count);
    return allChapterSids.slice(start, index);
  }

  private static getNextChapterSids(
    currentSid: string,
    count: number
  ): string[] {
    const index = allChapterSids.indexOf(currentSid);
    if (index < 0) return [];
    const start = index + 1;
    const end = start + count;
    return allChapterSids.slice(start, end);
  }
}
