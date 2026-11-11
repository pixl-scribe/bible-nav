import bookCodes from '$lib/assets/osis-to-usx-book-codes.yaml';

/** A verse or range of verses by sid. Matches `RefRange` in queries.rs. */
export interface RefRange {
  from: string;
  to?: string;
}

/** 'John.3.16' to 'JHN 3:16', or undefined if it isn't a full reference. */
function osisToSid(osis: string): string | undefined {
  const [book, chapter, verse] = osis.split('.');
  const code = bookCodes[book];
  if (!code || !/^\d+$/.test(chapter ?? '') || !/^\d+$/.test(verse ?? '')) {
    return undefined;
  }
  return `${code} ${chapter}:${verse}`;
}

/**
 * Turns the reference parser's OSIS output into verse ranges for the
 * `ref_sids` and `ref_verses` commands. Rust binds them as query
 * parameters, so nothing here becomes SQL.
 *
 * 'John.1.1-John.1.20,Rom.8.1' gives
 * [{ from: 'JHN 1:1', to: 'JHN 1:20' }, { from: 'ROM 8:1' }].
 * Parts it can't read are left out.
 */
export function osisToRanges(osis: string): RefRange[] {
  return osis
    .split(',')
    .map((part): RefRange | undefined => {
      const [fromOsis, toOsis] = part.trim().split('-');
      const from = osisToSid(fromOsis);
      if (!from) return undefined;
      if (toOsis === undefined) return { from };
      const to = osisToSid(toOsis);
      return to ? { from, to } : undefined;
    })
    .filter((range): range is RefRange => range !== undefined);
}
