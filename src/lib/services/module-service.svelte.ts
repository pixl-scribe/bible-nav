import Database from '@tauri-apps/plugin-sql'; // requires svelte service
import { appDataDir } from '@tauri-apps/api/path';
import type { Verse, VerseChild, VerseRow } from '$lib/model/verse';
import type { Book } from '$lib/model/book';
import { untrack } from 'svelte';
import debounce from '$lib/services/debounce';
import rawVerseCounts from '$lib/assets/bible-verse-counts.yaml';
import type { BibleVerseCounts } from '$lib/model/bible-verse-counts';
import { appSettingsService } from './app-settings-service.svelte';
import type { AutoCompleteOption } from '$lib/model/auto-complete-option';

export type SearchType = 'ref' | 'ref-point' | 'text';
export type GroupingMode = 'verse' | 'paragraph';
const defaultSearch = 'GEN 1:1';
const defaultSearchType: SearchType = 'ref-point';
const defaultGroupingMode: GroupingMode = 'paragraph';
const paraBuffer = 20;
const verseBuffer = 40;

const verseCounts = rawVerseCounts as BibleVerseCounts;
const allVerseCounts = { ...verseCounts.OT, ...verseCounts.NT };
const verseCount = Object.values(allVerseCounts).reduce(
  (acc, curr) => acc + curr.reduce((acc2, curr2) => acc2 + curr2, 0),
  0
); // 31,102 verses

const mapVerse = ({ id, sid, nbr, paragraph, children }: VerseRow): Verse => ({
  id,
  sid,
  nbr,
  paragraph,
  children: JSON.parse(children) as VerseChild[],
});

export default class ModuleService {
  private db: Database | undefined;
  private getTextStart = $state<number | undefined>();
  private getTextDebounced = debounce(this.getText, 100);
  private _groupingMode = $state<GroupingMode>(defaultGroupingMode);

  public searchInput = $state<string>('');
  public autoCompleteOptions = $state<AutoCompleteOption[]>([]);
  public currentSearch = $state<string | undefined>(defaultSearch);
  public currentSearchType = $state<SearchType | undefined>(defaultSearchType);
  public prevParaBuffer = $state<Record<number, Verse[]>>({});
  public activePara = $state<Verse[]>([]);
  public nextParaBuffer = $state<Record<number, Verse[]>>({});
  public prevVerseBuffer = $state<Verse[]>([]);
  public activeVerse = $state<Verse | undefined>(undefined);
  public nextVerseBuffer = $state<Verse[]>([]);
  public books = $state<Record<string, Book>>({});
  public scrollToSid = $state<string | undefined>();
  public selectInProgress = $state<boolean>(true);
  public scrollPct = $state<number | undefined>(undefined);
  public adjustingScrollPct = $state<boolean>(false);
  public moduleName = $state<string>('');
  public referenceLabel = $state<string>('');

  constructor(private moduleId: string) {
    this.getModule().then();
    this.getBooks().then();

    // Watch search and search type.
    $effect(() => {
      const groupingMode = appSettingsService.groupingMode;
      const currentSearch = this.currentSearch ?? defaultSearch;
      const currentSearchType = this.currentSearchType ?? defaultSearchType;
      untrack(() => {
        this.groupingMode = groupingMode;
        this.selectInProgress = true;
        this.getTextDebounced(currentSearch, currentSearchType).then(() => {
          // Need to wait for scrolling to finish.
          setTimeout(() => {
            this.selectInProgress = false;
          }, 1000);
        });
      });
    });

    /**
     * Setting the scroll to the top after books load.
     */
    $effect(() => {
      // Need to wait for books to be read before setting the scroll value.
      if (this.books && Object.keys(this.books).length > 0) {
        this.scrollPct = 0; // This sets the reference to GEN 1:1
      }
    });

    $effect(() => {
      const searchInput = this.searchInput;
      untrack(() => {
        this.evaluateSearchInput(searchInput);
      });
    });
  }

  public get groupingMode() {
    return this._groupingMode;
  }

  private set groupingMode(groupingMode: 'verse' | 'paragraph') {
    this._groupingMode = groupingMode;
  }

  public get bufferSize() {
    return this.groupingMode === 'paragraph' ? paraBuffer : verseBuffer;
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

  private formatAbbreviatedRefFromSid(sid: string) {
    const [bookId, verseRef] = sid.split(' ');
    if (!bookId || !verseRef) return '';
    const book = this.books[bookId];
    if (!book) return '';
    return `${book.toc3} ${verseRef}`;
  }

  /**
   * Called when the scrollbar changes position.
   */
  public setReference(newScrollValue: number | undefined): void {
    if (this.adjustingScrollPct) {
      return;
    }
    const books = this.books;
    if (newScrollValue === undefined || books === undefined) {
      this.referenceLabel = '';
      return;
    }
    const verseIndex = Math.round((newScrollValue / 100) * (verseCount - 1));
    let foundBookCode: string;
    let verseAccum = 0;
    for (const bookCode of Object.keys(allVerseCounts)) {
      foundBookCode = bookCode;
      const chapterCounts = allVerseCounts[bookCode];
      for (const [index, versesInChapter] of chapterCounts.entries()) {
        const foundChapter = index + 1;
        if (verseAccum + versesInChapter > verseIndex) {
          const book = books[foundBookCode];
          const verse = verseIndex - verseAccum + 1;
          this.referenceLabel = `${book?.toc3} ${foundChapter}:${verse}`;
          this.currentSearch = `${book?.code} ${foundChapter}:${verse}`;
          this.currentSearchType = 'ref-point';
          return;
        }
        verseAccum += versesInChapter;
      }
    }
    this.referenceLabel = '';
  }

  public async moveActiveDownOne(): Promise<void> {
    if (this.groupingMode === 'paragraph') {
      await this.moveActiveDownOnePara();
    } else {
      await this.moveActiveDownOneVerse();
    }
  }

  public async moveActiveUpOne(): Promise<void> {
    if (this.groupingMode === 'paragraph') {
      await this.moveActiveUpOnePara();
    } else {
      await this.moveActiveUpOneVerse();
    }
  }

  public autoCompleteUp() {
    if (this.autoCompleteOptions.length < 2) return;

    const currentSelectedIndex = this.autoCompleteOptions.findIndex(
      (op) => op.selected
    );
    const newSelectedIndex =
      currentSelectedIndex === 0
        ? this.autoCompleteOptions.length - 1
        : currentSelectedIndex - 1;
    this.autoCompleteOptions[currentSelectedIndex].selected = false;
    this.autoCompleteOptions[newSelectedIndex].selected = true;
  }

  public autoCompleteDown() {
    if (this.autoCompleteOptions.length < 2) return;

    const currentSelectedIndex = this.autoCompleteOptions.findIndex(
      (op) => op.selected
    );
    const newSelectedIndex =
      currentSelectedIndex === this.autoCompleteOptions.length - 1
        ? 0
        : currentSelectedIndex + 1;
    this.autoCompleteOptions[currentSelectedIndex].selected = false;
    this.autoCompleteOptions[newSelectedIndex].selected = true;
  }

  public autoCompletePick() {
    const selectedOption = this.autoCompleteOptions.find(
      (op) => op.selected
    );
    if (selectedOption) {
      this.searchInput =
        selectedOption.type === 'ref'
          ? `Ref: ${selectedOption.value}`
          : selectedOption.value;
    }
  }

  private evaluateSearchInput(searchInput: string) {
    this.autoCompleteOptions = [];
    const trimmedSearch = searchInput.trim().toLowerCase();

    if (trimmedSearch.length < 2) {
      this.currentSearch = defaultSearch;
      this.currentSearchType = defaultSearchType;
      // close auto complete
      return;
    }
    this.addReferenceAutoCompleteOptions(trimmedSearch);

    if (this.autoCompleteOptions.length > 0) {
      this.autoCompleteOptions[0].selected = true;
    }

    // TODO: perform search
  }

  private addReferenceAutoCompleteOptions(searchInput: string) {
    const books = Object.values(this.books).filter(
      (book) =>
        book.toc3.toLowerCase().startsWith(searchInput) ||
        book.toc2.toLowerCase().startsWith(searchInput)
    );
    this.autoCompleteOptions.push(
      ...books.map(
        (b) =>
          ({
            value: b.toc2,
            type: 'ref',
            count: undefined,
          }) as AutoCompleteOption
      )
    );
  }

  private async moveActiveDownOnePara(): Promise<void> {
    const keys = Object.keys(this.nextParaBuffer);
    if (keys.length === 0) return;

    this.selectInProgress = true;
    const nextParaNumber = parseInt(keys[0]);
    const { prev, active, next } =
      await this.getVersesByParagraphs(nextParaNumber);

    this.prevParaBuffer = prev;
    this.activePara = active;
    this.nextParaBuffer = next;
    const firstVerse = active?.[0];
    if (firstVerse) {
      this.setScrollPositionFromMouseScrolling(firstVerse);
    }
    this.selectInProgress = false;
  }

  private async moveActiveUpOnePara(): Promise<void> {
    const keys = Object.keys(this.prevParaBuffer);
    if (keys.length === 0) return;

    this.selectInProgress = true;
    const prevParaNumber = parseInt(keys[keys.length - 1]);
    const { prev, active, next } =
      await this.getVersesByParagraphs(prevParaNumber);

    this.prevParaBuffer = prev;
    this.activePara = active;
    this.nextParaBuffer = next;
    const firstVerse = active?.[0];
    if (firstVerse) {
      this.setScrollPositionFromMouseScrolling(firstVerse);
    }
    this.selectInProgress = false;
  }

  private async moveActiveDownOneVerse(): Promise<void> {
    this.selectInProgress = true;

    const nextVerseNumber = this.nextVerseBuffer?.[0]?.id;
    if (!nextVerseNumber) return;

    const { prev, active, next } = await this.getVerses(nextVerseNumber);
    this.prevVerseBuffer = prev;
    this.activeVerse = active;
    this.nextVerseBuffer = next;
    if (this.activeVerse) {
      this.setScrollPositionFromMouseScrolling(this.activeVerse);
    }
    this.selectInProgress = false;
  }

  private async moveActiveUpOneVerse(): Promise<void> {
    this.selectInProgress = true;

    const prevVerseNumber =
      this.prevVerseBuffer?.[this.prevVerseBuffer.length - 1]?.id;
    if (!prevVerseNumber) return;

    const { prev, active, next } = await this.getVerses(prevVerseNumber);
    this.prevVerseBuffer = prev;
    this.activeVerse = active;
    this.nextVerseBuffer = next;
    if (this.activeVerse) {
      this.setScrollPositionFromMouseScrolling(this.activeVerse);
    }
    this.selectInProgress = false;
  }

  private setScrollPositionFromMouseScrolling(verse: Verse): void {
    this.adjustingScrollPct = true;
    this.scrollPct = (verse.id / verseCount) * 100;
    this.referenceLabel = this.formatAbbreviatedRefFromSid(verse.sid);
    setTimeout(() => {
      this.adjustingScrollPct = false;
    }, 500);
  }

  private async getModule() {
    const db = await this.getDb();
    const moduleRows = await db.select<{ name: string }[]>(
      'SELECT name FROM module'
    );
    this.moduleName = moduleRows?.[0]?.name ?? '';
  }

  private async getBooks() {
    const db = await this.getDb();
    const books = await db.select<Book[]>('SELECT * FROM books');
    this.books = Object.fromEntries(books.map((book) => [book.code, book]));
  }

  private async getText(
    currentSearch: string,
    currentSearchType: SearchType
  ): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;
    if (
      currentSearchType === 'ref-point' &&
      this.groupingMode === 'paragraph'
    ) {
      const paraId = await this.getParagraphIdBySid(currentSearch);
      const { prev, active, next } = await this.getVersesByParagraphs(paraId);

      // Only set results if this is the last call to getText.
      if (this.getTextStart === start) {
        this.prevParaBuffer = prev;
        this.activePara = active;
        this.nextParaBuffer = next;
        this.scrollToSid = currentSearch; // Triggers scrollIntoView in paragraph.svelte
        this.prevVerseBuffer = [];
        this.activeVerse = undefined;
        this.nextVerseBuffer = [];
      }
    } else if (
      currentSearchType === 'ref-point' &&
      this.groupingMode === 'verse'
    ) {
      const verseId = await this.getVerseIdBySid(currentSearch);
      const { prev, active, next } = await this.getVerses(verseId);

      // Only set results if this is the last call to getText.
      if (this.getTextStart === start) {
        this.prevVerseBuffer = prev;
        this.activeVerse = active;
        this.nextVerseBuffer = next;
        this.scrollToSid = currentSearch; // Triggers scrollIntoView in verse.svelte
        this.prevParaBuffer = {};
        this.activePara = [];
        this.nextParaBuffer = {};
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

  private async getVerseIdBySid(verseSid: string): Promise<number> {
    const db = await this.getDb();
    const result = await db.select<{ id: number }[]>(
      'SELECT id FROM verses WHERE sid = $1',
      [verseSid]
    );
    return result?.[0]?.id ?? 0;
  }

  private async getVerses(verseId: number): Promise<{
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
