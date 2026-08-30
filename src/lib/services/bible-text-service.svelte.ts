import type { Verse } from '$lib/model/verse';
import type { Book } from '$lib/model/book';
import { tick, untrack } from 'svelte';
import debounce from '$lib/services/debounce';
import rawVerseCounts from '$lib/assets/bible-verse-counts.yaml';
import type { BibleVerseCounts } from '$lib/model/bible-verse-counts';
import { appSettingsService } from './app-settings-service.svelte';
import type { AutoCompleteOption } from '$lib/model/auto-complete-option';
import { splitByLastDelimiter } from '$lib/services/string-utils';
import { parseSearch, type SearchType } from '$lib/services/search-parser';
import ModuleDbService, {
  paraBuffer,
  verseBuffer,
} from '$lib/services/module-db-service.svelte';

export type GroupingMode = 'verse' | 'paragraph';
const defaultSearch = 'GEN 1:1';
const defaultSearchType: SearchType = 'ref-point';
const defaultGroupingMode: GroupingMode = 'paragraph';

const verseCounts = rawVerseCounts as BibleVerseCounts;
const allVerseCounts = { ...verseCounts.OT, ...verseCounts.NT };
const verseCount = Object.values(allVerseCounts).reduce(
  (acc, curr) => acc + curr.reduce((acc2, curr2) => acc2 + curr2, 0),
  0
); // 31,102 verses
const searchDelimiters = [' ', ',', ';'];

export default class BibleTextService {
  private getTextStart = $state<number | undefined>();
  private getTextDebounced = debounce(this.getText, 100);
  private _groupingMode = $state<GroupingMode>(defaultGroupingMode);
  private moduleDbService: ModuleDbService;

  // This field is only modified by user search input.
  public searchInput = $state<string>('');
  public autoCompleteOptions = $state<AutoCompleteOption[]>([]);
  // These are modified both by controls and the search input.
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
  public autoCompletePickInProgress = $state<boolean>(false);

  constructor(moduleId: string) {
    this.moduleDbService = new ModuleDbService(moduleId);
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

  public async autoCompletePick(
    selectedOption?: AutoCompleteOption
  ): Promise<void> {
    if (!selectedOption) {
      selectedOption = this.autoCompleteOptions.find((op) => op.selected);
    }
    if (selectedOption) {
      const [before, _after] = splitByLastDelimiter(
        this.searchInput,
        searchDelimiters
      );
      this.autoCompletePickInProgress = true;
      this.searchInput = `${before ?? ''} ${selectedOption.value}`.trim();
      this.autoCompleteOptions = [];
      await tick();
      this.autoCompletePickInProgress = false;
    }
  }

  private async evaluateSearchInput(searchInput: string): Promise<void> {
    if (this.autoCompletePickInProgress) {
      await this.performSearch();
      return;
    }

    this.autoCompleteOptions = [];
    const searchLower = searchInput.toLowerCase();
    const [_before, after] = splitByLastDelimiter(
      searchLower,
      searchDelimiters
    );

    // bring up auto complete
    if (after.length < 2) {
      this.currentSearch = defaultSearch;
      this.currentSearchType = defaultSearchType;
      // close auto complete
    } else {
      this.addReferenceAutoCompleteOptions(after);
      // TODO: Add word auto complete options.

      if (this.autoCompleteOptions.length > 0) {
        this.autoCompleteOptions[0].selected = true;
      }
    }

    await this.performSearch();
  }

  private async performSearch(): Promise<void> {
    const searchLower = this.searchInput.toLowerCase().trim();
    if (searchLower.length < 2) return;

    const { searchType, searchString } = parseSearch(searchLower);

    this.currentSearchType = searchType;
    this.currentSearch = searchString;
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
      await this.moduleDbService.getVersesByParagraphs(nextParaNumber);

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
      await this.moduleDbService.getVersesByParagraphs(prevParaNumber);

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

    const { prev, active, next } =
      await this.moduleDbService.getVerses(nextVerseNumber);
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

    const { prev, active, next } =
      await this.moduleDbService.getVerses(prevVerseNumber);
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

  private async getModule(): Promise<void> {
    this.moduleName = await this.moduleDbService.getModuleName();
  }

  private async getBooks(): Promise<void> {
    this.books = await this.moduleDbService.getBooks();
  }

  private async getText(
    currentSearch: string,
    currentSearchType: SearchType
  ): Promise<void> {
    switch (true) {
      case currentSearchType === 'ref-point' &&
        this.groupingMode === 'paragraph':
        await this.getVersesByParagraphs(currentSearch);
        break;
      case currentSearchType === 'ref-point' && this.groupingMode === 'verse':
        await this.getVerses(currentSearch);
        break;
      case currentSearchType === 'ref':
        await this.getVersesByRef(currentSearch);
        break;
    }
  }

  private async getVersesByParagraphs(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;

    const paraId =
      await this.moduleDbService.getParagraphIdBySid(currentSearch);
    const { prev, active, next } =
      await this.moduleDbService.getVersesByParagraphs(paraId);

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
  }

  private async getVerses(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;

    const verseId = await this.moduleDbService.getVerseIdBySid(currentSearch);
    const { prev, active, next } =
      await this.moduleDbService.getVerses(verseId);

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

  private async getVersesByRef(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;

    const { prev, active, next } =
      await this.moduleDbService.getVersesByRef(currentSearch);

    // Only set results if this is the last call to getText.
    if (this.getTextStart === start) {
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
      if (active) {
        this.scrollToSid = active.sid; // Triggers scrollIntoView in verse.svelte
      }
      this.prevParaBuffer = {};
      this.activePara = [];
      this.nextParaBuffer = {};
    }
  }
}
