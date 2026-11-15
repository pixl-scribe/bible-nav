<script lang="ts">
  /**
   * Draws verse or heading content (module format version 1). Pieces are
   * joined exactly as stored: the source's spacing is kept in its strings,
   * so no spaces are added here. Each branch below stays on one line so the
   * template itself adds no whitespace. It calls itself for nested content.
   * `sid` is the verse the content belongs to, which footnotes report when
   * clicked; headings have no footnotes and leave it out. `marks`, for a
   * word search, is a pattern for the matched words, which are wrapped in
   * <mark> without changing the text.
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

<!-- prettier-ignore -->
{#snippet text(value: string)}{#if marks}{#each splitMatches(value, marks) as part, index (index)}{#if part.hit}<mark class="rounded-sm bg-primary/30 text-inherit">{part.text}</mark>{:else}{part.text}{/if}{/each}{:else}{value}{/if}{/snippet}

<!-- prettier-ignore -->
{#each items as item, index (index)}{#if typeof item === 'string'}{@render text(item)}{:else if item.style === 'w'}<span class="word">{@render text(item.txt)}</span>{:else if item.style === 'f'}<Footnote note={item} {sid} />{:else if item.style === 'add'}<em class="add"><InlineContent items={item.children} {sid} {marks} /></em>{:else if item.style === 'wj'}<span class="wj text-error"><InlineContent items={item.children} {sid} {marks} /></span>{:else}<span class={item.style}><InlineContent items={item.children} {sid} {marks} /></span>{/if}{/each}
