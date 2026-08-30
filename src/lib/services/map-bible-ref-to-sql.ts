import bookCodes from '$lib/assets/osis-to-usx-book-codes.yaml';

/**
 * 'John 1:1-20; Romans 8:1-10; 2 Cor. 5:21' is parsed to
 * 'John.1.1-John.1.20,Rom.8.1-Rom.8.10,2Cor.5.21'
 * 'John 21 - Acts 3' => 'John.21.1-Acts.3.26'
 * @param ref
 */
export function mapBibleRefToSql(ref: string): string {
  const refs = ref.split(',');
  let sql = '';
  let delimiter = '';
  for (const ref of refs) {
    const [from, to] = ref.split('-');
    const [fromBook, fromChapter, fromVerse] = from.split('.');
    const fromUsxBookCode = bookCodes[fromBook];
    const fromSid = `${fromUsxBookCode} ${fromChapter}:${fromVerse}`;
    if (to) {
      const [toBook, toChapter, toVerse] = to.split('.');
      const toUsxBookCode = bookCodes[toBook];
      const toSid = `${toUsxBookCode} ${toChapter}:${toVerse}`;
      sql += `${delimiter}(v.id BETWEEN (SELECT id FROM verses WHERE sid = '${fromSid}') AND (SELECT id FROM verses WHERE sid = '${toSid}'))`;
    } else {
      sql += `${delimiter}(v.sid = '${fromSid}')`;
    }
    delimiter = ' OR ';
  }
  return sql;
}
