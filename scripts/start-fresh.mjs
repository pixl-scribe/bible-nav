/* global process, console, URL */
/**
 * Forgets the Bible modules folder setting, so the next run behaves like a
 * first run: it picks the default folder again and shows the first-run notice.
 *
 * Only deletes modules.json from the app's data folder. The modules folder
 * itself, the modules in it, and other settings (theme, language) are left
 * alone. Close the app first, or it may write the setting back.
 */
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const config = JSON.parse(
  readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')
);

// Where Tauri keeps app data on each system.
const dataRoot =
  process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? join(homedir(), 'Library', 'Application Support')
      : process.env.XDG_DATA_HOME || join(homedir(), '.local', 'share');

const settings = join(dataRoot, config.identifier, 'modules.json');

if (existsSync(settings)) {
  rmSync(settings);
  console.log(`Removed ${settings}`);
  console.log(
    'The next run will choose the default modules folder and show the first-run notice.'
  );
} else {
  console.log(`Nothing to remove: ${settings} doesn't exist.`);
}
