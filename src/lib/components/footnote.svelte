<script lang="ts">
  /**
   * A footnote marker showing its letter (a, b, ... restarting each
   * chapter). Hovering or focusing it asks the pane's shared tooltip
   * (footnote-tooltip.svelte) to show the note. Clicking it opens the note in
   * the parallel pane's Notes tab, by sending an event up to its search
   * (search-view.svelte).
   */
  import type { Note } from '$lib/model/verse';
  import { noteText } from '$lib/services/verse-content';
  import {
    requestFootnoteOpen,
    requestFootnoteTooltip,
  } from '$lib/services/footnote-events';

  let { note, sid }: { note: Note; sid: string | undefined } = $props();

  const text = $derived(noteText(note));
  const label = $derived(`Footnote ${note.letter ?? ''}: ${text}`);

  // The marker is an inline span acting as a button, not a <button>: a
  // button is laid out as an inline block, which lets the line break just
  // before it, so a letter could wrap onto a line of its own.
  //
  // Its ::before is an invisible layer around the letter, so the
  // marker is easier to hit with a mouse or finger without changing the
  // letter's size or moving the text around it. It's 8px wider on each side
  // and a fixed 28px tall, centered on the letter: the button is inline in a
  // small superscript, so a layer sized from the button's own height would
  // stay short.
  const show = (event: Event) =>
    requestFootnoteTooltip(event.currentTarget as HTMLElement, text);
  const hide = (event: Event) =>
    requestFootnoteTooltip(event.currentTarget as HTMLElement, null);
  const open = (event: Event) => {
    if (sid && note.letter) {
      requestFootnoteOpen(event.currentTarget as HTMLElement, sid, note.letter);
    }
  };
  const openByKey = (event: KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      open(event);
    }
  };
</script>

<sup class="footnote" data-footnote
  ><span
    role="button"
    tabindex="0"
    class="relative cursor-pointer rounded px-0.5 text-primary hover:bg-primary/15 before:absolute before:-inset-x-2 before:top-1/2 before:h-7 before:-translate-y-1/2 before:content-['']"
    aria-label={label}
    onmouseenter={show}
    onmouseleave={hide}
    onfocus={show}
    onblur={hide}
    onclick={open}
    onkeydown={openByKey}>{note.letter ?? '*'}</span
  ></sup
>
