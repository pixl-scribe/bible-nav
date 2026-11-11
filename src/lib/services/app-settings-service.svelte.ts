import { Store } from '@tauri-apps/plugin-store';
import debounce from '$lib/services/debounce';
import type { GroupingMode } from '$lib/services/bible-text-service.svelte.js';

/** How wide the reading column may grow. */
export type TextWidth = 'narrow' | 'medium' | 'wide' | 'full';

/**
 * The widest the reading column grows, in characters of the text's own font
 * (`ch`), so it holds about that many characters per line. Full has no limit.
 * The column is centered, and lines keep a ragged end (no justification).
 */
export const textMaxWidths: Record<TextWidth, string> = {
  narrow: '55ch',
  medium: '70ch',
  wide: '90ch',
  full: 'none',
};

export interface AppSettings {
  theme: string;
  locale: string;
  lastSearch: string;
  lastVerseSid: string;
  groupingMode: GroupingMode;
  /** Missing from settings saved before it existed; medium is the default. */
  textWidth?: TextWidth;
}

const defaultSettings: AppSettings = {
  theme: 'default',
  locale: 'en-US',
  lastSearch: '',
  lastVerseSid: 'GEN 1:1', // sid = verse start identifier. See https://ubsicap.github.io/usx/elements.html#verse
  groupingMode: 'paragraph',
  textWidth: 'medium',
};

class AppSettingsService {
  private _settings = $state<AppSettings | undefined>(undefined);
  private _isLoading = $state(false);
  private _store: Store | undefined = undefined;

  public saveLastSearchDebounced = debounce((verseSid: string) => {
    this.saveLastSearch(verseSid).then(() => {});
  }, 200);

  public saveLastVerseSidDebounced = debounce((verseSid: string) => {
    this.saveLastVerseSid(verseSid).then(() => {});
  }, 2000);

  public get isLoading(): boolean {
    return this._isLoading.valueOf();
  }

  public get groupingMode(): GroupingMode {
    return this._settings?.groupingMode ?? 'paragraph';
  }

  public get textWidth(): TextWidth {
    return this._settings?.textWidth ?? 'medium';
  }

  public async getSettings(): Promise<AppSettings> {
    if (this._settings === undefined) {
      this._settings = await this.fetchSettings();
    }
    return this._settings;
  }

  public async saveSettings(settings: AppSettings): Promise<void> {
    this._isLoading = true;
    await this._store?.set('settings', settings);
    this._isLoading = false;
    this._settings = settings;
  }

  private async saveLastSearch(search: string): Promise<void> {
    const settings = await this.getSettings();
    settings.lastSearch = search;
    return this.saveSettings(settings);
  }

  private async saveLastVerseSid(verseSid: string): Promise<void> {
    const settings = await this.getSettings();
    settings.lastVerseSid = verseSid;
    return this.saveSettings(settings);
  }

  private async fetchSettings(): Promise<AppSettings> {
    this._isLoading = true;
    this._store = await Store.load('settings.json');
    let settings = await this._store.get<AppSettings>('settings');
    if (!settings) {
      await this._store.set('settings', defaultSettings);
      settings = defaultSettings;
    }
    this._isLoading = false;
    return settings;
  }
}

export const appSettingsService = new AppSettingsService();
