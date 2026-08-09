export type AutoCompleteOptionType = 'ref' | 'search';

export interface AutoCompleteOption {
  selected: boolean;
  value: string;
  type: AutoCompleteOptionType;
  count: number | undefined;
}
