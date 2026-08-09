<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import type ModuleService from '$lib/services/module-service.svelte';
  import VerseText from '$lib/components/verse-text.svelte';

  let {
    verses,
    class: className = '',
    moduleService,
  }: {
    verses: Verse[];
    class?: string;
    moduleService: ModuleService | undefined;
  } = $props();
</script>

<p class="{className} my-2">
  {#each verses as verse, i (verse.sid)}
    {#if i === 0}
      <span class="text-xs text-base-content/60" id={verse.sid}
        >{moduleService?.formatRefFromSid(verse.sid)}</span
      >
    {:else}
      <span class="text-xs text-base-content/60" id={verse.sid}
        >{verse.nbr}</span
      >
    {/if}
    <VerseText {verse} />
  {/each}
</p>
