<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import type BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import VerseText from './verse-text.svelte';

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

<p class="{className} my-1">
  <span class="text-xs text-base-content/60" id={verse.sid}
    >{bibleTextService?.formatRefFromSid(verse.sid)}</span
  >
  <VerseText {verse} />
</p>
