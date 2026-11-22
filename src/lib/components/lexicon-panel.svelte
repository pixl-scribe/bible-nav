<script lang="ts">
  /**
   * A lexicon tab: the entry (or entries) for the latest word looked up,
   * laid out as decided on 2026-11-17 (canvas "Lexicon mockups"). Back
   * appears only once there's somewhere to go back to, and names it. The
   * large headword sits on the right in every lexicon. "Also in" offers the
   * same number in other installed lexicons. The license text, which holds
   * the attribution, closes every entry. It scrolls on its own and doesn't
   * follow the chapter.
   */
  import { tick, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import ModuleMessage from './module-message.svelte';
  import LexiconInline from './lexicon-inline.svelte';
  import {
    defaultLexicon,
    hasSenses,
    isRtl,
    isStrongKey,
    lexiconsFor,
    lookUp,
    type LexiconEntry,
    type LexiconPlace,
  } from '$lib/services/lexicon-service';
  import type {
    LexiconTab,
    ParallelPane,
  } from '$lib/services/parallel-pane.svelte';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';
  import { modulesDialogService } from '$lib/services/modules-dialog-service.svelte';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import {
    entryBlocks,
    headwordLine,
    type LexiconItem,
  } from '$lib/services/lexicon-service';

  let {
    tab,
    pane,
    textMaxWidth,
    service,
  }: {
    tab: LexiconTab;
    pane: ParallelPane;
    textMaxWidth: string;
    /** The main pane's text, which scripture references take the reader to. */
    service?: BibleTextService;
  } = $props();

  let entries = $state<LexiconEntry[]>([]);
  let loaded = $state<string | undefined>();
  let error = $state<unknown>();
  let scroller = $state<HTMLElement>();
  let heading = $state<HTMLElement>();
  /** Other lexicons that have this number, found after the entry loads. */
  let alsoIn = $state<{ id: string; abbreviation: string; name: string }[]>([]);

  const modules = $derived(moduleFolderService.status?.modules ?? []);
  const lexicon = $derived(modules.find((m) => m.id === tab.moduleId));
  const placeId = $derived(
    `${tab.moduleId}|${tab.place.kind}|${tab.place.value}`
  );
  const previous = $derived(tab.back[tab.back.length - 1]);

  /** Section headings for the top-level parts of a body. */
  const SECTIONS: Record<string, string> = {
    deriv: 'lexicon.from',
    def: 'lexicon.meaning',
    kjv: 'lexicon.kjv',
    etym: 'lexicon.etymology',
  };

  // Load the place's entries whenever the tab moves to another place.
  $effect(() => {
    const wanted = placeId;
    const place = tab.place;
    untrack(() => {
      if (wanted === loaded) return;
      error = undefined;
      alsoIn = [];
      lookUp(tab.moduleId, place)
        .then(async (found) => {
          if (wanted !== placeId) return;
          entries = found;
          loaded = wanted;
          // The tab's title and Back name the word by its headword.
          if (found[0]) {
            place.headword = found[0].headword;
            place.headwordLang = found[0].headwordLang;
          }
          await tick();
          scroller?.scrollTo({ top: 0 });
          findOthers(place, wanted);
        })
        .catch((err) => (error = err));
    });
  });

  /** "Also in": other installed lexicons with entries for this number. */
  async function findOthers(place: LexiconPlace, wanted: string) {
    if (place.kind !== 'strong') return;
    const others = lexiconsFor(place.value, modules).filter(
      (m) => m.id !== tab.moduleId
    );
    const found = [];
    for (const other of others) {
      try {
        if ((await lookUp(other.id, place)).length > 0) found.push(other);
      } catch {
        // A lexicon that can't answer is simply not offered.
      }
    }
    if (wanted === placeId) alsoIn = found;
  }

  /**
   * A link inside the entry. Within this lexicon it opens in place, kept for
   * Back. A number of another language (Strong's Greek's links to Hebrew
   * numbers) opens in that language's lexicon, as a new start there, or
   * offers the download when none is installed.
   */
  function open(place: LexiconPlace) {
    if (
      place.kind === 'strong' &&
      !lexicon?.lookups.includes(`strong:${place.value[0]}`)
    ) {
      const other = defaultLexicon(place.value, modules);
      if (other) pane.openLexicon(other.id, place);
      else modulesDialogService.open('download');
      return;
    }
    pane.openLexicon(tab.moduleId, place, true);
  }

  /**
   * A scripture reference: the main text goes there (a chapter's verse 1),
   * as from the reference picker, with the search box emptied.
   */
  function goToVerse(sid: string) {
    if (!service || !sid) return;
    service.goToReference(sid.includes(':') ? sid : `${sid}:1`);
  }

  /** A reference's hover text, naming the verse (or the verse meant). */
  function describeRef(sid: string, meant: string | undefined): string {
    const name = (s: string) =>
      service?.formatRefFromSid(s.includes(':') ? s : `${s}:1`) || s;
    return meant
      ? $_('lexicon.ref-meant', { values: { meant: name(meant) } })
      : $_('lexicon.ref', { values: { verse: name(sid) } });
  }

  /** Between the spellings on BDB's headword line. */
  const SPELLING_SEPARATOR = ', ';

  /** The text of a word element (an other spelling on the headword line). */
  const textOf = (items: LexiconItem[]) =>
    items.map((item) => (typeof item === 'string' ? item : '')).join('');

  async function back() {
    pane.lexiconBack(tab.key);
    // Back disappears when there's nothing before; keep focus in the tab.
    if (tab.back.length === 0) {
      await tick();
      heading?.focus();
    }
  }
</script>

{#snippet word(text: string, lang: string | undefined, extra = '')}
  <span {lang} dir={isRtl(lang) ? 'rtl' : undefined} class={extra}>{text}</span>
{/snippet}

<div
  class="flex items-center gap-2.5 border-b border-base-content/10 px-3 py-1.5"
>
  {#if previous}
    <button
      type="button"
      class="btn btn-soft btn-primary btn-sm"
      aria-label={$_('lexicon.back', {
        values: { word: previous.headword ?? previous.value },
      })}
      onclick={back}
    >
      ← {@render word(
        previous.headword ?? previous.value,
        previous.headwordLang
      )}
    </button>
  {/if}
  <span class="truncate text-xs text-base-content/60"
    >{lexicon?.name ?? ''}</span
  >
</div>

<div bind:this={scroller} class="min-h-0 flex-1 overflow-y-auto px-4 pt-4 pb-6">
  <div class="mx-auto flex flex-col gap-6" style:max-width={textMaxWidth}>
    {#if error}
      <div role="alert" class="alert alert-error whitespace-normal">
        <ModuleMessage {error} />
      </div>
    {:else if loaded !== placeId}
      <p class="text-sm text-base-content/60">{$_('lexicon.loading')}</p>
    {:else if entries.length === 0}
      <p class="text-sm text-base-content/60">
        {$_('lexicon.not-found', {
          values: { value: tab.place.value, name: lexicon?.name ?? '' },
        })}
      </p>
    {:else}
      {#each entries as entry (entry.key)}
        {@const line = headwordLine(entry)}
        <article class="flex flex-col gap-4">
          <header class="flex items-start justify-between gap-3">
            <div class="flex flex-col gap-1 pt-2">
              {#if entry.translit || entry.pronunciation}
                <!-- The separator is in an expression, so formatting can't
                     drop the spaces around it. -->
                <!-- In the text colour, not the primary colour: in this app,
                     primary text is something to click. Bold, as Strong's
                     prints it, with the pronunciation plain. -->
                <span class="text-sm"
                  ><span class="font-semibold">{entry.translit}</span
                  >{entry.translit && entry.pronunciation ? ' · ' : ''}<span
                    class="text-base-content/60">{entry.pronunciation}</span
                  ></span
                >
              {/if}
              <!-- A Strong's number means something to a reader; other keys
                   (Perseus's "n105") are internal ids. -->
              {#if entry.homograph || isStrongKey(entry.key)}
                <span class="text-xs text-base-content/60">
                  {entry.homograph ? `${entry.homograph} ` : ''}{isStrongKey(
                    entry.key
                  )
                    ? entry.key
                    : ''}
                </span>
              {/if}
              {#if alsoIn.length > 0 && entry === entries[0]}
                <nav
                  aria-label={$_('lexicon.also-in-label')}
                  class="mt-1 flex flex-wrap items-center gap-2"
                >
                  <span class="text-xs text-base-content/60"
                    >{$_('lexicon.also-in')}</span
                  >
                  {#each alsoIn as other (other.id)}
                    <button
                      type="button"
                      class="btn btn-soft btn-primary btn-xs"
                      onclick={() =>
                        pane.openLexicon(other.id, {
                          kind: 'strong',
                          value: tab.place.value,
                          word: tab.place.word,
                        })}>{other.abbreviation} · {other.name} ›</button
                    >
                  {/each}
                </nav>
              {/if}
            </div>
            <!-- BDB's headword line: ‡ (only in Biblical Aramaic), † (every
                 occurrence cited), brackets (the form itself doesn't
                 occur), other spellings, and the count printed under it. -->
            <h2
              bind:this={heading}
              tabindex="-1"
              class="m-0 text-end text-4xl leading-tight font-normal outline-none"
            >
              {#if line?.peculiar}<span
                  class="lexicon-mark"
                  title={$_('lexicon.peculiar')}>‡</span
                >{/if}{#if entry.allCited}<span
                  class="lexicon-mark"
                  title={$_('lexicon.all-cited')}>†</span
                >{/if}<span
                title={line?.brackets ? $_('lexicon.brackets') : undefined}
                >{line?.brackets ? '[' : ''}{@render word(
                  entry.headword,
                  entry.headwordLang
                )}{line?.brackets === 'first'
                  ? ']'
                  : ''}{#each line?.variants ?? [] as variant, v (v)}<span
                    class="text-2xl"
                    >{SPELLING_SEPARATOR}{@render word(
                      textOf(variant.children),
                      variant.lang as string
                    )}{#if variant.count}<sub
                        class="lexicon-count"
                        title={$_('lexicon.count')}>{variant.count}</sub
                      >{/if}</span
                  >{/each}{line?.brackets === 'all' ? ']' : ''}</span
              >{#if line?.count}<sub
                  class="lexicon-count"
                  title={$_('lexicon.count')}>{line.count}</sub
                >{/if}
            </h2>
          </header>

          {#each entryBlocks(entry) as block, index (index)}
            {@const part = block.part}
            <!-- Numbered senses (Liddell and Scott, BDB), deeper levels
                 indented; their entries need no section headings. A BDB
                 sense shows the words before its number, its dagger,
                 number, stem and count, then its text. -->
            <section
              class="flex flex-col gap-1"
              style:padding-inline-start={block.indent
                ? `${block.indent * 1.25}rem`
                : undefined}
            >
              {#if SECTIONS[part.style] && !hasSenses(entry)}
                <h3
                  class="m-0 text-[0.6875rem] tracking-wider text-base-content/60 uppercase"
                >
                  {$_(SECTIONS[part.style])}
                </h3>
              {/if}
              <p class="m-0 leading-relaxed">
                {#if part.pre}{`${part.pre} `}{/if}{#if part.allCited}<span
                    class="lexicon-mark me-0.5"
                    title={$_('lexicon.all-cited')}>†</span
                  >{/if}{#if part.n}<strong class="me-1.5">{part.n}.</strong
                  >{/if}{#if part.stem}<strong class="me-1.5"
                    >{part.stem}</strong
                  >{/if}{#if part.count}<sub
                    class="lexicon-count me-1.5"
                    title={$_('lexicon.count')}>{part.count}</sub
                  >{/if}<LexiconInline
                  items={block.text}
                  onOpen={open}
                  onVerse={goToVerse}
                  {describeRef}
                />
              </p>
            </section>
          {/each}
        </article>
      {/each}
      {#if lexicon?.license}
        <footer
          class="border-t border-base-content/10 pt-3 text-xs leading-normal text-base-content/60"
        >
          {lexicon.license}
        </footer>
      {/if}
    {/if}
  </div>
</div>

<style>
  .lexicon-mark {
    color: color-mix(in oklab, var(--color-base-content) 70%, transparent);
    margin-inline-end: 0.15em;
  }
  .lexicon-count {
    margin-inline-start: 0.15em;
    font-size: 0.45em;
    color: color-mix(in oklab, var(--color-base-content) 60%, transparent);
  }
  p .lexicon-count {
    font-size: 0.75em;
  }
</style>
