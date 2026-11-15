<script lang="ts">
  /**
   * A message for the reader from Rust (a problem or an error), translated,
   * with its English technical detail in small print beneath.
   */
  import { _ } from 'svelte-i18n';
  import {
    errorMessage,
    problemMessage,
    type Problem,
  } from '$lib/services/module-messages';

  let { problem, error }: { problem?: Problem; error?: unknown } = $props();

  const message = $derived(
    problem ? problemMessage(problem, $_) : errorMessage(error, $_)
  );
</script>

<span class="block">{message.text}</span>
{#if message.detail}
  <span class="block text-xs opacity-70 break-words">{message.detail}</span>
{/if}
