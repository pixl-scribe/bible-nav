<script lang="ts">
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import LeftNav from '$lib/components/left-nav.svelte';

  onMount(async () => {
    await themeService.loadTheme();
  });
</script>

<main class="flex p-1 flex-1">
  <div class="flex mr-1">
    <LeftNav />
  </div>
  <div
    class="flex bg-base-200 rounded-box rounded-xl w-full flex-1 max-w-140 p-2"
  >
    test
  </div>
</main>

<style>
  :global(html, body) {
    display: flex;
    flex: 1;
    height: 100%;
  }
</style>
