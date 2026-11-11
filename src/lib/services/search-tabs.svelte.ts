import { ParallelPane } from './parallel-pane.svelte';

/**
 * One search: a top-level tab with its own main text, parallel pane, and
 * place in the text. Switching tabs keeps each search's place.
 */
export interface SearchTab {
  id: number;
  moduleId: string;
  /** What the reader typed, or '' for a new search. */
  label: string;
  parallel: ParallelPane;
}

/** The open searches, any number of them, and which one is shown. */
export class SearchTabs {
  public tabs = $state<SearchTab[]>([]);
  public activeId = $state(0);
  private nextId = 1;

  constructor(private defaultModuleId: string) {
    this.add();
  }

  public get activeIndex(): number {
    return this.tabs.findIndex((tab) => tab.id === this.activeId);
  }

  /** Opens a new, empty search and shows it. */
  public add(): SearchTab {
    const tab: SearchTab = {
      id: this.nextId++,
      moduleId: this.defaultModuleId,
      label: '',
      parallel: new ParallelPane(),
    };
    this.tabs.push(tab);
    this.activeId = tab.id;
    return tab;
  }

  public select(id: number): void {
    if (this.tabs.some((tab) => tab.id === id)) this.activeId = id;
  }

  /**
   * Closes a search. Closing the one shown selects its neighbour (the one to
   * its left, or the new first tab). Closing the last search opens a new one,
   * so there's always a search to show.
   */
  public close(id: number): void {
    const index = this.tabs.findIndex((tab) => tab.id === id);
    if (index < 0) return;
    this.tabs.splice(index, 1);
    if (this.tabs.length === 0) {
      this.add();
    } else if (this.activeId === id) {
      this.activeId = this.tabs[Math.max(0, index - 1)].id;
    }
  }
}
