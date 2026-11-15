<script lang="ts">
  /**
   * Where the main text is, and how to go elsewhere: ‹ previous chapter,
   * the place (which opens the reference picker), › next chapter. The
   * arrows show only while reading chapters (an empty search or a single
   * place), not for a list of references or a word search, and always go
   * to verse 1 of that chapter. At the module's first or last chapter an
   * arrow is dimmed, never hidden, so the control doesn't shift; it stays
   * focusable and its tooltip says why.
   *
   * Ctrl+G (Cmd+G) opens the picker, like pressing the place.
   *
   * ← and → do the same when the cursor isn't in a text box and nothing
   * else (a menu, a dialog) has them. In a right-to-left layout they swap,
   * as the arrows do.
   */
  import {
    BookOpen,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
  } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import TipTooltip from './tip-tooltip.svelte';
  import { hitChapterStep } from '$lib/services/search-hits';

  let {
    service,
    onopenpicker,
  }: {
    service: BibleTextService;
    /** The place was pressed: open the reference picker. */
    onopenpicker: () => void;
  } = $props();

  let element = $state<HTMLElement>();

  // Reading chapters, the arrows step chapter by chapter; in a word search
  // they step between chapters that have matches. A reference list has no
  // arrows: its verses are only those listed.
  const searching = $derived(service.wordSearch !== undefined);
  const showArrows = $derived(service.mode === 'chapters' || searching);
  const fullPlace = $derived(
    service.topSid ? service.formatRefFromSid(service.topSid) : ''
  );
  const shortPlace = $derived(service.referenceLabel);

  /** 'GEN 5' as 'Genesis 5'. */
  function chapterName(chapterSid: string): string {
    return service.formatChapterFromSid(`${chapterSid}:1`);
  }

  /** Where an arrow goes: a verse, and its tooltip. */
  interface Step {
    sid: string;
    tip: string;
  }

  function target(direction: 1 | -1): Step | undefined {
    const key = direction === 1 ? 'next' : 'previous';
    if (searching) {
      const hits = service.hits;
      const from = service.topSid;
      const chapter =
        hits && from ? hitChapterStep(hits, from, direction) : undefined;
      if (!chapter) return undefined;
      return {
        sid: chapter.first,
        tip: $_(`place.${key}-match`, {
          values: {
            chapter: chapterName(chapter.chapter),
            count: chapter.count,
          },
        }),
      };
    }
    const chapter = service.chapterStep(direction);
    if (!chapter) return undefined;
    return {
      sid: `${chapter}:1`,
      tip: $_(`place.${key}`, { values: { chapter: chapterName(chapter) } }),
    };
  }

  const previous = $derived(target(-1));
  const next = $derived(target(1));
  const here = $derived(chapterName(service.placeChapter));
  const previousTip = $derived(
    previous?.tip ??
      (searching
        ? $_('place.no-earlier-match')
        : $_('place.first', { values: { chapter: here } }))
  );
  const nextTip = $derived(
    next?.tip ??
      (searching
        ? $_('place.no-later-match')
        : $_('place.last', { values: { chapter: here } }))
  );

  function step(direction: 1 | -1) {
    const to = direction === 1 ? next : previous;
    if (!to) return;
    if (searching) service.goToListSid(to.sid);
    else service.stepChapter(direction);
  }

  // Where the arrow keys mustn't step chapters: typing, menus, tab strips,
  // and dialogs, which use the arrows themselves.
  const ownArrows =
    'input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="menu"], [role="tablist"], [role="listbox"], dialog';

  function onKey(event: KeyboardEvent) {
    // Ctrl+G (Cmd+G on a Mac) opens the reference picker, even from the
    // search box: "Go to", as in most editors.
    if (
      (event.ctrlKey || event.metaKey) &&
      !event.altKey &&
      !event.shiftKey &&
      event.key.toLowerCase() === 'g'
    ) {
      if (!element || element.offsetParent === null) return;
      if (document.querySelector('dialog[open]')) return;
      event.preventDefault();
      onopenpicker();
      return;
    }
    // 'Left' and 'Right' are older names some engines still send.
    const left = event.key === 'ArrowLeft' || event.key === 'Left';
    const right = event.key === 'ArrowRight' || event.key === 'Right';
    if (!left && !right) return;
    if (event.defaultPrevented || event.altKey || event.ctrlKey) return;
    if (event.metaKey || event.shiftKey) return;
    // Only the search being shown: hidden searches keep their panes.
    if (!showArrows || !element || element.offsetParent === null) return;
    if ((event.target as Element | null)?.closest?.(ownArrows)) return;
    if (document.querySelector('dialog[open]')) return;
    event.preventDefault();
    const rtl = getComputedStyle(element).direction === 'rtl';
    const forward = right !== rtl;
    step(forward ? 1 : -1);
  }
</script>

<svelte:window onkeydown={onKey} />

<div
  bind:this={element}
  role="group"
  aria-label={$_('place.label')}
  class="flex h-8 shrink-0 items-stretch rounded-field border border-primary/50 bg-primary/15 text-base-content"
>
  {#if showArrows}
    <button
      type="button"
      data-tip={previousTip}
      class={[
        'flex w-8 items-center justify-center border-e border-primary/35 text-primary',
        previous ? 'hover:bg-primary/20' : 'cursor-default opacity-35',
      ]}
      aria-label={previousTip}
      aria-disabled={!previous}
      aria-keyshortcuts="ArrowLeft"
      onclick={() => step(-1)}
    >
      <ChevronLeft size={16} class="rtl:rotate-180" aria-hidden="true" />
    </button>
  {/if}
  <button
    type="button"
    class="flex items-center gap-1.5 px-2.5 text-sm font-semibold hover:bg-primary/20"
    data-tip={$_('place.tip')}
    aria-haspopup="dialog"
    aria-keyshortcuts="Control+G"
    aria-label={$_('place.go-to', { values: { place: fullPlace } })}
    onclick={onopenpicker}
  >
    <BookOpen size={15} class="text-primary" aria-hidden="true" />
    <span class="hidden @md:inline">{fullPlace}</span>
    <span class="@md:hidden">{shortPlace}</span>
    <ChevronDown size={14} class="text-primary" aria-hidden="true" />
  </button>
  {#if showArrows}
    <button
      type="button"
      data-tip={nextTip}
      class={[
        'flex w-8 items-center justify-center border-s border-primary/35 text-primary',
        next ? 'hover:bg-primary/20' : 'cursor-default opacity-35',
      ]}
      aria-label={nextTip}
      aria-disabled={!next}
      aria-keyshortcuts="ArrowRight"
      onclick={() => step(1)}
    >
      <ChevronRight size={16} class="rtl:rotate-180" aria-hidden="true" />
    </button>
  {/if}
</div>
<!-- The control's tooltips, below it, kept inside the window. -->
<TipTooltip root={element} placement="bottom" />
