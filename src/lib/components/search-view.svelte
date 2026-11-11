<script lang="ts">
  /**
   * One search: its main pane and, beside it, its parallel pane. Hidden
   * searches stay mounted, so switching tabs keeps each one's place.
   */
  import BibleTextService from '$lib/services/bible-text-service.svelte';
  import type { SearchTab } from '$lib/services/search-tabs.svelte';
  import {
    FOOTNOTE_OPEN_EVENT,
    type FootnoteOpenRequest,
  } from '$lib/services/footnote-events';
  import BibleText from './bible-text.svelte';
  import ParallelPane from './parallel-pane.svelte';

  let {
    tab,
    hidden,
    id,
    labelledBy,
  }: {
    tab: SearchTab;
    hidden: boolean;
    /** The tab panel's id, named by its search tab. */
    id: string;
    labelledBy: string;
  } = $props();

  let service = $state<BibleTextService | undefined>();
  let element = $state<HTMLElement>();

  // The search tab shows what was typed.
  $effect(() => {
    tab.label = service?.searchInput.trim() ?? '';
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
    parallelOpen={tab.parallel.open}
    onToggleParallel={() => tab.parallel.toggle()}
  />
  {#if tab.parallel.open}
    <ParallelPane pane={tab.parallel} {service} />
  {/if}
</div>
