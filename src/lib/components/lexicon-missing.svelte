<script lang="ts">
  /**
   * Shown beside a word with a Strong's number when no lexicon for its
   * language is installed (decided 2026-11-17): it offers Strong's
   * dictionary, opening the Modules dialog on Download. Escape, "Not now",
   * or a press elsewhere closes it.
   */
  import {
    autoUpdate,
    computePosition,
    flip,
    offset,
    shift,
  } from '@floating-ui/dom';
  import { _ } from 'svelte-i18n';
  import { modulesDialogService } from '$lib/services/modules-dialog-service.svelte';

  let {
    anchor,
    word,
    strong,
    onclose,
  }: {
    /** The word that was clicked. */
    anchor: HTMLElement;
    word: string;
    /** Its Strong's number, in normal form. */
    strong: string;
    onclose: () => void;
  } = $props();

  let panel = $state<HTMLElement>();
  let first = $state<HTMLButtonElement>();
  const hebrew = $derived(strong.startsWith('H'));

  $effect(() => {
    if (!panel) return;
    const box = panel;
    const place = async () => {
      const placed = await computePosition(anchor, box, {
        strategy: 'fixed',
        placement: 'bottom-start',
        middleware: [offset(8), flip({ padding: 8 }), shift({ padding: 8 })],
      });
      box.style.left = `${placed.x}px`;
      box.style.top = `${placed.y}px`;
    };
    return autoUpdate(anchor, box, place);
  });

  $effect(() => {
    first?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onclose();
    };
    const onPress = (event: PointerEvent) => {
      if (panel && !panel.contains(event.target as Node)) onclose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onPress, true);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onPress, true);
    };
  });

  function get() {
    onclose();
    modulesDialogService.open('download');
  }
</script>

<div
  bind:this={panel}
  role="dialog"
  aria-label={$_(
    hebrew ? 'lexicon.missing.title-hebrew' : 'lexicon.missing.title-greek'
  )}
  class="fixed top-0 left-0 z-50 flex w-96 max-w-[calc(100vw-2rem)] flex-col gap-2.5 rounded-box border border-base-content/15 bg-base-100 p-4 shadow-xl"
>
  <div class="flex items-baseline gap-2">
    <span class="text-base font-bold">{word}</span>
    <span class="text-sm text-base-content/60"
      >{$_('lexicon.missing.number', { values: { number: strong } })}</span
    >
  </div>
  <p class="m-0 text-sm leading-normal text-base-content/80">
    {$_(hebrew ? 'lexicon.missing.hebrew' : 'lexicon.missing.greek')}
  </p>
  <div class="flex justify-end gap-2 pt-1">
    <button
      bind:this={first}
      type="button"
      class="btn btn-sm btn-outline btn-primary"
      onclick={onclose}>{$_('lexicon.missing.not-now')}</button
    >
    <button type="button" class="btn btn-sm btn-primary" onclick={get}
      >{$_(
        hebrew ? 'lexicon.missing.get-hebrew' : 'lexicon.missing.get-greek'
      )}</button
    >
  </div>
</div>
