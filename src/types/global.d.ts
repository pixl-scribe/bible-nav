declare module '*bible-verse-counts.yaml' {
  import { BibleVerseCounts } from '$lib/model/bible-verse-counts';
  const data: BibleVerseCounts;
  export default data;
}

declare module '*osis-to-usx-book-codes.yaml' {
  import { BookCodes } from '$lib/model/book-codes';
  const data: BookCodes;
  export default data;
}
