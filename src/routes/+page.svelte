<script lang="ts">
  import { Plus, X } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import LeftNav from '$lib/components/left-nav.svelte';
  import { _ } from 'svelte-i18n';
  import SearchView from '$lib/components/search-view.svelte';
  import ModulesFolderNotice from '$lib/components/modules-folder-notice.svelte';
  import { SearchTabs } from '$lib/services/search-tabs.svelte';

  /** The text a new search reads. */
  const defaultModuleId = 'eng_kjv';
  const searches = new SearchTabs(defaultModuleId);

  const label = (text: string) => text || $_('search-tabs.new');

  onMount(async () => {
    await themeService.loadTheme();
  });
</script>

<main class="drawer flex min-h-0 flex-1 gap-2 bg-base-300 p-2">
  <LeftNav />
  <div class="flex min-w-0 flex-1 flex-col">
    <!-- Search tabs are classic tabs joined to the content below. The first
         sits flush with the content's left edge, so the content's top-left
         corner is square only while the first tab is selected. -->
    <div
      role="tablist"
      aria-label={$_('search-tabs.label')}
      class="flex flex-wrap items-end gap-0.5 pr-1"
    >
      {#each searches.tabs as tab (tab.id)}
        {@const selected = tab.id === searches.activeId}
        <div
          class={[
            'flex h-9 items-center rounded-t-lg text-sm',
            selected
              ? 'bg-base-100 text-base-content'
              : 'text-base-content/60 hover:bg-base-100/50',
          ]}
        >
          <button
            type="button"
            role="tab"
            id="search-tab-{tab.id}"
            aria-selected={selected}
            aria-controls="search-panel-{tab.id}"
            class="h-full max-w-56 cursor-pointer truncate pr-1 pl-3.5"
            onclick={() => searches.select(tab.id)}>{label(tab.label)}</button
          >
          <button
            type="button"
            class="btn btn-ghost btn-xs btn-square mr-1.5"
            aria-label={$_('search-tabs.close', {
              values: { label: label(tab.label) },
            })}
            onclick={() => searches.close(tab.id)}
          >
            <X size={14} />
          </button>
        </div>
      {/each}
      <button
        type="button"
        class="btn btn-soft btn-primary btn-sm btn-square mb-0.5 ml-1"
        aria-label={$_('main.new-search')}
        title={$_('main.new-search')}
        onclick={() => searches.add()}
      >
        <Plus size={18} />
      </button>
    </div>
    <div
      class={[
        'flex min-h-0 flex-1 rounded-box bg-base-100 p-2',
        searches.activeIndex === 0 && 'rounded-tl-none',
      ]}
    >
      {#each searches.tabs as tab (tab.id)}
        <SearchView
          {tab}
          hidden={tab.id !== searches.activeId}
          id="search-panel-{tab.id}"
          labelledBy="search-tab-{tab.id}"
        />
      {/each}
    </div>
  </div>
  <ModulesFolderNotice />
</main>

<style>
  :global(html, body) {
    display: flex;
    flex: 1;
    height: 100%;
  }
</style>
