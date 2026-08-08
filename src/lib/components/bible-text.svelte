<script lang="ts">
  import { Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService, {
    paraBuffer,
  } from '$lib/services/module-service.svelte';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';
  import Paragraph from './paragraph.svelte';

  let { moduleId }: { moduleId: string } = $props();

  let moduleService = $state<ModuleService | undefined>();
  let activeStatus = $state<'visible' | 'above' | 'below'>('visible');
  let scrollableElement: HTMLElement | undefined = $state();
  let prevParaElements = $state<HTMLDivElement[]>([]);
  let activeParaElement: HTMLElement | undefined = $state();
  let nextParaElements = $state<HTMLDivElement[]>([]);

  function updatePosition() {
    if (moduleService?.selectInProgress ?? true) return;
    if (!activeParaElement || !scrollableElement) return;
    const rect = activeParaElement.getBoundingClientRect();
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
        const toBeRemovesParaHt =
          prevParaElements.length >= paraBuffer
            ? prevParaElements?.[0]?.getBoundingClientRect()?.height
            : 0;
        moduleService?.moveActiveDownOnePara().then(() => {
          if (scrollableElement && toBeRemovesParaHt > 0) {
            scrollableElement.scrollTop -= toBeRemovesParaHt;
          }
          activeStatus = 'visible';
        });
      });
    } else if (activeStatus === 'below') {
      untrack(() => {
        moduleService?.moveActiveUpOnePara().then(() => {
          const newParaHt =
            prevParaElements.length >= paraBuffer
              ? prevParaElements?.[0]?.getBoundingClientRect()?.height
              : 0;
          if (scrollableElement) {
            scrollableElement.scrollTop += newParaHt;
          }
          activeStatus = 'visible';
        });
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
      {#each Object.entries(moduleService?.prevParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
        <div bind:this={prevParaElements[i]}>
          <Paragraph {verses} {moduleService} class="bg-accent/10" />
        </div>
      {/each}
      <div bind:this={activeParaElement}>
        <Paragraph verses={moduleService?.activePara ?? []} {moduleService} />
      </div>
      {#each Object.entries(moduleService?.nextParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
        <div bind:this={nextParaElements[i]}>
          <Paragraph {verses} {moduleService} class="bg-primary/10" />
        </div>
      {/each}
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
