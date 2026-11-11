<script lang="ts">
  // The {' '} below are deliberate: they put spaces between verse numbers
  // and verses, which the template would otherwise not keep.
  /* eslint-disable svelte/no-useless-mustaches */
  import type { Verse } from '$lib/model/verse';
  import type BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import VerseText from './verse-text.svelte';
  import HeadingText from './heading-text.svelte';

  let {
    verse,
    class: className = '',
    bibleTextService,
  }: {
    verse: Verse;
    class?: string;
    bibleTextService: BibleTextService | undefined;
  } = $props();

  // Grouping by verse puts a reference before every verse, so it uses the
  // book's short name (toc3, 'Gen 1:8'). Reference searches show verses with
  // this component in paragraph grouping too; they keep the full name.
  const label = $derived(
    bibleTextService?.groupingMode === 'verse'
      ? bibleTextService.formatAbbreviatedRefFromSid(verse.sid)
      : bibleTextService?.formatRefFromSid(verse.sid)
  );
</script>

<!-- The verse's id is on this wrapper, so going to a verse shows the
     headings before it, such as a Psalm title, instead of scrolling past. -->
<div id={verse.sid}>
  {#each verse.headingsBefore as heading (heading.id)}
    <HeadingText {heading} />
  {/each}
  <p
    class={[
      className,
      'my-1 rounded-sm',
      bibleTextService?.highlightSid === verse.sid && 'pair-highlight',
    ]}
  >
    <span class="text-xs text-base-content/60">{label}</span>{' '}<VerseText
      {verse}
    />
  </p>
  {#each verse.headingsAfter as heading (heading.id)}
    <HeadingText {heading} />
  {/each}
</div>
