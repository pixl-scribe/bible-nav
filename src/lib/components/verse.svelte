<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import type ModuleService from '$lib/services/module-service.svelte';
  import VerseText from './verse-text.svelte';

  let {
    verse,
    class: className = '',
    moduleService,
  }: {
    verse: Verse;
    class?: string;
    moduleService: ModuleService | undefined;
  } = $props();
</script>

<p class="{className} my-1">
  <span class="text-xs text-base-content/60" id={verse.sid}
    >{moduleService?.formatRefFromSid(verse.sid)}</span
  >
  <VerseText {verse} />
</p>
