import { describe, expect, test } from 'vitest';
import { formatAgo, formatPercent, formatSize } from './format';

describe('formatting', () => {
  test('sizes in megabytes or kilobytes', () => {
    expect(formatSize(9305192, 'en-US')).toBe('9.3 MB');
    expect(formatSize(450_000, 'en-US')).toBe('450 kB');
    expect(formatSize(9305192, 'de-DE')).toBe('9,3 MB');
  });

  test('how long ago, in words', () => {
    const now = 1_800_000_000;
    expect(formatAgo(now, 'en-US', now)).toBe('now');
    expect(formatAgo(now - 3 * 86400, 'en-US', now)).toBe('3 days ago');
    expect(formatAgo(now - 86400, 'en-US', now)).toBe('yesterday');
  });

  test('how much of a download is done', () => {
    expect(formatPercent(46, 100, 'en-US')).toBe('46%');
    expect(formatPercent(0, 0, 'en-US')).toBe('0%');
    expect(formatPercent(46, 100, 'de-DE')).toBe('46\u00a0%');
  });
});
