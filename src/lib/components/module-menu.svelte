<script lang="ts" module>
  export interface MenuItem {
    key: string;
    abbreviation: string;
    name: string;
    /** Smaller text after the name, such as "English · v1.2.0" or "Open". */
    detail?: string;
    checked?: boolean;
    pick: () => void;
  }

  export interface MenuSection {
    heading: string;
    items: MenuItem[];
  }
</script>

<script lang="ts">
  /**
   * A menu of modules, shared by the Bible selector, "New search in…", and
   * the parallel pane's +, so they look and behave alike: sections of
   * modules (abbreviation, name, a detail), an optional check on the
   * current one, and "Get more modules…", which opens the Modules dialog on
   * its Download tab. Escape or a click outside closes it.
   */
  import { Check, Download } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import { modulesDialogService } from '$lib/services/modules-dialog-service.svelte';

  let {
    sections,
    label,
    onclose,
    class: className = '',
  }: {
    sections: MenuSection[];
    /** The menu's name, for screen readers. */
    label: string;
    onclose: () => void;
    class?: string;
  } = $props();

  let element = $state<HTMLElement>();

  const checkable = $derived(
    sections.some((section) => section.items.some((item) => item.checked))
  );

  function onWindowClick(event: MouseEvent) {
    if (element && !element.contains(event.target as Node)) onclose();
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose();
  }

  function getMore() {
    onclose();
    modulesDialogService.open('download');
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div
  bind:this={element}
  role="menu"
  aria-label={label}
  class="absolute z-30 flex min-w-72 flex-col gap-0.5 rounded-box border border-base-content/15 bg-base-100 p-1.5 shadow-lg {className}"
>
  {#each sections as section, index (section.heading)}
    {#if index > 0}<div class="mx-1.5 my-1 h-px bg-base-content/10"></div>{/if}
    <div class="px-2.5 pt-1.5 pb-1 text-xs text-base-content/60">
      {section.heading}
    </div>
    {#each section.items as item (item.key)}
      <button
        type="button"
        role={checkable ? 'menuitemradio' : 'menuitem'}
        aria-checked={checkable ? !!item.checked : undefined}
        class={[
          'btn btn-ghost h-auto justify-start gap-3 py-2 text-left font-normal',
          item.checked && 'bg-primary/15',
        ]}
        onclick={() => {
          item.pick();
          onclose();
        }}
      >
        <span class="w-12 shrink-0 font-bold">{item.abbreviation}</span>
        <span class="flex min-w-0 flex-1 flex-col">
          <span class="truncate">{item.name}</span>
          {#if item.detail}
            <span class="text-xs text-base-content/60">{item.detail}</span>
          {/if}
        </span>
        {#if item.checked}
          <Check size={16} class="shrink-0 text-primary" aria-hidden="true" />
        {/if}
      </button>
    {/each}
  {/each}
  <div class="mx-1.5 my-1 h-px bg-base-content/10"></div>
  <button
    type="button"
    role="menuitem"
    class="btn btn-ghost h-auto justify-start gap-3 py-2 font-normal text-primary"
    onclick={getMore}
  >
    <Download size={16} aria-hidden="true" />
    {$_('module-menu.get-more')}
  </button>
</div>
