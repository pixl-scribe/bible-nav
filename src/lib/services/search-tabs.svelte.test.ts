import { describe, expect, test } from 'vitest';
import { SearchTabs } from './search-tabs.svelte';
import { ParallelPane } from './parallel-pane.svelte';

describe('SearchTabs', () => {
  test('starts with one new search, shown', () => {
    const searches = new SearchTabs('eng_kjv');
    expect(searches.tabs).toHaveLength(1);
    expect(searches.activeIndex).toBe(0);
    expect(searches.tabs[0].moduleId).toBe('eng_kjv');
  });

  test('shows each new search as it opens', () => {
    const searches = new SearchTabs('eng_kjv');
    const second = searches.add();
    expect(searches.activeId).toBe(second.id);
    expect(searches.activeIndex).toBe(1);
  });

  test('closing the shown search shows its left neighbour', () => {
    const searches = new SearchTabs('eng_kjv');
    const [first] = searches.tabs;
    const second = searches.add();
    const third = searches.add();
    searches.select(second.id);
    searches.close(second.id);
    expect(searches.activeId).toBe(first.id);
    expect(searches.tabs.map((t) => t.id)).toEqual([first.id, third.id]);
  });

  test('closing the first search, while shown, shows the new first', () => {
    const searches = new SearchTabs('eng_kjv');
    const [first] = searches.tabs;
    const second = searches.add();
    searches.select(first.id);
    searches.close(first.id);
    expect(searches.activeId).toBe(second.id);
  });

  test('closing another search keeps the shown one', () => {
    const searches = new SearchTabs('eng_kjv');
    const [first] = searches.tabs;
    const second = searches.add();
    searches.close(first.id);
    expect(searches.activeId).toBe(second.id);
  });

  test('closing the last search opens a new one', () => {
    const searches = new SearchTabs('eng_kjv');
    const [only] = searches.tabs;
    searches.close(only.id);
    expect(searches.tabs).toHaveLength(1);
    expect(searches.tabs[0].id).not.toBe(only.id);
    expect(searches.activeIndex).toBe(0);
  });

  test('each search has its own parallel pane', () => {
    const searches = new SearchTabs('eng_kjv');
    const second = searches.add();
    second.parallel.openNotes('eng_kjv', 'PSA 12:1', 'a');
    expect(searches.tabs[0].parallel.open).toBe(false);
    expect(second.parallel.open).toBe(true);
  });
});

describe('ParallelPane', () => {
  test('a footnote opens its Notes tab and the pane', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    expect(pane.open).toBe(true);
    expect(pane.active).toMatchObject({
      kind: 'notes',
      moduleId: 'eng_kjv',
      sid: 'PSA 12:3',
      focusLetter: 'd',
    });
  });

  test('another footnote reuses the tab and asks to scroll again', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    const firstRequest = pane.active!.scrollRequest;
    pane.toggle();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    expect(pane.tabs).toHaveLength(1);
    expect(pane.open).toBe(true);
    expect(pane.active!.scrollRequest).toBeGreaterThan(firstRequest);
  });

  test('focusing a note highlights it without asking to scroll', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    const request = pane.active!.scrollRequest;
    pane.focusNote(pane.active!.key, 'g');
    expect(pane.active!.focusLetter).toBe('g');
    expect(pane.active!.scrollRequest).toBe(request);
  });

  test('closing the last tab closes the pane', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    pane.close(pane.active!.key);
    expect(pane.tabs).toHaveLength(0);
    expect(pane.active).toBeUndefined();
    expect(pane.open).toBe(false);
  });

  test('hiding the pane keeps its tabs', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    pane.toggle();
    expect(pane.open).toBe(false);
    expect(pane.tabs).toHaveLength(1);
  });
});
