<script lang="ts">
  import { ChevronLeft } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import { themeService } from '$lib/services/theme-service.svelte';
  import I18nService from '$lib/services/i18n-service.svelte';
  import {
    type AppSettings,
    appSettingsService,
  } from '$lib/services/app-settings-service.svelte.js';
  import { onMount } from 'svelte';

  let appSettings = $state<AppSettings | undefined>(undefined);

  let { onClose }: { onClose: () => void } = $props();

  // eslint-disable-next-line no-undef
  async function setTheme(event: Event & { currentTarget: HTMLSelectElement }) {
    const theme = event.currentTarget.value;
    await themeService.setTheme(theme);
  }

  async function setLocale(
    // eslint-disable-next-line no-undef
    event: Event & { currentTarget: HTMLSelectElement }
  ) {
    const locale = event.currentTarget.value;
    await I18nService.setLocale(locale);
  }

  onMount(async () => {
    appSettings = await appSettingsService.getSettings();
  });
</script>

{#snippet themeOption(label: string, value: string)}
  <option {value} selected={appSettings?.theme === value}>{label}</option>
{/snippet}

{#snippet languageOption(label: string, value: string)}
  <option {value} selected={appSettings?.locale === value}>{label}</option>
{/snippet}

<div class="flex p-1 items-center border-b border-base-content/10">
  <button class="btn btn-soft btn-primary btn-square" onclick={onClose}>
    <ChevronLeft />
  </button>
  <div class="text-base ml-2 font-bold">{$_('settings.heading')}</div>
</div>

<div class="flex flex-col p-3 pt-1">
  <fieldset class="fieldset">
    <legend class="fieldset-legend">{$_('settings.theme')}</legend>
    <select class="select" onchange={setTheme}>
      {@render themeOption('Default', 'default')}
      {@render themeOption('Light', 'light')}
      {@render themeOption('Dark', 'dark')}
      {@render themeOption('Autumn', 'autumn')}
      {@render themeOption('Cupcake', 'cupcake')}
      {@render themeOption('Dim', 'dim')}
      {@render themeOption('Emerald', 'emerald')}
      {@render themeOption('Night', 'night')}
      {@render themeOption('Nord', 'nord')}
      {@render themeOption('Retro', 're etro')}
    </select>
  </fieldset>

  <fieldset class="fieldset">
    <legend class="fieldset-legend">{$_('settings.language')}</legend>
    <select class="select" onchange={setLocale}>
      {@render languageOption('English', 'en-US')}
      {@render languageOption('Pseudo Locale for Testing', 'en-XA')}
    </select>
  </fieldset>
</div>
