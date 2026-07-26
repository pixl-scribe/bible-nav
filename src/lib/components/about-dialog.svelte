<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { open } from '@tauri-apps/plugin-shell';

  let { dialog = $bindable() }: { dialog: HTMLDialogElement | undefined } =
    $props();

  let appVersion = $state('');

  function getCopyrightYear(): string {
    const copyStart = 2026;
    const currentYear = new Date().getFullYear();
    if (currentYear > copyStart) {
      return `${copyStart}-${currentYear}`;
    }
    return `${copyStart}`;
  }

  const licenseUrl =
    'https://github.com/pixl-scribe/bible-nav/blob/master/LICENSE';

  async function openLicense(event: MouseEvent) {
    event.preventDefault();
    await open(licenseUrl);
  }

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (err) {
      console.error('Failed to get version', err);
    }
  });
</script>

<dialog bind:this={dialog} class="modal">
  <div class="modal-box">
    <div class="flex">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="50"
        height="60"
        viewBox="0 0 354 483"
        class="text-primary shrink-0"
      >
        <path
          fill="currentColor"
          d="M0,424.402l0,-366.847c-0,-15.264 6.064,-29.904 16.857,-40.697c10.794,-10.794 25.433,-16.857 40.697,-16.857l272.742,-0c6.043,0 11.838,2.401 16.112,6.674c4.273,4.273 6.674,10.069 6.674,16.112l-0,359.988c0,3.347 -1.33,6.557 -3.696,8.923c-2.367,2.367 -5.576,3.696 -8.923,3.696l-282.381,-0c-14.029,0 -25.418,12.999 -25.418,29.01c0,16.011 11.389,29.01 25.418,29.01l282.287,0c7.021,0 12.713,5.692 12.713,12.713l0,3.45c0,6.935 -5.622,12.557 -12.557,12.557l-289.943,0c-27.917,0 -50.582,-25.868 -50.582,-57.729Zm176.541,-377.732l0,51.477c56.112,0 101.668,45.556 101.668,101.668c0,56.112 -45.556,101.668 -101.668,101.668c-56.112,0 -101.668,-45.556 -101.668,-101.668c0,-49.036 34.791,-90.011 81.014,-99.565l9.724,-25.226c-60.572,5.246 -108.868,53.679 -113.899,114.313l-28.317,10.478l28.317,10.478c5.044,60.784 53.567,109.307 114.351,114.351l10.478,28.317l10.478,-28.317c60.784,-5.044 109.307,-53.567 114.351,-114.351l28.317,-10.478l-28.317,-10.478c-5.044,-60.784 -53.567,-109.307 -114.351,-114.351l-10.478,-28.317Zm60.486,93.147l-84.448,36.036l-36.036,84.448l84.448,-36.036l36.036,-84.448Zm-60.242,46.279c7.707,0 13.963,6.257 13.963,13.963c-0,7.707 -6.257,13.963 -13.963,13.963c-7.707,0 -13.963,-6.257 -13.963,-13.963c0,-7.707 6.257,-13.963 13.963,-13.963Z"
        />
      </svg>
      <div class="flex flex-col ml-6 space-y-1">
        <h1 class="text-2xl font-bold">{$_('common.app-name')}</h1>
        <p>{$_('about.version-label')} {appVersion}</p>
        <p class="text-base-content/40 text-sm/6">
          {$_('about.copyright', {
            values: { copyrightYear: getCopyrightYear() },
          })}
          <a
            class="link hover-underline"
            href={licenseUrl}
            onclick={openLicense}
          >
            {licenseUrl}
          </a>
        </p>
      </div>
    </div>
    <div class="modal-action">
      <form method="dialog">
        <button class="btn">{$_('common.close')}</button>
      </form>
    </div>
  </div>
</dialog>
