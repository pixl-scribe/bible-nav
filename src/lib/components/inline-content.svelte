<script lang="ts">
  /**
   * Draws verse or heading content (module format version 1). Pieces are
   * joined exactly as stored: the source's spacing is kept in its strings,
   * so no spaces are added here. Each branch below stays on one line so the
   * template itself adds no whitespace. It calls itself for nested content.
   * `sid` is the verse the content belongs to, which footnotes report when
   * clicked; headings have no footnotes and leave it out.
   */
  import type { VerseChild } from '$lib/model/verse';
  import InlineContent from './inline-content.svelte';
  import Footnote from './footnote.svelte';

  let { items, sid }: { items: VerseChild[]; sid?: string } = $props();
</script>

<!-- prettier-ignore -->
{#each items as item, index (index)}{#if typeof item === 'string'}{item}{:else if item.style === 'w'}<span class="word">{item.txt}</span>{:else if item.style === 'f'}<Footnote note={item} {sid} />{:else if item.style === 'add'}<em class="add"><InlineContent items={item.children} {sid} /></em>{:else if item.style === 'wj'}<span class="wj text-error"><InlineContent items={item.children} {sid} /></span>{:else}<span class={item.style}><InlineContent items={item.children} {sid} /></span>{/if}{/each}
