<script lang="ts">
  /**
   * The Installed tab: each module in the folder with its version, license,
   * and older versions, updates found by the last check, and the folder
   * itself. Checking for updates goes online, only when the reader asks.
   */
  import { Check, Info } from '@lucide/svelte';
  import { _, locale } from 'svelte-i18n';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import {
    moduleCatalogService,
    offerKey,
    type Offer,
  } from '$lib/services/module-catalog-service.svelte';
  import {
    newerTextFor,
    shortLicense,
    updateFor,
  } from '$lib/services/module-view';
  import { formatAgo, formatSize } from '$lib/services/format';
  import ModuleMessage from './module-message.svelte';
  import DownloadProgress from './download-progress.svelte';
  import DownloadStatus from './download-status.svelte';
  import ChangeFolderDialog from './change-folder-dialog.svelte';

  const status = $derived(moduleFolderService.status);
  const offers = $derived(moduleCatalogService.offers ?? []);
  const lang = $derived($locale ?? 'en-US');
  const updates = $derived(
    (status?.modules ?? [])
      .map((module) => updateFor(module.id, offers))
      .filter((offer): offer is Offer => !!offer)
  );

  let expanded = $state<Record<string, boolean>>({});
  let confirmDelete = $state<string | undefined>();
  let changeDialog = $state<ReturnType<typeof ChangeFolderDialog>>();

  function updateAll() {
    for (const offer of updates) moduleCatalogService.download(offer);
  }

  async function deleteVersion(file: string) {
    confirmDelete = undefined;
    await moduleFolderService.deleteVersion(file);
  }
</script>

<div class="flex flex-col gap-3 min-h-0 flex-1">
  <div class="flex flex-wrap items-center justify-end gap-3 text-sm">
    <span class="text-base-content/70">
      {#if status?.catalogCheckedAt}
        {$_('modules.checked-ago', {
          values: { when: formatAgo(status.catalogCheckedAt, lang) },
        })}
      {:else}
        {$_('modules.checked-never')}
      {/if}
    </span>
    {#if updates.length > 1}
      <button class="btn btn-sm btn-primary" onclick={updateAll}>
        {$_('modules.update-all', { values: { count: updates.length } })}
      </button>
    {/if}
    <button
      class="btn btn-sm btn-outline"
      disabled={moduleCatalogService.checking}
      aria-busy={moduleCatalogService.checking}
      onclick={() => moduleCatalogService.check()}
    >
      {#if moduleCatalogService.checking}
        <span class="loading loading-spinner loading-sm" aria-hidden="true"
        ></span>
        {$_('modules.checking')}
      {:else}
        {$_('modules.check-updates')}
      {/if}
    </button>
  </div>
  {#if moduleCatalogService.error}
    <p role="alert" class="text-error text-sm">
      <ModuleMessage error={moduleCatalogService.error} />
    </p>
  {/if}

  <ul class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-3">
    {#each status?.modules ?? [] as module (module.id)}
      {@const update = updateFor(module.id, offers)}
      {@const newerText = newerTextFor(module.id, offers)}
      {@const progress = update
        ? moduleCatalogService.downloads[offerKey(update)]
        : undefined}
      {@const failure = update
        ? moduleCatalogService.failures[offerKey(update)]
        : undefined}
      <li class="bg-base-200 rounded-box p-4 flex flex-col gap-3">
        <div class="flex items-start justify-between gap-4">
          <div class="flex flex-col gap-1 min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h3 class="text-base font-bold">{module.name}</h3>
              <span class="text-sm text-base-content/70">v{module.version}</span
              >
              {#if module.builtIn}
                <span class="badge badge-sm badge-neutral"
                  >{$_('modules.built-in')}</span
                >
              {/if}
              {#if module.textVerified}
                <span
                  class="badge badge-sm badge-success badge-soft"
                  title={$_('modules.text-verified.hint')}
                >
                  <Check size={12} aria-hidden="true" />
                  {$_('modules.text-verified')}
                </span>
              {/if}
            </div>
            <p class="text-sm text-base-content/70">
              {[module.language, shortLicense(module.license)]
                .filter(Boolean)
                .join(' · ')}
            </p>
          </div>
          <button
            class="btn btn-sm btn-outline shrink-0"
            aria-expanded={!!expanded[module.id]}
            onclick={() => (expanded[module.id] = !expanded[module.id])}
          >
            {expanded[module.id]
              ? $_('modules.hide-details')
              : $_('modules.details')}
          </button>
        </div>

        {#if expanded[module.id]}
          <div
            class="border-t border-base-300 pt-3 text-sm flex flex-col gap-2"
          >
            {#if module.description}<p>{module.description}</p>{/if}
            <p>
              <strong>{$_('modules.license')}</strong>
              {module.license}
            </p>
            <p class="text-base-content/70">
              {$_('modules.file', { values: { file: module.file } })}
            </p>
          </div>
        {/if}

        {#if newerText}
          <p
            class="flex items-center gap-2 bg-base-300 rounded-field px-3 py-2 text-sm"
          >
            <Info size={16} aria-hidden="true" class="text-primary shrink-0" />
            {$_('modules.needs-newer-app', {
              values: { version: newerText.version },
            })}
          </p>
        {/if}

        {#if update}
          <div
            class="border-t border-base-300 pt-3 flex flex-wrap items-center justify-between gap-3"
          >
            <div class="flex flex-col gap-1">
              <span class="text-sm font-semibold">
                {$_('modules.update-available', {
                  values: { from: module.version, to: update.version },
                })}
              </span>
              <span class="text-sm text-base-content/70">
                {$_(
                  update.textChanges
                    ? 'modules.update.text-changes'
                    : 'modules.update.same-text',
                  { values: { size: formatSize(update.size, lang) } }
                )}
              </span>
            </div>
            {#if progress}
              <DownloadStatus {progress} />
            {:else}
              <button
                class="btn btn-sm btn-primary"
                onclick={() => moduleCatalogService.download(update)}
              >
                {$_('modules.update')}
              </button>
            {/if}
          </div>
          {#if progress}
            <DownloadProgress
              {progress}
              label={module.name}
              oncancel={() => moduleCatalogService.cancel(update)}
            />
          {/if}
          {#if failure}
            <p role="alert" class="text-error text-sm">
              <ModuleMessage error={failure} />
            </p>
          {/if}
        {/if}

        {#each module.older as older (older.file)}
          <div
            class="border-t border-base-300 pt-3 flex flex-wrap items-center justify-between gap-3 text-sm"
          >
            <span class="text-base-content/70">
              {$_('modules.older', { values: { version: older.version } })}
            </span>
            {#if confirmDelete === older.file}
              <span class="flex items-center gap-2">
                {$_('modules.delete-confirm', {
                  values: { version: older.version },
                })}
                <button
                  class="btn btn-sm btn-error"
                  onclick={() => deleteVersion(older.file)}
                >
                  {$_('modules.delete-yes')}
                </button>
                <button
                  class="btn btn-sm btn-ghost"
                  onclick={() => (confirmDelete = undefined)}
                >
                  {$_('modules.delete-no')}
                </button>
              </span>
            {:else}
              <button
                class="btn btn-sm btn-ghost text-error"
                onclick={() => (confirmDelete = older.file)}
              >
                {$_('modules.delete-version', {
                  values: { version: older.version },
                })}
              </button>
            {/if}
          </div>
        {/each}
      </li>
    {:else}
      <li class="text-base-content/70">{$_('modules.none-installed')}</li>
    {/each}
  </ul>

  {#if moduleFolderService.error || (status?.problems.length ?? 0) > 0}
    <div
      role="alert"
      class="alert alert-warning text-sm flex flex-col items-start"
    >
      <span class="font-bold">{$_('modules.problems')}</span>
      <ul class="list-disc pl-4">
        {#if moduleFolderService.error}
          <li><ModuleMessage error={moduleFolderService.error} /></li>
        {/if}
        {#each status?.problems ?? [] as problem, index (index)}
          <li><ModuleMessage {problem} /></li>
        {/each}
      </ul>
    </div>
  {/if}

  <div
    class="border-t border-base-300 pt-3 flex flex-wrap items-center justify-between gap-3"
  >
    <div class="flex flex-col min-w-0">
      <span class="text-sm text-base-content/70">{$_('modules.folder')}</span>
      <span class="text-sm break-all">{status?.folder ?? ''}</span>
      {#if status?.portable}
        <span class="text-xs text-base-content/70"
          >{$_('modules.folder.portable')}</span
        >
      {/if}
    </div>
    <div class="flex gap-2">
      <button
        class="btn btn-sm btn-outline"
        onclick={() => moduleFolderService.openFolder()}
      >
        {$_('modules.open-folder')}
      </button>
      {#if status && !status.portable}
        <button
          class="btn btn-sm btn-primary"
          disabled={moduleFolderService.changing}
          onclick={() => changeDialog?.start()}
        >
          {$_('modules.change-folder')}
        </button>
      {/if}
    </div>
  </div>
</div>

<ChangeFolderDialog bind:this={changeDialog} />
