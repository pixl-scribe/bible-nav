/**
 * Highlighting a word search's matches in verse text. Rust reports the words
 * that matched in each verse, as written there ("covenants" for a search for
 * "covenant", since the search stems words), and the page marks each whole
 * occurrence of them. Splitting never adds or drops a character, so the
 * verse still reads exactly as its text.
 */

export interface TextPart {
  text: string;
  hit: boolean;
}

/**
 * A pattern for whole occurrences of `words`, ignoring case; undefined when
 * there's nothing to highlight. A word is whole when no letter or digit
 * touches it, so "covenant" isn't marked inside "covenanted".
 */
export function matchPattern(words: string[] | undefined): RegExp | undefined {
  const wanted = (words ?? []).filter((word) => word.length > 0);
  if (wanted.length === 0) return undefined;
  const escaped = wanted
    // Longest first, so a longer word wins where one starts another.
    .sort((a, b) => b.length - a.length)
    .map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  return new RegExp(
    `(?<![\\p{L}\\p{N}])(?:${escaped.join('|')})(?![\\p{L}\\p{N}])`,
    'giu'
  );
}

/** `text` in pieces, with the pattern's matches marked as hits. */
export function splitMatches(text: string, pattern: RegExp): TextPart[] {
  const parts: TextPart[] = [];
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    const start = match.index ?? 0;
    if (start > last) parts.push({ text: text.slice(last, start), hit: false });
    parts.push({ text: match[0], hit: true });
    last = start + match[0].length;
  }
  if (last < text.length) parts.push({ text: text.slice(last), hit: false });
  return parts;
}
