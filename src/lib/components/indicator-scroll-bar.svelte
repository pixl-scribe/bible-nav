<script lang="ts">
  import type ModuleService from '$lib/services/module-service.svelte';
  import { untrack } from 'svelte';

  let { moduleService }: { moduleService: ModuleService | undefined } =
    $props();

  let trackEl = $state<HTMLDivElement>();

  function handlePointerDown(e: PointerEvent) {
    updateValue(e);
    window.addEventListener('pointermove', updateValue);
    window.addEventListener('pointerup', stopDragging);
  }

  function updateValue(e: PointerEvent) {
    if (!trackEl) return;
    const rect = trackEl.getBoundingClientRect();
    let pct = ((e.clientY - rect.top) / rect.height) * 100;
    if (pct < 0) pct = 0;
    if (pct > 100) pct = 100;
    if (moduleService) {
      moduleService.scrollPct = pct;
    }
  }

  function stopDragging() {
    window.removeEventListener('pointermove', updateValue);
    window.removeEventListener('pointerup', stopDragging);
  }

  /**
   * Watches scroll value and reloads text when it changes.
   */
  $effect(() => {
    const newScrollValue = moduleService?.scrollPct;
    untrack(() => {
      moduleService?.setReference(newScrollValue);
    });
  });
</script>

<div class="flex justify-center flex-1 py-2">
  <div
    bind:this={trackEl}
    class="slider-track bg-base-content/10 relative w-2 rounded h-full cursor-pointer"
    onpointerdown={handlePointerDown}
    role="slider"
    aria-valuenow={moduleService?.scrollPct}
    aria-valuemin={0}
    aria-valuemax={100}
    tabindex="0"
  >
    <div
      class="slider-thumb flex justify-center align-items-center absolute h-6 left-1/2 tooltip tooltip-primary z-10"
      data-tip={moduleService?.referenceLabel}
      style="top: {moduleService?.scrollPct}%"
    >
      <div
        class="bg-primary mask mask-hexagon-2 w-8 h-6 absolute top-0 left-0"
      ></div>
      <div class="bg-primary w-6 h-6 absolute top-0 left-2 rounded"></div>
    </div>
  </div>
</div>

<style>
  .slider-track {
    touch-action: none;
  }
  .slider-thumb {
    transform: translate(-50%, -50%);
    width: 40px;
  }
</style>
