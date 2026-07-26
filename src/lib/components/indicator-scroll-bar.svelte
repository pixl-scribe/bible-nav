<script lang="ts">
  let {
    value = $bindable(0),
    min = 0,
    max = 100,
  }: { value?: number; min?: number; max?: number } = $props();

  let trackEl = $state<HTMLDivElement>();

  // Compute thumb position percentage
  let percentage = $derived(((value - min) / (max - min)) * 100);

  function handlePointerDown(e: PointerEvent) {
    updateValue(e);
    window.addEventListener('pointermove', updateValue);
    window.addEventListener('pointerup', stopDragging);
  }

  function updateValue(e: PointerEvent) {
    if (!trackEl) return;
    const rect = trackEl.getBoundingClientRect();
    const clickY = e.clientY;

    // Height from bottom up (inverted for vertical layout)
    const bottom = rect.bottom;
    const height = rect.height;
    const rawPx = bottom - clickY;

    let pct = (rawPx / height) * 100;
    if (pct < 0) pct = 0;
    if (pct > 100) pct = 100;

    value = min + (pct / 100) * (max - min);
  }

  function stopDragging() {
    window.removeEventListener('pointermove', updateValue);
    window.removeEventListener('pointerup', stopDragging);
  }
</script>

<div class="flex justify-center flex-1 py-2">
  <div
    bind:this={trackEl}
    class="slider-track bg-base-content/10 relative w-2 rounded h-full cursor-pointer"
    onpointerdown={handlePointerDown}
    role="slider"
    aria-valuenow={value}
    aria-valuemin={min}
    aria-valuemax={max}
    tabindex="0"
  >
    <div
      class="slider-thumb flex justify-center align-items-center absolute h-6 left-1/2 tooltip tooltip-primary"
      data-tip="Gen 1"
      style="bottom: {percentage}%"
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
    transform: translate(-50%, 50%);
    width: 40px;
  }
</style>
