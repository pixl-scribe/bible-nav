<script lang="ts">
  /**
   * Text between verses: a Psalm title (d), or a section heading or closing
   * note (s1), such as Psalm 119's "א ALEPH." or an epistle's subscription.
   */
  import type { Heading } from '$lib/model/verse';
  import InlineContent from './inline-content.svelte';

  let { heading }: { heading: Heading } = $props();
</script>

{#if heading.style === 'd'}
  <p class="my-2 italic text-base-content/70" dir="auto">
    <InlineContent items={heading.children} />
  </p>
{:else}
  <p
    class="my-3 text-center text-sm font-semibold text-base-content/80"
    dir="auto"
  >
    <InlineContent items={heading.children} />
  </p>
{/if}
