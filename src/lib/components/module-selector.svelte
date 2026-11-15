<script lang="ts">
  /**
   * The Bible selector: a button naming the module a search or parallel
   * text reads (abbreviation, name, version), which opens a menu of the
   * installed Bibles to switch to. The same control heads the main text and
   * each parallel text.
   */
  import { ChevronDown } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import ModuleMenu, { type MenuSection } from './module-menu.svelte';

  let {
    moduleId,
    name,
    abbreviation,
    version,
    label,
    onselect,
  }: {
    moduleId: string;
    name: string;
    abbreviation: string;
    version: string;
    /** What it chooses for, such as "Bible for this search". */
    label: string;
    onselect: (moduleId: string) => void;
  } = $props();

  let open = $state(false);

  const sections = $derived<MenuSection[]>([
    {
      heading: label,
      items: (moduleFolderService.status?.modules ?? [])
        .filter((module) => module.moduleType === 'bible')
        .map((module) => ({
          key: module.id,
          abbreviation: module.abbreviation,
          name: module.name,
          detail: $_('module-menu.language-version', {
            values: { language: module.language, version: module.version },
          }),
          checked: module.id === moduleId,
          pick: () => onselect(module.id),
        })),
    },
  ]);
</script>

<div class="relative">
  <button
    type="button"
    class="btn btn-sm h-8 gap-2 border-primary/40 bg-primary/10 px-1 pr-2.5 font-normal hover:bg-primary/20"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={$_('module-menu.selector-label', {
      values: { label, name },
    })}
    onclick={(event) => {
      event.stopPropagation();
      open = !open;
    }}
  >
    {#if name}
      <span
        class="rounded bg-primary px-1.5 py-0.5 text-xs font-bold text-primary-content"
        >{abbreviation}</span
      >
      <span class="text-xs">
        {$_('module.nameAndVersion', { values: { name, version } })}
      </span>
    {/if}
    <ChevronDown size={14} aria-hidden="true" />
  </button>
  {#if open}
    <ModuleMenu
      {sections}
      {label}
      class="top-9 left-0"
      onclose={() => (open = false)}
    />
  {/if}
</div>
