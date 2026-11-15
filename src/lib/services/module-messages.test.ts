import { describe, expect, test } from 'vitest';
import {
  errorMessage,
  isCancelled,
  problemMessage,
  type Translate,
} from './module-messages';

const t: Translate = (key, options) =>
  `${key}${options?.values ? JSON.stringify(options.values) : ''}`;

describe('module messages', () => {
  test('a problem is its key with values, and its detail apart', () => {
    expect(
      problemMessage(
        {
          key: 'problem.notAModule',
          values: { file: 'x.db' },
          detail: 'not signed',
        },
        t
      )
    ).toEqual({
      text: 'problem.notAModule{"file":"x.db"}',
      detail: 'not signed',
    });
  });

  test('a command error is its code; anything else is a general error', () => {
    expect(errorMessage({ code: 'network', detail: 'timed out' }, t)).toEqual({
      text: 'error.network',
      detail: 'timed out',
    });
    expect(errorMessage({ code: 'cancelled', detail: '' }, t)).toEqual({
      text: 'error.cancelled',
      detail: undefined,
    });
    expect(errorMessage('boom', t)).toEqual({
      text: 'error.task',
      detail: 'boom',
    });
    expect(isCancelled({ code: 'cancelled', detail: '' })).toBe(true);
    expect(isCancelled('cancelled')).toBe(false);
  });
});
