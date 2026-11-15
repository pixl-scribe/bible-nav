<script lang="ts">
  /**
   * A parallel text: another Bible beside the main text, with its own Bible
   * selector but no search. It opens at the main text's verse and keeps in
   * step with it as the main text scrolls, verse by verse.
   * A footnote clicked here opens this module's Notes tab.
   */
  import { untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import type {
    ParallelPane,
    TextTab,
  } from '$lib/services/parallel-pane.svelte';
  import {
    FOOTNOTE_OPEN_EVENT,
    type FootnoteOpenRequest,
  } from '$lib/services/footnote-events';
  import BibleText from './bible-text.svelte';

  let {
    tab,
    pane,
    main,
  }: {
    tab: TextTab;
    pane: ParallelPane;
    /** The main pane's text, which this one follows. */
    main: BibleTextService | undefined;
  } = $props();

  let service = $state<BibleTextService | undefined>();
  let element = $state<HTMLElement>();

  // Keep in step with the main text: whenever the verse at its top
  // changes, put the same verse (or the nearest this Bible has) at the top
  // here. If this text was still loading, try once more when it's done, so
  // a jump in the main text (a new search, a cleared search) is caught. One
  // way only: the reader can still scroll this text by itself until the
  // main text moves again.
  let aligned: string | undefined;
  let retried: string | undefined;
  $effect(() => {
    const sid = main?.topSid;
    const loading = service?.selectInProgress ?? true;
    if (!sid || !service) return;
    untrack(() => {
      if (!service) return;
      if (sid !== aligned) {
        aligned = sid;
        retried = undefined;
        service.alignSid = sid;
      } else if (!loading && service.topSid !== sid && retried !== sid) {
        retried = sid;
        service.alignSid = sid;
      }
    });
  });

  // This text's footnotes open its own module's Notes tab, not the main
  // text's.
  $effect(() => {
    if (!element) return;
    const onOpen = (event: Event) => {
      event.stopPropagation();
      const { sid, letter } = (event as CustomEvent<FootnoteOpenRequest>)
        .detail;
      pane.openNotes(tab.moduleId, sid, letter);
    };
    element.addEventListener(FOOTNOTE_OPEN_EVENT, onOpen);
    return () => element?.removeEventListener(FOOTNOTE_OPEN_EVENT, onOpen);
  });
</script>

<div bind:this={element} class="flex min-h-0 flex-1">
  <BibleText
    moduleId={tab.moduleId}
    bind:service
    showSearch={false}
    startSid={tab.startSid}
    selectorLabel={$_('module-menu.for-parallel')}
    onModuleChange={(moduleId) => pane.setTextModule(tab.key, moduleId)}
  />
</div>
