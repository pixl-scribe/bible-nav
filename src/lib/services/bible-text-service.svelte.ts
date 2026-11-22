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
import { adjacentChapter, type BookOutline } from './chapter-steps';
import { indexHits, type HitIndex } from './search-hits';
import { recentPlaces } from './recent-places.svelte';

/**
 * How the pane shows text:
 * - 'chapters': whole chapters, in the reader's grouping, scrolled through
 *   endlessly (an empty search, or a place such as "John 3:16").
 * - 'list': verses one by one, a page at a time, loading more near either
 *   end: a reference range or list ("John 3:16-18"), a word search, or a
 *   parallel text showing its main text's verses.
 */
export type ViewMode = 'chapters' | 'list';

/** What a list shows. */
export type ListSource =
  /** A reference range or list, in OSIS form. */
  | { kind: 'ref'; osis: string }
  /** A word search, as typed. */
  | { kind: 'text'; search: string }
  /** These verses: a parallel text showing its main text's list. */
  | { kind: 'sids'; sids: string[]; key: string };

/** A string that's the same for the same list, to tell lists apart. */
export function listKey(source: ListSource): string {
  if (source.kind === 'ref') return `ref:${source.osis}`;
  if (source.kind === 'text') return `text:${source.search}`;
  return source.key;
}

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
/** Verses loaded before the one a list opens at, so there's text above. */
const listBefore = 20;
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
  /** Every book's chapters and verse counts, in order. */
  public outline = $state<BookOutline[]>([]);
  /** The last load's error, for `errorMessage` to describe. */
  public loadError = $state<unknown>(undefined);

  // This field is only modified by user search input.
  public searchInput = $state<string>('');
  public autoCompleteOptions = $state<AutoCompleteOption[]>([]);
  public autoCompletePickInProgress = $state<boolean>(false);

  public mode = $state<ViewMode>('chapters');
  /** The loaded chapters, in reading order, next to each other. */
  public chapters = $state<Chapter[]>([]);
  /** What the list shows. */
  public listSource = $state<ListSource | undefined>();
  /** Every verse in the list, in its order (Bible order for a search). */
  public listSids = $state.raw<string[]>([]);
  /**
   * The loaded part of the list: positions `listStart` up to (not
   * including) `listEnd` of `listSids`. A list can start anywhere, so a
   * jump to its 9,000th verse doesn't load the 8,999 before it.
   */
  public listStart = $state(0);
  public listEnd = $state(0);
  public listVerses = $state<Verse[]>([]);
  /**
   * Where the list's verses are, for the picker and the arrows: counted
   * once per list, not on every look.
   */
  public hits = $derived<HitIndex | undefined>(
    this.mode === 'list' ? indexHits(this.listSids) : undefined
  );
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
  /**
   * The hit the reader last jumped to in the results (picker or arrows),
   * where clearing the search shows the text (decided 2026-11-22).
   */
  private jumpedSid: string | undefined;
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

  /** How many verses the whole list has. */
  public get listTotal(): number {
    return this.listSids.length;
  }

  /** Tells this list from any other. */
  public get listKey(): string | undefined {
    return this.listSource ? listKey(this.listSource) : undefined;
  }

  /** The words of a word search being shown, as typed. */
  public get wordSearch(): string | undefined {
    return this.mode === 'list' && this.listSource?.kind === 'text'
      ? this.listSource.search
      : undefined;
  }

  /** The place shown beside the selector, such as 'Gen 1:8'. */
  public get referenceLabel(): string {
    return this.topSid ? this.formatAbbreviatedRefFromSid(this.topSid) : '';
  }

  /** The chapter being read, such as 'GEN 4': the one at the top. */
  public get placeChapter(): string {
    return chapterOfSid(this.topSid ?? this.browseSid);
  }

  /**
   * The chapter before (-1) or after (1) the one being read, such as
   * 'GEN 5'; undefined at either end of the module.
   */
  public chapterStep(step: 1 | -1): string | undefined {
    return adjacentChapter(this.outline, this.placeChapter, step);
  }

  /**
   * Goes to verse `sid` from the reference picker: reads chapters from
   * there, with the verse at the top. The search box is emptied, since the
   * reader has left whatever it held (a list of references, say), and both
   * the place left and the place reached go into the picker's Recent row.
   */
  public goTo(sid: string): void {
    this.lead();
    const from = this.topSid;
    if (from && this.mode === 'chapters') recentPlaces.add(from);
    recentPlaces.add(sid);
    if (this.searchInput) this.searchInput = '';
    this.openAt(sid);
  }

  /**
   * Goes to verse `sid` from a scripture reference in a lexicon (decided
   * 2026-11-22): the picker's route, since the reader has left whatever the
   * search held (the box is emptied, and the place left and the place
   * reached go into Recent, the way back), with the verse highlighted as a
   * Notes heading does. The highlight is set once emptying the box has run
   * its effect, which clears any highlight.
   */
  public goToReference(sid: string): void {
    this.goTo(sid);
    tick().then(() => (this.highlightSid = sid));
  }

  /** Goes to verse 1 of the previous (-1) or next (1) chapter, at the top. */
  public stepChapter(step: 1 | -1): void {
    const target = this.chapterStep(step);
    if (!target) return;
    this.lead();
    this.openAt(`${target}:1`);
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
    this.listSource = undefined;
    this.listSids = [];
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
   * Shows a list one verse at a time: a reference list, a word search, or
   * another text's verses. It starts at verse `at` if the list has it (a
   * parallel text opening beside its main text, a Bible switched, a jump
   * from the picker), else at the top, and loads a page around it.
   */
  public async showList(source: ListSource, at?: string): Promise<void> {
    const seq = ++this.loadSeq;
    this.jumpedSid = undefined;
    this.mode = 'list';
    this.listSource = source;
    this.listSids = [];
    this.listVerses = [];
    this.chapters = [];
    this.loading = true;
    try {
      const sids = await this.listSidsFor(source);
      if (seq !== this.loadSeq) return;
      this.listSids = sids;
      await this.openListAt(at ? sids.indexOf(at) : -1, seq);
    } catch (err) {
      if (seq === this.loadSeq) this.loadError = err;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /**
   * Goes to verse `sid` in the list being shown, from the picker or the
   * arrows: brings it to the top if it's loaded, or loads a page around it.
   */
  public async goToListSid(sid: string): Promise<void> {
    if (this.mode !== 'list') return;
    this.lead();
    if (this.listVerses.some((verse) => verse.sid === sid)) {
      this.jumpedSid = sid;
      this.scrollTarget = { sid };
      return;
    }
    const index = this.listSids.indexOf(sid);
    if (index < 0) return;
    this.jumpedSid = sid;
    const seq = ++this.loadSeq;
    this.loading = true;
    try {
      await this.openListAt(index, seq);
    } catch (err) {
      if (seq === this.loadSeq) this.loadError = err;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /**
   * Loads a page of the list around position `index` (from the top if -1),
   * with a few verses before it so there's text above, and brings that
   * verse to the top.
   */
  private async openListAt(index: number, seq: number): Promise<void> {
    const start = index < 0 ? 0 : Math.max(0, index - listBefore);
    const end = Math.min(this.listSids.length, start + listPage);
    const verses = await this.listPage(start, end);
    if (seq !== this.loadSeq) return;
    this.listStart = start;
    this.listEnd = end;
    this.listVerses = verses;
    this.loadError = undefined;
    const target = index >= 0 ? this.listSids[index] : verses[0]?.sid;
    if (target) this.scrollTarget = { sid: target };
  }

  /**
   * Loads the page after the loaded verses (`direction` 1) or before them
   * (-1). Returns whether any came: false at either end of the list, or
   * while another load is under way.
   */
  public async extendList(direction: 1 | -1): Promise<boolean> {
    if (this.loading || this.mode !== 'list') return false;
    const [start, end] =
      direction === 1
        ? [
            this.listEnd,
            Math.min(this.listSids.length, this.listEnd + listPage),
          ]
        : [Math.max(0, this.listStart - listPage), this.listStart];
    if (start >= end) return false;
    const seq = this.loadSeq;
    this.loading = true;
    try {
      const verses = await this.listPage(start, end);
      if (seq !== this.loadSeq) return false;
      if (direction === 1) {
        this.listVerses = [...this.listVerses, ...verses];
        this.listEnd = end;
      } else {
        this.listVerses = [...verses, ...this.listVerses];
        this.listStart = start;
      }
      return true;
    } catch (err) {
      this.loadError = err;
      return false;
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  /**
   * Drops `count` loaded verses from the start (`front`) or the end, when
   * they're far out of view.
   */
  public dropList(count: number, front: boolean): void {
    if (count <= 0 || count >= this.listVerses.length) return;
    if (front) {
      const firstKept = this.listVerses[count].sid;
      this.listVerses = this.listVerses.slice(count);
      this.listStart = this.positionOf(firstKept, this.listStart);
    } else {
      const lastKept = this.listVerses[this.listVerses.length - count - 1].sid;
      this.listVerses = this.listVerses.slice(
        0,
        this.listVerses.length - count
      );
      this.listEnd = this.positionOf(lastKept, this.listStart) + 1;
    }
  }

  /** Where verse `sid` is in the list, looking from position `from`. */
  private positionOf(sid: string, from: number): number {
    const index = this.listSids.indexOf(sid, from);
    return index < 0 ? this.listSids.indexOf(sid) : index;
  }

  private async listSidsFor(source: ListSource): Promise<string[]> {
    if (source.kind === 'ref') {
      return this.moduleDbService.getSidsByRef(source.osis);
    }
    if (source.kind === 'text') {
      return this.moduleDbService.getTextSids(source.search);
    }
    return source.sids;
  }

  /** The verses at list positions `start` up to `end`. */
  private async listPage(start: number, end: number): Promise<Verse[]> {
    const source = this.listSource;
    if (!source || start >= end) return [];
    if (source.kind === 'ref') {
      return this.moduleDbService.getRefVerses(source.osis, start, end - start);
    }
    if (source.kind === 'text') {
      return this.moduleDbService.getTextVerses(
        source.search,
        start,
        end - start
      );
    }
    // Another text's verses: this module may lack some, which are left out.
    return this.moduleDbService.getVersesForSids(
      this.listSids.slice(start, end)
    );
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
      } else if (this.listSids.includes(sid)) {
        // Listed but not loaded: load a page around it. goToListSid would
        // make this text the leader, which following mustn't do.
        const seq = ++this.loadSeq;
        await this.openListAt(this.listSids.indexOf(sid), seq);
      }
    }
  }

  /**
   * The search is cleared (decided 2026-11-22): if the reader jumped to a
   * hit in the results (picker or arrows), the text shows that verse in its
   * chapter, and the place read before the search goes into Recent, the
   * way back; otherwise, after only scrolling through the results, the text
   * returns to where the reader was before searching.
   */
  private leaveSearch(): void {
    const jumped = this.jumpedSid;
    this.jumpedSid = undefined;
    if (!jumped) {
      this.openAt(this.browseSid);
      return;
    }
    recentPlaces.add(this.browseSid);
    recentPlaces.add(jumped);
    this.openAt(jumped);
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
    } else if (this.listSource) {
      // A word search runs again in the new Bible, whose words differ.
      await this.showList(this.listSource, this.topSid);
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
      let value = `${before ?? ''} ${selectedOption.value}`.trim();
      // A word that would read as a reference ("job", "acts") goes in
      // quotes: quotes always mean words, never a reference.
      if (
        selectedOption.type === 'word' &&
        parseSearch(value).searchType !== 'text'
      ) {
        value = `${before ?? ''} "${selectedOption.value}"`.trim();
      }
      this.searchInput = value;
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
    this.highlightSid = undefined;
    this.suggest(searchInput);
    this.searchDebounced();
  }

  /** The suggestions for what's typed: books, then words. */
  private suggest(searchInput: string): void {
    this.autoCompleteOptions = [];
    const [_before, after] = splitByLastDelimiter(
      searchInput.toLowerCase(),
      searchDelimiters
    );
    if (after.length >= 2) {
      this.addReferenceAutoCompleteOptions(after);
      if (this.autoCompleteOptions.length > 0) {
        this.autoCompleteOptions[0].selected = true;
      }
      this.addWordAutoCompleteOptions(searchInput, after);
    }
  }

  /** Whether the suggestions list is showing. */
  public get autoCompleteOpen(): boolean {
    return this.autoCompleteOptions.length > 0;
  }

  /** Closes the suggestions, keeping the search and its results. */
  public closeAutoComplete(): void {
    this.autoCompleteOptions = [];
  }

  /** Shows the suggestions for what's typed again, without searching. */
  public openAutoComplete(): void {
    this.suggest(this.searchInput);
  }

  /**
   * Words in the module beginning with what's being typed, after the books,
   * each with how many verses have it. They come from Rust, so they're
   * added only if the search hasn't changed meanwhile.
   */
  private async addWordAutoCompleteOptions(
    searchInput: string,
    prefix: string
  ): Promise<void> {
    const word = prefix.replace(/["\u201C\u201D]/g, '');
    if (word.length < 2) return;
    try {
      const words = await this.moduleDbService.getWordSuggestions(word, 5);
      if (this.searchInput !== searchInput || words.length === 0) return;
      const anySelected = this.autoCompleteOptions.length > 0;
      this.autoCompleteOptions = [
        ...this.autoCompleteOptions,
        ...words.map((found, index) => ({
          value: found.word,
          type: 'word' as const,
          count: found.verses,
          selected: !anySelected && index === 0,
        })),
      ];
    } catch {
      // Suggestions are only a help: a failure leaves the books shown.
    }
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
      if (this.mode !== 'chapters') this.leaveSearch();
      return;
    }
    const request = parseSearch(search);
    let source: ListSource | undefined;
    if (request.searchType === 'point' && request.sid) {
      this.openAt(request.sid);
      return;
    } else if (request.searchType === 'ref' && request.osis) {
      source = { kind: 'ref', osis: request.osis };
    } else if (request.text) {
      source = { kind: 'text', search: request.text };
    }
    if (source && (this.mode !== 'list' || this.listKey !== listKey(source))) {
      this.showList(source);
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
            count: this.outline.find((o) => o.code === b.code)?.verses.length,
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
    [this.books, this.outline] = await Promise.all([
      this.moduleDbService.getBooks(),
      this.moduleDbService.getOutline(),
    ]);
  }
}
