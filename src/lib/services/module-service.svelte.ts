import Database from '@tauri-apps/plugin-sql';
import { appDataDir } from '@tauri-apps/api/path';

export default class ModuleService {
  public static async search(moduleId: string) {
    const appData = await appDataDir();
    const dbPath = `sqlite:${appData}/modules/${moduleId}.db`;
    let db: Database | undefined;
    try {
      db = await Database.load(dbPath);
    } catch (err) {
      console.error(`Error connecting to ${dbPath}`, err);
      throw err;
    }

    try {
      const books = await db.select('SELECT * FROM books');
      console.log(books);
    } finally {
      db?.close();
    }
  }
}
