<script lang="ts">
  /**
   * A search's parallel pane: box tabs separated by short vertical rules (no
   * tray; the selected tab is a filled box with bold text), a + menu to add
   * tabs, and the selected tab's content. Today the only tab is the text's
   * Notes tab; parallel texts come later.
   */
  import { Plus, X } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import {
    notesTabKey,
    type ParallelPane,
    type ParallelTab,
  } from '$lib/services/parallel-pane.svelte';
  import {
    appSettingsService,
    textMaxWidths,
  } from '$lib/services/app-settings-service.svelte';
  import NotesPanel from './notes-panel.svelte';

  let {
    pane,
    service,
  }: {
    pane: ParallelPane;
    /** The main pane's text. */
    service: BibleTextService | undefined;
  } = $props();

  let menuOpen = $state(false);
  const textMaxWidth = $derived(textMaxWidths[appSettingsService.textWidth]);

  const notesTitle = $derived(
    $_('notes.tab', { values: { abbr: service?.moduleAbbreviation ?? '' } })
  );

  function title(tab: ParallelTab): string {
    return tab.kind === 'notes' ? notesTitle : '';
  }

  /** Tabs that can be added: the text's Notes tab, if it isn't open. */
  const addOptions = $derived.by(() => {
    if (!service) return [];
    const key = notesTabKey(service.moduleId);
    if (pane.tabs.some((tab) => tab.key === key)) return [];
    return [
      {
        title: notesTitle,
        hint: $_('notes.hint'),
        add: () => {
          const sid = service?.sidInView;
          if (service && sid) pane.openNotes(service.moduleId, sid);
        },
      },
    ];
  });

  function onMenuKey(event: KeyboardEvent) {
    if (event.key === 'Escape') menuOpen = false;
  }
</script>

<section
  aria-label={$_('parallel.label')}
  class="flex min-w-0 flex-1 basis-0 flex-col rounded-box bg-base-200"
>
  <div
    class="relative flex items-center gap-1.5 border-b border-base-content/10 p-2"
  >
    <div
      role="tablist"
      aria-label={$_('parallel.tabs')}
      class="flex flex-wrap items-center gap-0.5"
    >
      {#each pane.tabs as tab (tab.key)}
        {@const selected = tab.key === pane.activeKey}
        <div class={['ptab', selected && 'on']}>
          <button
            type="button"
            role="tab"
            aria-selected={selected}
            class="ptab-label"
            onclick={() => pane.select(tab.key)}>{title(tab)}</button
          >
          <button
            type="button"
            class="btn btn-ghost btn-xs btn-square"
            aria-label={$_('parallel.close-tab', {
              values: { title: title(tab) },
            })}
            onclick={() => pane.close(tab.key)}
          >
            <X size={14} />
          </button>
        </div>
      {/each}
    </div>
    <button
      type="button"
      class="btn btn-soft btn-primary btn-sm btn-square"
      aria-label={$_('parallel.add')}
      aria-expanded={menuOpen}
      onclick={() => (menuOpen = !menuOpen)}
      onkeydown={onMenuKey}
    >
      <Plus size={16} />
    </button>
    {#if menuOpen}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="absolute top-11 left-2 z-20 flex min-w-56 flex-col gap-0.5 rounded-box border border-base-content/15 bg-base-100 p-1.5 shadow-lg"
        onkeydown={onMenuKey}
      >
        {#each addOptions as option (option.title)}
          <button
            type="button"
            class="btn btn-ghost h-auto justify-start py-2 text-left font-normal"
            onclick={() => {
              option.add();
              menuOpen = false;
            }}
          >
            <span>
              {option.title}
              <span class="block text-xs text-base-content/60"
                >{option.hint}</span
              >
            </span>
          </button>
        {:else}
          <p class="m-2 text-sm text-base-content/60">
            {$_('parallel.all-open')}
          </p>
        {/each}
      </div>
    {/if}
  </div>

  {#if pane.active?.kind === 'notes' && service}
    <NotesPanel
      tab={pane.active}
      {service}
      {textMaxWidth}
      onFocusNote={(letter) => {
        if (pane.activeKey) pane.focusNote(pane.activeKey, letter);
      }}
    />
  {:else}
    <div
      class="flex flex-1 items-center justify-center p-6 text-center text-sm text-base-content/60"
    >
      {$_('parallel.empty')}
    </div>
  {/if}
</section>

<style>
  /* Box tabs separated by short vertical rules, as in Brave's tab strip. The
     rules beside the selected or hovered tab are hidden. */
  .ptab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 1.875rem;
    padding: 0 0.25rem 0 0.75rem;
    margin: 0 3px;
    border-radius: 0.45rem;
    color: color-mix(in oklab, var(--color-base-content) 65%, transparent);
    font-size: 0.8125rem;
  }
  .ptab:hover {
    background: color-mix(in oklab, var(--color-base-content) 7%, transparent);
  }
  .ptab.on {
    background: color-mix(in oklab, var(--color-primary) 25%, transparent);
    color: var(--color-base-content);
    font-weight: 600;
  }
  .ptab + .ptab::before,
  .ptab:last-child::after {
    content: '';
    position: absolute;
    top: 8px;
    bottom: 8px;
    width: 1px;
    background: color-mix(in oklab, var(--color-base-content) 25%, transparent);
  }
  .ptab + .ptab::before {
    left: -4px;
  }
  .ptab:last-child::after {
    right: -4px;
  }
  .ptab.on::before,
  .ptab.on + .ptab::before,
  .ptab.on:last-child::after,
  .ptab:hover::before,
  .ptab:hover + .ptab::before,
  .ptab:hover:last-child::after {
    display: none;
  }
  .ptab-label {
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
</style>
