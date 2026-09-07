import { expect, test } from 'vitest';
import { calcLimitAndOffsetWithButter } from '$lib/services/calc-limit-and-offset';

test('calcLimitAndOffset index 0', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 0);
  expect(limit).toBe(41);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset index 0', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 10);
  expect(limit).toBe(51);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset index 0', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 40);
  expect(limit).toBe(81);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset index 0', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 100);
  expect(limit).toBe(81);
  expect(offset).toBe(60);
});
