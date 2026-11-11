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
