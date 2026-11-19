import { describe, expect, test } from 'vitest';
import { ParallelPane, type LexiconTab } from './parallel-pane.svelte';

const word = (value: string, text?: string) => ({
  kind: 'strong' as const,
  value,
  word: text,
});

function lexiconTab(pane: ParallelPane): LexiconTab {
  const tab = pane.active;
  if (tab?.kind !== 'lexicon') throw new Error('no lexicon tab');
  return tab;
}

describe('lexicon tabs', () => {
  test('one tab per lexicon, opening the pane', () => {
    const pane = new ParallelPane();
    pane.openLexicon('eng_strongs_heb', word('H7225', 'beginning'));
    pane.openLexicon('eng_strongs_heb', word('H1254', 'created'));
    expect(pane.open).toBe(true);
    expect(pane.tabs).toHaveLength(1);
    expect(lexiconTab(pane).place.value).toBe('H1254');
  });

  test('Back retraces links followed inside entries', () => {
    const pane = new ParallelPane();
    pane.openLexicon('eng_strongs_heb', word('H7225', 'beginning'));
    pane.openLexicon('eng_strongs_heb', word('H7218'), true);
    pane.openLexicon('eng_strongs_heb', word('H7219'), true);
    expect(lexiconTab(pane).back.map((p) => p.value)).toEqual([
      'H7225',
      'H7218',
    ]);
    pane.lexiconBack('lexicon:eng_strongs_heb');
    expect(lexiconTab(pane).place.value).toBe('H7218');
    expect(lexiconTab(pane).back.map((p) => p.value)).toEqual(['H7225']);
  });

  test('a word from a Bible text starts the history afresh', () => {
    const pane = new ParallelPane();
    pane.openLexicon('eng_strongs_heb', word('H7225', 'beginning'));
    pane.openLexicon('eng_strongs_heb', word('H7218'), true);
    pane.openLexicon('eng_strongs_heb', word('H1254', 'created'));
    expect(lexiconTab(pane).place.value).toBe('H1254');
    expect(lexiconTab(pane).back).toEqual([]);
  });

  test('the same word again keeps its loaded headword', () => {
    const pane = new ParallelPane();
    pane.openLexicon('eng_strongs_heb', word('H7225', 'beginning'));
    lexiconTab(pane).place.headword = 'reshit';
    pane.openLexicon('eng_strongs_heb', word('H7225', 'firstfruits'));
    expect(lexiconTab(pane).place.headword).toBe('reshit');
    expect(lexiconTab(pane).place.word).toBe('firstfruits');
  });
});
