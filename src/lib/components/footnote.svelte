<script lang="ts">
  /**
   * A footnote marker. Clicking, tapping, or pressing Enter on it opens the
   * note in a popover, which the browser shows above everything else, so it
   * can't be cut off by the reading column. Clicking elsewhere closes it.
   * While closed it takes no space, so it can't widen the column either.
   * Hovering shows the note as a plain tooltip.
   */
  import type { Note } from '$lib/model/verse';
  import { noteText } from '$lib/services/verse-content';

  let { note }: { note: Note } = $props();

  const id = $props.id();
  const text = $derived(noteText(note));
</script>

<!-- prettier-ignore -->
<sup class="footnote" data-footnote><button type="button" popovertarget={id} class="cursor-help px-0.5 text-primary" title={text} aria-label={text}>*</button><span {id} popover class="footnote-note m-auto max-w-sm rounded-box border border-base-content/20 bg-base-200 p-4 text-sm leading-normal text-base-content shadow-lg">{text}</span></sup>
