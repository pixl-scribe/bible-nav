<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  import type ModuleService from '$lib/services/module-service.svelte';

  let {
    verses,
    class: className = '',
    moduleService,
  }: {
    verses: Verse[];
    class?: string;
    moduleService: ModuleService | undefined;
  } = $props();

  const isTextRegex = /^[a-zA-Z¶]/;

  $effect(() => {
    if (moduleService && moduleService?.scrollToSid) {
      const el = document.getElementById(moduleService.scrollToSid);
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
      moduleService.scrollToSid = undefined;
    }
  });
</script>

<div class="flex flex-wrap items-center {className} my-2">
  {#each verses as verse, i (verse.sid)}
    {#if i === 0}
      <span class="flex text-xs text-base-content/60" id={verse.sid}
        >{moduleService?.formatRefFromSid(verse.sid)}</span
      >
    {:else}
      <span class="flex text-xs text-base-content/60" id={verse.sid}
        >{verse.nbr}</span
      >
    {/if}
    {#each verse.children as child, index (index)}
      {#if typeof child === 'string'}
        {#if isTextRegex.test(child)}
          &nbsp;
        {/if}
        <span class="flex flex-wrap min-w-0 wrap-anywhere">{child}</span>
      {:else if typeof child === 'object' && child.style === 'w'}
        {#if isTextRegex.test(child.txt)}
          &nbsp;
        {/if}
        <span class="flex flex-wrap">{child.txt}</span>
      {:else if typeof child === 'object' && child.style === 'wj'}
        <!-- TODO: Work on this more. -->
      {/if}
    {/each}&nbsp;
  {/each}
</div>
