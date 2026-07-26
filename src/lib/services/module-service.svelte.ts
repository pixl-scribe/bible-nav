import Database from '@tauri-apps/plugin-sql'; // requires svelte service
import { appDataDir } from '@tauri-apps/api/path';
import verseCounts from '../assets/bible-verse-counts.yaml';
import type { TestamentVerseCounts } from '$lib/model/bible-verse-counts';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';

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
  private dbPath: string | undefined;
  public currentSearch = $state<string | undefined>(defaultSearch);
  public currentSearchType = $state<SearchType | undefined>(defaultSearchType);
  public verses = $state<Verse[]>([]);

  constructor(private moduleId: string) {
    $effect(() => {
      const currentSearch = this.currentSearch ?? defaultSearch;
      const currentSearchType = this.currentSearchType ?? defaultSearchType;
      this.getText(currentSearch, currentSearchType).then();
    });
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

  private async getDbPath(): Promise<string> {
    if (this.dbPath) return this.dbPath;
    const appData = await appDataDir();
    this.dbPath = `sqlite:${appData}/modules/${this.moduleId}.db`;
    return this.dbPath;
  }

  private async getChapters(chapterSids: string[]): Promise<Verse[]> {
    const dbPath = await this.getDbPath();
    let db: Database | undefined;
    try {
      db = await Database.load(dbPath);
    } catch (err) {
      console.error(`Error connecting to ${dbPath}`, err);
      throw err;
    }

    let verses: Verse[];
    try {
      const placeholders = chapterSids
        .map((_, index) => `$${index + 1}`)
        .join(', ');

      const verseResults = await db.select<VerseRow[]>(
        `
        SELECT v.sid, v.paragraph, v.children
        FROM chapters c JOIN main.verses v ON c.bookId = v.bookId AND c.nbr = v.chapter
        WHERE c.sid in (${placeholders})`,
        chapterSids
      );
      verses = verseResults.map(({ sid, paragraph, children }) => ({
        sid,
        paragraph,
        children: JSON.parse(children) as VerseChild[],
      }));
    } finally {
      db?.close();
    }
    return verses;
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
