import Database from '@tauri-apps/plugin-sql'; // requires svelte service
import { appDataDir } from '@tauri-apps/api/path';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';
import type { Book } from '$lib/model/book';

export type SearchType = 'ref' | 'ref-point' | 'text';
const defaultSearch = 'GEN 1:1';
const defaultSearchType: SearchType = 'ref-point';
const paraBuffer = 2;

export default class ModuleService {
  private db: Database | undefined;
  private getTextStart = $state<number | undefined>();
  public currentSearch = $state<string | undefined>(defaultSearch);
  public currentSearchType = $state<SearchType | undefined>(defaultSearchType);
  public prevParaBuffer = $state<Record<number, Verse[]>>({});
  public activePara = $state<Verse[]>([]);
  public nextParaBuffer = $state<Record<number, Verse[]>>({});
  public books = $state<Record<string, Book>>({});
  public scrollToSid = $state<string | undefined>();

  constructor(private moduleId: string) {
    this.getBooks().then();
    $effect(() => {
      const currentSearch = this.currentSearch ?? defaultSearch;
      const currentSearchType = this.currentSearchType ?? defaultSearchType;
      this.getText(currentSearch, currentSearchType).then();
    });
  }

  /**
   * Converts verse refs (e.g. 'GEN 1:1') to formatted ref (e.g. 'Genesis 1:1')
   */
  public formatRefFromSid(sid: string) {
    const [bookId, verseRef] = sid.split(' ');
    if (!bookId || !verseRef) return '';
    const book = this.books[bookId];
    if (!book) return '';
    return `${book.toc2} ${verseRef}`;
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
        this.scrollToSid = currentSearch;
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
          SELECT v.sid, v.nbr, v.paragraph, v.children
          FROM verses v
          WHERE v.paragraph BETWEEN $1 AND $2`,
      [fromPara, thruPara]
    );
    console.log(`sel ran in ${Date.now() - start}ms.`); // TODO: remove this

    const mapVerse = ({ sid, nbr, paragraph, children }: VerseRow): Verse => ({
      sid,
      nbr,
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
}
