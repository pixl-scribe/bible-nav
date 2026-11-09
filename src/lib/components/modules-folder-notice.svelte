<script lang="ts">
  /**
   * A small, non-blocking message in the corner: on first run, where Bible
   * modules are stored, and any problems found with the modules folder.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { _ } from 'svelte-i18n';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';

  let problemsDismissed = $state(false);

  const status = $derived(moduleFolderService.status);
  const showProblems = $derived(
    !problemsDismissed &&
      ((status?.problems.length ?? 0) > 0 || !!moduleFolderService.error)
  );

  onMount(() => {
    moduleFolderService.load();

    // Rescan when the window regains focus, so modules copied into the
    // folder while the app was in the background appear without a restart.
    let stopListening: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) {
          moduleFolderService.rescan();
        }
      })
      .then((unlisten) => (stopListening = unlisten));
    return () => stopListening?.();
  });
</script>

{#if status?.showNotice || showProblems}
  <div class="toast toast-end z-50">
    {#if status?.showNotice}
      <div role="status" class="alert flex flex-col items-start max-w-sm">
        <span class="whitespace-normal">
          {$_('modules-folder.notice', {
            values: { folder: status.folder ?? '' },
          })}
        </span>
        <div class="flex gap-2 self-end">
          <button
            class="btn btn-sm"
            disabled={moduleFolderService.changing}
            onclick={() => moduleFolderService.changeFolder()}
          >
            {$_('modules-folder.change')}
          </button>
          <button
            class="btn btn-sm btn-primary"
            onclick={() => moduleFolderService.dismissNotice()}
          >
            {$_('common.ok')}
          </button>
        </div>
      </div>
    {/if}

    {#if showProblems}
      <div
        role="alert"
        class="alert alert-warning flex flex-col items-start max-w-sm"
      >
        <span class="font-bold">{$_('modules-folder.problems')}</span>
        <ul class="list-disc pl-4 text-sm whitespace-normal">
          {#if moduleFolderService.error}
            <li>{moduleFolderService.error}</li>
          {/if}
          {#each status?.problems ?? [] as problem, index (index)}
            <li>{problem}</li>
          {/each}
        </ul>
        <button
          class="btn btn-sm self-end"
          onclick={() => (problemsDismissed = true)}
        >
          {$_('common.close')}
        </button>
      </div>
    {/if}
  </div>
{/if}
