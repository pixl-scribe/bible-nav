<script lang="ts">
  import '../app.css';
  import { waitLocale } from 'svelte-i18n';
</script>

{#await waitLocale()}
  <!-- Optional loading spinner or blank screen while dictionaries load -->
  <div class="loading-screen">Loading languages...</div>
{:then}
  <!-- App content only renders once i18n is initialized -->
  <slot />
{/await}