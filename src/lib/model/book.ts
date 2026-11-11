/** A book of the module. Matches `Book` in src-tauri/src/modules/queries.rs. */
export interface Book {
  id: number;
  testamentCode: string | null;
  name: string;
  /** USX book code, such as JHN. */
  code: string;
  header: string;
  /** Long title. */
  toc1: string;
  /** Short name, such as John. */
  toc2: string;
  /** Abbreviation, such as Jhn. */
  toc3: string;
  /** Main title. */
  mt1: string;
  /** Secondary title, such as "Commonly Called the Third Book of the Kings". */
  mt2: string | null;
}
