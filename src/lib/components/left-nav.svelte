<script lang="ts">
  import { LibraryBig } from '@lucide/svelte';
  import { Settings } from '@lucide/svelte';
  import { _ } from 'svelte-i18n';
</script>

<ul class="menu bg-base-200 rounded-box rounded-xl">
  <li>
    <!-- svelte-ignore a11y_missing_attribute -->
    <a class="tooltip tooltip-right" data-tip={$_('left-nav.modules')}>
      <LibraryBig />
    </a>
  </li>
  <li>
    <!-- svelte-ignore a11y_missing_attribute -->
    <a class="tooltip tooltip-right" data-tip={$_('left-nav.settings')}>
      <Settings />
    </a>
  </li>
</ul>
