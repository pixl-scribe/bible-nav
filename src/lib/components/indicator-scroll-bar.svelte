<script lang="ts">
  import type { BibleVerseCounts } from '$lib/model/bible-verse-counts';
  import rawVerseCounts from '../assets/bible-verse-counts.yaml';
  import debounce from '$lib/services/debounce';
  import type ModuleService from '$lib/services/module-service.svelte';

  const verseCounts = rawVerseCounts as BibleVerseCounts;

  let {
    value = $bindable(undefined), // represents the scroll position 0(top)..100(bottom)
    moduleService,
  }: { value: number | undefined; moduleService: ModuleService | undefined } =
    $props();

  let referenceLabel = $state<string>('');
  const allVerseCounts = { ...verseCounts.OT, ...verseCounts.NT };
  const verseCount = Object.values(allVerseCounts).reduce(
    (acc, curr) => acc + curr.reduce((acc2, curr2) => acc2 + curr2, 0),
    0
  ); // 31,102 verses

  //const totalVersesInBible = Object.values(books).reduce((c) );
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
    value = pct;
  }

  function stopDragging() {
    window.removeEventListener('pointermove', updateValue);
    window.removeEventListener('pointerup', stopDragging);
  }

  const setRefenceDeBounced = debounce(setReference, 100);

  function setReference(value: number | undefined): void {
    const books = moduleService?.books;
    if (
      value === undefined ||
      moduleService === undefined ||
      books === undefined
    ) {
      referenceLabel = '';
      return;
    }
    const verseIndex = Math.round((value / 100) * (verseCount - 1));
    let foundBookCode: string;
    let verseAccum = 0;
    for (const bookCode of Object.keys(allVerseCounts)) {
      foundBookCode = bookCode;
      const chapterCounts = allVerseCounts[bookCode];
      for (const [index, versesInChapter] of chapterCounts.entries()) {
        const foundChapter = index + 1;
        if (verseAccum + versesInChapter > verseIndex) {
          const book = books[foundBookCode];
          const verse = verseIndex - verseAccum + 1;
          referenceLabel = `${book?.toc3} ${foundChapter}:${verse}`;
          moduleService.currentSearch = `${book?.code} ${foundChapter}:${verse}`;
          moduleService.currentSearchType = 'ref-point';
          return;
        }
        verseAccum += versesInChapter;
      }
    }
    referenceLabel = '';
  }

  /**
   * Watches scroll value and reloads text when it changes.
   */
  $effect(() => {
    setRefenceDeBounced(value);
  });
</script>

<div class="flex justify-center flex-1 py-2">
  <div
    bind:this={trackEl}
    class="slider-track bg-base-content/10 relative w-2 rounded h-full cursor-pointer"
    onpointerdown={handlePointerDown}
    role="slider"
    aria-valuenow={value}
    aria-valuemin={0}
    aria-valuemax={100}
    tabindex="0"
  >
    <div
      class="slider-thumb flex justify-center align-items-center absolute h-6 left-1/2 tooltip tooltip-primary z-10"
      data-tip={referenceLabel}
      style="top: {value}%"
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
