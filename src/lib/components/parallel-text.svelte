<script lang="ts">
  /**
   * A parallel text: another Bible beside the main text, with its own Bible
   * selector but no search. It mirrors the main text: the same place when
   * the main text reads chapters, the same verses when it shows a list, and
   * the two keep in step by the verse at the top, whichever one the reader
   * scrolls: the text moved last leads (`leadAt`), and the other follows.
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
    active,
  }: {
    tab: TextTab;
    pane: ParallelPane;
    /** The main pane's text, which this one follows. */
    main: BibleTextService | undefined;
    /**
     * Whether this tab is the one shown. Other tabs stay loaded but hidden,
     * and a hidden pane can't scroll, so only the shown one keeps in step.
     */
    active: boolean;
  } = $props();

  let service = $state<BibleTextService | undefined>();
  let element = $state<HTMLElement>();

  // Mirror the main text's mode: its list, or chapters at its place.
  // Opening beside a list, start at the main text's verse; after that, a
  // new list starts at its top, like the main text (whose top verse is
  // still the old one while its new list loads).
  let opening = true;
  $effect(() => {
    const mode = main?.mode;
    const osis = main?.listOsis;
    if (!service || !main) return;
    untrack(() => {
      if (!service || !main) return;
      if (mode === 'list' && osis) {
        if (service.mode !== 'list' || service.listOsis !== osis) {
          service.showList(osis, opening ? main.topSid : undefined);
        }
      } else if (mode === 'chapters' && service.mode !== 'chapters') {
        service.openAt(main.topSid ?? main.browseSid);
      } else if (mode === 'text' && service.mode !== 'text') {
        service.showText();
      }
      opening = false;
    });
  });

  // Keep in step: the text the reader moved last brings its top verse to
  // the top of the other. The main text leads until this one is scrolled.
  $effect(() => {
    const sid = main?.topSid;
    if (!active || !sid || !service || !main) return;
    untrack(() => {
      if (!service || !main || main.leadAt < service.leadAt) return;
      if (service.topSid !== sid) service.align(sid);
    });
  });
  $effect(() => {
    const sid = service?.topSid;
    if (!active || !sid || !service || !main) return;
    untrack(() => {
      if (!service || !main || service.leadAt <= main.leadAt) return;
      if (main.topSid !== sid) main.align(sid);
    });
  });

  // Shown again (its tab selected): catch up with the main text, which
  // leads, since this text was last scrolled before it was hidden.
  $effect(() => {
    if (!active || !service || !main) return;
    untrack(() => {
      if (!service || !main) return;
      service.leadAt = 0;
      const sid = main.topSid;
      if (sid && service.topSid !== sid) service.align(sid);
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
