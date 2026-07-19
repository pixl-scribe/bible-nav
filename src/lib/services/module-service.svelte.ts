import Database from '@tauri-apps/plugin-sql';
import { appDataDir } from '@tauri-apps/api/path';
import verseCounts from '../assets/bible-verse-counts.yaml';

export default class ModuleService {
  constructor(private moduleId: string) {}

  public async getText() {
    const appData = await appDataDir();
    const dbPath = `sqlite:${appData}/modules/${this.moduleId}.db`;
    let db: Database | undefined;
    try {
      db = await Database.load(dbPath);
    } catch (err) {
      console.error(`Error connecting to ${dbPath}`, err);
      throw err;
    }

    try {
      const books = await db.select('SELECT * FROM books');
      console.log({ verseCounts, keys: Object.keys(verseCounts.OT), books });
    } finally {
      db?.close();
    }
  }
}
