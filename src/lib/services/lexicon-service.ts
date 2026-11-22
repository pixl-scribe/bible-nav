import { invoke } from '@tauri-apps/api/core';
import type { ModuleSummary } from './module-folder-service.svelte';

/**
 * Lexicons (bible-nav-modules/docs/lexicon-format.md): reading their
 * entries through the Rust commands, and choosing which lexicon a word
 * opens.
 */

/** An item of an entry's body: text, or an element with its attributes. */
export type LexiconItem = string | LexiconElement;

export interface LexiconElement {
  style: string;
  children: LexiconItem[];
  /** Attributes, such as a link's `strong` or a word's `lang`. */
  [attribute: string]: unknown;
}

export interface LexiconEntry {
  key: string;
  headword: string;
  /** hbo, arc, or grc. */
  headwordLang: string;
  homograph: string;
  translit: string;
  pronunciation: string;
  allCited: boolean;
  body: LexiconItem[];
}

interface LexiconEntryRow extends Omit<LexiconEntry, 'body'> {
  body: string;
}

function toEntry(row: LexiconEntryRow): LexiconEntry {
  return { ...row, body: JSON.parse(row.body) as LexiconItem[] };
}

/**
 * What a lexicon tab shows: the entries a Strong's number leads to, the
 * entries with a headword (a lemma: Liddell and Scott's links to every
 * homograph), or one entry by key.
 */
export interface LexiconPlace {
  kind: 'strong' | 'lemma' | 'key';
  value: string;
  /** The Bible's word that was clicked, such as "beginning", if any. */
  word?: string;
  /** The first entry's headword, once loaded, for the tab and Back. */
  headword?: string;
  headwordLang?: string;
}

export async function lookUp(
  moduleId: string,
  place: LexiconPlace
): Promise<LexiconEntry[]> {
  if (place.kind === 'key') {
    const row = await invoke<LexiconEntryRow | null>('lexicon_entry', {
      moduleId,
      key: place.value,
    });
    return row ? [toEntry(row)] : [];
  }
  const rows = await invoke<LexiconEntryRow[]>('lexicon_lookup', {
    moduleId,
    kind: place.kind,
    value: place.value,
  });
  return rows.map(toEntry);
}

/**
 * A Strong's number in normal form: H or G, the number without leading
 * zeros, and a letter suffix in lower case ("H0430" is "H430"). The same
 * rule as the generator's and the Rust side's.
 */
export function normalizeStrong(raw: string): string | undefined {
  const match = /^([HGhg])0*([1-9]\d*)([A-Za-z]?)$/.exec(raw.trim());
  if (!match) return undefined;
  return `${match[1].toUpperCase()}${match[2]}${match[3].toLowerCase()}`;
}

/**
 * Whether an entry is laid out in numbered senses (Liddell and Scott)
 * rather than Strong's sections: its own text then says "from ...", so it
 * gets no section headings.
 */
export function hasSenses(entry: LexiconEntry): boolean {
  return entry.body.some(
    (item) => typeof item !== 'string' && item.style === 'sense'
  );
}

/** BDB's headword line (`hwline`): its marks and other spellings. */
export interface HeadwordLine {
  /** ‡: a word found only in Biblical Aramaic. */
  peculiar: boolean;
  /** Square brackets: around the headword, or the whole headword line. */
  brackets?: 'first' | 'all';
  /** The occurrence count BDB prints under the headword. */
  count?: string;
  /** Other spellings, each a `word` element with its own `count`. */
  variants: LexiconElement[];
}

export function headwordLine(entry: LexiconEntry): HeadwordLine | undefined {
  const line = entry.body.find(
    (item): item is LexiconElement =>
      typeof item !== 'string' && item.style === 'hwline'
  );
  if (!line) return undefined;
  const brackets = line.brackets as string | undefined;
  return {
    peculiar: line.peculiar === '1',
    brackets: brackets === 'first' || brackets === 'all' ? brackets : undefined,
    count: line.count as string | undefined,
    variants: line.children.filter(
      (item): item is LexiconElement => typeof item !== 'string'
    ),
  };
}

/**
 * A part of an entry as shown: its own labels (a sense's number, stem,
 * dagger, the words before its number, its count; a note's number) and its
 * text, indented by depth.
 */
export interface EntryBlock {
  part: LexiconElement;
  /** The text shown after the labels. */
  text: LexiconItem[];
  /** 0 for the outermost senses, up to 3. */
  indent: number;
}

/** A sense or note's labels, which its text follows. */
const LABELS = ['n', 'stem', 'allCited', 'pre', 'count'];

function hasLabel(part: LexiconElement): boolean {
  return LABELS.some((name) => part[name] !== undefined);
}

/**
 * The parts of an entry as shown, the headword line left out (it is drawn
 * with the headword). BDB writes a number or a stem as a sense of its own
 * with no text ("1.", "Qal"), followed by its text one level deeper; the
 * two are shown together, as BDB prints them ("1. lit. night", "Qal Pf.
 * ..."). Depth is measured from the entry's outermost senses, so Liddell
 * and Scott (whose senses start at level 2) and BDB (level 1) indent alike.
 */
export function entryBlocks(entry: LexiconEntry): EntryBlock[] {
  const parts = entry.body.filter(
    (item): item is LexiconElement =>
      typeof item !== 'string' && item.style !== 'hwline'
  );
  const levels = parts
    .map((part) => Number(part.level))
    .filter((level) => Number.isFinite(level) && level > 0);
  const base = levels.length > 0 ? Math.min(...levels) : 0;
  const blocks: EntryBlock[] = [];
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    const next = parts[i + 1];
    let text = part.children;
    if (
      (part.style === 'sense' || part.style === 'note') &&
      part.children.length === 0 &&
      hasLabel(part) &&
      next?.style === 'sense' &&
      !hasLabel(next) &&
      next.children.length > 0
    ) {
      text = next.children;
      i++;
    }
    const level = Number(part.level);
    const indent = Number.isFinite(level) && level > 0 ? level - base : 0;
    blocks.push({ part, text, indent: Math.max(0, Math.min(3, indent)) });
  }
  return blocks;
}

/** A key that is a Strong's number (shown); other keys are internal ids. */
export function isStrongKey(key: string): boolean {
  return /^[HG][1-9]\d*[a-z]?$/.test(key);
}

/** Languages written right to left. */
export function isRtl(lang: string | undefined): boolean {
  return lang === 'hbo' || lang === 'arc' || lang === 'he' || lang === 'syc';
}

/**
 * The lexicon a Strong's number opens by default: Strong's own dictionary
 * for its language (decided 2026-11-17), or else any installed lexicon
 * that can be found by such numbers.
 */
const DEFAULT_LEXICONS: Record<string, string> = {
  H: 'eng_strongs_heb',
  G: 'eng_strongs_grc',
};

export function lexiconsFor(
  strong: string,
  modules: ModuleSummary[]
): ModuleSummary[] {
  const prefix = strong[0];
  return modules.filter(
    (module) =>
      module.moduleType === 'lexicon' &&
      module.lookups.includes(`strong:${prefix}`)
  );
}

export function defaultLexicon(
  strong: string,
  modules: ModuleSummary[]
): ModuleSummary | undefined {
  const candidates = lexiconsFor(strong, modules);
  return (
    candidates.find((module) => module.id === DEFAULT_LEXICONS[strong[0]]) ??
    candidates[0]
  );
}
