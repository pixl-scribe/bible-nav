import Database from '@tauri-apps/plugin-sql'; // requires svelte service
import { appDataDir } from '@tauri-apps/api/path';
import verseCounts from '../assets/bible-verse-counts.yaml';
import type { TestamentVerseCounts } from '$lib/model/bible-verse-counts';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';
import type { Book } from '$lib/model/book';

export type SearchType = 'ref' | 'ref-point' | 'text';
const defaultSearch = 'GEN 1:1';
const defaultSearchType: SearchType = 'ref-point';
const paraBuffer = 2;

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
  private getTextStart = $state<number | undefined>();
  public currentSearch = $state<string | undefined>(defaultSearch);
  public currentSearchType = $state<SearchType | undefined>(defaultSearchType);
  public prevParaBuffer = $state<Record<number, Verse[]>>({});
  public activePara = $state<Verse[]>([]);
  public nextParaBuffer = $state<Record<number, Verse[]>>({});
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
    const start = Date.now();
    this.getTextStart = start;
    if (currentSearchType === 'ref-point') {
      const paraId = await this.getParagraphIdBySid(currentSearch);
      const { prev, active, next } = await this.getVersesByParagraphs(paraId);

      // Only set results if this is the last call to getText.
      if (this.getTextStart === start) {
        this.prevParaBuffer = prev;
        this.activePara = active;
        this.nextParaBuffer = next;
      }
    }
  }
  private async getParagraphIdBySid(verseSid: string): Promise<number> {
    const db = await this.getDb();
    const result = await db.select<{ paragraph: number }[]>(
      'SELECT paragraph FROM verses WHERE sid = $1',
      [verseSid]
    );
    return result?.[0]?.paragraph ?? 0;
  }

  private async getVersesByParagraphs(paraId: number): Promise<{
    prev: Record<number, Verse[]>;
    active: Verse[];
    next: Record<number, Verse[]>;
  }> {
    const fromPara = paraId - paraBuffer;
    const thruPara = paraId + paraBuffer;

    const db = await this.getDb();
    const start = Date.now();
    const verseResults = await db.select<VerseRow[]>(
      `
          SELECT v.sid, v.paragraph, v.children
          FROM verses v
          WHERE v.paragraph BETWEEN $1 AND $2`,
      [fromPara, thruPara]
    );
    console.log(`sel ran in ${Date.now() - start}ms.`); // TODO: remove this

    const mapVerse = ({ sid, paragraph, children }: VerseRow): Verse => ({
      sid,
      paragraph,
      children: JSON.parse(children) as VerseChild[],
    });

    const prev = Object.groupBy(
      verseResults.filter((v) => v.paragraph < paraId).map(mapVerse),
      (v) => v.paragraph
    ) as Record<string, Verse[]>;
    const active = verseResults
      .filter((v) => v.paragraph === paraId)
      .map(mapVerse);
    const next = Object.groupBy(
      verseResults.filter((v) => v.paragraph > paraId).map(mapVerse),
      (v) => v.paragraph
    ) as Record<string, Verse[]>;
    return { prev, active, next };
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
