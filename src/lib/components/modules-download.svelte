<script lang="ts">
  /**
   * The Download tab. Before the reader presses Check for modules, nothing
   * has gone online and the tab says what checking will contact. After, it
   * lists each module the catalog offers whose signatures checked.
   */
  import { Check, Download, Info } from '@lucide/svelte';
  import { _, locale } from 'svelte-i18n';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import {
    moduleCatalogService,
    offerKey,
  } from '$lib/services/module-catalog-service.svelte';
  import { newestOffers, shortLicense } from '$lib/services/module-view';
  import { formatAgo, formatSize } from '$lib/services/format';
  import ModuleMessage from './module-message.svelte';
  import DownloadProgress from './download-progress.svelte';
  import DownloadStatus from './download-status.svelte';

  const status = $derived(moduleFolderService.status);
  const offers = $derived(
    moduleCatalogService.offers
      ? newestOffers(moduleCatalogService.offers)
      : undefined
  );
  const lang = $derived($locale ?? 'en-US');
</script>

{#if offers === undefined}
  <div
    class="flex-1 flex flex-col items-center justify-center gap-4 text-center"
  >
    <Download size={48} class="text-primary" aria-hidden="true" />
    <h3 class="text-lg font-bold">{$_('modules.download.heading')}</h3>
    <p class="max-w-md text-sm">{$_('modules.download.online')}</p>
    <p class="max-w-md text-sm text-base-content/70">
      {$_('modules.download.caution')}
    </p>
    <button
      class="btn btn-primary"
      disabled={moduleCatalogService.checking}
      aria-busy={moduleCatalogService.checking}
      onclick={() => moduleCatalogService.check()}
    >
      {#if moduleCatalogService.checking}
        <span class="loading loading-spinner loading-sm" aria-hidden="true"
        ></span>
        {$_('modules.checking')}
      {:else}
        {$_('modules.download.check')}
      {/if}
    </button>
    {#if moduleCatalogService.error}
      <p role="alert" class="text-error text-sm max-w-md">
        <ModuleMessage error={moduleCatalogService.error} />
      </p>
    {/if}
  </div>
{:else}
  <div class="flex flex-col gap-3 min-h-0 flex-1">
    <div class="flex flex-wrap items-center justify-end gap-3 text-sm">
      {#if status?.catalogCheckedAt}
        <span class="text-base-content/70">
          {$_('modules.checked-ago', {
            values: { when: formatAgo(status.catalogCheckedAt, lang) },
          })}
        </span>
      {/if}
      <button
        class="btn btn-sm btn-outline btn-primary"
        disabled={moduleCatalogService.checking}
        aria-busy={moduleCatalogService.checking}
        onclick={() => moduleCatalogService.check()}
      >
        {#if moduleCatalogService.checking}
          <span class="loading loading-spinner loading-sm" aria-hidden="true"
          ></span>
          {$_('modules.checking')}
        {:else}
          {$_('modules.download.check-again')}
        {/if}
      </button>
    </div>
    {#if moduleCatalogService.error}
      <p role="alert" class="text-error text-sm">
        <ModuleMessage error={moduleCatalogService.error} />
      </p>
    {/if}

    <ul class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-3">
      {#each offers as offer (offerKey(offer))}
        {@const progress = moduleCatalogService.downloads[offerKey(offer)]}
        {@const failure = moduleCatalogService.failures[offerKey(offer)]}
        <li class="bg-base-200 rounded-box p-4 flex flex-col gap-3">
          <!-- The name may wrap onto more lines; what's on the right keeps
               its size beside it, never dropping below. -->
          <div class="flex items-center justify-between gap-4">
            <div class="flex flex-col gap-1 min-w-0 flex-1">
              <!-- The version follows the name's last word, wherever the
                   name wraps. -->
              <h3 class="text-base font-bold">
                {offer.name}<span
                  class="ms-2 whitespace-nowrap text-sm font-normal text-base-content/70"
                  >v{offer.version}</span
                >
              </h3>
              <p class="text-sm text-base-content/70">
                {[
                  offer.language,
                  shortLicense(offer.license),
                  formatSize(offer.size, lang),
                ]
                  .filter(Boolean)
                  .join(' · ')}
              </p>
            </div>

            <div class="shrink-0">
              {#if progress}
                <DownloadStatus {progress} />
              {:else if offer.relation === 'installed'}
                <span class="flex items-center gap-1 text-sm text-success">
                  <Check size={16} aria-hidden="true" />
                  {$_('modules.download.installed')}
                </span>
              {:else if offer.relation === 'older'}
                <span class="text-sm text-base-content/70">
                  {$_('modules.download.newer-installed')}
                </span>
              {:else if offer.needsNewerApp}
                <span
                  class="flex items-center gap-1 text-sm text-base-content/70"
                >
                  <Info size={16} aria-hidden="true" />
                  {$_('modules.download.needs-newer-app')}
                </span>
              {:else}
                <button
                  class="btn btn-sm btn-primary"
                  onclick={() => moduleCatalogService.download(offer)}
                >
                  <Download size={16} aria-hidden="true" />
                  {offer.relation === 'update'
                    ? $_('modules.update')
                    : $_('modules.download.download')}
                </button>
              {/if}
            </div>
          </div>

          {#if progress}
            <DownloadProgress
              {progress}
              label={offer.name}
              oncancel={() => moduleCatalogService.cancel(offer)}
            />
          {/if}
          {#if failure}
            <p role="alert" class="text-error text-sm">
              <span class="block font-semibold">
                {$_('modules.download.failed', {
                  values: { name: offer.name },
                })}
              </span>
              <ModuleMessage error={failure} />
            </p>
          {/if}
        </li>
      {:else}
        <li class="text-base-content/70">{$_('modules.download.none')}</li>
      {/each}
    </ul>

    {#if moduleCatalogService.problems.length > 0}
      <details class="text-sm">
        <summary class="cursor-pointer text-base-content/70">
          {$_('modules.download.skipped')}
        </summary>
        <ul class="list-disc pl-5 mt-2">
          {#each moduleCatalogService.problems as problem, index (index)}
            <li><ModuleMessage {problem} /></li>
          {/each}
        </ul>
      </details>
    {/if}

    <p class="text-sm text-base-content/70">
      {$_('modules.download.signed-note')}
    </p>
  </div>
{/if}

{#if status?.portable}
  <p
    class="mt-3 flex items-center gap-2 rounded-box bg-base-300 px-3 py-2 text-sm text-base-content/70"
  >
    <Info size={16} aria-hidden="true" class="shrink-0" />
    {$_('modules.download.portable')}
  </p>
{/if}
