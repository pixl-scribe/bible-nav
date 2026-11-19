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

/** What a lexicon tab shows: the entries a Strong's number leads to, or one entry by key. */
export interface LexiconPlace {
  kind: 'strong' | 'key';
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
    kind: 'strong',
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
