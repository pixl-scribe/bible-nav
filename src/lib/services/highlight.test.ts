import { describe, expect, it } from 'vitest';
import { matchPattern, splitMatches } from './highlight';

describe('highlighting matches', () => {
  it('marks whole words, ignoring case, and keeps every character', () => {
    const text =
      'This is the token of the covenant which I make; Covenant, covenanted.';
    const pattern = matchPattern(['covenant'])!;
    const parts = splitMatches(text, pattern);
    expect(parts.map((part) => part.text).join('')).toBe(text);
    expect(parts.filter((part) => part.hit).map((part) => part.text)).toEqual([
      'covenant',
      'Covenant',
    ]);
  });

  it('marks every matched form, longest first', () => {
    const pattern = matchPattern(['covenant', 'covenants'])!;
    const parts = splitMatches('my covenants and my covenant', pattern);
    expect(parts.filter((part) => part.hit).map((part) => part.text)).toEqual([
      'covenants',
      'covenant',
    ]);
  });

  it('works next to punctuation such as a curly apostrophe', () => {
    const pattern = matchPattern(['sons'])!;
    const parts = splitMatches('thy sons’ wives', pattern);
    expect(parts).toEqual([
      { text: 'thy ', hit: false },
      { text: 'sons', hit: true },
      { text: '’ wives', hit: false },
    ]);
  });

  it('gives nothing to highlight without words', () => {
    expect(matchPattern(undefined)).toBeUndefined();
    expect(matchPattern([])).toBeUndefined();
  });

  it('treats words as text, not patterns', () => {
    const pattern = matchPattern(['a.b'])!;
    expect(splitMatches('axb a.b', pattern).filter((p) => p.hit)).toEqual([
      { text: 'a.b', hit: true },
    ]);
  });
});
