<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import InlineContent from './inline-content.svelte';

  let { verse }: { verse: Verse } = $props();
</script>

<InlineContent items={verse.children} sid={verse.sid} />
