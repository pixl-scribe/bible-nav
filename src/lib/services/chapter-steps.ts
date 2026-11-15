/**
 * One book's chapters, from the `book_outline` command: `verses[0]` is how
 * many verses chapter 1 has, and so on. Matches `BookOutline` in queries.rs.
 */
export interface BookOutline {
  code: string;
  verses: number[];
}

/**
 * The chapter `step` chapters before (-1) or after (1) `chapterSid`, such as
 * 'GEN 4' to 'GEN 5', crossing into the next or previous book: 'GEN 50' to
 * 'EXO 1', 'EXO 1' to 'GEN 50'. Undefined past either end of the module (or
 * for a chapter it doesn't have), so the reader's arrows can be dimmed
 * there: a module's ends aren't always Genesis 1 and Revelation 22.
 */
export function adjacentChapter(
  outline: BookOutline[],
  chapterSid: string,
  step: 1 | -1
): string | undefined {
  const [code, number] = chapterSid.split(' ');
  const chapter = Number(number);
  const index = outline.findIndex((book) => book.code === code);
  const book = outline[index];
  if (!book || !(chapter >= 1 && chapter <= book.verses.length)) {
    return undefined;
  }
  const next = chapter + step;
  if (next >= 1 && next <= book.verses.length) return `${code} ${next}`;
  const neighbour = outline[index + step];
  if (!neighbour || neighbour.verses.length === 0) return undefined;
  return step === 1
    ? `${neighbour.code} 1`
    : `${neighbour.code} ${neighbour.verses.length}`;
}
