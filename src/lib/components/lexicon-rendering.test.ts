/**
 * Draws real Strong's Hebrew entries through the lexicon renderer and checks
 * what a reader sees: the stored text with no space added or lost, links
 * shown with their number, and alternative spellings with their
 * transliteration and pronunciation.
 */
import { describe, expect, test } from 'vitest';
import { render } from 'svelte/server';
import LexiconInline from '$lib/components/lexicon-inline.svelte';
import type {
  LexiconElement,
  LexiconItem,
} from '$lib/services/lexicon-service';
import samples from '$lib/services/fixtures/sh-samples.json';
import lsSamples from '$lib/services/fixtures/ls-samples.json';
import bdbSamples from '$lib/services/fixtures/bdb-samples.json';
import {
  entryBlocks,
  headwordLine,
  type LexiconEntry,
} from '$lib/services/lexicon-service';
import type { LexiconPlace } from '$lib/services/lexicon-service';

function body(key: string): LexiconElement[] {
  const entry = samples.entries.find((e) => e.key === key);
  if (!entry) throw new Error(key);
  return JSON.parse(entry.body) as LexiconElement[];
}

function part(key: string, style: string): LexiconItem[] {
  const found = body(key).find((p) => p.style === style);
  if (!found) throw new Error(`${key} ${style}`);
  return found.children;
}

/** The visible text of rendered HTML. */
function visibleText(html: string): string {
  return html
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<[^>]+>/g, '')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, '&');
}

/** The stored text of items, as joined in the module. */
function storedText(items: LexiconItem[]): string {
  return items
    .map((item) =>
      typeof item === 'string' ? item : storedText(item.children)
    )
    .join('');
}

const show = (items: LexiconItem[]) =>
  render(LexiconInline, { props: { items, onOpen: () => {} } }).body;

describe('lexicon entries', () => {
  test('text with italics reads exactly as stored', () => {
    const items = part('H7225', 'def');
    expect(visibleText(show(items))).toBe(storedText(items));
    expect(show(items)).toContain('<em>');
  });

  test('a link shows its number before the word, as a button', () => {
    const items = part('H7225', 'deriv');
    const html = show(items);
    expect(html).toMatch(/<button[^>]*class="lexicon-ref/);
    const linked = (items.find((i) => typeof i !== 'string') as LexiconElement)
      .children[0] as string;
    expect(visibleText(html)).toBe(`from the same as H7218 ${linked};`);
    expect(html).toContain('dir="rtl"');
  });

  test('an alternative spelling keeps its transliteration and pronunciation', () => {
    const text = visibleText(show(part('H113', 'deriv')));
    expect(text).toContain("ʼâdôn, aw-done'");
    expect(text.startsWith('or (shortened) ')).toBe(true);
    expect(text.endsWith('; from an unused root (meaning to rule);')).toBe(
      true
    );
  });

  test('a word given only as a pronunciation shows it', () => {
    expect(visibleText(show(part('H595', 'deriv')))).toBe(
      "sometimes, aw-no'-kee; a primitive pronoun;"
    );
  });
});

/** Liddell and Scott: real entries from the generated module. */
describe('Liddell and Scott entries', () => {
  const lsBody = (key: string): LexiconElement[] => {
    const entry = lsSamples.entries.find((e) => e.key === key);
    if (!entry) throw new Error(key);
    return JSON.parse(entry.body) as LexiconElement[];
  };

  test('every part reads exactly as stored', () => {
    for (const entry of lsSamples.entries) {
      for (const part of JSON.parse(entry.body) as LexiconElement[]) {
        expect(visibleText(show(part.children))).toBe(
          storedText(part.children)
        );
      }
    }
  });

  test('translations are in italics, and "Id." names its author', () => {
    const sense = lsBody('n105').find((p) => p.style === 'sense');
    const html = show(sense?.children ?? []);
    expect(html).toContain('<em class="lexicon-tr">');
    // Svelte's comment markers may sit around the text inside the <abbr>.
    expect(html).toMatch(
      /<abbr[^>]*title="Plat\."[^>]*>(?:<!--[^>]*-->)*Id\.(?:<!--[^>]*-->)*<\/abbr>/
    );
  });

  test('a link by headword opens every entry with that headword', () => {
    const items = lsBody('n15674')[0].children;
    const opened: LexiconPlace[] = [];
    const html = render(LexiconInline, {
      props: { items, onOpen: (place: LexiconPlace) => opened.push(place) },
    }).body;
    expect(html).toMatch(/<button[^>]*class="lexicon-ref/);
    const ref = items.find((i) => typeof i !== 'string') as LexiconElement;
    expect(ref.lemma).toBe(storedText(ref.children));
  });
});

describe('Brown-Driver-Briggs', () => {
  const bdb = (key: string): LexiconEntry => {
    const row = bdbSamples.entries.find((e) => e.key === key);
    if (!row) throw new Error(key);
    return { ...row, body: JSON.parse(row.body) as LexiconItem[] };
  };
  const showBdb = (
    items: LexiconItem[],
    describe = (sid: string, meant: string | undefined) =>
      meant ? `meant ${meant}` : `go ${sid}`
  ) =>
    render(LexiconInline, {
      props: { items, onOpen: () => {}, describeRef: describe },
    }).body;

  test('every part reads exactly as stored', () => {
    for (const row of bdbSamples.entries) {
      for (const block of entryBlocks(bdb(row.key))) {
        expect(visibleText(showBdb(block.text))).toBe(storedText(block.text));
      }
    }
  });

  test('a number or stem is shown with the text that follows it', () => {
    const blocks = entryBlocks(bdb('BDB04901'));
    const qal = blocks.find((b) => b.part.stem === 'Qal');
    expect(qal?.part.allCited).toBe('1');
    expect(storedText(qal?.text ?? []).startsWith('(poet.) only Imv.')).toBe(
      true
    );
    const niph = blocks.find((b) => b.part.stem === 'Niph.');
    expect(niph?.part.count).toBe('167');
    // No block is left empty when text follows it.
    expect(blocks.every((b) => b.text.length > 0)).toBe(true);
  });

  test('depth is counted from the outermost senses', () => {
    const blocks = entryBlocks(bdb('BDB02244'));
    expect(blocks[0].indent).toBe(0);
    expect(Math.max(...blocks.map((b) => b.indent))).toBeGreaterThan(0);
    expect(blocks.every((b) => b.indent <= 3)).toBe(true);
  });

  test('the headword line: brackets, count, double dagger, other spellings', () => {
    expect(headwordLine(bdb('BDB04901'))).toMatchObject({
      peculiar: false,
      brackets: 'first',
      count: '171',
    });
    expect(headwordLine(bdb('BDBA0011'))).toMatchObject({
      peculiar: true,
      brackets: 'first',
    });
    const six = headwordLine(bdb('BDB09984'));
    expect(six?.variants).toHaveLength(1);
    expect(six?.variants[0].count).toBe('216');
    // The headword line is drawn with the headword, not as a part.
    expect(
      entryBlocks(bdb('BDB09984')).some((b) => b.part.style === 'hwline')
    ).toBe(false);
  });

  test('a misprinted reference is shown as printed and leads to the verse meant', () => {
    const block = entryBlocks(bdb('BDB02244')).find((b) =>
      storedText(b.text).includes('Is 55:2')
    );
    const html = showBdb(block?.text ?? []);
    expect(html).toMatch(
      /lexicon-bref-meant" title="meant ISA 58:2">(?:<!--[^>]*-->)*Is 55:2</
    );
    expect(html).toMatch(/class="lexicon-bref[^"]*" title="go [A-Z0-9]+ \d+/);
  });

  test('line breaks and large type are drawn', () => {
    const items = entryBlocks(bdb('BDB00262')).flatMap((b) => b.text);
    const html = showBdb(items);
    expect(html).toContain('<br/>');
    expect(html).toContain('class="lexicon-big');
  });
});
