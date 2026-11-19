import { describe, expect, test } from 'vitest';
import { defaultLexicon, normalizeStrong } from './lexicon-service';
import type { ModuleSummary } from './module-folder-service.svelte';

describe("Strong's numbers", () => {
  // The same cases as the generator's strong-number.test.ts and the Rust
  // side's lexicon.rs.
  test('each spelling has one normal form', () => {
    expect(normalizeStrong('H0430')).toBe('H430');
    expect(normalizeStrong('H430')).toBe('H430');
    expect(normalizeStrong('G0026')).toBe('G26');
    expect(normalizeStrong('g26')).toBe('G26');
    expect(normalizeStrong('H1234A')).toBe('H1234a');
    expect(normalizeStrong(' H7225 ')).toBe('H7225');
  });

  test('anything else is not a number', () => {
    for (const raw of ['', 'H', 'H0', 'X12', '7225', 'H12ab', 'H-1']) {
      expect(normalizeStrong(raw)).toBeUndefined();
    }
  });
});

function module(id: string, lookups: string[]): ModuleSummary {
  return {
    id,
    name: id,
    abbreviation: id,
    moduleType: lookups.length ? 'lexicon' : 'bible',
    hasNotes: false,
    lookups,
    version: '1.0.0',
    file: `${id}-1.0.0.db`,
    description: '',
    language: 'English',
    license: '',
    builtIn: false,
    textVerified: false,
    older: [],
  };
}

describe('which lexicon a number opens', () => {
  const kjv = module('eng_kjv', []);
  const bdb = module('eng_bdb', ['strong:H']);
  const sh = module('eng_strongs_heb', ['strong:H']);

  test("Strong's own dictionary first, then any that knows the numbers", () => {
    expect(defaultLexicon('H7225', [kjv, bdb, sh])?.id).toBe('eng_strongs_heb');
    expect(defaultLexicon('H7225', [kjv, bdb])?.id).toBe('eng_bdb');
  });

  test('none for numbers no lexicon knows', () => {
    expect(defaultLexicon('G26', [kjv, bdb, sh])).toBeUndefined();
    expect(defaultLexicon('H7225', [kjv])).toBeUndefined();
  });
});
