<script lang="ts">
  import { Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService from '$lib/services/module-service.svelte';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';
  import Paragraph from './paragraph.svelte';

  let { moduleId }: { moduleId: string } = $props();

  let moduleService = $state<ModuleService | undefined>();
  let scrollValue = $state<number | undefined>(undefined);

  onMount(async () => {
    moduleService = new ModuleService(moduleId);
  });

  $effect(() => {
    // Need to wait for books to be read before setting the scroll value.
    if (moduleService?.books && Object.keys(moduleService?.books).length > 0) {
      scrollValue = 0;
    }
  });
</script>

<div
  class="flex flex-col bg-base-200 rounded-box rounded-xl w-full flex-1 max-w-140 ml-1 h-full items-start"
>
  <div class="flex w-full p-2 border-b border-base-content/10">
    <label class="input w-full">
      <Search />
      <input type="search" required placeholder={$_('bible-text.search')} />
    </label>
    <div class="divider divider-horizontal mx-1"></div>
    <button
      class="btn btn-soft btn-primary px-1 tooltip tooltip-bottom"
      data-tip={$_('bible-text.add-parallel')}
    >
      <Plus size={16} />
      <Columns2 />
    </button>
  </div>
  <div class="flex h-[calc(100vh-66px)] w-full pt-2 pl-2 pb-2">
    <div
      class="flex flex-col h-[calc(100vh-80px)] w-full overflow-y-auto text-lg leading-relaxed scrollable-region"
    >
      {moduleService?.currentSearchType}
      {moduleService?.currentSearch}
      {#each Object.entries(moduleService?.prevParaBuffer ?? {}) as [paraIndex, verses] (paraIndex)}
        <Paragraph {verses} {moduleService} class="bg-green-950" />
      {/each}
      <Paragraph verses={moduleService?.activePara ?? []} {moduleService} />
      {#each Object.entries(moduleService?.nextParaBuffer ?? {}) as [paraIndex, verses] (paraIndex)}
        <Paragraph {verses} {moduleService} class="bg-blue-950" />
      {/each}
    </div>
    <div class="flex h-[calc(100vh-80px)] justify-center w-10 ml-1">
      <IndicatorScrollBar bind:value={scrollValue} {moduleService} />
    </div>
  </div>
</div>

<style>
  .scrollable-region {
    scrollbar-width: none; /* Firefox */
    scrollbar-color: rgba(0, 0, 0, 0) rgba(0, 0, 0, 0); /* Do not want to show the real scrollbar. */
    &::-webkit-scrollbar {
      display: none;
    }
  }
</style>
