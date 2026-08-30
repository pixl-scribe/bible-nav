<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';
  import Paragraph from './paragraph.svelte';
  import Verse from './verse.svelte';
  import SearchRow from './search-row.svelte';

  let { moduleId }: { moduleId: string } = $props();

  let bibleTextService = $state<BibleTextService | undefined>();
  let activeStatus = $state<'visible' | 'above' | 'below'>('visible');
  let scrollableElement: HTMLElement | undefined = $state();
  let prevElements = $state<HTMLDivElement[]>([]);
  let activeElement: HTMLElement | undefined = $state();
  let nextElements = $state<HTMLDivElement[]>([]);

  function updatePosition() {
    if (bibleTextService?.selectInProgress ?? true) return;
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
    bibleTextService = new BibleTextService(moduleId);

    scrollableElement?.addEventListener('scroll', updatePosition);
    updatePosition();
    return () => {
      scrollableElement?.removeEventListener('scroll', updatePosition);
    };
  });

  $effect(() => {
    if (activeStatus === 'above') {
      untrack(() => {
        if (bibleTextService === undefined) return;
        const toBeRemovedHt =
          prevElements.length >= bibleTextService.bufferSize
            ? prevElements?.[0]?.getBoundingClientRect()?.height
            : 0;
        bibleTextService.moveActiveDownOne().then(() => {
          if (scrollableElement && toBeRemovedHt > 0) {
            scrollableElement.scrollTop -= toBeRemovedHt;
          }
          activeStatus = 'visible';
        });
      });
    } else if (activeStatus === 'below') {
      untrack(() => {
        if (bibleTextService === undefined) return;
        bibleTextService.moveActiveUpOne().then(() => {
          if (bibleTextService === undefined) return;
          const newParaHt =
            prevElements.length >= bibleTextService.bufferSize
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
    if (bibleTextService && bibleTextService?.scrollToSid) {
      const scrollId = bibleTextService.scrollToSid;
      untrack(() => {
        if (bibleTextService === undefined) return;
        const el = document.getElementById(scrollId);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
        bibleTextService.scrollToSid = undefined;
      });
    }
  });
</script>

<div
  class="flex flex-col bg-base-200 rounded-xl w-full flex-1 max-w-140 ml-1 h-full items-start"
>
  <SearchRow {bibleTextService} />
  <div class="flex w-full pt-2 pl-2 pb-2">
    <div
      class="flex flex-col h-[calc(100vh-104px)] w-full overflow-y-auto text-lg leading-relaxed scrollable-region"
      bind:this={scrollableElement}
    >
      {#if bibleTextService?.groupingMode === 'paragraph'}
        {#each Object.entries(bibleTextService?.prevParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
          <div bind:this={prevElements[i]}>
            <Paragraph {verses} {bibleTextService} />
          </div>
        {/each}
        <div bind:this={activeElement}>
          <Paragraph
            verses={bibleTextService?.activePara ?? []}
            {bibleTextService}
          />
        </div>
        {#each Object.entries(bibleTextService?.nextParaBuffer ?? {}) as [paraIndex, verses], i (paraIndex)}
          <div bind:this={nextElements[i]}>
            <Paragraph {verses} {bibleTextService} />
          </div>
        {/each}
      {:else}
        {#each bibleTextService?.prevVerseBuffer ?? [] as verse, i (verse.id)}
          <div bind:this={prevElements[i]}>
            <Verse {verse} {bibleTextService} />
          </div>
        {/each}
        {#if bibleTextService?.activeVerse !== undefined}
          <div bind:this={activeElement}>
            <Verse verse={bibleTextService.activeVerse} {bibleTextService} />
          </div>
        {/if}
        {#each bibleTextService?.nextVerseBuffer ?? [] as verse, i (verse.id)}
          <div bind:this={nextElements[i]}>
            <Verse {verse} {bibleTextService} />
          </div>
        {/each}
      {/if}
    </div>
    <div class="flex h-[calc(100vh-104px)] justify-center w-10 ml-1">
      <IndicatorScrollBar {bibleTextService} />
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
