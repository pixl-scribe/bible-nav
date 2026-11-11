import { expect, test } from 'vitest';
import { calcLimitAndOffsetWithButter } from '$lib/services/calc-limit-and-offset';

test('calcLimitAndOffset at the start', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 0);
  expect(limit).toBe(41);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset with fewer verses before than the buffer', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 10);
  expect(limit).toBe(51);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset with exactly a buffer of verses before', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 40);
  expect(limit).toBe(81);
  expect(offset).toBe(0);
});

test('calcLimitAndOffset past the first buffer', () => {
  const { limit, offset } = calcLimitAndOffsetWithButter(40, 100);
  expect(limit).toBe(81);
  expect(offset).toBe(60);
});
