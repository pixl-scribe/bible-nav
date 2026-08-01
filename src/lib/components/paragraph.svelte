<script lang="ts">
  /* eslint-disable svelte/no-useless-mustaches */
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

  const isTextRegex = /^[a-zA-Z]/;

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
    {#each verse.children as child, index (index)}
      {#if typeof child === 'string'}
        {#if isTextRegex.test(child)}{' '}{/if}{child}
      {:else if typeof child === 'object' && child.style === 'w'}
        {#if isTextRegex.test(child.txt)}{' '}{/if}<span>{child.txt}</span>
      {:else if typeof child === 'object' && child.style === 'wj'}
        {#each child.children as wjChild, index (index)}
          {#if typeof wjChild === 'string'}
            {#if isTextRegex.test(wjChild)}{' '}{/if}<span class="text-error"
              >{wjChild}</span
            >
          {:else if typeof wjChild === 'object' && wjChild.style === 'w'}
            {#if isTextRegex.test(wjChild.txt)}{' '}{/if}<span
              class="text-error">{wjChild.txt}</span
            >
          {/if}
        {/each}
      {/if}
    {/each}&nbsp;
  {/each}
</p>
