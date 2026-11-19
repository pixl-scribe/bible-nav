/**
 * The parallel pane beside a search's main text. Each search has at most one,
 * holding any number of tabs: parallel texts (another Bible, following the
 * main text), modules' Notes tabs, and lexicon tabs.
 */
import type { LexiconPlace } from './lexicon-service';

export interface NotesTab {
  kind: 'notes';
  /** One Notes tab per module, so its key is the module id. */
  key: string;
  moduleId: string;
  /** A verse in the chapter whose notes are listed. */
  sid: string;
  /** The note to highlight, by its letter in that chapter. */
  focusLetter: string | undefined;
  /** Goes up each time the focused note should be scrolled to the top. */
  scrollRequest: number;
}

export interface TextTab {
  kind: 'text';
  /** Stays the same when the tab switches modules, so it keeps its place. */
  uid: number;
  /** One tab per module, so its key is the module id. */
  key: string;
  moduleId: string;
  /** Where it opened: the main text's verse at that moment. */
  startSid: string;
}

/**
 * A lexicon tab: one per lexicon, showing the latest word looked up in it.
 * It scrolls on its own and doesn't follow the chapter. Back retraces the
 * links followed inside the lexicon ("from H7218"); a word opened from a
 * Bible text, or from "Also in", starts afresh (decided 2026-11-18).
 */
export interface LexiconTab {
  kind: 'lexicon';
  /** One tab per lexicon, so its key is the module id. */
  key: string;
  moduleId: string;
  place: LexiconPlace;
  /** Earlier places, most recent last. */
  back: LexiconPlace[];
}

export type ParallelTab = NotesTab | TextTab | LexiconTab;

/** How many places Back remembers in each lexicon tab. */
const BACK_LIMIT = 50;

export function lexiconTabKey(moduleId: string): string {
  return `lexicon:${moduleId}`;
}

function samePlace(a: LexiconPlace, b: LexiconPlace): boolean {
  return a.kind === b.kind && a.value === b.value;
}

export function notesTabKey(moduleId: string): string {
  return `notes:${moduleId}`;
}

export function textTabKey(moduleId: string): string {
  return `text:${moduleId}`;
}

/** The chapter of a verse: 'PSA 12:3' is in 'PSA 12'. */
export function chapterOf(sid: string): string {
  return sid.split(':')[0];
}

export class ParallelPane {
  public open = $state(false);
  public tabs = $state<ParallelTab[]>([]);
  public activeKey = $state<string | undefined>(undefined);
  private nextUid = 1;

  public get active(): ParallelTab | undefined {
    return this.tabs.find((tab) => tab.key === this.activeKey);
  }

  /** Shows or hides the pane. Its tabs are kept while it's hidden. */
  public toggle(): void {
    this.open = !this.open;
  }

  public select(key: string): void {
    if (this.tabs.some((tab) => tab.key === key)) this.activeKey = key;
  }

  /**
   * Opens a module's Notes tab on the chapter holding `sid`, opening the pane
   * if it's closed. With `letter`, that note is highlighted and scrolled to
   * the top; without it, the list starts at the top of the chapter.
   */
  public openNotes(moduleId: string, sid: string, letter?: string): void {
    const key = notesTabKey(moduleId);
    const existing = this.tabs.find(
      (tab): tab is NotesTab => tab.kind === 'notes' && tab.key === key
    );
    if (existing) {
      existing.sid = sid;
      existing.focusLetter = letter;
      existing.scrollRequest += 1;
    } else {
      this.tabs.push({
        kind: 'notes',
        key,
        moduleId,
        sid,
        focusLetter: letter,
        scrollRequest: 1,
      });
    }
    this.activeKey = key;
    this.open = true;
  }

  /**
   * Opens a parallel text of a module at the main text's verse `sid`, or
   * selects it if it's open, opening the pane if it's closed.
   */
  public openText(moduleId: string, sid: string): void {
    const key = textTabKey(moduleId);
    if (!this.tabs.some((tab) => tab.key === key)) {
      this.tabs.push({
        kind: 'text',
        uid: this.nextUid++,
        key,
        moduleId,
        startSid: sid,
      });
    }
    this.activeKey = key;
    this.open = true;
  }

  /**
   * Opens a lexicon's tab at `place`, or moves its tab there, and opens the
   * pane if it's closed. With `followed` (a link inside an entry), the place
   * it showed is kept for Back; otherwise its history starts afresh.
   */
  public openLexicon(
    moduleId: string,
    place: LexiconPlace,
    followed = false
  ): void {
    const key = lexiconTabKey(moduleId);
    const existing = this.tabs.find(
      (tab): tab is LexiconTab => tab.kind === 'lexicon' && tab.key === key
    );
    if (!existing) {
      this.tabs.push({ kind: 'lexicon', key, moduleId, place, back: [] });
    } else if (!followed) {
      // The same entry keeps its loaded headword; another replaces it.
      existing.place = samePlace(existing.place, place)
        ? { ...existing.place, word: place.word ?? existing.place.word }
        : place;
      existing.back = [];
    } else if (samePlace(existing.place, place)) {
      // The same entry again: keep its history, take the newer word.
      if (place.word) existing.place.word = place.word;
    } else {
      existing.back = [...existing.back, existing.place].slice(-BACK_LIMIT);
      existing.place = place;
    }
    this.activeKey = key;
    this.open = true;
  }

  /** Goes back to the lexicon tab's previous place, if it has one. */
  public lexiconBack(key: string): void {
    const tab = this.tabs.find((t) => t.key === key);
    if (tab?.kind !== 'lexicon' || tab.back.length === 0) return;
    tab.place = tab.back[tab.back.length - 1];
    tab.back = tab.back.slice(0, -1);
  }

  /**
   * Switches a parallel text to another module, from its Bible selector.
   * If that module is already open in another tab, that tab is selected
   * instead, so each module is open once.
   */
  public setTextModule(key: string, moduleId: string): void {
    const tab = this.tabs.find((t) => t.key === key);
    if (!tab || tab.kind !== 'text') return;
    const other = textTabKey(moduleId);
    if (this.tabs.some((t) => t.key === other)) {
      this.activeKey = other;
      return;
    }
    tab.moduleId = moduleId;
    tab.key = other;
    if (this.activeKey === key) this.activeKey = other;
  }

  /**
   * Moves every Notes tab to the chapter holding `sid`: the chapter now in
   * view. Called only when that chapter changes, so a note clicked further
   * down, in the next chapter, stays shown until the reader scrolls into
   * another chapter. The highlighted note is kept, for when the reader comes
   * back to its chapter.
   */
  public followChapter(sid: string): void {
    for (const tab of this.tabs) {
      if (tab.kind !== 'notes' || chapterOf(tab.sid) === chapterOf(sid)) {
        continue;
      }
      tab.sid = sid;
      tab.scrollRequest += 1;
    }
  }

  /** Highlights a note without scrolling the list. */
  public focusNote(key: string, letter: string): void {
    const tab = this.tabs.find((t) => t.key === key);
    if (tab?.kind === 'notes') tab.focusLetter = letter;
  }

  /**
   * Closes a tab. Closing the selected tab selects the last one left, and
   * closing the last tab closes the pane.
   */
  public close(key: string): void {
    this.tabs = this.tabs.filter((tab) => tab.key !== key);
    if (this.activeKey === key) {
      this.activeKey = this.tabs[this.tabs.length - 1]?.key;
    }
    if (this.tabs.length === 0) this.open = false;
  }
}
