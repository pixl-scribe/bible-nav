<script lang="ts">
  import { Columns2, Search } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import type { AutoCompleteOption } from '$lib/model/auto-complete-option';
  import ModuleSelector from './module-selector.svelte';
  import ModuleMenu, { type MenuSection } from './module-menu.svelte';
  import PlaceControl from './place-control.svelte';
  import ReferencePicker from './reference-picker.svelte';
  import TipTooltip from './tip-tooltip.svelte';

  let {
    bibleTextService,
    parallelOpen = false,
    onToggleParallel,
    parallelMenu,
    showSearch = true,
    selectorLabel,
    onSelectModule,
  }: {
    bibleTextService: BibleTextService | undefined;
    parallelOpen?: boolean;
    onToggleParallel?: () => void;
    /**
     * While the parallel pane has no tabs, the Parallel button drops this
     * menu down (Bibles, then notes) instead of opening an empty pane; the
     * pick opens the pane. Otherwise the button shows or hides the pane.
     */
    parallelMenu?: MenuSection[];
    /** A parallel text has no search of its own: just the selector. */
    showSearch?: boolean;
    /** What the selector chooses for, such as "Bible for this search". */
    selectorLabel: string;
    onSelectModule: (moduleId: string) => void;
  } = $props();

  let inputRef = $state<HTMLInputElement | undefined>(undefined);
  let picker = $state<ReferencePicker | undefined>();
  let searchBar = $state<HTMLElement | undefined>();
  let parallelMenuOpen = $state(false);
  const offersMenu = $derived(!parallelOpen && !!parallelMenu);

  function onParallelClick(event: MouseEvent) {
    if (offersMenu) {
      // Kept from reaching the window, where it would close the menu.
      event.stopPropagation();
      parallelMenuOpen = !parallelMenuOpen;
    } else {
      parallelMenuOpen = false;
      onToggleParallel?.();
    }
  }

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
  <div class="flex w-full p-2 relative" bind:this={searchBar}>
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
        data-tip={offersMenu
          ? $_('parallel.tip-open')
          : parallelOpen
            ? $_('parallel.tip-hide')
            : $_('parallel.tip-show')}
        aria-pressed={offersMenu ? undefined : parallelOpen}
        aria-haspopup={offersMenu ? 'menu' : undefined}
        aria-expanded={offersMenu ? parallelMenuOpen : undefined}
        onclick={onParallelClick}
      >
        <Columns2 size={18} />
        {$_('bible-text.parallel')}
      </button>
      {#if parallelMenuOpen && parallelMenu}
        <ModuleMenu
          sections={parallelMenu}
          label={$_('parallel.add')}
          class="top-13 right-2"
          onclose={() => (parallelMenuOpen = false)}
        />
      {/if}
    {/if}
    {#if bibleTextService && bibleTextService.autoCompleteOptions.length > 0}
      <!-- Books first, then words; each says what it opens. -->
      <ul
        class="menu menu-sm bg-base-100 rounded-box w-72 absolute top-12 left-2 z-40 border border-base-content/20 shadow-md"
      >
        {#each bibleTextService.autoCompleteOptions as option, i (i)}
          <li
            class={[
              'rounded-field',
              option.selected && 'bg-primary text-primary-content',
            ]}
          >
            <button
              class="flex items-baseline gap-2"
              onclick={() => onClickAutoComplete(option)}
            >
              <span class="w-10 shrink-0 text-xs opacity-70">
                {option.type === 'ref'
                  ? $_('autocomplete.book')
                  : $_('autocomplete.word')}
              </span>
              <span class="flex-1 font-semibold">{option.value}</span>
              {#if option.count !== undefined}
                <span class="text-xs opacity-70">
                  {option.type === 'ref'
                    ? $_('autocomplete.chapters', {
                        values: { count: option.count },
                      })
                    : $_('autocomplete.verses', {
                        values: { count: option.count },
                      })}
                </span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <!-- The Parallel button's tooltip. -->
  <TipTooltip root={searchBar} placement="bottom" />
{/if}
<div
  class={[
    '@container flex items-center justify-between gap-2 text-xs text-base-content/60 w-full px-2 pb-2 border-b border-base-content/10',
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
  {#if showSearch && bibleTextService}
    <!-- The main text's place, with the chapter arrows; the place (or
         Ctrl+G) opens the reference picker. -->
    <PlaceControl
      service={bibleTextService}
      onopenpicker={() => picker?.open()}
    />
  {:else}
    <!-- A parallel text follows the main text: its place is shown only. -->
    <div class="shrink-0">{bibleTextService?.referenceLabel}</div>
  {/if}
</div>
{#if showSearch && bibleTextService}
  <!-- Outside the row above: a dialog still inherits text styles from where
       it sits in the page, and that row's text is small and dimmed. -->
  <ReferencePicker bind:this={picker} service={bibleTextService} />
{/if}
