/** A book (a reference) or a word in the module (a word search). */
export type AutoCompleteOptionType = 'ref' | 'word';

export interface AutoCompleteOption {
  selected: boolean;
  /** The book's name, or the word. */
  value: string;
  type: AutoCompleteOptionType;
  /** A book's chapters, or how many verses have the word. */
  count: number | undefined;
}
