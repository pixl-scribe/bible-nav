/**
 * Where the verses of a search result are, for the reference picker and the
 * chapter arrows: how many matching verses each book and chapter has, the
 * first in each, and the chapters with hits in order. Counts are verses,
 * not occurrences of the words, so each count is how many entries that
 * place adds to the results list.
 */
export interface HitChapter {
  /** Such as 'GEN 9'. */
  chapter: string;
  /** Its first matching verse, such as 'GEN 9:9'. */
  first: string;
  /** How many of its verses match. */
  count: number;
}

export interface HitIndex {
  /** Matching verses per book code. */
  byBook: Map<string, number>;
  /** Matching verses per chapter, such as 'GEN 9'. */
  byChapter: Map<string, number>;
  /** The first matching verse in each book. */
  firstInBook: Map<string, string>;
  /** Every matching verse. */
  verses: Set<string>;
  /** Chapters with hits, in the order the results list them. */
  chapters: HitChapter[];
}

const chapterOf = (sid: string) => sid.split(':')[0];
const bookOf = (sid: string) => sid.split(' ')[0];

export function indexHits(sids: string[]): HitIndex {
  const index: HitIndex = {
    byBook: new Map(),
    byChapter: new Map(),
    firstInBook: new Map(),
    verses: new Set(sids),
    chapters: [],
  };
  const chapters = new Map<string, HitChapter>();
  for (const sid of sids) {
    const book = bookOf(sid);
    const chapter = chapterOf(sid);
    index.byBook.set(book, (index.byBook.get(book) ?? 0) + 1);
    index.byChapter.set(chapter, (index.byChapter.get(chapter) ?? 0) + 1);
    if (!index.firstInBook.has(book)) index.firstInBook.set(book, sid);
    const known = chapters.get(chapter);
    if (known) {
      known.count++;
    } else {
      const entry = { chapter, first: sid, count: 1 };
      chapters.set(chapter, entry);
      index.chapters.push(entry);
    }
  }
  return index;
}

/**
 * The chapter with hits before (-1) or after (1) the one holding `sid`, for
 * the ‹ › arrows in a word search; undefined at either end.
 */
export function hitChapterStep(
  index: HitIndex,
  sid: string,
  step: 1 | -1
): HitChapter | undefined {
  const at = index.chapters.findIndex((c) => c.chapter === chapterOf(sid));
  if (at < 0) return undefined;
  return index.chapters[at + step];
}
