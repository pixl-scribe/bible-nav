/**
 * Draws real KJV verses through the actual components and checks that the
 * text a reader sees is exactly the module's stored text: no word dropped,
 * and no space added or lost by the templates.
 */
import { describe, expect, test } from 'vitest';
import { render } from 'svelte/server';
import VerseText from '$lib/components/verse-text.svelte';
import HeadingText from '$lib/components/heading-text.svelte';
import ParagraphView from '$lib/components/paragraph.svelte';
import type {
  Heading,
  Inline,
  Paragraph,
  Verse,
  VerseChild,
} from '$lib/model/verse';
import { letterNotes, parseChildren } from '$lib/services/verse-content';
import samples from '$lib/services/fixtures/kjv-samples.json';

function toVerse(sample: (typeof samples.verses)[number]): Verse {
  return {
    id: 1,
    bookId: 1,
    chapter: 1,
    nbr: 1,
    paragraph: 0,
    sid: sample.sid,
    children: letterNotes(parseChildren(sample.children), 0),
    noteStart: 0,
    headings: [],
  };
}

/** The visible text of rendered HTML, with footnote markers left out. */
function visibleText(html: string): string {
  return html
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<sup[^>]*data-footnote[^>]*>[\s\S]*?<\/sup>/g, '')
    .replace(/<[^>]+>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, '&');
}

describe('verse rendering', () => {
  test.each(samples.verses.map((v) => v.sid))(
    '%s shows exactly its stored text',
    (sid) => {
      const sample = samples.verses.find((v) => v.sid === sid)!;
      const { body } = render(VerseText, {
        props: { verse: toVerse(sample) },
      });
      expect(visibleText(body)).toBe(sample.raw);
    }
  );

  test('shows translator additions in italics and words of Jesus in red', () => {
    const psalm = samples.verses.find((v) => v.sid === 'PSA 23:1')!;
    expect(
      render(VerseText, { props: { verse: toVerse(psalm) } }).body
    ).toMatch(/<em class="add">[^<]*(<!--[^>]*-->)*is/);
    const john = samples.verses.find((v) => v.sid === 'JHN 3:16')!;
    expect(
      render(VerseText, { props: { verse: toVerse(john) } }).body
    ).toContain('class="wj text-error"');
  });

  test('shows a marker with the note for each footnote', () => {
    const sample = samples.verses.find((v) => v.sid === 'GEN 4:10')!;
    const { body } = render(VerseText, { props: { verse: toVerse(sample) } });
    expect(body).toContain('data-footnote');
    // The marker names its note, which starts with its reference, for screen
    // readers. Clicking it opens the Notes tab, so it holds no note text.
    expect(body).toMatch(/aria-label="Footnote a: 4\.10 [^"]+"/);
    expect(body).not.toContain('popover');
  });

  test.each(samples.headings.map((h) => h.raw))(
    'heading "%s" shows exactly its stored text',
    (raw) => {
      const sample = samples.headings.find((h) => h.raw === raw)!;
      const heading: Heading = {
        id: 1,
        bookId: 19,
        chapter: 3,
        paragraph: 0,
        afterVerseId: 0,
        style: sample.style,
        children: parseChildren<Inline>(sample.children),
      };
      const { body } = render(HeadingText, { props: { heading } });
      expect(visibleText(body).trim()).toBe(sample.raw);
    }
  );

  test('separates verses in a paragraph with a space and their number', () => {
    const [first, second] = ['PSA 23:1', 'EXO 15:13'].map((sid) =>
      samples.verses.find((v) => v.sid === sid)!
    );
    const paragraph: Paragraph = {
      id: 0,
      bookId: 1,
      chapter: 1,
      style: 'p',
      verses: [
        { ...toVerse(first), nbr: 1 },
        { ...toVerse(second), sid: 'TST 1:2', nbr: 2 },
      ],
    };
    const { body } = render(ParagraphView, {
      props: { paragraph, bibleTextService: undefined },
    });
    expect(visibleText(body)).toContain(`${first.raw} 2 ${second.raw}`);
  });

  test('labels each footnote with its letter', () => {
    const sample = samples.verses.find((v) => v.sid === 'GEN 4:10')!;
    const verse = {
      ...toVerse(sample),
      children: letterNotes(parseChildren(sample.children), 2),
    };
    const { body } = render(VerseText, { props: { verse } });
    expect(body).toMatch(
      /role="button"[^>]*aria-label="Footnote c: 4\.10 [^"]*"[^>]*>c<\/span>/
    );
    // The marker asks the pane's shared tooltip to show the note, so it
    // carries no tooltip of its own.
    expect(body).not.toContain('data-tip');
  });

  test.each(samples.verses.map((v) => v.sid))(
    '%s shows each footnote letter exactly in place, with no added spaces',
    (sid) => {
      const sample = samples.verses.find((v) => v.sid === sid)!;
      const children = letterNotes(parseChildren(sample.children), 0);
      // The stored text with each footnote's letter where the note sits.
      const withLetters = (items: VerseChild[]): string =>
        items
          .map((item) => {
            if (typeof item === 'string') return item;
            if (item.style === 'f') return item.letter;
            if (item.style === 'w') return item.txt;
            return withLetters(item.children);
          })
          .join('');
      const { body } = render(VerseText, {
        props: { verse: { ...toVerse(sample), children } },
      });
      // Keep the markers' letters.
      expect(visibleText(body.replace(/data-footnote/g, ''))).toBe(
        withLetters(children)
      );
    }
  );
});
