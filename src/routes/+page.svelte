<script lang="ts">
  import { BookSearch, Plus } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import LeftNav from '$lib/components/left-nav.svelte';
  import { _ } from 'svelte-i18n';
  import BibleText from '$lib/components/bible-text.svelte';

  const openTexts = $state<string[]>(['eng_kjv']);

  onMount(async () => {
    await themeService.loadTheme();
  });
</script>

<main class="flex p-1 flex-1 items-start drawer">
  <LeftNav />
  {#each openTexts as moduleId, index (index)}
    <BibleText {moduleId} />
  {/each}
  <button
    class="btn btn-soft btn-primary ml-2 px-1 mt-2 tooltip tooltip-bottom"
    data-tip={$_('main.new-search')}
  >
    <Plus size={16} />
    <BookSearch />
  </button>
</main>

<style>
  :global(html, body) {
    display: flex;
    flex: 1;
    height: 100%;
  }
</style>
