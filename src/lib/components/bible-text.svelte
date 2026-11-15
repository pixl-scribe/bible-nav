<script lang="ts">
  import ModuleMessage from './module-message.svelte';
  import { onMount, tick, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import IndicatorScrollBar from './indicator-scroll-bar.svelte';
  import Paragraph from './paragraph.svelte';
  import Verse from './verse.svelte';
  import SearchRow from './search-row.svelte';
  import FootnoteTooltip from './footnote-tooltip.svelte';
  import {
    appSettingsService,
    textMaxWidths,
  } from '$lib/services/app-settings-service.svelte';

  /**
   * A search's main pane: the search box and one text, scrolled through
   * without ever loading it all. `service` is bound by the search
   * (search-view.svelte) so its parallel pane can read and steer this text.
   */
  let {
    moduleId,
    // Assigned in onMount; the binding carries it to the parent, which ESLint
    // can't see.
    // eslint-disable-next-line no-useless-assignment
    service = $bindable(),
    parallelOpen = false,
    onToggleParallel,
    showSearch = true,
    startSid,
    selectorLabel,
    onModuleChange,
  }: {
    moduleId: string;
    service?: BibleTextService;
    parallelOpen?: boolean;
    onToggleParallel?: () => void;
    /** A parallel text shows no search box, only the Bible selector. */
    showSearch?: boolean;
    /** Where to start, such as the main text's verse for a parallel text. */
    startSid?: string;
    /** What the selector chooses for, such as "Bible for this search". */
    selectorLabel: string;
    /** The reader picked another module in the selector. */
    onModuleChange: (moduleId: string) => void;
  } = $props();

  let bibleTextService = $state<BibleTextService | undefined>();
  const textMaxWidth = $derived(textMaxWidths[appSettingsService.textWidth]);
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
    bibleTextService = new BibleTextService(
      untrack(() => moduleId),
      untrack(() => startSid)
    );
    service = bibleTextService;

    scrollableElement?.addEventListener('scroll', updatePosition);
    updatePosition();
    return () => {
      scrollableElement?.removeEventListener('scroll', updatePosition);
    };
  });

  // A module picked in the selector: switch this text to it, keeping the
  // reader's place.
  $effect(() => {
    const id = moduleId;
    untrack(() => bibleTextService?.setModule(id));
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
   * A verse's element in this pane. Each search keeps its own pane, so the
   * same verse can be on the page more than once: never look it up in the
   * whole document.
   */
  function findVerse(sid: string): HTMLElement | null {
    return (
      scrollableElement?.querySelector<HTMLElement>(
        `[id="${CSS.escape(sid)}"]`
      ) ?? null
    );
  }

  /**
   * Bring a verse into view when asked (a Notes tab heading): scroll to it if
   * it's loaded, or load the text there.
   */
  $effect(() => {
    const sid = bibleTextService?.revealSid;
    if (!sid) return;
    untrack(() => {
      if (bibleTextService === undefined) return;
      const el = findVerse(sid);
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      } else {
        bibleTextService.loadAtSid(sid);
      }
      bibleTextService.revealSid = undefined;
    });
  });

  /**
   * The verse at the top of the pane: the first verse whose bottom edge is
   * below the pane's top. Every verse is marked with `data-sid` (its whole
   * text, in both groupings), in reading order, so a binary search finds it
   * in a few steps. Measured at most once a frame while scrolling, for
   * parallel texts to follow.
   */
  let topPending = false;
  function measureTop() {
    if (topPending) return;
    topPending = true;
    requestAnimationFrame(() => {
      topPending = false;
      if (!scrollableElement || !bibleTextService) return;
      const top = scrollableElement.getBoundingClientRect().top + 4;
      const verses =
        scrollableElement.querySelectorAll<HTMLElement>('[data-sid]');
      let low = 0;
      let high = verses.length - 1;
      let found: HTMLElement | undefined;
      while (low <= high) {
        const middle = (low + high) >> 1;
        if (verses[middle].getBoundingClientRect().bottom > top) {
          found = verses[middle];
          high = middle - 1;
        } else {
          low = middle + 1;
        }
      }
      const sid = found?.dataset.sid;
      if (sid && bibleTextService.topSid !== sid) {
        bibleTextService.topSid = sid;
      }
    });
  }

  $effect(() => {
    const element = scrollableElement;
    if (!element) return;
    element.addEventListener('scroll', measureTop, { passive: true });
    return () => element.removeEventListener('scroll', measureTop);
  });

  // Measure again once newly loaded text is in place.
  $effect(() => {
    if (bibleTextService && !bibleTextService.selectInProgress) {
      untrack(measureTop);
    }
  });

  /**
   * Put a verse at the top at once (a parallel text keeping in step): scroll
   * to it if it's loaded, or load the text there, at the nearest verse this
   * module has.
   */
  $effect(() => {
    const sid = bibleTextService?.alignSid;
    if (!sid) return;
    untrack(() => {
      if (!bibleTextService || !scrollableElement) return;
      bibleTextService.alignSid = undefined;
      const el = findVerse(sid);
      if (el) {
        scrollToTop(el);
      } else {
        bibleTextService.loadNearest(sid);
      }
    });
  });

  /** Puts a loaded verse at the top of the pane at once. */
  function scrollToTop(el: HTMLElement) {
    if (!scrollableElement) return;
    const offset =
      el.getBoundingClientRect().top -
      scrollableElement.getBoundingClientRect().top;
    scrollableElement.scrollTop += offset - 4;
  }

  // Finish an alignment that needed a load: once the text is in place, put
  // the verse at the top.
  $effect(() => {
    const sid = bibleTextService?.alignAfterLoad;
    if (!sid || bibleTextService?.selectInProgress) return;
    untrack(async () => {
      if (!bibleTextService) return;
      bibleTextService.alignAfterLoad = undefined;
      await tick();
      const el = findVerse(sid);
      if (el) scrollToTop(el);
      measureTop();
    });
  });

  /**
   * Scroll to verse reference when scrollToSid changes.
   */
  $effect(() => {
    if (bibleTextService && bibleTextService?.scrollToSid) {
      const scrollId = bibleTextService.scrollToSid;
      untrack(() => {
        if (bibleTextService === undefined) return;
        const el = findVerse(scrollId);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
        bibleTextService.scrollToSid = undefined;
      });
    }
  });
</script>

<section
  aria-label={$_('bible-text.main-text')}
  class="flex min-w-0 flex-1 basis-0 flex-col rounded-box bg-base-200"
>
  <SearchRow
    {bibleTextService}
    {parallelOpen}
    {onToggleParallel}
    {showSearch}
    {selectorLabel}
    onSelectModule={onModuleChange}
  />
  {#if bibleTextService?.loadError}
    <div role="alert" class="alert alert-error m-2 whitespace-normal">
      <ModuleMessage error={bibleTextService.loadError} />
    </div>
  {/if}
  <div class="flex min-h-0 w-full flex-1 pt-2 pl-2 pb-2">
    <div
      class="h-full flex-1 overflow-y-auto overflow-x-hidden text-lg leading-relaxed scrollable-region"
      bind:this={scrollableElement}
    >
      <!-- The reading column: centered, at most the chosen text width. -->
      <div class="mx-auto flex flex-col" style:max-width={textMaxWidth}>
        {#if bibleTextService?.groupingMode === 'paragraph' && bibleTextService?.currentSearchType !== 'ref'}
          {#each bibleTextService?.prevParaBuffer ?? [] as paragraph, i (paragraph.id)}
            <div bind:this={prevElements[i]}>
              <Paragraph {paragraph} {bibleTextService} />
            </div>
          {/each}
          <div bind:this={activeElement}>
            {#if bibleTextService?.activePara}
              <Paragraph
                paragraph={bibleTextService.activePara}
                {bibleTextService}
              />
            {/if}
          </div>
          {#each bibleTextService?.nextParaBuffer ?? [] as paragraph, i (paragraph.id)}
            <div bind:this={nextElements[i]}>
              <Paragraph {paragraph} {bibleTextService} />
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
    </div>
    <!-- The pane's one shared footnote tooltip, kept inside the pane. -->
    <FootnoteTooltip pane={scrollableElement} />
    <div class="flex justify-center w-10 ml-1">
      <IndicatorScrollBar {bibleTextService} />
    </div>
  </div>
</section>

<style>
  .scrollable-region {
    scrollbar-width: none; /* Firefox */
    scrollbar-color: rgba(0, 0, 0, 0) rgba(0, 0, 0, 0); /* Do not want to show the real scrollbar. */
    &::-webkit-scrollbar {
      display: none;
    }
  }
</style>
