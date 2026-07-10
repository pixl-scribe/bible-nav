<script lang="ts">
  import { ChevronLeft, ChevronDown } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import { themeService } from '$lib/services/theme-service.svelte';
  import I18nService from '$lib/services/i18n-service.svelte';

  let { onClose }: { onClose: () => void } = $props();

  // eslint-disable-next-line no-undef
  async function setTheme(event: Event) {
    // eslint-disable-next-line no-undef
    const select = event.target as HTMLSelectElement;
    const theme = select.value;
    await themeService.setTheme(theme);
  }

  // eslint-disable-next-line no-undef
  async function setLocale(event: Event) {
    // eslint-disable-next-line no-undef
    const select = event.target as HTMLSelectElement;
    const locale = select.value;
    await I18nService.setLocale(locale);
  }
</script>

{#snippet themeOption(
  label: string,
  value: string,
  // eslint-disable-next-line no-undef
  onChange: (event: Event) => Promise<void>
)}
  <li>
    <input
      type="radio"
      name="theme-dropdown"
      class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
      aria-label={label}
      {value}
      onchange={onChange}
    />
  </li>
{/snippet}

<div class="flex p-1 items-center border-b border-base-content/10">
  <button class="btn btn-soft btn-primary btn-square" onclick={onClose}>
    <ChevronLeft />
  </button>
  <div class="text-base ml-2 font-bold">{$_('settings.heading')}</div>
</div>
<div class="flex flex-col p-1 space-y-1">
  <div class="dropdown">
    <div tabindex="0" role="button" class="btn btn-soft btn-primary w-full">
      {$_('settings.theme')}
      <ChevronDown size={16} />
    </div>
    <ul
      tabindex="-1"
      class="dropdown-content m-0 bg-base-300 rounded-box z-1 w-full p-0 shadow-2xl"
    >
      {@render themeOption('Default', 'default', setTheme)}
      {@render themeOption('Light', 'light', setTheme)}
      {@render themeOption('Dark', 'dark', setTheme)}
      {@render themeOption('Autumn', 'autumn', setTheme)}
      {@render themeOption('Cupcake', 'cupcake', setTheme)}
      {@render themeOption('Dim', 'dim', setTheme)}
      {@render themeOption('Emerald', 'emerald', setTheme)}
      {@render themeOption('Night', 'night', setTheme)}
      {@render themeOption('Nord', 'nord', setTheme)}
      {@render themeOption('Retro', 're etro', setTheme)}
    </ul>
  </div>

  <!-- change popover-1 and --anchor-1 names. Use unique names for each dropdown -->
  <button
    class="btn btn-soft btn-primary w-full"
    popovertarget="lang-popover"
    style="anchor-name:--lang-anchor"
  >
    {$_('settings.language')}
    <ChevronDown size={16} />
  </button>
  <ul
    class="dropdown menu w-52 rounded-box bg-base-100 shadow-sm"
    popover
    id="lang-popover"
    style="position-anchor:--lang-anchor"
  >
    <li><a>Item 1</a></li>
    <li><a>Item 2</a></li>
  </ul>

  <div class="dropdown mb-2">
    <div tabindex="0" role="button" class="btn m-1">
      {$_('settings.language')}
      <ChevronDown size={16} />
    </div>
    <ul
      tabindex="-1"
      class="dropdown-content bg-base-300 rounded-box z-1 w-52 p-2 shadow-2xl"
    >
      <li>
        <input
          type="radio"
          name="theme-dropdown"
          class="w-full btn btn-sm btn-block btn-ghost justify-start"
          aria-label="English"
          value="en-US"
          onchange={setLocale}
        />
      </li>
      <li>
        <input
          type="radio"
          name="theme-dropdown"
          class="w-full btn btn-sm btn-block btn-ghost justify-start"
          aria-label="Pseudo Locale for Testing"
          value="en-XA"
          onchange={setLocale}
        />
      </li>
    </ul>
  </div>
</div>
