<script lang="ts">
  import { Columns2, Search } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import type { AutoCompleteOption } from '$lib/model/auto-complete-option';
  import ModuleSelector from './module-selector.svelte';

  let {
    bibleTextService,
    parallelOpen = false,
    onToggleParallel,
    showSearch = true,
    selectorLabel,
    onSelectModule,
  }: {
    bibleTextService: BibleTextService | undefined;
    parallelOpen?: boolean;
    onToggleParallel?: () => void;
    /** A parallel text has no search of its own: just the selector. */
    showSearch?: boolean;
    /** What the selector chooses for, such as "Bible for this search". */
    selectorLabel: string;
    onSelectModule: (moduleId: string) => void;
  } = $props();

  let inputRef = $state<HTMLInputElement | undefined>(undefined);

  let searchInput = $state({
    get value() {
      return bibleTextService?.searchInput ?? '';
    },
    set value(val: string) {
      if (bibleTextService) {
        bibleTextService.searchInput = val;
      }
    },
  });

  function handleSearchKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'ArrowUp':
        event.preventDefault();
        bibleTextService?.autoCompleteUp();
        break;
      case 'ArrowDown':
        event.preventDefault();
        bibleTextService?.autoCompleteDown();
        break;
      case 'Enter':
        event.preventDefault();
        bibleTextService?.autoCompletePick();
        break;
    }
  }

  function onClickAutoComplete(option: AutoCompleteOption) {
    bibleTextService?.autoCompletePick(option);
    inputRef?.focus();
  }
</script>

{#if showSearch}
  <div class="flex w-full p-2 relative">
    <label class="input w-full">
      <Search size={18} />
      <input
        type="search"
        required
        placeholder={$_('bible-text.search')}
        bind:this={inputRef}
        bind:value={searchInput.value}
        onkeydown={handleSearchKeydown}
      />
    </label>
    {#if onToggleParallel}
      <button
        type="button"
        class={['btn btn-soft btn-primary ml-2', parallelOpen && 'btn-active']}
        aria-pressed={parallelOpen}
        onclick={onToggleParallel}
      >
        <Columns2 size={18} />
        {$_('bible-text.parallel')}
      </button>
    {/if}
    {#if bibleTextService && bibleTextService.autoCompleteOptions.length > 0}
      <ul
        class="menu menu-sm bg-base-100 rounded-box w-56 absolute top-12 left-2 z-40 border border-base-content/20 shadow-md"
      >
        {#each bibleTextService.autoCompleteOptions as option, i (i)}
          <li class={{ 'bg-primary': option.selected }}>
            <button onclick={() => onClickAutoComplete(option)}>
              {#if option.type === 'ref'}
                <span class="text-base-content/60">{$_('bible-text.book')}</span
                >
              {/if}
              {option.value}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}
<div
  class={[
    'flex items-center justify-between gap-2 text-xs text-base-content/60 w-full px-2 pb-2 border-b border-base-content/10',
    !showSearch && 'pt-2',
  ]}
>
  {#if bibleTextService}
    <ModuleSelector
      moduleId={bibleTextService.moduleId}
      name={bibleTextService.moduleName}
      abbreviation={bibleTextService.moduleAbbreviation}
      version={bibleTextService.moduleVersion}
      label={selectorLabel}
      onselect={onSelectModule}
    />
  {/if}
  <div>{bibleTextService?.referenceLabel}</div>
</div>
