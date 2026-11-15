import { describe, expect, test } from 'vitest';
import type { Heading, Paragraph, Verse } from '$lib/model/verse';
import { chapterOfSid, chapterVerses, type Chapter } from './chapter-text';

function verse(sid: string): Verse {
  return {
    id: 0,
    bookId: 1,
    chapter: 1,
    nbr: Number(sid.split(':')[1]),
    paragraph: 0,
    sid,
    children: [],
    noteStart: 0,
    headingsBefore: [],
    headingsAfter: [],
  } as Verse;
}

function heading(id: number, style: string): Heading {
  return {
    id,
    bookId: 1,
    chapter: 1,
    paragraph: 0,
    afterVerseId: 0,
    style,
    children: [],
  } as Heading;
}

function paragraph(contents: {
  heading?: Heading;
  verses?: Verse[];
}): Paragraph {
  return {
    id: 0,
    bookId: 1,
    chapter: 1,
    style: contents.heading ? contents.heading.style : 'p',
    heading: contents.heading,
    verses: contents.verses ?? [],
  } as Paragraph;
}

describe('chapter text', () => {
  test('grouping by verse puts each heading with the verse it belongs to', () => {
    const chapter: Chapter = {
      sid: 'PSA 3',
      bookCode: 'PSA',
      chapter: 3,
      paragraphs: [
        paragraph({ heading: heading(1, 'd') }),
        paragraph({ verses: [verse('PSA 3:1'), verse('PSA 3:2')] }),
        paragraph({ verses: [verse('PSA 3:3')] }),
        paragraph({ heading: heading(2, 's1') }),
      ],
    };
    const verses = chapterVerses(chapter);
    expect(verses.map((v) => v.sid)).toEqual(['PSA 3:1', 'PSA 3:2', 'PSA 3:3']);
    expect(verses[0].headingsBefore.map((h) => h.id)).toEqual([1]);
    expect(verses[1].headingsBefore).toEqual([]);
    expect(verses[2].headingsAfter.map((h) => h.id)).toEqual([2]);
  });

  test('the chapter of a verse', () => {
    expect(chapterOfSid('PSA 119:176')).toBe('PSA 119');
  });
});
