import type { Inline, Note, VerseChild } from '$lib/model/verse';

/**
 * Parses stored verse or heading content (a JSON array) into its parts.
 */
export function parseChildren<T extends VerseChild>(json: string): T[] {
  const parsed: unknown = JSON.parse(json);
  if (!Array.isArray(parsed)) {
    throw new Error('verse content should be a JSON array');
  }
  return parsed as T[];
}

function isNote(child: VerseChild): child is Note {
  return typeof child === 'object' && child.style === 'f';
}

/**
 * The reading text of inline content, without footnotes. Pieces join
 * directly, with no added spaces, for any language. For a verse this equals
 * the module's stored `raw` text.
 */
export function plainText(children: VerseChild[]): string {
  return children
    .map((child) => {
      if (typeof child === 'string') return child;
      if (isNote(child)) return '';
      if (child.style === 'w') return child.txt;
      return plainText(child.children);
    })
    .join('');
}

/**
 * A footnote as one line of text, such as "4.10 voice: Heb. bloods". Parts
 * are stored trimmed, so a space goes between them.
 */
export function noteText(note: Note): string {
  return note.children
    .map((segment) => plainText(segment.children as Inline[]))
    .filter((text) => text !== '')
    .join(' ');
}

/**
 * The footnote letter for a 0-based position within a chapter: a..z, then
 * aa..az, ba..bz, and so on, like spreadsheet columns.
 */
export function footnoteLetter(index: number): string {
  let letters = '';
  let n = index + 1;
  while (n > 0) {
    const remainder = (n - 1) % 26;
    letters = String.fromCharCode(97 + remainder) + letters;
    n = Math.floor((n - 1) / 26);
  }
  return letters;
}

/**
 * Gives each footnote its letter, in reading order, starting from the
 * number of footnotes earlier in the chapter (`noteStart` from Rust).
 */
export function letterNotes(
  children: VerseChild[],
  noteStart: number
): VerseChild[] {
  let next = noteStart;
  const walk = (items: VerseChild[]): VerseChild[] =>
    items.map((child) => {
      if (typeof child === 'string' || child.style === 'w') return child;
      if (isNote(child)) return { ...child, letter: footnoteLetter(next++) };
      return { ...child, children: walk(child.children) as Inline[] };
    });
  return walk(children);
}
