<script lang="ts">
  /**
   * The Modules dialog, opened from the left nav: the modules installed in
   * the folder, and modules to download. Nothing in it goes online until
   * the reader presses a check button.
   */
  import { X } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import ModulesInstalled from './modules-installed.svelte';
  import ModulesDownload from './modules-download.svelte';
  import TipTooltip from './tip-tooltip.svelte';
  import {
    modulesDialogService,
    type ModulesTab,
  } from '$lib/services/modules-dialog-service.svelte';

  let { dialog = $bindable() }: { dialog: HTMLDialogElement | undefined } =
    $props();

  const tabs: ModulesTab[] = ['installed', 'download'];
  let box = $state<HTMLElement | undefined>();
  // The tab to show; "Get more modules…" opens the dialog on Download.
  const selected = $derived(modulesDialogService.tab);
</script>

<dialog bind:this={dialog} class="modal" aria-labelledby="modules-title">
  <div
    bind:this={box}
    class="modal-box w-11/12 max-w-3xl h-[85vh] max-h-[760px] flex flex-col gap-4 p-6"
  >
    <div class="flex items-center justify-between">
      <h2 id="modules-title" class="text-xl font-bold">
        {$_('modules.title')}
      </h2>
      <form method="dialog">
        <button
          class="btn btn-ghost btn-circle"
          aria-label={$_('common.close')}
        >
          <X size={20} />
        </button>
      </form>
    </div>

    <div role="tablist" aria-label={$_('modules.tabs')} class="flex gap-1">
      {#each tabs as tab (tab)}
        <button
          role="tab"
          id="modules-tab-{tab}"
          aria-selected={selected === tab}
          aria-controls="modules-panel-{tab}"
          class="btn btn-sm {selected === tab ? 'btn-primary' : 'btn-ghost'}"
          onclick={() => (modulesDialogService.tab = tab)}
        >
          {$_(`modules.tab.${tab}`)}
        </button>
      {/each}
    </div>

    <div
      role="tabpanel"
      id="modules-panel-{selected}"
      aria-labelledby="modules-tab-{selected}"
      class="flex-1 min-h-0 flex flex-col"
    >
      {#if selected === 'installed'}
        <ModulesInstalled />
      {:else}
        <ModulesDownload />
      {/if}
    </div>
  </div>
  <form method="dialog" class="modal-backdrop">
    <button tabindex="-1">{$_('common.close')}</button>
  </form>
  <!-- Inside the dialog, so it shows above it. -->
  <TipTooltip root={box} boundary={box} />
</dialog>
