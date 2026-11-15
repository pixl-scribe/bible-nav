import { describe, expect, test, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

const { newerTextFor, newestOffers, shortLicense, updateFor } =
  await import('./module-view');

function offer(id: string, version: string, extra = {}) {
  return {
    id,
    version,
    name: id,
    abbreviation: id,
    description: '',
    language: '',
    license: '',
    size: 1,
    needsNewerApp: false,
    installedVersion: '1.0.0',
    relation: 'update' as const,
    textChanges: false,
    textVerified: false,
    ...extra,
  };
}

describe('module view', () => {
  test('each module once, by its newest offer', () => {
    const offers = [
      offer('b', '1.2.0'),
      offer('b', '1.10.0'),
      offer('a', '1.0.0'),
    ];
    expect(newestOffers(offers).map((o) => `${o.id} ${o.version}`)).toEqual([
      'a 1.0.0',
      'b 1.10.0',
    ]);
  });

  test('updates this app can use, and texts that need a newer app', () => {
    const offers = [
      offer('eng_kjv', '1.2.0'),
      offer('eng_kjv', '2.0.0', { needsNewerApp: true }),
      offer('eng_lsv', '1.0.0', { relation: 'installed' }),
    ];
    expect(updateFor('eng_kjv', offers)?.version).toBe('1.2.0');
    expect(newerTextFor('eng_kjv', offers)?.version).toBe('2.0.0');
    expect(updateFor('eng_lsv', offers)).toBeUndefined();
  });

  test('a long license is shown by its first sentence', () => {
    expect(shortLicense('Public Domain')).toBe('Public Domain');
    expect(
      shortLicense(
        'Copyright © 2020 Covenant Press. Licensed under the Creative Commons Attribution-ShareAlike 4.0 International license.'
      )
    ).toBe('Copyright © 2020 Covenant Press');
  });
});
