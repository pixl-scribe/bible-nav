import { Store } from '@tauri-apps/plugin-store';

export interface AppSettings {
  theme: string;
  locale: string;
}

const defaultSettings: AppSettings = {
  theme: 'default',
  locale: 'en-US',
};

class AppSettingsService {
  private _settings = $state<AppSettings | undefined>(undefined);
  private _isLoading = $state(false);
  private _store: Store | undefined = undefined;

  public get isLoading(): boolean {
    return this._isLoading.valueOf();
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
