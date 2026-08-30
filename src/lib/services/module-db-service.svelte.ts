import Database from '@tauri-apps/plugin-sql';
import { appDataDir } from '@tauri-apps/api/path';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';
import type { Book } from '$lib/model/book'; // requires svelte service

export const paraBuffer = 20;
export const verseBuffer = 40;

const mapVerse = ({ id, sid, nbr, paragraph, children }: VerseRow): Verse => ({
  id,
  sid,
  nbr,
  paragraph,
  children: JSON.parse(children) as VerseChild[],
});

export default class ModuleDbService {
  private db: Database | undefined;

  constructor(private moduleId: string) {}

  public async getModuleName(): Promise<string> {
    const db = await this.getDb();
    const moduleRows = await db.select<{ name: string }[]>(
      'SELECT name FROM module'
    );
    return moduleRows?.[0]?.name ?? '';
  }

  public async getBooks(): Promise<Record<string, Book>> {
    const db = await this.getDb();
    const books = await db.select<Book[]>('SELECT * FROM books');
    return Object.fromEntries(books.map((book) => [book.code, book]));
  }

  public async getParagraphIdBySid(verseSid: string): Promise<number> {
    const db = await this.getDb();
    const result = await db.select<{ paragraph: number }[]>(
      'SELECT paragraph FROM verses WHERE sid = $1',
      [verseSid]
    );
    return result?.[0]?.paragraph ?? 0;
  }

  public async getVersesByParagraphs(paraId: number): Promise<{
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
          SELECT v.id, v.sid, v.nbr, v.paragraph, v.children
          FROM verses v
          WHERE v.paragraph BETWEEN $1 AND $2`,
      [fromPara, thruPara]
    );
    console.log(`sel ran in ${Date.now() - start}ms.`); // TODO: remove this

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

  public async getVerseIdBySid(verseSid: string): Promise<number> {
    const db = await this.getDb();
    const result = await db.select<{ id: number }[]>(
      'SELECT id FROM verses WHERE sid = $1',
      [verseSid]
    );
    return result?.[0]?.id ?? 0;
  }

  public async getVerses(verseId: number): Promise<{
    prev: Verse[];
    active: Verse | undefined;
    next: Verse[];
  }> {
    const fromVerse = verseId - verseBuffer;
    const thruVerse = verseId + verseBuffer;

    const db = await this.getDb();
    const start = Date.now();
    const verseResults = await db.select<VerseRow[]>(
      `
          SELECT v.id, v.sid, v.nbr, v.paragraph, v.children
          FROM verses v
          WHERE v.id BETWEEN $1 AND $2`,
      [fromVerse, thruVerse]
    );
    console.log(`sel ran in ${Date.now() - start}ms.`); // TODO: remove this
    const prev = verseResults.filter((v) => v.id < verseId).map(mapVerse);
    const activeRow = verseResults.find((v) => v.id === verseId);
    const active = activeRow ? mapVerse(activeRow) : undefined;
    const next = verseResults.filter((v) => v.id > verseId).map(mapVerse);
    return { prev, active, next };
  }

  /**
   * @param ref
   */
  public async getVersesByRef(ref: string): Promise<{
    prev: Verse[];
    active: Verse | undefined;
    next: Verse[];
  }> {
    const refs = ref.split(',');
    console.log({ refs });
    for (const ref of refs) {
      const [from, to] = ref.split('-');
    }
    return { prev: [], active: undefined, next: [] };
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
