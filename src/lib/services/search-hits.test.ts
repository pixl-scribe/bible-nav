import { describe, expect, it } from 'vitest';
import { hitChapterStep, indexHits } from './search-hits';

// The first hits for "covenant" in the KJV.
const sids = [
  'GEN 6:18',
  'GEN 9:9',
  'GEN 9:11',
  'GEN 9:12',
  'GEN 15:18',
  'EXO 2:24',
];

describe('indexHits', () => {
  it('counts matching verses per book and chapter', () => {
    const index = indexHits(sids);
    expect(index.byBook.get('GEN')).toBe(5);
    expect(index.byBook.get('EXO')).toBe(1);
    expect(index.byBook.get('LEV')).toBeUndefined();
    expect(index.byChapter.get('GEN 9')).toBe(3);
    expect(index.firstInBook.get('GEN')).toBe('GEN 6:18');
    expect(index.verses.has('GEN 9:11')).toBe(true);
    expect(index.chapters.map((c) => c.chapter)).toEqual([
      'GEN 6',
      'GEN 9',
      'GEN 15',
      'EXO 2',
    ]);
  });
});

describe('hitChapterStep', () => {
  const index = indexHits(sids);

  it('steps between chapters with hits, across books', () => {
    expect(hitChapterStep(index, 'GEN 6:18', 1)).toEqual({
      chapter: 'GEN 9',
      first: 'GEN 9:9',
      count: 3,
    });
    expect(hitChapterStep(index, 'GEN 9:12', -1)?.first).toBe('GEN 6:18');
    expect(hitChapterStep(index, 'GEN 15:18', 1)?.first).toBe('EXO 2:24');
  });

  it('stops at either end', () => {
    expect(hitChapterStep(index, 'GEN 6:18', -1)).toBeUndefined();
    expect(hitChapterStep(index, 'EXO 2:24', 1)).toBeUndefined();
    expect(hitChapterStep(index, 'REV 1:1', 1)).toBeUndefined();
  });
});
