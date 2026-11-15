import { describe, expect, test } from 'vitest';
import { parseSearch } from './search-parser';

describe('parseSearch', () => {
  test('a single verse, chapter, or book is a place to go to', () => {
    expect(parseSearch('john 3:16')).toEqual({
      searchType: 'point',
      sid: 'JHN 3:16',
    });
    expect(parseSearch('gen 1')).toEqual({
      searchType: 'point',
      sid: 'GEN 1:1',
    });
    expect(parseSearch('ps 119')).toEqual({
      searchType: 'point',
      sid: 'PSA 119:1',
    });
    expect(parseSearch('romans')).toEqual({
      searchType: 'point',
      sid: 'ROM 1:1',
    });
  });

  test('a range or list is shown verse by verse', () => {
    expect(parseSearch('john 3:16-18')).toEqual({
      searchType: 'ref',
      osis: 'John.3.16-John.3.18',
    });
    expect(parseSearch('rom 5:8; 8:28')).toMatchObject({ searchType: 'ref' });
    expect(parseSearch('gen 1-3')).toMatchObject({ searchType: 'ref' });
  });

  test('anything else is a word search', () => {
    expect(parseSearch('grace')).toEqual({ searchType: 'text', text: 'grace' });
  });
});
