<script lang="ts">
  /**
   * One search: its main pane and, beside it, its parallel pane. Hidden
   * searches stay mounted, so switching tabs keeps each one's place.
   */
  import { untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import BibleTextService from '$lib/services/bible-text-service.svelte';
  import { chapterOf } from '$lib/services/parallel-pane.svelte';
  import type { SearchTab } from '$lib/services/search-tabs.svelte';
  import {
    FOOTNOTE_OPEN_EVENT,
    type FootnoteOpenRequest,
  } from '$lib/services/footnote-events';
  import { parallelMenuSections } from '$lib/services/parallel-menu';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import BibleText from './bible-text.svelte';
  import ParallelPane from './parallel-pane.svelte';

  let {
    tab,
    hidden,
    id,
    labelledBy,
    onLabel,
    onModuleChange,
  }: {
    tab: SearchTab;
    /** The search's label changed: what the reader typed. */
    onLabel: (label: string) => void;
    /** The reader picked another Bible for this search. */
    onModuleChange: (moduleId: string) => void;
    hidden: boolean;
    /** The tab panel's id, named by its search tab. */
    id: string;
    labelledBy: string;
  } = $props();

  let service = $state<BibleTextService | undefined>();
  let element = $state<HTMLElement>();

  // While the parallel pane has no tabs, the Parallel button offers what
  // can open in it, instead of opening an empty pane.
  const parallelMenu = $derived(
    tab.parallel.tabs.length === 0
      ? parallelMenuSections(
          tab.parallel,
          service,
          moduleFolderService.status?.modules ?? [],
          $_
        )
      : undefined
  );

  // The search tab shows what was typed.
  $effect(() => {
    onLabel(service?.searchInput.trim() ?? '');
  });

  // The chapter at the top of the main pane. As a $derived, it only signals
  // a change when the chapter itself changes, not on every scrolled verse.
  const chapterInView = $derived(
    service?.sidInView ? chapterOf(service.sidInView) : undefined
  );

  // The Notes tabs follow the chapter in view.
  $effect(() => {
    if (!chapterInView) return;
    untrack(() => {
      const sid = service?.sidInView;
      if (sid) tab.parallel.followChapter(sid);
    });
  });

  // A footnote clicked in the main pane opens it in the Notes tab.
  $effect(() => {
    if (!element) return;
    const onOpen = (event: Event) => {
      const { sid, letter } = (event as CustomEvent<FootnoteOpenRequest>)
        .detail;
      if (!service) return;
      tab.parallel.openNotes(service.moduleId, sid, letter);
      // Highlight the note's verse too: the pair always changes together.
      service.highlightSid = sid;
    };
    element.addEventListener(FOOTNOTE_OPEN_EVENT, onOpen);
    return () => element?.removeEventListener(FOOTNOTE_OPEN_EVENT, onOpen);
  });
</script>

<div
  bind:this={element}
  {id}
  role="tabpanel"
  aria-labelledby={labelledBy}
  class={['flex min-h-0 min-w-0 flex-1 gap-2', hidden && 'hidden']}
>
  <BibleText
    moduleId={tab.moduleId}
    bind:service
    selectorLabel={$_('module-menu.for-search')}
    {onModuleChange}
    parallelOpen={tab.parallel.open}
    onToggleParallel={() => tab.parallel.toggle()}
    {parallelMenu}
  />
  {#if tab.parallel.open}
    <ParallelPane pane={tab.parallel} {service} />
  {/if}
</div>
