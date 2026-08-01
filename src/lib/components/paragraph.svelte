<script lang="ts">
  import type { Verse } from '$lib/model/verse';
  let { verses, class: className = '' }: { verses: Verse[]; class?: string } =
    $props();

  const isTextRegex = /^[a-zA-Z¶]/;
</script>

<div class="flex flex-wrap items-center {className}">
  {#each verses as verse (verse.sid)}
    <div class="flex text-xs text-base-content/60">{verse.sid}</div>
    {#each verse.children as child, index (index)}
      {#if typeof child === 'string'}
        {#if isTextRegex.test(child)}
          &nbsp;
        {/if}
        <div class="flex">{child}</div>
      {:else if typeof child === 'object' && child.style === 'w'}
        {#if isTextRegex.test(child.txt)}
          &nbsp;
        {/if}
        <span class="flex">{child.txt}</span>
      {:else if typeof child === 'object' && child.style === 'wj'}
        <!-- TODO: Work on this more. -->
      {/if}
    {/each}
  {/each}
</div>
