import { register, locale, init } from 'svelte-i18n';
import { appSettingsService } from './app-settings-service.svelte';

const fallbackLocale = 'en-US';

export default class I18nService {
  public static async initialize() {
    register('en-US', () => import('../../locales/en-US.json'));
    register('en-XA', () => import('../../locales/en-XA.json'));

    init({
      fallbackLocale,
      initialLocale: fallbackLocale,
    });

    const settings = await appSettingsService.getSettings();
    await I18nService.setLocale(settings.locale);
  }

  public static async setLocale(newLocale: string) {
    const settings = await appSettingsService.getSettings();
    settings.locale = newLocale;
    await appSettingsService.saveSettings(settings);
    locale?.set(newLocale);
  }
}
