import type { Paragraph, Verse } from '$lib/model/verse';
import type { Book } from '$lib/model/book';
import { tick, untrack } from 'svelte';
import debounce from '$lib/services/debounce';
import rawVerseCounts from '$lib/assets/bible-verse-counts.yaml';
import type { BibleVerseCounts } from '$lib/model/bible-verse-counts';
import { appSettingsService } from './app-settings-service.svelte';
import type { AutoCompleteOption } from '$lib/model/auto-complete-option';
import { splitByLastDelimiter } from '$lib/services/string-utils';
import { parseSearch, type SearchType } from '$lib/services/search-parser';
import { moduleAbbreviation } from '$lib/services/verse-content';
import ModuleDbService, {
  paraBuffer,
  verseBuffer,
} from '$lib/services/module-db-service.svelte';

export type GroupingMode = 'verse' | 'paragraph';
const defaultSearch = 'GEN 1:1';
//const defaultSearch = 'Gen.1.1-Gen.5.20';
const defaultSearchType: SearchType = 'ref-point';
//const defaultSearchType: SearchType = 'ref';
const defaultGroupingMode: GroupingMode = 'paragraph';
//const defaultGroupingMode: GroupingMode = 'verse';
const selectInProgressTimeout = 500;

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
  private getVersesByRefAndIndexDebounced = debounce(
    this.getVersesByRefAndIndex,
    100
  );
  private _groupingMode = $state<GroupingMode>(defaultGroupingMode);
  private moduleDbService: ModuleDbService;
  private _scrollPct = $state<number | undefined>(undefined);

  // This field is only modified by user search input.
  public searchInput = $state<string>('');
  public autoCompleteOptions = $state<AutoCompleteOption[]>([]);
  // These are modified both by controls and the search input.
  public currentSearch = $state<string>(defaultSearch);
  public currentSearchType = $state<SearchType>(defaultSearchType);
  public prevParaBuffer = $state<Paragraph[]>([]);
  public activePara = $state<Paragraph | undefined>(undefined);
  public nextParaBuffer = $state<Paragraph[]>([]);
  public prevVerseBuffer = $state<Verse[]>([]);
  public activeVerse = $state<Verse | undefined>(undefined);
  public nextVerseBuffer = $state<Verse[]>([]);
  public sids = $state<string[]>([]);
  public sidIndex = $state<number>(0);
  public books = $state<Record<string, Book>>({});
  public scrollToSid = $state<string | undefined>();
  public selectInProgress = $state<boolean>(true);
  public adjustingScrollPct = $state<boolean>(false);
  public moduleName = $state<string>('');
  /** A short name for the module, such as "KJV". */
  public moduleAbbreviation = $state<string>('');
  /** The verse to highlight, after a Notes tab heading took the reader there. */
  public highlightSid = $state<string | undefined>();
  /** A verse the pane should bring into view (see bible-text.svelte). */
  public revealSid = $state<string | undefined>();
  public referenceLabel = $state<string>('');
  public autoCompletePickInProgress = $state<boolean>(false);
  /** A problem loading text, such as a missing module, to show the reader. */
  public loadError = $state<string | undefined>(undefined);

  constructor(public readonly moduleId: string) {
    this.moduleDbService = new ModuleDbService(moduleId);
    this.getModule().catch((err) => (this.loadError = String(err)));
    this.getBooks().catch((err) => (this.loadError = String(err)));
    this.getText(this.currentSearch, this.currentSearchType).then();

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

    // Watch scroll pct.
    /**
     * Watches scroll value and reloads text when it changes.
     */
    $effect(() => {
      const newScrollValue = this.scrollPct;
      untrack(() => {
        if (this.adjustingScrollPct) return;
        switch (this.currentSearchType) {
          case 'ref-point':
            this.setSearchFromPct(newScrollValue);
            break;
          case 'ref':
            this.setSearchWithinSidList(newScrollValue).then();
            break;
        }
      });
    });

    /**
     * Setting the scroll to the top after books load.
     */
    $effect(() => {
      // Need to wait for books to be read before setting the scroll value.
      if (this.books && Object.keys(this.books).length > 0) {
        this._scrollPct = 0; // This sets the reference to GEN 1:1
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

  public get scrollPct() {
    return this._scrollPct;
  }

  public set scrollPct(pct: number | undefined) {
    switch (true) {
      case this.currentSearchType === 'ref-point':
        this._scrollPct = pct;
        break;
      case this.currentSearchType === 'ref':
        this.setScrollPctRef(pct);
        break;
    }
  }

  private setScrollPctRef(pct: number | undefined): void {
    if (this.sids.length <= 1 || !pct) return;
    const stepSize = Math.max(Math.round(100 / (this.sids.length - 1)), 1);
    this._scrollPct = Math.round(pct / stepSize) * stepSize;
  }

  /** A verse in the text being read, such as 'PSA 12:1'. */
  public get sidInView(): string | undefined {
    const paragraphs = [this.activePara, ...this.nextParaBuffer];
    for (const paragraph of paragraphs) {
      const verse = paragraph?.verses[0];
      if (verse) return verse.sid;
    }
    return this.activeVerse?.sid;
  }

  /**
   * Takes the reader to verse `sid` and highlights it. The pane scrolls to it
   * if it's loaded, or loads the text there (see bible-text.svelte).
   */
  public goToSid(sid: string): void {
    this.highlightSid = sid;
    this.revealSid = sid;
  }

  /** The footnotes in the chapter holding verse `sid`, for the Notes tab. */
  public getChapterNotes(sid: string) {
    return this.moduleDbService.getChapterNotes(sid);
  }

  /** Loads the text at verse `sid`, which then scrolls into view. */
  public loadAtSid(sid: string): void {
    this.currentSearch = sid;
    this.currentSearchType = 'ref-point';
  }

  /** A chapter's name from any verse in it: 'PSA 12:3' gives 'Psalms 12'. */
  public formatChapterFromSid(sid: string) {
    const [bookId, verseRef] = sid.split(' ');
    const book = this.books[bookId];
    if (!book || !verseRef) return '';
    return `${book.toc2} ${verseRef.split(':')[0]}`;
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

  /** Like formatRefFromSid, with the book's short name: 'Gen 1:1'. */
  public formatAbbreviatedRefFromSid(sid: string) {
    const [bookId, verseRef] = sid.split(' ');
    if (!bookId || !verseRef) return '';
    const book = this.books[bookId];
    if (!book) return '';
    return `${book.toc3} ${verseRef}`;
  }

  /**
   * Sets currentSearch based on percent of entire bible.
   */
  private setSearchFromPct(newScrollValue: number | undefined): void {
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

  /**
   * Sets currentSearch based on pct within a set of search result sids.
   * buffer = 40 verses
   */
  private async setSearchWithinSidList(
    newScrollValue: number | undefined
  ): Promise<void> {
    if (newScrollValue === undefined || this.sids.length === 0) {
      this.referenceLabel = '';
      return;
    }

    const index = Math.round((this.sids.length - 1) * (newScrollValue / 100));
    const sid = this.sids[index];
    this.referenceLabel = this.formatAbbreviatedRefFromSid(sid);
    this.sidIndex = index; // No need to change currentSearch here.
    await this.getVersesByRefAndIndexDebounced(this.currentSearch, index);
  }

  public async moveActiveDownOne(): Promise<void> {
    if (this.selectInProgress) return;

    if (
      this.currentSearchType === 'ref-point' &&
      this.groupingMode === 'paragraph'
    ) {
      await this.moveActiveDownOnePara();
    } else {
      await this.moveActiveDownOneVerse();
    }
  }

  public async moveActiveUpOne(): Promise<void> {
    if (this.selectInProgress) return;

    if (
      this.currentSearchType === 'ref-point' &&
      this.groupingMode === 'paragraph'
    ) {
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
    this.highlightSid = undefined;
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
    this.currentSearch = searchString ?? defaultSearch;
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
    const nextParagraph = this.nextParaBuffer[0];
    if (!nextParagraph) return;

    const { prev, active, next } = await this.moduleDbService.getParagraphs(
      nextParagraph.id
    );

    this.prevParaBuffer = prev;
    this.activePara = active;
    this.nextParaBuffer = next;
    const firstVerse = active?.verses[0];
    if (firstVerse) {
      this.setScrollPosition(firstVerse);
    }
  }

  private async moveActiveUpOnePara(): Promise<void> {
    const prevParagraph = this.prevParaBuffer[this.prevParaBuffer.length - 1];
    if (!prevParagraph) return;

    const { prev, active, next } = await this.moduleDbService.getParagraphs(
      prevParagraph.id
    );

    this.prevParaBuffer = prev;
    this.activePara = active;
    this.nextParaBuffer = next;
    const firstVerse = active?.verses[0];
    if (firstVerse) {
      this.setScrollPosition(firstVerse);
    }
  }

  private async moveActiveDownOneVerse(): Promise<void> {
    const nextVerseNumber = this.nextVerseBuffer?.[0]?.id;
    if (!nextVerseNumber) return;

    if (this.currentSearchType === 'ref-point') {
      const { prev, active, next } =
        await this.moduleDbService.getVerses(nextVerseNumber);
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
    } else {
      if (this.sidIndex < this.sids.length - 1) {
        this.sidIndex = this.sidIndex + 1;
        await this.getVersesByRefAndIndex(this.currentSearch, this.sidIndex);
      }
    }

    if (this.activeVerse) {
      this.setScrollPosition(this.activeVerse);
    }
  }

  private async moveActiveUpOneVerse(): Promise<void> {
    const prevVerseNumber =
      this.prevVerseBuffer?.[this.prevVerseBuffer.length - 1]?.id;
    if (!prevVerseNumber) return;

    if (this.currentSearchType === 'ref-point') {
      const { prev, active, next } =
        await this.moduleDbService.getVerses(prevVerseNumber);
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
    } else {
      if (this.sidIndex > 0) {
        this.sidIndex = this.sidIndex - 1;
        await this.getVersesByRefAndIndex(this.currentSearch, this.sidIndex);
      }
    }

    if (this.activeVerse) {
      this.setScrollPosition(this.activeVerse);
    }
  }

  private setScrollPosition(verse: Verse): void {
    this.adjustingScrollPct = true;
    if (this.currentSearchType === 'ref-point') {
      this._scrollPct = (verse.id / verseCount) * 100;
    } else {
      this._scrollPct = (this.sidIndex / (this.sids.length - 1)) * 100;
    }
    this.referenceLabel = this.formatAbbreviatedRefFromSid(verse.sid);
    setTimeout(() => {
      this.adjustingScrollPct = false;
    }, 500);
  }

  private async getModule(): Promise<void> {
    this.moduleName = await this.moduleDbService.getModuleName();
    this.moduleAbbreviation = moduleAbbreviation(this.moduleName);
  }

  private async getBooks(): Promise<void> {
    this.books = await this.moduleDbService.getBooks();
  }

  /**
   * This is called anytime currentSearch changes, which includes manual scrolling as well scroll bar changes.
   * @param currentSearch
   * @param currentSearchType
   * @private
   */
  private async getText(
    currentSearch: string,
    currentSearchType: SearchType
  ): Promise<void> {
    try {
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
      this.loadError = undefined;
    } catch (err) {
      this.loadError = String(err);
      this.selectInProgress = false;
    }
  }

  /**
   * Search type is ref point with paragraph grouping.
   * @param currentSearch
   * @private
   */
  private async getVersesByParagraphs(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;
    this.selectInProgress = true;

    const { prev, active, next } =
      await this.moduleDbService.getParagraphsAtSid(currentSearch);

    // Only set results if this is the last call to getText.
    if (this.getTextStart === start) {
      this.prevParaBuffer = prev;
      this.activePara = active;
      this.nextParaBuffer = next;
      this.scrollToSid = currentSearch; // Triggers scrollIntoView in paragraph.svelte
      this.sids = [];
      this.prevVerseBuffer = [];
      this.activeVerse = undefined;
      this.nextVerseBuffer = [];
    }

    const firstVerse = this.activePara?.verses[0];
    if (firstVerse) {
      this.setScrollPosition(firstVerse);
    }

    setTimeout(() => {
      this.selectInProgress = false;
    }, selectInProgressTimeout);
  }

  /**
   * Search type is ref point with verse grouping.
   * @param currentSearch
   * @private
   */
  private async getVerses(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;
    this.selectInProgress = true;

    const { prev, active, next } =
      await this.moduleDbService.getVersesAtSid(currentSearch);

    // Only set results if this is the last call to getText.
    if (this.getTextStart === start) {
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
      this.scrollToSid = currentSearch; // Triggers scrollIntoView in verse.svelte
      this.sids = [];
      this.prevParaBuffer = [];
      this.activePara = undefined;
      this.nextParaBuffer = [];
    }

    if (this.activeVerse) {
      this.setScrollPosition(this.activeVerse);
    }

    setTimeout(() => {
      this.selectInProgress = false;
    }, selectInProgressTimeout);
  }

  /**
   * Search type is ref from string search (verse grouping only).
   * @param currentSearch
   * @private
   */
  private async getVersesByRef(currentSearch: string): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;
    this.selectInProgress = true;

    const sidPromise = this.moduleDbService.getSidsByRef(currentSearch);
    const versePromise = this.moduleDbService.getVersesByRef(currentSearch);
    const [sids, verseResults] = await Promise.all([sidPromise, versePromise]);

    const { prev, active, next } = verseResults;

    // Only set results if this is the last call to getText.
    if (this.getTextStart === start) {
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
      this.sids = sids;
      if (active) {
        this.scrollToSid = active.sid; // Triggers scrollIntoView in verse.svelte
      }
      this.prevParaBuffer = [];
      this.activePara = undefined;
      this.nextParaBuffer = [];
    }

    if (this.activeVerse) {
      this.setScrollPosition(this.activeVerse);
    }

    setTimeout(() => {
      this.selectInProgress = false;
    }, selectInProgressTimeout);
  }

  /**
   * Search type is ref from string search (verse grouping only) and sidIndex has changed.
   * @param currentSearch
   * @param sidIndex
   * @private
   */
  private async getVersesByRefAndIndex(
    currentSearch: string,
    sidIndex: number
  ): Promise<void> {
    const start = Date.now();
    this.getTextStart = start;
    this.selectInProgress = true;

    const { prev, active, next } = await this.moduleDbService.getVersesByRef(
      currentSearch,
      sidIndex
    );

    // Only set results if this is the last call to getText.
    if (this.getTextStart === start) {
      this.prevVerseBuffer = prev;
      this.activeVerse = active;
      this.nextVerseBuffer = next;
      if (active) {
        this.scrollToSid = active.sid; // Triggers scrollIntoView in verse.svelte
      }
      this.prevParaBuffer = [];
      this.activePara = undefined;
      this.nextParaBuffer = [];
    }

    setTimeout(() => {
      this.selectInProgress = false;
    }, selectInProgressTimeout);
  }
}
