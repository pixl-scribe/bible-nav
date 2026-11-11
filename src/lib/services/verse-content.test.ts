import { describe, expect, test } from 'vitest';
import type { Note, VerseChild } from '$lib/model/verse';
import {
  footnoteLetter,
  letterNotes,
  noteText,
  parseChildren,
  plainText,
} from '$lib/services/verse-content';
import samples from '$lib/services/fixtures/kjv-samples.json';

const verse = (sid: string) => {
  const found = samples.verses.find((v) => v.sid === sid);
  if (!found) throw new Error(`missing fixture ${sid}`);
  return { ...found, children: parseChildren(found.children) };
};

const notes = (children: VerseChild[]): Note[] =>
  children.filter(
    (child): child is Note => typeof child === 'object' && child.style === 'f'
  );

describe('plainText', () => {
  test.each(samples.verses.map((v) => v.sid))(
    'joins %s to exactly its stored raw text',
    (sid) => {
      const { children, raw } = verse(sid);
      expect(plainText(children)).toBe(raw);
    }
  );

  test('keeps the words the old format lost', () => {
    expect(plainText(verse('PSA 23:1').children)).toContain('The LORD is');
    expect(plainText(verse('EXO 15:13').children)).toContain(
      'the people which thou hast redeemed'
    );
    expect(plainText(verse('MAT 26:59').children)).toContain(
      'sought false witness'
    );
  });

  test('joins headings, including transliterated text', () => {
    for (const heading of samples.headings) {
      expect(plainText(parseChildren(heading.children))).toBe(heading.raw);
    }
  });
});

describe('noteText', () => {
  test('starts with the footnote reference, kept as text', () => {
    const [note] = notes(verse('GEN 4:10').children);
    expect(noteText(note)).toMatch(/^4\.10 /);
  });

  test('keeps nested text inside a note', () => {
    const [note] = notes(verse('EXO 6:2').children);
    expect(noteText(note)).toContain('JEHOVAH');
    expect(noteText(note)).toContain('LORD');
  });
});

describe('parseChildren', () => {
  test('rejects content that is not an array', () => {
    expect(() => parseChildren('{"style":"w"}')).toThrow();
  });
});

describe('footnoteLetter', () => {
  test('runs a..z, then aa..az, ba..bz, and on', () => {
    expect([0, 1, 25].map(footnoteLetter)).toEqual(['a', 'b', 'z']);
    expect([26, 27, 51].map(footnoteLetter)).toEqual(['aa', 'ab', 'az']);
    expect([52, 77].map(footnoteLetter)).toEqual(['ba', 'bz']);
    expect([701, 702].map(footnoteLetter)).toEqual(['zz', 'aaa']);
  });
});

describe('letterNotes', () => {
  test('letters notes in reading order from the chapter count', () => {
    const children = parseChildren<VerseChild>(
      '["x",{"style":"f","children":[]},{"style":"wj","children":["y"]},{"style":"f","children":[]}]'
    );
    const lettered = letterNotes(children, 25);
    expect(notes(lettered).map((note) => note.letter)).toEqual(['z', 'aa']);
    expect(plainText(lettered)).toBe('xy');
  });

  test('leaves the stored content unchanged', () => {
    const children = verse('GEN 4:10').children;
    letterNotes(children, 0);
    expect(notes(children)[0].letter).toBeUndefined();
  });
});
