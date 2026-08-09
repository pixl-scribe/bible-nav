<script lang="ts">
  import { Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService from '$lib/services/module-service.svelte';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';
  import Paragraph from './paragraph.svelte';
  import Verse from './verse.svelte';

  let { moduleId }: { moduleId: string } = $props();

  let moduleService = $state<ModuleService | undefined>();
  let activeStatus = $state<'visible' | 'above' | 'below'>('visible');
  let scrollableElement: HTMLElement | undefined = $state();
  let prevElements = $state<HTMLDivElement[]>([]);
  let activeElement: HTMLElement | undefined = $state();
  let nextElements = $state<HTMLDivElement[]>([]);

  function updatePosition() {
    if (moduleService?.selectInProgress ?? true) return;
    if (!activeElement || !scrollableElement) return;
    const rect = activeElement.getBoundingClientRect();
    const scrollableArea = scrollableElement.getBoundingClientRect();
    if (rect.bottom < scrollableArea.top) {
      activeStatus = 'above';
    } else if (rect.top > scrollableArea.bottom) {
      activeStatus = 'below';
    } else {
      activeStatus = 'visible';
    }
  }

  onMount(() => {
    moduleService = new ModuleService(moduleId);

    scrollableElement?.addEventListener('scroll', updatePosition);
    updatePosition();
    return () => {
      scrollableElement?.removeEventListener('scroll', updatePosition);
    };
  });

  $effect(() => {
    if (activeStatus === 'above') {
      untrack(() => {
        if (moduleService === undefined) return;
        const toBeRemovedHt =
          prevElements.length >= moduleService.bufferSize
            ? prevElements?.[0]?.getBoundingClientRect()?.height
            : 0;
        moduleService.moveActiveDownOne().then(() => {
          if (scrollableElement && toBeRemovedHt > 0) {
            scrollableElement.scrollTop -= toBeRemovedHt;
          }
          activeStatus = 'visible';
        });
      });
    } else if (activeStatus === 'below') {
      untrack(() => {
        if (moduleService === undefined) return;
        moduleService.moveActiveUpOne().then(() => {
          if (moduleService === undefined) return;
          const newParaHt =
            prevElements.length >= moduleService.bufferSize
              ? prevElements?.[0]?.getBoundingClientRect()?.height
              : 0;
          if (scrollableElement) {
            scrollableElement.scrollTop += newParaHt;
          }
          activeStatus = 'visible';
        });
      });
    }
  });

  /**
   * Scroll to verse reference when scrollToSid changes.
   */
  $effect(() => {
    if (moduleService && moduleService?.scrollToSid) {
      const scrollId = moduleService.scrollToSid;
      untrack(() => {
        if (moduleService === undefined) return;
        const el = document.getElementById(scrollId);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
        moduleService.scrollToSid = undefined;
      });
    }
  });
</script>

<div
  class="flex flex-col bg-base-200 rounded-xl w-full flex-1 max-w-140 ml-1 h-full items-start"
>
  <div class="flex w-full p-2">
    <label class="input w-full">
      <Search />
      <input type="search" required placeholder={$_('bible-text.search')} />
    </label>
    <button
      class="btn btn-soft btn-primary px-1 ml-2 tooltip tooltip-bottom"
      data-tip={$_('bible-text.add-parallel')}
    >
      <Plus size={16} />
      <Columns2 />
    </button>
  </div>
  <div
    class="flex justify-between text-xs text-base-content/60 w-full px-2 pb-2 border-b border-base-content/10"
  >
    <div>{moduleService?.moduleName}</div>
    <div>{moduleService?.referenceLabel}</div>
  </div>
  <div class="flex w-full pt-2 pl-2 pb-2">
    <div
      class="flex flex-col h-[calc(100vh-104px)] w-full overflow-y-auto text-lg leading-relaxed scrollable-region"
      bind:this={scrollableElement}
    >
      {#if moduleService?.groupingMode === 'paragraph'}
        {#each Object.entries(moduleService?.prevParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
          <div bind:this={prevElements[i]}>
            <Paragraph {verses} {moduleService} />
          </div>
        {/each}
        <div bind:this={activeElement}>
          <Paragraph verses={moduleService?.activePara ?? []} {moduleService} />
        </div>
        {#each Object.entries(moduleService?.nextParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
          <div bind:this={nextElements[i]}>
            <Paragraph {verses} {moduleService} />
          </div>
        {/each}
      {:else}
        {#each moduleService?.prevVerseBuffer ?? [] as verse, i (verse.id)}
          <div bind:this={prevElements[i]}>
            <Verse {verse} {moduleService} />
          </div>
        {/each}
        {#if moduleService?.activeVerse !== undefined}
          <div bind:this={activeElement}>
            <Verse verse={moduleService.activeVerse} {moduleService} />
          </div>
        {/if}
        {#each moduleService?.nextVerseBuffer ?? [] as verse, i (verse.id)}
          <div bind:this={nextElements[i]}>
            <Verse {verse} {moduleService} />
          </div>
        {/each}
      {/if}
    </div>
    <div class="flex h-[calc(100vh-104px)] justify-center w-10 ml-1">
      <IndicatorScrollBar {moduleService} />
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
