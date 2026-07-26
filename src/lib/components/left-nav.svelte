<script lang="ts">
  import { Info, LibraryBig, Settings as SettingsIcon } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import Settings from './settings.svelte';
  import AboutDialog from './about-dialog.svelte';

  let isSettingsOpen = $state(false);
  let aboutDialogRef = $state<HTMLDialogElement>();

  function openSettingsDrawer() {
    isSettingsOpen = true;
  }

  function closeSettingsDrawer() {
    isSettingsOpen = false;
  }
</script>

<ul class="menu bg-base-200 rounded-box rounded-xl space-y-1">
  <li>
    <!-- svelte-ignore a11y_missing_attribute -->
    <a
      class="btn btn-soft btn-primary tooltip tooltip-right"
      data-tip={$_('left-nav.modules')}
    >
      <LibraryBig />
    </a>
  </li>
  <li>
    <!-- svelte-ignore a11y_missing_attribute, a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <a
      class="btn btn-soft btn-primary tooltip tooltip-right"
      data-tip={$_('left-nav.settings')}
      onclick={openSettingsDrawer}
    >
      <SettingsIcon />
    </a>
  </li>
  <li>
    <!-- svelte-ignore a11y_missing_attribute, a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <a
      class="btn btn-soft btn-primary tooltip tooltip-right"
      data-tip={$_('left-nav.about')}
      onclick={() => {
        aboutDialogRef?.showModal();
      }}
    >
      <Info />
    </a>
  </li>
</ul>

<input
  id="my-drawer"
  type="checkbox"
  class="drawer-toggle"
  bind:checked={isSettingsOpen}
/>

<div class="drawer-side">
  <label for="my-drawer" aria-label="close sidebar" class="drawer-overlay"
  ></label>
  <div class="menu p-0 w-80 min-h-full bg-base-200 text-base-content">
    <Settings onClose={closeSettingsDrawer} />
  </div>
</div>

<AboutDialog bind:dialog={aboutDialogRef} />
