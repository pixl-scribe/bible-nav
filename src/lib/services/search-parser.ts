import { bcv_parser } from 'bible-passage-reference-parser';
import * as langEn from 'bible-passage-reference-parser/esm/lang/en.js';
import bookCodes from '$lib/assets/osis-to-usx-book-codes.yaml';

/**
 * - 'point': a single verse, a whole chapter, or a whole book: a place to
 *   go to and read on from ("John 3:16", "Gen 1", "Romans").
 * - 'ref': anything else that's a reference, such as a range or a list
 *   ("John 3:16-18", "Rom 5:8; 8:28"), shown verse by verse.
 * - 'text': a word search: anything in quotes, or anything that isn't
 *   wholly a reference ("faith hope", "job patience").
 */
export type SearchType = 'point' | 'ref' | 'text';

export interface SearchRequest {
  searchType: SearchType;
  /** For 'ref', the OSIS reference, every range down to verses. */
  osis?: string;
  /** For 'point', the verse to go to, such as 'JHN 3:16' or 'GEN 1:1'. */
  sid?: string;
  /** For 'text', the words, as typed (quotes included). */
  text?: string;
}

function parser(compaction: 'b' | 'bcv') {
  const bcv = new bcv_parser(langEn);
  bcv.set_options({
    versification_system: 'kjv',
    osis_compaction_strategy: compaction,
    // A book name on its own ("Romans") is a reference to the whole book.
    // So a word search for "Job", "Mark", or "Acts" goes to the book;
    // revisit when word search is built.
    book_alone_strategy: 'full',
  });
  return bcv;
}

/**
 * 'Gen' gives 'GEN 1:1', 'Gen.5' gives 'GEN 5:1', 'John.3.16' gives
 * 'JHN 3:16'; undefined for anything that isn't one place.
 */
function pointSid(osis: string): string | undefined {
  if (osis.includes('-') || osis.includes(',')) return undefined;
  const [book, chapter = '1', verse = '1'] = osis.split('.');
  const code = bookCodes[book];
  return code ? `${code} ${chapter}:${verse}` : undefined;
}

/**
 * Whether the references found cover the whole search: nothing but spaces
 * and punctuation is left once they're taken out. "job patience" isn't a
 * reference to Job, because "patience" is left over.
 */
function coversAll(
  search: string,
  found: { indices: [number, number] }[]
): boolean {
  let rest = search;
  for (const { indices } of [...found].reverse()) {
    rest = rest.slice(0, indices[0]) + rest.slice(indices[1]);
  }
  return !/[\p{L}\p{N}]/u.test(rest);
}

export function parseSearch(search: string): SearchRequest {
  // Anything in quotes is a word search, never a reference: "job" is the
  // word, Job the book.
  if (/["\u201C\u201D]/.test(search)) {
    return { searchType: 'text', text: search.trim() };
  }
  // Whole chapters and books stay whole in this form ('Gen.1', 'Rom'), so a
  // single place can be told from a range.
  const parsed = parser('b').parse(search);
  const compact = coversAll(search, parsed.osis_and_indices())
    ? parsed.osis()
    : '';
  if (compact) {
    const sid = pointSid(compact);
    if (sid) return { searchType: 'point', sid };
    return { searchType: 'ref', osis: parser('bcv').parse(search).osis() };
  }
  return { searchType: 'text', text: search };
}
