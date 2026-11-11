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
</script>

{#each verse.headings as heading (heading.id)}
  <HeadingText {heading} />
{/each}
<p class="{className} my-1">
  <span class="text-xs text-base-content/60" id={verse.sid}
    >{bibleTextService?.formatRefFromSid(verse.sid)}</span
  >{' '}<VerseText {verse} />
</p>
