<script lang="ts">
  /**
   * Draws verse or heading content (module format version 1). Pieces are
   * joined exactly as stored: the source's spacing is kept in its strings,
   * so no spaces are added here. The template has no whitespace between
   * elements: line breaks fall inside tags (the `>` that starts a line), as
   * Prettier writes them. verse-rendering.test.ts checks the drawn text
   * equals `raw`. It calls itself for nested content.
   * `sid` is the verse the content belongs to, which footnotes report when
   * clicked; headings have no footnotes and leave it out. `marks`, for a
   * word search, is a pattern for the matched words, which are wrapped in
   * <mark> without changing the text. A word with a Strong's number keeps
   * it in `data-strong`: its search handles every click and hover on such
   * words at once (search-view.svelte), so no word needs listeners of its
   * own.
   */
  import type { VerseChild } from '$lib/model/verse';
  import InlineContent from './inline-content.svelte';
  import Footnote from './footnote.svelte';
  import { splitMatches } from '$lib/services/highlight';

  let {
    items,
    sid,
    marks,
  }: { items: VerseChild[]; sid?: string; marks?: RegExp } = $props();
</script>

{#snippet text(
  value: string
)}{#if marks}{#each splitMatches(value, marks) as part, index (index)}{#if part.hit}<mark
          class="rounded-sm bg-primary/30 text-inherit">{part.text}</mark
        >{:else}{part.text}{/if}{/each}{:else}{value}{/if}{/snippet}

{#each items as item, index (index)}{#if typeof item === 'string'}{@render text(
      item
    )}{:else if item.style === 'w'}<span
      class="word"
      data-strong={item.strong || undefined}>{@render text(item.txt)}</span
    >{:else if item.style === 'f'}<Footnote
      note={item}
      {sid}
    />{:else if item.style === 'add'}<em class="add"
      ><InlineContent items={item.children} {sid} {marks} /></em
    >{:else if item.style === 'wj'}<span class="wj text-error"
      ><InlineContent items={item.children} {sid} {marks} /></span
    >{:else}<span class={item.style}
      ><InlineContent items={item.children} {sid} {marks} /></span
    >{/if}{/each}
