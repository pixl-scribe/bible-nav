<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import InlineContent from './inline-content.svelte';
  import { matchPattern } from '$lib/services/highlight';

  let { verse }: { verse: Verse } = $props();

  // A word search's matched words, highlighted where they occur.
  const marks = $derived(matchPattern(verse.matches));
</script>

<InlineContent items={verse.children} sid={verse.sid} {marks} />
