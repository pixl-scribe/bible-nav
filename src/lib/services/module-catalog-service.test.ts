import { describe, expect, test, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

const { compareVersions, relateTo } =
  await import('./module-catalog-service.svelte');

const offer = {
  id: 'eng_lsv',
  version: '1.1.0',
  name: 'Literal Standard Version',
  abbreviation: 'LSV',
  description: '',
  language: 'English',
  license: '',
  size: 1,
  needsNewerApp: false,
  installedVersion: null,
  relation: 'new' as const,
  textChanges: false,
  textVerified: false,
};

describe('module catalog', () => {
  test('versions compare by number', () => {
    expect(compareVersions('1.10.0', '1.9.2')).toBe(1);
    expect(compareVersions('1.0.0', '1.0.0')).toBe(0);
    expect(compareVersions('0.9.0', '1.0.0')).toBe(-1);
  });

  test('an offer relates to the installed version as Rust says', () => {
    expect(relateTo(offer, '1.1.0').relation).toBe('installed');
    expect(relateTo(offer, '1.0.0')).toMatchObject({
      relation: 'update',
      textChanges: false,
    });
    expect(relateTo(offer, '0.9.0')).toMatchObject({
      relation: 'update',
      textChanges: true,
    });
    expect(relateTo(offer, '2.0.0').relation).toBe('older');
  });
});
