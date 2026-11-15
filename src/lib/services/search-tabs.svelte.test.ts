import { describe, expect, test } from 'vitest';
import { SearchTabs } from './search-tabs.svelte';
import { chapterOf, ParallelPane, type NotesTab } from './parallel-pane.svelte';

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

/** The active tab, which these tests know is a Notes tab. */
function notesOf(pane: ParallelPane): NotesTab {
  return pane.active as NotesTab;
}

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
    const firstRequest = notesOf(pane).scrollRequest;
    pane.toggle();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    expect(pane.tabs).toHaveLength(1);
    expect(pane.open).toBe(true);
    expect(notesOf(pane).scrollRequest).toBeGreaterThan(firstRequest);
  });

  test('focusing a note highlights it without asking to scroll', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    const request = notesOf(pane).scrollRequest;
    pane.focusNote(pane.active!.key, 'g');
    expect(notesOf(pane).focusLetter).toBe('g');
    expect(notesOf(pane).scrollRequest).toBe(request);
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

  test('the Notes tab follows the reader to another chapter', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    const request = notesOf(pane).scrollRequest;
    pane.followChapter('PSA 13:1');
    expect(notesOf(pane).sid).toBe('PSA 13:1');
    expect(notesOf(pane).scrollRequest).toBeGreaterThan(request);
    // The highlighted note is kept for when the reader comes back.
    expect(notesOf(pane).focusLetter).toBe('d');
  });

  test('following within the same chapter changes nothing', () => {
    const pane = new ParallelPane();
    pane.openNotes('eng_kjv', 'PSA 12:3', 'd');
    const request = notesOf(pane).scrollRequest;
    pane.followChapter('PSA 12:8');
    expect(notesOf(pane).sid).toBe('PSA 12:3');
    expect(notesOf(pane).scrollRequest).toBe(request);
  });

  test('following never opens a Notes tab', () => {
    const pane = new ParallelPane();
    pane.followChapter('PSA 13:1');
    expect(pane.tabs).toHaveLength(0);
    expect(pane.open).toBe(false);
  });

  test('chapterOf drops the verse number', () => {
    expect(chapterOf('PSA 119:176')).toBe('PSA 119');
  });
});

describe('parallel texts', () => {
  test("a parallel text opens once per Bible, at the main text's verse", () => {
    const pane = new ParallelPane();
    pane.openText('eng_lsv', 'GEN 1:3');
    pane.openNotes('eng_kjv', 'GEN 1:3');
    pane.openText('eng_lsv', 'GEN 2:1');
    expect(pane.tabs.map((tab) => tab.key)).toEqual([
      'text:eng_lsv',
      'notes:eng_kjv',
    ]);
    expect(pane.activeKey).toBe('text:eng_lsv');
    expect(pane.active).toMatchObject({ kind: 'text', startSid: 'GEN 1:3' });
  });

  test("switching a parallel text's Bible keeps the tab, or selects an open one", () => {
    const pane = new ParallelPane();
    pane.openText('eng_lsv', 'GEN 1:3');
    const uid = pane.active?.kind === 'text' ? pane.active.uid : 0;
    pane.setTextModule('text:eng_lsv', 'eng_kjv');
    expect(pane.active).toMatchObject({
      key: 'text:eng_kjv',
      moduleId: 'eng_kjv',
      uid,
    });
    pane.openText('eng_lsv', 'GEN 1:3');
    pane.setTextModule('text:eng_lsv', 'eng_kjv');
    expect(pane.tabs).toHaveLength(2);
    expect(pane.activeKey).toBe('text:eng_kjv');
  });

  test('every Notes tab follows the chapter, and parallel texts are left alone', () => {
    const pane = new ParallelPane();
    pane.openText('eng_lsv', 'GEN 1:3');
    pane.openNotes('eng_kjv', 'GEN 1:3');
    pane.followChapter('GEN 2:1');
    expect(pane.tabs.find((tab) => tab.kind === 'notes')).toMatchObject({
      sid: 'GEN 2:1',
    });
    expect(pane.tabs.find((tab) => tab.kind === 'text')).toMatchObject({
      startSid: 'GEN 1:3',
    });
  });
});

describe('new searches', () => {
  test('a new search reads the Bible of the last tab, unless another is chosen', () => {
    const searches = new SearchTabs('eng_kjv');
    searches.tabs[0].moduleId = 'eng_lsv';
    expect(searches.add().moduleId).toBe('eng_lsv');
    expect(searches.add('eng_kjv').moduleId).toBe('eng_kjv');
    expect(searches.add().moduleId).toBe('eng_kjv');
  });
});
