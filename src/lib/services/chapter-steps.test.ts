import { describe, expect, it } from 'vitest';
import { adjacentChapter, type BookOutline } from './chapter-steps';

const bible: BookOutline[] = [
  { code: 'GEN', verses: Array(50).fill(30) },
  { code: 'EXO', verses: Array(40).fill(30) },
  { code: 'REV', verses: Array(22).fill(20) },
];

describe('adjacentChapter', () => {
  it('steps within a book', () => {
    expect(adjacentChapter(bible, 'GEN 4', 1)).toBe('GEN 5');
    expect(adjacentChapter(bible, 'GEN 4', -1)).toBe('GEN 3');
  });

  it('crosses into the next and previous books', () => {
    expect(adjacentChapter(bible, 'GEN 50', 1)).toBe('EXO 1');
    expect(adjacentChapter(bible, 'EXO 1', -1)).toBe('GEN 50');
  });

  it('stops at the ends of the module', () => {
    expect(adjacentChapter(bible, 'GEN 1', -1)).toBeUndefined();
    expect(adjacentChapter(bible, 'REV 22', 1)).toBeUndefined();
  });

  it('finds the ends of a partial module', () => {
    const newTestament: BookOutline[] = [
      { code: 'MAT', verses: Array(28).fill(20) },
    ];
    expect(adjacentChapter(newTestament, 'MAT 1', -1)).toBeUndefined();
    expect(adjacentChapter(newTestament, 'MAT 28', 1)).toBeUndefined();
  });

  it("gives nothing for a chapter the module doesn't have", () => {
    expect(adjacentChapter(bible, 'GEN 51', 1)).toBeUndefined();
    expect(adjacentChapter(bible, 'MAT 1', 1)).toBeUndefined();
    expect(adjacentChapter([], 'GEN 1', 1)).toBeUndefined();
  });
});
