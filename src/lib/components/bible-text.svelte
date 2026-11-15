<script lang="ts">
  import ModuleMessage from './module-message.svelte';
  import { onMount, tick, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import BibleTextService from '$lib/services/bible-text-service.svelte.js';
  import Paragraph from './paragraph.svelte';
  import Verse from './verse.svelte';
  import SearchRow from './search-row.svelte';
  import FootnoteTooltip from './footnote-tooltip.svelte';
  import type { MenuSection } from './module-menu.svelte';
  import {
    appSettingsService,
    textMaxWidths,
  } from '$lib/services/app-settings-service.svelte';
  import {
    chapterOfSid,
    chapterVerses,
    firstVerseOf,
  } from '$lib/services/chapter-text';

  /**
   * A reading pane: the search box and one text, scrolled through with the
   * mouse and the browser's own scroll bar. `service` is bound by the search
   * (search-view.svelte) so its parallel pane can read and steer this text.
   *
   * In chapters mode, chapters are loaded as the reader nears either end of
   * what's loaded: at least one each side of the chapter at the top, and
   * more until a full screen of text lies beyond the visible area each way
   * (so a run of short chapters, such as Psalm 117, still fills it).
   * Chapters more than two screens away, other than the current chapter's
   * neighbours, are dropped. When text comes or goes above the reading
   * point, the scroll position is corrected by its height, so the text
   * never jumps. The browser's own scroll anchoring is turned off
   * (`overflow-anchor: none`): older WebKitGTK lacks it, and newer versions
   * (2.54 has it) would correct the position a second time.
   */
  let {
    moduleId,
    // Assigned in onMount; the binding carries it to the parent, which ESLint
    // can't see.
    // eslint-disable-next-line no-useless-assignment
    service = $bindable(),
    parallelOpen = false,
    onToggleParallel,
    parallelMenu,
    showSearch = true,
    startSid,
    selectorLabel,
    onModuleChange,
  }: {
    moduleId: string;
    service?: BibleTextService;
    parallelOpen?: boolean;
    onToggleParallel?: () => void;
    /** What the Parallel button offers while the parallel pane is empty. */
    parallelMenu?: MenuSection[];
    /** A parallel text shows no search box, only the Bible selector. */
    showSearch?: boolean;
    /** Where to start, such as the main text's verse for a parallel text. */
    startSid?: string;
    /** What the selector chooses for, such as "Bible for this search". */
    selectorLabel: string;
    /** The reader picked another module in the selector. */
    onModuleChange: (moduleId: string) => void;
  } = $props();

  let bibleTextService = $state<BibleTextService | undefined>();
  const textMaxWidth = $derived(textMaxWidths[appSettingsService.textWidth]);
  let scrollableElement = $state<HTMLElement | undefined>();

  onMount(() => {
    bibleTextService = new BibleTextService(
      untrack(() => moduleId),
      untrack(() => startSid)
    );
    service = bibleTextService;
  });

  // A module picked in the selector: switch this text to it, keeping the
  // reader's place.
  $effect(() => {
    const id = moduleId;
    untrack(() => bibleTextService?.setModule(id));
  });

  /**
   * A verse's element in this pane. Each search keeps its own pane, so the
   * same verse can be on the page more than once: never look it up in the
   * whole document.
   */
  function findVerse(sid: string): HTMLElement | null {
    return (
      scrollableElement?.querySelector<HTMLElement>(
        `[data-sid="${CSS.escape(sid)}"]`
      ) ?? null
    );
  }

  function findChapter(chapterSid: string): HTMLElement | null {
    return (
      scrollableElement?.querySelector<HTMLElement>(
        `[data-chapter="${CSS.escape(chapterSid)}"]`
      ) ?? null
    );
  }

  /**
   * The verse at the top of the pane: the first verse that goes on past the
   * top line. A verse that ends on the top line counts as passed: in
   * paragraph grouping that line also starts the next verse, and going to a
   * verse puts its first line at the top, so both panes then name the same
   * verse. Every verse is marked with `data-sid` over its whole text, in
   * reading order, so a binary search finds it quickly.
   */
  function measureTop() {
    if (!scrollableElement || !bibleTextService) return;
    const line =
      parseFloat(getComputedStyle(scrollableElement).lineHeight) || 28;
    const top = scrollableElement.getBoundingClientRect().top + line + 4;
    const verses =
      scrollableElement.querySelectorAll<HTMLElement>('[data-sid]');
    let low = 0;
    let high = verses.length - 1;
    let found: HTMLElement | undefined;
    while (low <= high) {
      const middle = (low + high) >> 1;
      if (verses[middle].getBoundingClientRect().bottom > top) {
        found = verses[middle];
        high = middle - 1;
      } else {
        low = middle + 1;
      }
    }
    const sid = found?.dataset.sid;
    if (found && sid) {
      const box = found.getBoundingClientRect();
      anchor = {
        sid,
        offset:
          (box.top - scrollableElement.getBoundingClientRect().top) /
          Math.max(box.height, 1),
      };
    }
    if (sid && bibleTextService.topSid !== sid) bibleTextService.topSid = sid;
  }

  /**
   * The verse at the top and how far it's scrolled past the top edge, as a
   * fraction of its height (0 when it starts exactly at the top, -0.5 when
   * half of it is above). Kept so the same place can be shown again when
   * the lines wrap differently.
   */
  let anchor: { sid: string; offset: number } | undefined;

  /**
   * When the text gets narrower or wider (the parallel pane opens or closes,
   * the window is resized, the text width setting changes), its lines wrap
   * again and every verse changes height, so the same scroll position would
   * show other verses. Bring the verse at the top back to where it was.
   */
  function keepAnchor() {
    const pane = scrollableElement;
    if (!pane || !anchor || gliding || bibleTextService?.scrollTarget) return;
    const element = findVerse(anchor.sid);
    if (!element) return;
    const box = element.getBoundingClientRect();
    pane.scrollTop +=
      box.top - pane.getBoundingClientRect().top - anchor.offset * box.height;
  }

  // Watch the widths of the pane and of the reading column (which the text
  // width setting limits). Heights change whenever chapters load, so only
  // a change of width counts.
  let column = $state<HTMLElement | undefined>();
  $effect(() => {
    const elements = [scrollableElement, column].filter(
      (element): element is HTMLElement => !!element
    );
    if (elements.length === 0) return;
    const widths = new WeakMap<Element, number>();
    const observer = new ResizeObserver((entries) => {
      let changed = false;
      for (const entry of entries) {
        const width = entry.contentRect.width;
        const before = widths.get(entry.target);
        widths.set(entry.target, width);
        if (before !== undefined && before !== width) changed = true;
      }
      if (changed) keepAnchor();
    });
    for (const element of elements) observer.observe(element);
    return () => observer.disconnect();
  });

  /** Loads and drops chapters to keep a screen of text beyond each edge. */
  let maintaining = false;
  async function maintain() {
    const pane = scrollableElement;
    const svc = bibleTextService;
    if (maintaining || gliding || !pane || !svc || svc.scrollTarget) return;
    maintaining = true;
    try {
      if (svc.mode === 'list') {
        const below = pane.scrollHeight - pane.scrollTop - pane.clientHeight;
        if (below < pane.clientHeight) await svc.loadMoreList();
        return;
      }
      if (svc.mode !== 'chapters' || svc.chapters.length === 0) return;
      // Fill: at least one chapter each side of the current one, and a
      // screen of text beyond the visible area each way. A few chapters at
      // most per pass; scrolling calls again.
      for (let step = 0; step < 6; step++) {
        const screen = pane.clientHeight;
        const below = pane.scrollHeight - pane.scrollTop - screen;
        const current = svc.chapters.findIndex(
          (c) => c.sid === currentChapter()
        );
        const needAfter =
          below < screen ||
          (current >= 0 && current === svc.chapters.length - 1);
        const needBefore = pane.scrollTop < screen || current === 0;
        if (needAfter) {
          if (await svc.extend(1)) {
            await tick();
            continue;
          }
        }
        if (needBefore) {
          const before = pane.scrollHeight;
          if (await svc.extend(-1)) {
            await tick();
            pane.scrollTop += pane.scrollHeight - before;
            continue;
          }
        }
        break;
      }
      await trim();
    } finally {
      maintaining = false;
    }
  }

  function currentChapter(): string | undefined {
    const sid = bibleTextService?.topSid;
    return sid ? chapterOfSid(sid) : undefined;
  }

  /** Drops chapters more than two screens away, keeping the neighbours. */
  async function trim() {
    const pane = scrollableElement;
    const svc = bibleTextService;
    if (!pane || !svc) return;
    const current = svc.chapters.findIndex((c) => c.sid === currentChapter());
    if (current < 0) return;
    const area = pane.getBoundingClientRect();
    const reach = pane.clientHeight * 2;

    let front = 0;
    for (let i = 0; i < current - 1; i++) {
      const element = findChapter(svc.chapters[i].sid);
      if (
        !element ||
        element.getBoundingClientRect().bottom > area.top - reach
      ) {
        break;
      }
      front = i + 1;
    }
    let back = 0;
    for (let i = svc.chapters.length - 1; i > current + 1; i--) {
      const element = findChapter(svc.chapters[i].sid);
      if (
        !element ||
        element.getBoundingClientRect().top < area.bottom + reach
      ) {
        break;
      }
      back++;
    }
    if (front > 0) {
      const before = pane.scrollHeight;
      svc.drop(front, true);
      await tick();
      pane.scrollTop -= before - pane.scrollHeight;
    }
    if (back > 0) svc.drop(back, false);
  }

  let framePending = false;
  function onScroll() {
    if (framePending) return;
    framePending = true;
    requestAnimationFrame(() => {
      framePending = false;
      measureTop();
      maintain();
    });
  }

  /**
   * A smooth scroll (a Notes heading) is under way. Setting the scroll
   * position stops a smooth scroll where it is, so chapters aren't loaded
   * or dropped (which corrects the position) until it ends: when the
   * browser says so (`scrollend`), when the reader scrolls themselves, or
   * after a second and a half at most.
   */
  let gliding = false;
  let glideTimer: ReturnType<typeof setTimeout> | undefined;
  function glide() {
    gliding = true;
    clearTimeout(glideTimer);
    glideTimer = setTimeout(endGlide, 1500);
  }
  function endGlide() {
    if (!gliding) return;
    gliding = false;
    clearTimeout(glideTimer);
    measureTop();
    maintain();
  }

  // The reader's own scrolling (wheel or touchpad, touch, keys, or a press
  // on the scroll bar) makes this text the one its partner follows. Scrolls
  // made by the code don't, so the two panes can't steer each other in turn.
  const leadEvents = ['wheel', 'touchstart', 'keydown', 'pointerdown'];
  function onLead() {
    bibleTextService?.lead();
    // The reader's own scrolling takes over from a smooth scroll.
    endGlide();
  }

  $effect(() => {
    const pane = scrollableElement;
    if (!pane) return;
    pane.addEventListener('scroll', onScroll, { passive: true });
    pane.addEventListener('scrollend', endGlide);
    for (const name of leadEvents) {
      pane.addEventListener(name, onLead, { passive: true });
    }
    return () => {
      pane.removeEventListener('scroll', onScroll);
      pane.removeEventListener('scrollend', endGlide);
      for (const name of leadEvents) pane.removeEventListener(name, onLead);
    };
  });

  /**
   * Bring a verse to the top once its text is drawn. The first verse of a
   * chapter brings the chapter's start, so a Psalm title above it shows. A
   * smooth target glides to the verse and centres it (a Notes tab heading).
   */
  $effect(() => {
    const target = bibleTextService?.scrollTarget;
    if (!target) return;
    void bibleTextService?.chapters;
    void bibleTextService?.listVerses;
    untrack(async () => {
      await tick();
      const pane = scrollableElement;
      const svc = bibleTextService;
      if (!pane || !svc || svc.scrollTarget !== target) return;
      const isFirst =
        svc.mode === 'chapters' &&
        target.sid === firstVerseOf(chapterOfSid(target.sid));
      const element =
        (isFirst ? findChapter(chapterOfSid(target.sid)) : null) ??
        findVerse(target.sid);
      if (!element) return;
      svc.scrollTarget = undefined;
      if (target.smooth) {
        // A chapter's start goes to the top: centring a whole chapter
        // would show its middle instead of verse 1.
        glide();
        element.scrollIntoView({
          behavior: 'smooth',
          block: isFirst ? 'start' : 'center',
        });
      } else {
        pane.scrollTop +=
          element.getBoundingClientRect().top -
          pane.getBoundingClientRect().top;
      }
      measureTop();
      await maintain();
      measureTop();
    });
  });

  // Switching grouping keeps the verse at the top.
  $effect(() => {
    void appSettingsService.groupingMode;
    untrack(() => {
      const sid = bibleTextService?.topSid;
      if (sid && bibleTextService) bibleTextService.scrollTarget = { sid };
    });
  });
</script>

<section
  aria-label={$_('bible-text.main-text')}
  class="flex min-w-0 flex-1 basis-0 flex-col rounded-box bg-base-200"
>
  <SearchRow
    {bibleTextService}
    {parallelOpen}
    {onToggleParallel}
    {parallelMenu}
    {showSearch}
    {selectorLabel}
    onSelectModule={onModuleChange}
  />
  {#if bibleTextService?.loadError}
    <div role="alert" class="alert alert-error m-2 whitespace-normal">
      <ModuleMessage error={bibleTextService.loadError} />
    </div>
  {/if}
  <div class="flex min-h-0 w-full flex-1 pt-2 pl-2 pb-2">
    <div
      class="reading-scroll h-full flex-1 overflow-y-auto overflow-x-hidden pr-2 text-lg leading-relaxed"
      bind:this={scrollableElement}
    >
      <!-- The reading column: centered, at most the chosen text width. -->
      <div
        class="mx-auto flex flex-col"
        style:max-width={textMaxWidth}
        bind:this={column}
      >
        {#if bibleTextService?.mode === 'chapters'}
          {#each bibleTextService.chapters as chapter (chapter.sid)}
            <div data-chapter={chapter.sid}>
              {#if bibleTextService.groupingMode === 'paragraph'}
                {#each chapter.paragraphs as paragraph (paragraph.id)}
                  <Paragraph {paragraph} {bibleTextService} />
                {/each}
              {:else}
                {#each chapterVerses(chapter) as verse (verse.sid)}
                  <Verse {verse} {bibleTextService} />
                {/each}
              {/if}
            </div>
          {/each}
        {:else if bibleTextService?.mode === 'list'}
          {#each bibleTextService.listVerses as verse (verse.sid)}
            <Verse {verse} {bibleTextService} />
          {/each}
        {:else if bibleTextService?.mode === 'text'}
          <p class="m-6 text-center text-base text-base-content/60">
            {$_('bible-text.word-search-later')}
          </p>
        {/if}
      </div>
    </div>
    <!-- The pane's one shared footnote tooltip, kept inside the pane. -->
    <FootnoteTooltip pane={scrollableElement} />
  </div>
</section>

<style>
  /* The browser's own scroll bar, in the theme's colours: a thin rounded
     thumb in the text colour, faint, on a clear track. */
  .reading-scroll {
    /* The scroll position is corrected by hand when chapters come or go
       above the reading point; the browser mustn't correct it too. */
    overflow-anchor: none;
    scrollbar-width: thin;
    scrollbar-color: color-mix(
        in oklab,
        var(--color-base-content) 35%,
        transparent
      )
      transparent;
  }
  .reading-scroll::-webkit-scrollbar {
    width: 10px;
  }
  .reading-scroll::-webkit-scrollbar-track {
    background: transparent;
  }
  .reading-scroll::-webkit-scrollbar-thumb {
    border: 2px solid transparent;
    border-radius: 999px;
    background-clip: padding-box;
    background-color: color-mix(
      in oklab,
      var(--color-base-content) 30%,
      transparent
    );
  }
  .reading-scroll::-webkit-scrollbar-thumb:hover {
    background-color: color-mix(
      in oklab,
      var(--color-base-content) 50%,
      transparent
    );
  }
</style>
