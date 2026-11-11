import { expect, test } from 'vitest';
import { osisToRanges } from '$lib/services/osis-to-ranges';

test('maps a single verse', () => {
  expect(osisToRanges('2Cor.5.21')).toEqual([{ from: '2CO 5:21' }]);
});

test('maps several references', () => {
  expect(osisToRanges('John.1.1-John.1.20,Rom.8.1-Rom.8.10,2Cor.5.21')).toEqual(
    [
      { from: 'JHN 1:1', to: 'JHN 1:20' },
      { from: 'ROM 8:1', to: 'ROM 8:10' },
      { from: '2CO 5:21' },
    ]
  );
});

test('maps a range across books', () => {
  expect(osisToRanges('John.21.1-Acts.3.26')).toEqual([
    { from: 'JHN 21:1', to: 'ACT 3:26' },
  ]);
});

test('leaves out parts it cannot read', () => {
  expect(osisToRanges('Nope.1.1,John.3')).toEqual([]);
  expect(osisToRanges('John.3.16-Nope.1.1,Rom.8.1')).toEqual([
    { from: 'ROM 8:1' },
  ]);
  expect(osisToRanges('')).toEqual([]);
});

test('passes text through as data, never as SQL', () => {
  expect(osisToRanges("John.3.16') OR 1=1 --")).toEqual([]);
});
