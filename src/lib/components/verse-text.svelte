<script lang="ts">
  /* eslint-disable svelte/no-useless-mustaches */
  import type { Verse } from '$lib/model/verse';

  let { verse }: { verse: Verse } = $props();

  const isTextRegex = /^[a-zA-Z]/;
</script>

{#each verse.children as child, index (index)}
  {#if typeof child === 'string'}
    {#if isTextRegex.test(child)}{' '}{/if}{child}
  {:else if typeof child === 'object' && child.style === 'w'}
    {#if isTextRegex.test(child.txt)}{' '}{/if}<span
      class="link no-underline hover:text-primary hover:underline"
      >{child.txt}</span
    >
  {:else if typeof child === 'object' && child.style === 'wj'}
    {#each child.children as wjChild, index (index)}
      {#if typeof wjChild === 'string'}
        {#if isTextRegex.test(wjChild)}{' '}{/if}<span class="text-error"
          >{wjChild}</span
        >
      {:else if typeof wjChild === 'object' && wjChild.style === 'w'}
        {#if isTextRegex.test(wjChild.txt)}{' '}{/if}<span
          class="text-error link no-underline hover:underline"
          >{wjChild.txt}</span
        >
      {/if}
    {/each}
  {/if}
{/each}&nbsp;
