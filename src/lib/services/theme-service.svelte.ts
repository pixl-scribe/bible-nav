import { appSettingsService } from './app-settings-service.svelte';

class ThemeService {
  public async loadTheme(): Promise<void> {
    const settings = await appSettingsService.getSettings();
    document.documentElement.setAttribute('data-theme', settings.theme);
  }

  public async setTheme(theme: string): Promise<void> {
    const settings = await appSettingsService.getSettings();
    settings.theme = theme;
    await appSettingsService.saveSettings(settings);
    document.documentElement.setAttribute('data-theme', settings.theme);
  }
}

export const themeService = new ThemeService();
