<script lang="ts">
  /**
   * The reference picker: a full-window "Go to" dialog with three columns,
   * books (the module's short names, Old Testament then New), the chosen
   * book's chapters, and the chosen chapter's verses. It opens with the
   * reader's place already chosen in all three, for context.
   *
   * - A click on a book or chapter chooses it; a click on a verse goes
   *   there. A double-click on a chapter goes to its verse 1, and on a book
   *   to its 1:1.
   * - Enter chooses and moves on: on a book it chooses the book and moves
   *   to its chapter 1, on a chapter to its verse 1, and on a verse it goes
   *   there. So Enter, Enter, Enter from a book reaches its 1:1, and the
   *   keyboard needs no double-click.
   * - Each column is one Tab stop (a roving tab stop); the arrow keys move
   *   within it, by position on screen, so they work with any number of
   *   columns. Typing letters jumps to a book ("ro" for Romans, "1c" for
   *   1 Chronicles); typing digits in the chapter or verse column jumps to
   *   that number.
   * - Chapters and verses sit in rows of ten, with a little more room after
   *   the fifth, so rows start at 1, 11, 21...
   *
   * There's no search box here on purpose: references are typed in the
   * main search box, so the app teaches one way to type them.
   */
  import { X } from '@lucide/svelte';
  import { tick } from 'svelte';
  import { _ } from 'svelte-i18n';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import { recentPlaces } from '$lib/services/recent-places.svelte';
  import TipTooltip from './tip-tooltip.svelte';

  let { service }: { service: BibleTextService } = $props();

  type Column = 'book' | 'chapter' | 'verse';
  interface Place {
    book: string;
    chapter: number;
    verse: number;
  }

  let dialog = $state<HTMLDialogElement>();
  let box = $state<HTMLElement>();
  /** Where the reader is: outlined in each column. */
  let here = $state<Place>({ book: '', chapter: 0, verse: 0 });
  /** What's chosen: filled in. No verse is chosen once anything changes. */
  let book = $state('');
  let chapter = $state(1);
  let verse = $state<number | undefined>();
  /** The cell in each column that Tab lands on. */
  let stops = $state<Record<Column, string>>({
    book: '',
    chapter: '1',
    verse: '1',
  });

  const books = $derived(
    service.outline.map((outline) => {
      const info = service.books[outline.code];
      return {
        code: outline.code,
        verses: outline.verses,
        short: info?.toc3 || outline.code,
        name: info?.toc2 || outline.code,
        newTestament: info?.testamentCode === 'NT',
      };
    })
  );
  const oldTestament = $derived(books.filter((b) => !b.newTestament));
  const newTestament = $derived(books.filter((b) => b.newTestament));
  const chosen = $derived(books.find((b) => b.code === book));
  const bookName = $derived(chosen?.name ?? '');
  const chapters = $derived(numbers(chosen?.verses.length ?? 0));
  const verses = $derived(numbers(chosen?.verses[chapter - 1] ?? 0));
  const hereSid = $derived(`${here.book} ${here.chapter}:${here.verse}`);
  /**
   * While a list is shown (a word search or a reference list), the picker
   * goes to its verses only: books and chapters show how many of their
   * verses are listed, and places with none are dimmed and can't be picked.
   */
  const hits = $derived(service.hits);
  // Recent places would leave the list, so they're left out while one is
  // shown.
  const recent = $derived(
    hits
      ? []
      : recentPlaces.places.filter(
          (sid) => sid !== hereSid && service.books[sid.split(' ')[0]]
        )
  );

  function bookCount(code: string): number {
    return hits?.byBook.get(code) ?? 0;
  }

  function chapterCount(n: number): number {
    return hits?.byChapter.get(`${book} ${n}`) ?? 0;
  }

  /** Whether a cell can be picked: anything, unless a list is shown. */
  function pickable(column: Column, key: string): boolean {
    if (!hits) return true;
    if (column === 'book') return bookCount(key) > 0;
    if (column === 'chapter') return chapterCount(Number(key)) > 0;
    return hits.verses.has(`${book} ${chapter}:${key}`);
  }

  /** The first chapter of a book to show: its first listed one, or 1. */
  function firstChapter(code: string): number {
    const first = hits?.firstInBook.get(code);
    return first ? parse(first).chapter : 1;
  }

  /** The first verse of a chapter to land on: its first listed one, or 1. */
  function firstVerse(code: string, n: number): number {
    if (!hits) return 1;
    const count = books.find((b) => b.code === code)?.verses[n - 1] ?? 0;
    for (let v = 1; v <= count; v++) {
      if (hits.verses.has(`${code} ${n}:${v}`)) return v;
    }
    return 1;
  }

  function numbers(count: number): number[] {
    return Array.from({ length: count }, (_, i) => i + 1);
  }

  function parse(sid: string): Place {
    const [code, reference = ''] = sid.split(' ');
    const [c, v] = reference.split(':').map(Number);
    return { book: code, chapter: c || 1, verse: v || 1 };
  }

  /** Opens the picker at the reader's place, with the book focused. */
  export async function open() {
    if (!dialog || dialog.open) return;
    here = parse(service.topSid ?? service.browseSid);
    book = here.book;
    chapter = here.chapter;
    verse = here.verse;
    stops = {
      book: here.book,
      chapter: String(here.chapter),
      verse: String(here.verse),
    };
    dialog.showModal();
    await tick();
    for (const column of ['chapter', 'verse'] as const) {
      cell(column, stops[column])?.scrollIntoView({ block: 'nearest' });
    }
    cell('book', here.book)?.focus();
  }

  function cell(column: Column, key: string): HTMLButtonElement | null {
    return (
      dialog?.querySelector<HTMLButtonElement>(
        `[data-column="${column}"] [data-key="${CSS.escape(key)}"]`
      ) ?? null
    );
  }

  function chooseBook(code: string) {
    if (!pickable('book', code)) return;
    const n = firstChapter(code);
    book = code;
    chapter = n;
    verse = undefined;
    stops = {
      book: code,
      chapter: String(n),
      verse: String(firstVerse(code, n)),
    };
  }

  function chooseChapter(n: number) {
    if (!pickable('chapter', String(n))) return;
    chapter = n;
    verse = undefined;
    stops = {
      ...stops,
      chapter: String(n),
      verse: String(firstVerse(book, n)),
    };
  }

  /**
   * Goes to a verse: within the list while one is shown, otherwise to read
   * chapters from there.
   */
  function go(sid: string) {
    dialog?.close();
    if (hits) service.goToListSid(sid);
    else service.goTo(sid);
  }

  /**
   * Enter: choose a book or chapter and move to the next column, at its
   * first chapter or verse; on a verse, go there.
   */
  async function enter(column: Column, key: string) {
    if (!pickable(column, key)) return;
    if (column === 'book') {
      chooseBook(key);
      await tick();
      cell('chapter', stops.chapter)?.focus();
    } else if (column === 'chapter') {
      chooseChapter(Number(key));
      await tick();
      cell('verse', stops.verse)?.focus();
    } else {
      go(`${book} ${chapter}:${key}`);
    }
  }

  /**
   * A double-click: go to the start of a book or chapter, or, while a list
   * is shown, to its first listed verse there.
   */
  function activate(column: Column, key: string) {
    if (!pickable(column, key)) return;
    if (column === 'book') {
      go(hits?.firstInBook.get(key) ?? `${key} 1:1`);
    } else if (column === 'chapter') {
      go(`${book} ${key}:${firstVerse(book, Number(key))}`);
    } else {
      go(`${book} ${chapter}:${key}`);
    }
  }

  /**
   * The nearest cell in the next row up (-1) or down (1), by position on
   * screen, so it works across the Old and New Testament grids too.
   */
  function vertical(
    cells: HTMLElement[],
    from: HTMLElement,
    direction: 1 | -1
  ) {
    const start = from.getBoundingClientRect();
    const centre = start.left + start.width / 2;
    let best: HTMLElement | undefined;
    let bestRow = Infinity;
    let bestOffset = Infinity;
    for (const candidate of cells) {
      const box = candidate.getBoundingClientRect();
      const row = (box.top - start.top) * direction;
      if (row <= 1) continue;
      const offset = Math.abs(box.left + box.width / 2 - centre);
      if (
        row < bestRow - 1 ||
        (Math.abs(row - bestRow) <= 1 && offset < bestOffset)
      ) {
        best = candidate;
        bestRow = row;
        bestOffset = offset;
      }
    }
    return best;
  }

  function onColumnKey(event: KeyboardEvent, column: Column) {
    const area = event.currentTarget as HTMLElement;
    // Dimmed cells are skipped: there's nothing listed there to go to.
    const cells = [
      ...area.querySelectorAll<HTMLElement>(
        'button[data-key]:not([aria-disabled="true"])'
      ),
    ];
    const current = document.activeElement as HTMLElement;
    const index = cells.indexOf(current);
    if (index < 0) return;
    const rtl = getComputedStyle(area).direction === 'rtl';
    let target: HTMLElement | undefined;
    switch (event.key) {
      case 'ArrowRight':
        target = cells[index + (rtl ? -1 : 1)];
        break;
      case 'ArrowLeft':
        target = cells[index + (rtl ? 1 : -1)];
        break;
      case 'ArrowDown':
        target = vertical(cells, current, 1);
        break;
      case 'ArrowUp':
        target = vertical(cells, current, -1);
        break;
      case 'Home':
        target = cells[0];
        break;
      case 'End':
        target = cells[cells.length - 1];
        break;
      case 'Enter':
        event.preventDefault();
        enter(column, current.dataset.key ?? '');
        return;
      default:
        return;
    }
    event.preventDefault();
    target?.focus();
  }

  // Remember where focus is in a column, so Tab comes back to it.
  function onColumnFocus(event: FocusEvent, column: Column) {
    const key = (event.target as HTMLElement).dataset?.key;
    if (key) stops = { ...stops, [column]: key };
  }

  // Typing jumps: letters to a book, digits to a chapter or verse.
  let typed = '';
  let typedAt = 0;
  function onDialogKey(event: KeyboardEvent) {
    if (event.ctrlKey || event.altKey || event.metaKey) return;
    if (event.key.length !== 1 || event.key === ' ') return;
    const now = performance.now();
    const key = event.key.toLowerCase();
    typed = now - typedAt < 900 ? typed + key : key;
    typedAt = now;
    const column = (document.activeElement as HTMLElement | null)
      ?.closest('[data-column]')
      ?.getAttribute('data-column');
    if ((column === 'chapter' || column === 'verse') && /^\d+$/.test(typed)) {
      event.preventDefault();
      cell(column, String(Number(typed)))?.focus();
      return;
    }
    const flat = (text: string) => text.toLowerCase().replace(/\s+/g, '');
    const match = books.find(
      (b) =>
        pickable('book', b.code) &&
        (flat(b.name).startsWith(typed) || flat(b.short).startsWith(typed))
    );
    if (!match) return;
    event.preventDefault();
    chooseBook(match.code);
    tick().then(() => cell('book', match.code)?.focus());
  }

  /**
   * The grid column for number `n` in rows of ten: columns 1-5, then a
   * narrow spacer column (6), then 7-11. A spacer column rather than a
   * margin keeps every cell the same width.
   */
  function tensColumn(n: number): string {
    const place = (n - 1) % 10;
    return String(place < 5 ? place + 1 : place + 2);
  }

  function cellClass(
    picked: boolean,
    isHere: boolean,
    enabled = true,
    height = 'h-8'
  ) {
    return [
      // Raised while focused, so its neighbours can't cover its outline.
      'relative flex min-w-0 flex-col overflow-hidden rounded-field border px-0 text-sm tabular-nums focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-primary',
      height,
      !enabled
        ? 'cursor-default border-transparent bg-transparent text-base-content/30'
        : picked
          ? 'border-primary bg-primary font-semibold text-primary-content'
          : isHere
            ? 'border-primary/70 bg-primary/15'
            : 'border-base-content/10 bg-base-content/5 hover:bg-base-content/10',
    ];
  }

  /** A listed place's count, in a band across the bottom of its cell. */
  function bandClass(picked: boolean) {
    return [
      'flex flex-1 items-center justify-center text-[0.6875rem] leading-none font-bold',
      picked
        ? 'bg-black/20 text-primary-content'
        : 'border-t border-primary/35 bg-primary/20 text-primary',
    ];
  }
</script>

{#snippet bookGrid(list: typeof books)}
  <div class="grid grid-cols-8 gap-1.5">
    {#each list as b (b.code)}
      {@const count = bookCount(b.code)}
      {@const enabled = pickable('book', b.code)}
      <button
        type="button"
        data-key={b.code}
        tabindex={stops.book === b.code ? 0 : -1}
        class={cellClass(
          b.code === book,
          b.code === here.book,
          enabled,
          hits ? 'h-11' : 'h-8'
        )}
        data-tip={!hits
          ? $_('picker.book-tip', { values: { name: b.name } })
          : count
            ? $_('picker.book-hits', { values: { name: b.name, count } })
            : $_('picker.book-none', { values: { name: b.name } })}
        aria-label={b.name}
        aria-pressed={b.code === book}
        aria-disabled={!enabled}
        aria-current={b.code === here.book ? 'location' : undefined}
        onclick={() => chooseBook(b.code)}
        ondblclick={() => activate('book', b.code)}
        ><span class="flex flex-[2] items-center justify-center">{b.short}</span
        >{#if count}<span class={bandClass(b.code === book)}>{count}</span
          >{/if}</button
      >
    {/each}
  </div>
{/snippet}

<dialog bind:this={dialog} class="modal" aria-labelledby="picker-title">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={box}
    class="modal-box flex text-base text-base-content h-[calc(100dvh-3rem)] max-h-none w-[calc(100vw-3rem)] max-w-none flex-col gap-0 overflow-hidden p-0"
    onkeydown={onDialogKey}
  >
    <div
      class="flex items-center gap-3 border-b border-base-content/10 px-5 py-3"
    >
      <h2 id="picker-title" class="text-xl font-bold">
        {$_('picker.title')}
      </h2>
      <span class="text-sm">
        {#if service.wordSearch !== undefined}
          {$_('picker.now-search', {
            values: {
              words: service.wordSearch.replace(/["\u201C\u201D]/g, ''),
              count: service.listTotal,
              place: service.formatRefFromSid(hereSid),
            },
          })}
        {:else if hits}
          {$_('picker.now-list', {
            values: {
              count: service.listTotal,
              place: service.formatRefFromSid(hereSid),
            },
          })}
        {:else}
          {$_('picker.now', {
            values: { place: service.formatRefFromSid(hereSid) },
          })}
        {/if}
      </span>
      <span class="flex-1"></span>
      <span class="hidden text-xs text-base-content/60 sm:inline">
        {$_('picker.keys')}
      </span>
      <form method="dialog">
        <button
          class="btn btn-circle btn-ghost"
          aria-label={$_('common.close')}
        >
          <X size={20} />
        </button>
      </form>
    </div>

    <div class="flex min-h-0 flex-1">
      <section
        aria-label={$_('picker.books')}
        class="flex min-w-0 flex-1 flex-col"
      >
        <!-- Only the books scroll (if the window is short); Recent stays
             below them. -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          data-column="book"
          class="flex min-h-0 flex-1 flex-col gap-2 overflow-x-hidden overflow-y-auto p-4"
          onkeydown={(event) => onColumnKey(event, 'book')}
          onfocusin={(event) => onColumnFocus(event, 'book')}
        >
          <h3
            class="text-xs font-semibold tracking-wide text-base-content/60 uppercase"
          >
            {$_('picker.old-testament')}
          </h3>
          {@render bookGrid(oldTestament)}
          {#if newTestament.length > 0}
            <h3
              class="mt-2 text-xs font-semibold tracking-wide text-base-content/60 uppercase"
            >
              {$_('picker.new-testament')}
            </h3>
            {@render bookGrid(newTestament)}
          {/if}
        </div>
        {#if recent.length > 0}
          <div
            class="flex shrink-0 flex-col gap-2 border-t border-base-content/10 px-4 py-3"
          >
            <h3
              class="text-xs font-semibold tracking-wide text-base-content/60 uppercase"
            >
              {$_('picker.recent')}
            </h3>
            <!-- At most two rows: whatever doesn't fit is left out. -->
            <div
              class="flex max-h-[4.375rem] flex-wrap gap-1.5 overflow-hidden"
            >
              {#each recent as sid (sid)}
                <button
                  type="button"
                  class="btn btn-sm btn-outline btn-primary font-normal"
                  onclick={() => go(sid)}
                  >{service.formatRefFromSid(sid)}</button
                >
              {/each}
            </div>
          </div>
        {/if}
      </section>

      <section
        aria-label={$_('picker.chapters')}
        class="flex w-[clamp(15rem,30vw,24rem)] shrink-0 flex-col gap-2 border-s border-base-content/10 bg-base-300/30 p-4"
      >
        <div class="flex items-baseline justify-between gap-2">
          <h3 class="truncate text-lg font-bold">{bookName}</h3>
          <span class="text-xs text-base-content/60"
            >{$_('picker.chapter')}</span
          >
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          data-column="chapter"
          class="-m-1 grid min-h-0 grid-cols-[repeat(5,minmax(0,1fr))_0.25rem_repeat(5,minmax(0,1fr))] content-start gap-1 overflow-y-auto p-1"
          onkeydown={(event) => onColumnKey(event, 'chapter')}
          onfocusin={(event) => onColumnFocus(event, 'chapter')}
        >
          {#each chapters as n (n)}
            {@const count = chapterCount(n)}
            {@const enabled = pickable('chapter', String(n))}
            <button
              type="button"
              data-key={n}
              tabindex={stops.chapter === String(n) ? 0 : -1}
              style:grid-column={tensColumn(n)}
              class={cellClass(
                n === chapter,
                book === here.book && n === here.chapter,
                enabled,
                hits ? 'h-10' : 'h-8'
              )}
              data-tip={!hits
                ? $_('picker.chapter-tip', {
                    values: { place: `${bookName} ${n}:1` },
                  })
                : count
                  ? $_('picker.chapter-hits', {
                      values: { place: `${bookName} ${n}`, count },
                    })
                  : $_('picker.chapter-none', {
                      values: { place: `${bookName} ${n}` },
                    })}
              aria-label={`${bookName} ${n}`}
              aria-pressed={n === chapter}
              aria-disabled={!enabled}
              onclick={() => chooseChapter(n)}
              ondblclick={() => activate('chapter', String(n))}
              ><span class="flex flex-[2] items-center justify-center">{n}</span
              >{#if count}<span class={bandClass(n === chapter)}>{count}</span
                >{/if}</button
            >
          {/each}
        </div>
      </section>

      <section
        aria-label={$_('picker.verses')}
        class="flex w-[clamp(15rem,30vw,24rem)] shrink-0 flex-col gap-2 border-s border-base-content/10 bg-base-300/50 p-4"
      >
        <div class="flex items-baseline justify-between gap-2">
          <h3 class="truncate text-lg font-bold">{bookName} {chapter}</h3>
          <span class="text-xs text-base-content/60">{$_('picker.verse')}</span>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          data-column="verse"
          class="-m-1 grid min-h-0 grid-cols-[repeat(5,minmax(0,1fr))_0.25rem_repeat(5,minmax(0,1fr))] content-start gap-1 overflow-y-auto p-1"
          onkeydown={(event) => onColumnKey(event, 'verse')}
          onfocusin={(event) => onColumnFocus(event, 'verse')}
        >
          {#each verses as v (v)}
            {@const enabled = pickable('verse', String(v))}
            <button
              type="button"
              data-key={v}
              tabindex={stops.verse === String(v) ? 0 : -1}
              style:grid-column={tensColumn(v)}
              class={cellClass(
                v === verse,
                book === here.book &&
                  chapter === here.chapter &&
                  v === here.verse,
                enabled
              )}
              aria-label={`${bookName} ${chapter}:${v}`}
              aria-disabled={!enabled}
              onclick={() => enabled && go(`${book} ${chapter}:${v}`)}
              ><span class="flex flex-1 items-center justify-center">{v}</span
              ></button
            >
          {/each}
        </div>
      </section>
    </div>

    <div
      class="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-base-content/10 px-5 py-2 text-xs text-base-content/60"
    >
      <span>{hits ? $_('picker.help-hits') : $_('picker.help')}</span>
      <span class="flex-1"></span>
      <span>{$_('picker.keyboard')}</span>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop">
    <button tabindex="-1">{$_('common.close')}</button>
  </form>
  <!-- The book and chapter tooltips: one, shared, inside the dialog. -->
  <TipTooltip root={box} boundary={box} />
</dialog>
