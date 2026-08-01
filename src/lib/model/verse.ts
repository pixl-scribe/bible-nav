/**
 * See https://ubsicap.github.io/usx/charstyles.html#add
 */
export interface TranslatorsAddition {
  style: 'add';
  txt: string;
}

/**
 * See https://ubsicap.github.io/usx/charstyles.html#w
 */
export interface GlossaryWord {
  style: 'w';
  txt: string;
  strong: string;
}

export type NoteSegmentStyle = 'fr' | 'ft' | 'fk' | 'fq' | 'fqa' | 'fl' | 'fw';

export interface NoteSegment {
  style: NoteSegmentStyle;
  txt: string;
}

export interface Note {
  style: 'f';
  children: NoteSegment[];
}

export interface WordsOfJesus {
  style: 'wj';
  children: (string | GlossaryWord | TranslatorsAddition)[];
}

export type VerseChild =
  string | GlossaryWord | TranslatorsAddition | Note | WordsOfJesus;

export interface Verse {
  paragraph: number;
  sid: string;
  children: VerseChild[];
}

export interface VerseRow {
  sid: string;
  paragraph: number;
  children: string;
}
