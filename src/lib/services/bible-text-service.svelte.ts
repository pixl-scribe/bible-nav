import type { Verse } from '$lib/model/verse';
import type { Book } from '$lib/model/book';
import { tick, untrack } from 'svelte';
import debounce from '$lib/services/debounce';
import {
  appSettingsService,
  type GroupingMode,
} from './app-settings-service.svelte';
import type { AutoCompleteOption } from '$lib/model/auto-complete-option';
import { splitByLastDelimiter } from '$lib/services/string-utils';
import { parseSearch } from '$lib/services/search-parser';
import ModuleDbService from '$lib/services/module-db-service.svelte';
import {
  chapterOfSid,
  firstVerseOf,
  type Chapter,
} from '$lib/services/chapter-text';

/**
 * How the pane shows text:
 * - 'chapters': whole chapters, in the reader's grouping, scrolled through
 *   endlessly (an empty search, or a place such as "John 3:16").
 * - 'list': the verses of a reference range or list, one by one ("John
 *   3:16-18"), loading more near the end.
 * - 'text': a word search, not built yet.
 */
export type ViewMode = 'chapters' | 'list' | 'text';

/** A verse for the pane to bring to its top (or, `smooth`, into view). */
export interface ScrollTarget {
  sid: string;
  /** Glide to it and centre it, as for a Notes tab heading. */
  smooth?: boolean;
}

export type { GroupingMode };

/** Where reading starts. */
const startSid = 'GEN 1:1';
/** Verses loaded at a time in a list. */
const listPage = 100;
const searchDelimiters = [' ', ',', ';'];

/**
 * One reading pane's text: which module, what the search asks for, and the
 * chapters or verses loaded. The pane (bible-text.svelte) does everything
 * that needs the screen: loading more near either end, keeping the text
 * still when chapters come and go above, and bringing verses to the top.
 */
export default class BibleTextService {
  private moduleDbService: ModuleDbService;
  /** Counts loads, so an answer for a superseded one is dropped. */
  private loadSeq = 0;
  private searchDebounced = debounce(() => this.performSearch(), 250);

  /** The module this text reads; `setModule` switches it. */
  public moduleId = $state<string>('');
  public moduleName = $state<string>('');
  public moduleVersion = $state<string>('');
  /** A short name for the module, such as "KJV". */
  public moduleAbbreviation = $state<string>('');
  public books = $state<Record<string, Book>>({});
  /** The last load's error, for `errorMessage` to describe. */
  public loadError = $state<unknown>(undefined);

  // This field is only modified by user search input.
  public searchInput = $state<string>('');
  public autoCompleteOptions = $state<AutoCompleteOption[]>([]);
  public autoCompletePickInProgress = $state<boolean>(false);

  public mode = $state<ViewMode>('chapters');
  /** The loaded chapters, in reading order, next to each other. */
  public chapters = $state<Chapter[]>([]);
  /** The reference shown as a list, in OSIS form. */
  public listOsis = $state<string | undefined>();
  public listVerses = $state<Verse[]>([]);
  /** How many verses the whole list has. */
  public listTotal = $state(0);
  /** A chapter or page is loading. */
  public loading = $state(false);

  /** The verse at the top of the pane, measured as the reader scrolls. */
  public topSid = $state<string | undefined>();
  /**
   * When the reader last moved this text themselves (scrolling it, or a
   * search or a Notes heading taking it somewhere), from
   * `performance.now()`. Of a main text and its parallel text, the one
   * moved last leads and the other follows. Not reactive: it's only read
   * when a pane's top verse changes.
   */
  public leadAt = 0;
  /**
   * Where the reader was last reading chapters, to come back to when a
   * search is cleared.
   */
  public browseSid = $state<string>(startSid);
  /** A verse the pane should bring to its top. */
  public scrollTarget = $state<ScrollTarget | undefined>();
  /** The verse highlighted with its note in a Notes tab. */
  public highlightSid = $state<string | undefined>();

  /**
   * @param moduleId the module to read.
   * @param start a verse to start at, such as the main text's place when a
   *   parallel text opens; otherwise Genesis 1:1.
   */
  constructor(moduleId: string, start?: string) {
    this.moduleId = moduleId;
    this.moduleDbService = new ModuleDbService(moduleId);
    this.getModule().catch((err) => (this.loadError = err));
    this.getBooks().catch((err) => (this.loadError = err));
    this.openAt(start ?? startSid);

    // Search only when the words change. The first run sees the empty box
    // the pane starts with: searching then would count as clearing a search
    // and send the pane back to Genesis 1:1, over the list a new parallel
    // text has just loaded to match its main text.
    let evaluated = this.searchInput;
    $effect(() => {
      const searchInput = this.searchInput;
      if (searchInput === evaluated) return;
      evaluated = searchInput;
      untrack(() => this.evaluateSearchInput(searchInput));
    });

    // Remember the reading place, for when a search is cleared.
    $effect(() => {
      const sid = this.topSid;
      if (sid && this.mode === 'chapters') {
        untrack(() => (this.browseSid = sid));
      }
    });
  }

  public get groupingMode(): GroupingMode {
    return appSettingsService.groupingMode;
  }

  /** A verse in the text being read, such as 'PSA 12:1'. */
  public get sidInView(): string | undefined {
    return this.topSid;
  }

  /** The place shown beside the selector, such as 'Gen 1:8'. */
  public get referenceLabel(): string {
    return this.topSid ? this.formatAbbreviatedRefFromSid(this.topSid) : '';
  }

  /** Whether the chapter holding `sid` is loaded. */
  public hasChapterOf(sid: string): boolean {
    const chapter = chapterOfSid(sid);
    return this.chapters.some((loaded) => loaded.sid === chapter);
  }

  /**
   * Shows chapters from verse `sid` (or the nearest verse this module has):
   * loads its chapter and brings the verse to the top. The pane then loads
   * the chapters around it.
   */
  public async openAt(sid: string): Promise<void> {
    const seq = ++this.loadSeq;
    this.mode = 'chapters';
    this.listOsis = undefined;
    this.listVerses = [];
    this.loading = true;
    try {
      const target = (await this.moduleDbService.nearestSid(sid)) ?? sid;
      const chapter = await this.moduleDbService.getChapter(target);
      if (seq !== this.loadSeq) return;
      this.chapters = chapter ? [chapter] : [];
      this.loadError = undefined;
      this.scrollTarget = { sid: target };
    } catch (err) {
      if (seq === this.loadSeq) this.loadError = err;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /**
   * Loads the chapter after the last one (`direction` 1) or before the
   * first (-1). Returns whether one was added: false at either end of the
   * module, or while another load is under way.
   */
  public async extend(direction: 1 | -1): Promise<boolean> {
    if (this.loading || this.mode !== 'chapters') return false;
    const edge =
      direction === 1
        ? this.chapters[this.chapters.length - 1]
        : this.chapters[0];
    if (!edge) return false;
    const seq = this.loadSeq;
    this.loading = true;
    try {
      const chapter = await this.moduleDbService.getChapter(
        firstVerseOf(edge.sid),
        direction
      );
      if (!chapter || seq !== this.loadSeq) return false;
      this.chapters =
        direction === 1
          ? [...this.chapters, chapter]
          : [chapter, ...this.chapters];
      return true;
    } catch (err) {
      this.loadError = err;
      return false;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /** Drops `count` chapters from the start (`front`) or the end. */
  public drop(count: number, front: boolean): void {
    if (count <= 0) return;
    this.chapters = front
      ? this.chapters.slice(count)
      : this.chapters.slice(0, this.chapters.length - count);
  }

  /**
   * Shows the verses of an OSIS reference one by one, a page at a time,
   * starting at verse `at` if the list has it (a parallel text opening
   * beside the main text's list, or a Bible switched), else at the top.
   */
  public async showList(osis: string, at?: string): Promise<void> {
    const seq = ++this.loadSeq;
    this.mode = 'list';
    this.listOsis = osis;
    this.listVerses = [];
    this.listTotal = 0;
    this.chapters = [];
    this.loading = true;
    try {
      const sids = await this.moduleDbService.getSidsByRef(osis);
      // Load as far as `at`, if it's in the list, and a page beyond.
      const index = at ? sids.indexOf(at) : -1;
      const verses = await this.moduleDbService.getRefVerses(
        osis,
        0,
        Math.max(listPage, index + listPage)
      );
      if (seq !== this.loadSeq) return;
      this.listTotal = sids.length;
      this.listVerses = verses;
      this.loadError = undefined;
      const target = index >= 0 ? at : verses[0]?.sid;
      if (target) this.scrollTarget = { sid: target };
    } catch (err) {
      if (seq === this.loadSeq) this.loadError = err;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /** Loads the next page of a list. Returns whether any verses came. */
  public async loadMoreList(): Promise<boolean> {
    const osis = this.listOsis;
    if (
      this.loading ||
      this.mode !== 'list' ||
      !osis ||
      this.listVerses.length >= this.listTotal
    ) {
      return false;
    }
    const seq = this.loadSeq;
    this.loading = true;
    try {
      const more = await this.moduleDbService.getRefVerses(
        osis,
        this.listVerses.length,
        listPage
      );
      if (seq !== this.loadSeq || more.length === 0) return false;
      this.listVerses = [...this.listVerses, ...more];
      return true;
    } catch (err) {
      this.loadError = err;
      return false;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /** A word search: not built yet, so the pane says so. */
  public showText(): void {
    ++this.loadSeq;
    this.mode = 'text';
    this.chapters = [];
    this.listOsis = undefined;
    this.listVerses = [];
    this.loading = false;
  }

  /**
   * Brings verse `sid` to the top at once, loading its chapter if needed:
   * one of two texts keeping in step with the other.
   */
  public async align(sid: string): Promise<void> {
    if (this.mode === 'chapters') {
      if (this.hasChapterOf(sid)) {
        this.scrollTarget = { sid };
      } else {
        await this.openAt(sid);
      }
    } else if (this.mode === 'list') {
      if (this.listVerses.some((verse) => verse.sid === sid)) {
        this.scrollTarget = { sid };
      }
    }
  }

  /** The reader is moving this text: it leads, and its partner follows. */
  public lead(): void {
    this.leadAt = performance.now();
  }

  /**
   * Takes the reader to verse `sid` and highlights it, as a Notes tab heading
   * does: glides there if its chapter is loaded, or loads it there.
   */
  public goToSid(sid: string): void {
    this.lead();
    this.highlightSid = sid;
    if (this.mode === 'chapters' && this.hasChapterOf(sid)) {
      this.scrollTarget = { sid, smooth: true };
    } else {
      this.openAt(sid);
    }
  }

  /** The footnotes in the chapter holding verse `sid`, for the Notes tab. */
  public getChapterNotes(sid: string) {
    return this.moduleDbService.getChapterNotes(sid);
  }

  /**
   * Switches this text to another module, keeping the reader's place:
   * chapters reopen at the verse at the top (or the nearest the new module
   * has), and a list is shown again from the new module.
   */
  public async setModule(moduleId: string): Promise<void> {
    if (moduleId === this.moduleId) return;
    this.moduleId = moduleId;
    this.moduleDbService = new ModuleDbService(moduleId);
    this.loadError = undefined;
    try {
      await Promise.all([this.getModule(), this.getBooks()]);
    } catch (err) {
      this.loadError = err;
    }
    if (this.mode === 'chapters') {
      await this.openAt(this.topSid ?? this.browseSid);
    } else if (this.mode === 'list' && this.listOsis) {
      await this.showList(this.listOsis, this.topSid);
    }
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

  private evaluateSearchInput(searchInput: string): void {
    if (this.autoCompletePickInProgress) {
      this.performSearch();
      return;
    }
    this.autoCompleteOptions = [];
    this.highlightSid = undefined;
    const [_before, after] = splitByLastDelimiter(
      searchInput.toLowerCase(),
      searchDelimiters
    );
    if (after.length >= 2) {
      this.addReferenceAutoCompleteOptions(after);
      // TODO: Add word auto complete options.
      if (this.autoCompleteOptions.length > 0) {
        this.autoCompleteOptions[0].selected = true;
      }
    }
    this.searchDebounced();
  }

  /**
   * Shows what the search asks for: an empty search goes back to where the
   * reader was reading; a place opens chapters there; a range or list shows
   * its verses; anything else is a word search.
   */
  private performSearch(): void {
    this.lead();
    const search = this.searchInput.toLowerCase().trim();
    if (search.length < 2) {
      if (this.mode !== 'chapters') this.openAt(this.browseSid);
      return;
    }
    const request = parseSearch(search);
    if (request.searchType === 'point' && request.sid) {
      this.openAt(request.sid);
    } else if (request.searchType === 'ref' && request.osis) {
      if (request.osis !== this.listOsis || this.mode !== 'list') {
        this.showList(request.osis);
      }
    } else {
      this.showText();
    }
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

  private async getModule(): Promise<void> {
    const info = await this.moduleDbService.getModuleInfo();
    this.moduleName = info.name;
    this.moduleVersion = info.version;
    this.moduleAbbreviation = info.abbreviation;
  }

  private async getBooks(): Promise<void> {
    this.books = await this.moduleDbService.getBooks();
  }
}
