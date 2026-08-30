import { expect, test } from 'vitest';
import { mapBibleRefToSql } from '$lib/services/map-bible-ref-to-sql';

test('mapBibleRefToSql maps a single verse', () => {
  expect(mapBibleRefToSql('2Cor.5.21')).toBe("(v.sid = '2CO 5:21')");
});

test('mapBibleRefToSql maps multiple references', () => {
  expect(
    mapBibleRefToSql('John.1.1-John.1.20,Rom.8.1-Rom.8.10,2Cor.5.21')
  ).toBe(
    "(v.id BETWEEN (SELECT id FROM verses WHERE sid = 'JHN 1:1') AND (SELECT id FROM verses WHERE sid = 'JHN 1:20')) OR (v.id BETWEEN (SELECT id FROM verses WHERE sid = 'ROM 8:1') AND (SELECT id FROM verses WHERE sid = 'ROM 8:10')) OR (v.sid = '2CO 5:21')"
  );
});

test('mapBibleRefToSql maps cross book range', () => {
  expect(mapBibleRefToSql('John.21.1-Acts.3.26')).toBe(
    "(v.id BETWEEN (SELECT id FROM verses WHERE sid = 'JHN 21:1') AND (SELECT id FROM verses WHERE sid = 'ACT 3:26'))"
  );
});
