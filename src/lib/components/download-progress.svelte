<script lang="ts">
  /**
   * A download's progress bar, with Cancel at its end, and how much has
   * arrived. Cancel sits here, under the row, rather than where the
   * Download or Update button was, so a double-click on that button can't
   * cancel the download it just started.
   */
  import { _, locale } from 'svelte-i18n';
  import type { DownloadState } from '$lib/services/module-catalog-service.svelte';
  import { formatSize } from '$lib/services/format';

  let {
    progress,
    label,
    oncancel,
  }: { progress: DownloadState; label: string; oncancel: () => void } =
    $props();

  const lang = $derived($locale ?? 'en-US');
</script>

<div class="flex flex-col gap-1">
  <div class="flex items-center gap-3">
    <progress
      class="progress progress-primary flex-1"
      value={progress.done}
      max={progress.total}
      aria-label={label}
    ></progress>
    <button class="btn btn-xs btn-ghost" onclick={oncancel}>
      {$_('modules.download.cancel')}
    </button>
  </div>
  <div
    class="flex flex-wrap justify-between gap-2 text-sm text-base-content/70"
  >
    <span>
      {$_('modules.download.progress', {
        values: {
          done: formatSize(progress.done, lang),
          total: formatSize(progress.total, lang),
        },
      })}
    </span>
    <span>{$_('modules.download.then-check')}</span>
  </div>
</div>
