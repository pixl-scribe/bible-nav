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
    expect(parseSearch('faith hope')).toMatchObject({ searchType: 'text' });
  });

  test('a book name on its own is the book, but in quotes it is the word', () => {
    expect(parseSearch('job')).toMatchObject({ searchType: 'point' });
    expect(parseSearch('"job"')).toEqual({ searchType: 'text', text: '"job"' });
    expect(parseSearch('\u201Cmark\u201D')).toMatchObject({
      searchType: 'text',
    });
  });

  test('a reference must be the whole search', () => {
    expect(parseSearch('job patience')).toMatchObject({ searchType: 'text' });
    expect(parseSearch('rom 8:28 love')).toMatchObject({ searchType: 'text' });
    expect(parseSearch('Job 42')).toEqual({
      searchType: 'point',
      sid: 'JOB 42:1',
    });
    expect(parseSearch('gen 1:1;')).toMatchObject({ searchType: 'point' });
    expect(parseSearch('John 3:16, Romans 8')).toMatchObject({
      searchType: 'ref',
    });
  });
});
