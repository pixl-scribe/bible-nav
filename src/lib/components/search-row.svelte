<script lang="ts">
  import { Columns2, Plus, Search } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import ModuleService from '$lib/services/module-service.svelte';

  let { moduleService }: { moduleService: ModuleService | undefined } =
    $props();

  let searchInput = $state({
    get value() {
      return moduleService?.searchInput ?? '';
    },
    set value(val: string) {
      if (moduleService) {
        moduleService.searchInput = val;
      }
    },
  });

  function handleSearchKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'ArrowUp':
        event.preventDefault();
        moduleService?.autoCompleteUp();
        break;
      case 'ArrowDown':
        event.preventDefault();
        moduleService?.autoCompleteDown();
        break;
      case 'Enter':
        event.preventDefault();
        moduleService?.autoCompletePick();
        break;
    }
  }
</script>

<div class="flex w-full p-2 relative">
  <label class="input w-full">
    <Search />
    <input
      type="search"
      required
      placeholder={$_('bible-text.search')}
      bind:value={searchInput.value}
      onkeydown={handleSearchKeydown}
    />
  </label>
  <button
    class="btn btn-soft btn-primary px-1 ml-2 tooltip tooltip-bottom"
    data-tip={$_('bible-text.add-parallel')}
  >
    <Plus size={16} />
    <Columns2 />
  </button>
  {#if moduleService && moduleService.autoCompleteOptions.length > 0}
    <ul
      class="menu menu-sm bg-base-100 rounded-box w-56 absolute top-12 left-2 border border-base-content/20 shadow-md"
    >
      {#each moduleService.autoCompleteOptions as option, i (i)}
        <li class={{ 'bg-primary': option.selected }}>
          <button onclick={() => moduleService?.autoCompletePick(option)}>
            {#if option.type === 'ref'}
              <span class="text-base-content/60">{$_('bible-text.book')}</span>
            {/if}
            {option.value}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
<div
  class="flex justify-between text-xs text-base-content/60 w-full px-2 pb-2 border-b border-base-content/10"
>
  <div>{moduleService?.moduleName}</div>
  <div>{moduleService?.referenceLabel}</div>
</div>
