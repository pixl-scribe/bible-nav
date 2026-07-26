<script lang="ts">
  import { Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService from '$lib/services/module-service.svelte';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';

  let { moduleId }: { moduleId: string } = $props();

  let moduleService = $state<ModuleService | undefined>();
  let scrollValue = $state<number>(50);

  onMount(async () => {
    moduleService = new ModuleService(moduleId);
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
      {scrollValue}
      {#each moduleService?.verses as verse (verse.sid)}
        <div class="flex flex-wrap items-center">
          <div class="flex text-xs text-base-content/60">{verse.sid}</div>
          {#each verse.children as child, index (index)}
            {#if typeof child === 'string'}
              {#if /^[a-zA-Z]/.test(child)}
                &nbsp;
              {/if}
              <div class="flex">{child}</div>
            {:else if typeof child === 'object' && child.style === 'w'}
              {#if /^[a-zA-Z]/.test(child.txt)}
                &nbsp;
              {/if}
              <span class="flex">{child.txt}</span>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
    <div class="flex h-[calc(100vh-80px)] justify-center w-10">
      <IndicatorScrollBar bind:value={scrollValue} />
    </div>
  </div>
</div>

<style>
  .scrollable-region {
    scrollbar-color: rgba(0, 0, 0, 0) rgba(0, 0, 0, 0); /* Do not want to show the real scrollbar. */
    &::-webkit-scrollbar-thumb {
      background-color: rgba(0, 0, 0, 0);
    }
  }
</style>
