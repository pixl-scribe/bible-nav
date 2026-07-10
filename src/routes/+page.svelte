<script lang="ts">
  import { BookSearch, Columns2, Search, Plus } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import LeftNav from '$lib/components/left-nav.svelte';
  import { _ } from 'svelte-i18n';

  onMount(async () => {
    await themeService.loadTheme();
  });
</script>

<main class="flex p-1 flex-1 items-start drawer">
  <LeftNav />
  <div
    class="flex bg-base-200 rounded-box rounded-xl w-full flex-1 max-w-140 p-2 ml-1 h-full items-start"
  >
    <div class="flex w-full">
      <label class="input w-full">
        <Search />
        <input type="search" required placeholder="Search" />
      </label>
      <div class="divider divider-horizontal mx-1"></div>
      <button
        class="btn btn-soft btn-primary px-1 tooltip tooltip-bottom"
        data-tip={$_('main.add-parallel')}
      >
        <Plus size={16} />
        <Columns2 />
      </button>
    </div>
  </div>
  <button
    class="btn btn-soft btn-primary ml-1 px-1 mt-2 tooltip tooltip-bottom"
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
