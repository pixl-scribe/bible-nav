<script lang="ts">
  import TipTooltip from '$lib/components/tip-tooltip.svelte';
  import { ChevronDown, Plus, X } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import LeftNav from '$lib/components/left-nav.svelte';
  import { _ } from 'svelte-i18n';
  import SearchView from '$lib/components/search-view.svelte';
  import ModulesFolderNotice from '$lib/components/modules-folder-notice.svelte';
  import { SearchTabs } from '$lib/services/search-tabs.svelte';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import ModuleMenu, {
    type MenuSection,
  } from '$lib/components/module-menu.svelte';

  /** The text a new search reads. */
  const defaultModuleId = 'eng_kjv';
  const searches = new SearchTabs(defaultModuleId);

  const label = (text: string) => text || $_('search-tabs.new');

  const modules = $derived(moduleFolderService.status?.modules ?? []);
  const abbreviationOf = (moduleId: string) =>
    modules.find((module) => module.id === moduleId)?.abbreviation ?? '';

  /** The module a plain + opens: the last tab's, as SearchTabs.add does. */
  const nextModuleId = $derived(
    searches.tabs[searches.tabs.length - 1]?.moduleId ?? defaultModuleId
  );

  let newSearchGroup = $state<HTMLElement | undefined>();
  let newSearchMenuOpen = $state(false);
  const newSearchSections = $derived<MenuSection[]>([
    {
      heading: $_('module-menu.new-search-in'),
      items: modules
        .filter((module) => module.moduleType === 'bible')
        .map((module) => ({
          key: module.id,
          abbreviation: module.abbreviation,
          name: module.name,
          detail:
            module.id === nextModuleId
              ? $_('module-menu.same-as-now')
              : undefined,
          pick: () => searches.add(module.id),
        })),
    },
  ]);

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
            onclick={() => searches.select(tab.id)}
            >{label(tab.label)}{#if abbreviationOf(tab.moduleId)}<span
                class="ml-2 rounded bg-base-content/10 px-1.5 py-0.5 text-[0.6875rem]"
                >{abbreviationOf(tab.moduleId)}</span
              >{/if}</button
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
      <div class="join relative mb-0.5 ml-1" bind:this={newSearchGroup}>
        <button
          type="button"
          class="btn btn-soft btn-primary btn-sm btn-square join-item"
          aria-label={$_('main.new-search-in', {
            values: { abbr: abbreviationOf(nextModuleId) },
          })}
          data-tip={$_('main.new-search-in', {
            values: { abbr: abbreviationOf(nextModuleId) },
          })}
          onclick={() => searches.add()}
        >
          <Plus size={18} />
        </button>
        <button
          type="button"
          class="btn btn-soft btn-primary btn-sm join-item w-6 px-0"
          aria-label={$_('main.new-search-choose')}
          data-tip={$_('main.new-search-choose')}
          aria-haspopup="menu"
          aria-expanded={newSearchMenuOpen}
          onclick={(event) => {
            event.stopPropagation();
            newSearchMenuOpen = !newSearchMenuOpen;
          }}
        >
          <ChevronDown size={14} />
        </button>
        {#if newSearchMenuOpen}
          <ModuleMenu
            sections={newSearchSections}
            label={$_('module-menu.new-search-in')}
            class="top-9 left-0"
            onclose={() => (newSearchMenuOpen = false)}
          />
        {/if}
      </div>
      <TipTooltip root={newSearchGroup} placement="bottom" />
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
          onLabel={(text) => searches.setLabel(tab.id, text)}
          onModuleChange={(moduleId) => searches.setModule(tab.id, moduleId)}
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
