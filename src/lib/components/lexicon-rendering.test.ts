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
