import { bcv_parser } from 'bible-passage-reference-parser';
import * as langEn from 'bible-passage-reference-parser/esm/lang/en.js';
import bookCodes from '$lib/assets/osis-to-usx-book-codes.yaml';

/**
 * - 'point': a single verse, a whole chapter, or a whole book: a place to
 *   go to and read on from ("John 3:16", "Gen 1", "Romans").
 * - 'ref': anything else that's a reference, such as a range or a list
 *   ("John 3:16-18", "Rom 5:8; 8:28"), shown verse by verse.
 * - 'text': not a reference: a word search.
 */
export type SearchType = 'point' | 'ref' | 'text';

export interface SearchRequest {
  searchType: SearchType;
  /** For 'ref', the OSIS reference, every range down to verses. */
  osis?: string;
  /** For 'point', the verse to go to, such as 'JHN 3:16' or 'GEN 1:1'. */
  sid?: string;
  /** For 'text', the words. */
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

export function parseSearch(search: string): SearchRequest {
  // Whole chapters and books stay whole in this form ('Gen.1', 'Rom'), so a
  // single place can be told from a range.
  const compact = parser('b').parse(search).osis();
  if (compact) {
    const sid = pointSid(compact);
    if (sid) return { searchType: 'point', sid };
    return { searchType: 'ref', osis: parser('bcv').parse(search).osis() };
  }
  return { searchType: 'text', text: search };
}
