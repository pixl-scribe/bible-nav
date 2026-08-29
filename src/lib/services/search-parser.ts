import { bcv_parser } from 'bible-passage-reference-parser';
import * as langEn from 'bible-passage-reference-parser/esm/lang/en.js';

export type SearchType = 'ref' | 'ref-point' | 'text';
export interface SearchRequest {
  searchType: SearchType;
  searchString?: string;
}

export function parseSearch(search: string): SearchRequest {
  const bcv = new bcv_parser(langEn);
  bcv.set_options({
    versification_system: 'kjv',
  });
  const result = bcv.parse(search).osis();
  if (result) {
    return {
      searchType: 'ref',
      searchString: result,
    };
  }

  return {
    searchType: 'text',
    searchString: search,
  };
}
