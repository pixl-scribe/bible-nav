/**
 * The parallel pane beside a search's main text. Each search has at most one,
 * holding any number of tabs. Today the only kind of tab is a module's Notes
 * tab; parallel texts and other tools will be further kinds.
 */

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

export type ParallelTab = NotesTab;

export function notesTabKey(moduleId: string): string {
  return `notes:${moduleId}`;
}

/** The chapter of a verse: 'PSA 12:3' is in 'PSA 12'. */
export function chapterOf(sid: string): string {
  return sid.split(':')[0];
}

export class ParallelPane {
  public open = $state(false);
  public tabs = $state<ParallelTab[]>([]);
  public activeKey = $state<string | undefined>(undefined);

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
    const existing = this.tabs.find((tab) => tab.key === key);
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
   * Moves a module's Notes tab, if it's open, to the chapter holding `sid`:
   * the chapter now in view. Called only when that chapter changes, so a
   * note clicked further down, in the next chapter, stays shown until the
   * reader scrolls into another chapter. The highlighted note is kept, for
   * when the reader comes back to its chapter.
   */
  public followChapter(moduleId: string, sid: string): void {
    const tab = this.tabs.find((t) => t.key === notesTabKey(moduleId));
    if (!tab || chapterOf(tab.sid) === chapterOf(sid)) return;
    tab.sid = sid;
    tab.scrollRequest += 1;
  }

  /** Highlights a note without scrolling the list. */
  public focusNote(key: string, letter: string): void {
    const tab = this.tabs.find((t) => t.key === key);
    if (tab) tab.focusLetter = letter;
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
