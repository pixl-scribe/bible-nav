<script lang="ts">
  /**
   * A footnote marker showing its letter (a, b, ... restarting each
   * chapter). Hovering or focusing it asks the pane's shared tooltip
   * (footnote-tooltip.svelte) to show the note. Clicking opens the note in a
   * popover for now; the parallel pane's Notes tab replaces that.
   */
  import type { Note } from '$lib/model/verse';
  import { noteText } from '$lib/services/verse-content';
  import { requestFootnoteTooltip } from '$lib/services/footnote-tooltip';

  let { note }: { note: Note } = $props();

  const id = $props.id();
  const text = $derived(noteText(note));
  const label = $derived(`Footnote ${note.letter ?? ''}: ${text}`);

  // The button's ::before is an invisible layer around the letter, so the
  // marker is easier to hit with a mouse or finger without changing the
  // letter's size or moving the text around it. It's 8px wider on each side
  // and a fixed 28px tall, centered on the letter: the button is inline in a
  // small superscript, so a layer sized from the button's own height would
  // stay short.
  const show = (event: Event) =>
    requestFootnoteTooltip(event.currentTarget as HTMLElement, text);
  const hide = (event: Event) =>
    requestFootnoteTooltip(event.currentTarget as HTMLElement, null);
</script>

<sup class="footnote" data-footnote
  ><button
    type="button"
    popovertarget={id}
    class="relative cursor-pointer rounded px-0.5 text-primary hover:bg-primary/15 before:absolute before:-inset-x-2 before:top-1/2 before:h-7 before:-translate-y-1/2 before:content-['']"
    aria-label={label}
    onmouseenter={show}
    onmouseleave={hide}
    onfocus={show}
    onblur={hide}>{note.letter ?? '*'}</button
  ><span
    {id}
    popover
    class="footnote-note m-auto max-w-sm rounded-box border border-base-content/20 bg-base-200 p-4 text-sm leading-normal text-base-content shadow-lg"
    >{text}</span
  ></sup
>
