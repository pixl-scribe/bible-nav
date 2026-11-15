/**
 * Numbers and times in the reader's own language and conventions, through
 * the browser's Intl, so no format is written in English here.
 */

/** A size such as "9.3 MB", in `locale`'s number style. */
export function formatSize(bytes: number, locale: string): string {
  const megabytes = bytes / 1_000_000;
  return new Intl.NumberFormat(locale, {
    style: 'unit',
    unit: megabytes >= 1 ? 'megabyte' : 'kilobyte',
    maximumFractionDigits: 1,
  }).format(megabytes >= 1 ? megabytes : bytes / 1000);
}

/**
 * How long ago `seconds` (since 1970) was, such as "3 days ago" or
 * "now", in `locale`.
 */
export function formatAgo(
  seconds: number,
  locale: string,
  now: number = Date.now() / 1000
): string {
  const format = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  const elapsed = Math.max(0, now - seconds);
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 365 * 86400],
    ['month', 30 * 86400],
    ['week', 7 * 86400],
    ['day', 86400],
    ['hour', 3600],
    ['minute', 60],
  ];
  for (const [unit, size] of units) {
    if (elapsed >= size)
      return format.format(-Math.floor(elapsed / size), unit);
  }
  return format.format(0, 'second');
}

/** How much of a total is done, such as "46%", in `locale`'s style. */
export function formatPercent(
  done: number,
  total: number,
  locale: string
): string {
  const share = total > 0 ? Math.min(1, done / total) : 0;
  return new Intl.NumberFormat(locale, {
    style: 'percent',
    maximumFractionDigits: 0,
  }).format(share);
}
