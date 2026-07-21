<script lang="ts">
  import { Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService from '$lib/services/module-service.svelte';
  import type {GlossaryWord} from "$lib/model/verse";

  let { moduleId }: { moduleId: string } = $props();

  let moduleService = $state<ModuleService | undefined>();

  onMount(async () => {
    moduleService = new ModuleService(moduleId);
  });
</script>

<div
  class="flex flex-col bg-base-200 rounded-box rounded-xl w-full flex-1 max-w-140 p-2 ml-1 h-full items-start"
>
  <div class="flex w-full">
    <label class="input w-full">
      <Search />
      <input type="search" required placeholder={$_('bible-text.search')} />
    </label>
    <div class="divider divider-horizontal mx-1"></div>
    <button
      class="btn btn-soft btn-primary px-1 tooltip tooltip-bottom"
      data-tip={$_('bible-text.add-parallel')}
    >
      <Plus size={16} />
      <Columns2 />
    </button>
  </div>
  <div class="flex flex-col w-full overflow-y-auto">
    {moduleId}
    {moduleService?.verses.length}
    {#each moduleService?.verses as verse (verse.sid)}
      <div class="flex flex-wrap">
        <span>{verse.sid}&nbsp;</span>
        {#each verse.children as child, index (index)}
          {#if typeof child === 'string'}
            <span class="flex">{child}&nbsp;</span>
          {:else if typeof child === 'object' && child.style === 'w'}
            <span class="flex">{child.txt}&nbsp;</span>
          {/if}
        {/each}
      </div>
    {/each}
  </div>
</div>
