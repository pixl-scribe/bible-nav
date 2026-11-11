<script lang="ts">
  // The {' '} below are deliberate: they put spaces between verse numbers
  // and verses, which the template would otherwise not keep.
  /* eslint-disable svelte/no-useless-mustaches */
  /**
   * One paragraph, laid out by its style (see the module format contract):
   * p flows as prose, q1 is an indented poetic line whose wrapped lines hang
   * further in, b is a blank line between stanzas, and d / s1 hold a heading.
   */
  import type { Paragraph } from '$lib/model/verse';
  import type BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import VerseText from '$lib/components/verse-text.svelte';
  import HeadingText from '$lib/components/heading-text.svelte';

  let {
    paragraph,
    class: className = '',
    bibleTextService,
  }: {
    paragraph: Paragraph;
    class?: string;
    bibleTextService: BibleTextService | undefined;
  } = $props();

  const poetry = $derived(paragraph.style === 'q1');

  /**
   * The first verse of a prose paragraph shows its full reference. Each
   * poetic line holds one verse, so poetry shows the full reference only at
   * the start of a chapter, and verse numbers otherwise.
   */
  function showFullRef(index: number, verseNumber: number): boolean {
    return index === 0 && (!poetry || verseNumber === 1);
  }
</script>

{#if paragraph.heading}
  <HeadingText heading={paragraph.heading} />
{:else if paragraph.style === 'b'}
  <div class="h-3" aria-hidden="true"></div>
{:else}
  <p class="{className} {poetry ? 'my-0.5 pl-10 -indent-6' : 'my-2'}">
    {#each paragraph.verses as verse, i (verse.sid)}
      <span
        class={[
          'rounded-sm box-decoration-clone',
          bibleTextService?.highlightSid === verse.sid && 'pair-highlight',
        ]}
        ><span class="text-xs text-base-content/60" id={verse.sid}
          >{showFullRef(i, verse.nbr)
            ? bibleTextService?.formatRefFromSid(verse.sid)
            : verse.nbr}</span
        >{' '}<VerseText {verse} /></span
      >{' '}
    {/each}
  </p>
{/if}
