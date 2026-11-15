<script lang="ts">
  /**
   * Where the Download or Update button was, while it downloads: a status
   * that can't be clicked, such as "Downloading… 46%".
   */
  import { _, locale } from 'svelte-i18n';
  import type { DownloadState } from '$lib/services/module-catalog-service.svelte';
  import { formatPercent } from '$lib/services/format';

  let { progress }: { progress: DownloadState } = $props();

  const lang = $derived($locale ?? 'en-US');
</script>

<span class="flex items-center gap-2 text-sm text-base-content/70">
  <span class="loading loading-spinner loading-xs" aria-hidden="true"></span>
  {$_('modules.download.status', {
    values: { percent: formatPercent(progress.done, progress.total, lang) },
  })}
</span>
