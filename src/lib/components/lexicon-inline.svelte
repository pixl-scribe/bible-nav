<script lang="ts">
  /**
   * Draws the inline content of a lexicon entry
   * (bible-nav-modules/docs/lexicon-format.md, "Entry content"). Text is
   * joined exactly as stored, so the template has no whitespace between
   * elements: line breaks fall inside tags, as Prettier writes them
   * (lexicon-rendering.test.ts checks the drawn text). A link to another entry opens it in
   * the same tab (`onOpen`); a link by headword (`lemma`) opens every entry
   * with that headword. A translation (`tr`) is in italics, as Liddell and
   * Scott print it. A scripture reference (BDB's `bref`) takes the main
   * text there (`onVerse`): to the verse meant, where BDB misprinted the
   * number. Styles not listed here show their content.
   */
  import type {
    LexiconItem,
    LexiconPlace,
  } from '$lib/services/lexicon-service';
  import { isRtl } from '$lib/services/lexicon-service';
  import LexiconInline from './lexicon-inline.svelte';

  let {
    items,
    onOpen,
    onVerse,
    describeRef,
  }: {
    items: LexiconItem[];
    onOpen: (place: LexiconPlace) => void;
    /** Takes the main text to a scripture reference (BDB's `bref`). */
    onVerse?: (sid: string) => void;
    /** The hover text of a scripture reference. */
    describeRef?: (sid: string, meant: string | undefined) => string;
  } = $props();

  const attr = (item: LexiconItem, name: string): string | undefined =>
    typeof item === 'string' ? undefined : (item[name] as string | undefined);

  function open(item: LexiconItem) {
    const strong = attr(item, 'strong');
    const key = attr(item, 'key');
    const lemma = attr(item, 'lemma');
    if (strong) onOpen({ kind: 'strong', value: strong });
    else if (key) onOpen({ kind: 'key', value: key });
    else if (lemma) onOpen({ kind: 'lemma', value: lemma });
  }

  /** The language of a link's word: from its Strong's number, or Greek (Liddell and Scott). */
  function refLang(item: LexiconItem): string | undefined {
    const strong = attr(item, 'strong');
    if (!strong)
      return attr(item, 'key') || attr(item, 'lemma') ? 'grc' : undefined;
    return strong.startsWith('G') ? 'grc' : 'hbo';
  }

  /** A word's transliteration and pronunciation, after it. */
  function sounds(item: LexiconItem): string {
    return [attr(item, 'translit'), attr(item, 'pronunciation')]
      .filter(Boolean)
      .join(', ');
  }
</script>

{#each items as item, index (index)}{#if typeof item === 'string'}{item}{:else if item.style === 'em'}<em
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></em
    >{:else if item.style === 'tr'}<em class="lexicon-tr"
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></em
    >{:else if item.style === 'strong'}<strong class="font-semibold"
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></strong
    >{:else if item.style === 'big'}<span class="lexicon-big"
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></span
    >{:else if item.style === 'br'}<br />{:else if item.style === 'sup'}<sup
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></sup
    >{:else if item.style === 'sub'}<sub
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></sub
    >{:else if item.style === 'usg'}<abbr
      class="text-base-content/70 no-underline"
      title={attr(item, 'title')}
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></abbr
    >{:else if item.style === 'bref'}<button
      type="button"
      class="lexicon-bref"
      class:lexicon-bref-meant={attr(item, 'meant')}
      title={describeRef?.(attr(item, 'sid') ?? '', attr(item, 'meant'))}
      onclick={() => onVerse?.(attr(item, 'meant') ?? attr(item, 'sid') ?? '')}
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></button
    >{:else if item.style === 'foreign'}<span
      lang={attr(item, 'lang')}
      dir={isRtl(attr(item, 'lang')) ? 'rtl' : undefined}
      class="original"
      ><LexiconInline
        items={item.children}
        {onOpen}
        {onVerse}
        {describeRef}
      /></span
    >{:else if item.style === 'ref'}<button
      type="button"
      class="lexicon-ref"
      onclick={() => open(item)}
      >{#if attr(item, 'strong')}<span class="text-xs font-semibold"
          >{`${attr(item, 'strong')} `}</span
        >{/if}<span
        lang={refLang(item)}
        dir={isRtl(refLang(item)) ? 'rtl' : undefined}
        class="original"><LexiconInline items={item.children} {onOpen} /></span
      ></button
    >{:else if item.style === 'word'}<span
      lang={attr(item, 'lang')}
      dir={isRtl(attr(item, 'lang')) ? 'rtl' : undefined}
      class="original"><LexiconInline items={item.children} {onOpen} /></span
    >{#if attr(item, 'count')}<sub class="lexicon-count"
        >{attr(item, 'count')}</sub
      >{/if}{#if sounds(item)}{item.children.length ? ' ' : ''}<span
        class="text-base-content/70">{sounds(item)}</span
      >{/if}{:else}<LexiconInline
      items={item.children}
      {onOpen}
      {onVerse}
      {describeRef}
    />{/if}{/each}

<style>
  .original {
    font-size: 1.15em;
  }
  .lexicon-ref {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35em;
    padding: 0 0.4em;
    border: 1px solid color-mix(in oklab, var(--color-primary) 50%, transparent);
    border-radius: 0.375rem;
    background: color-mix(in oklab, var(--color-primary) 12%, transparent);
    color: var(--color-primary);
    font: inherit;
    line-height: inherit;
    cursor: pointer;
  }
  .lexicon-ref:hover {
    background: color-mix(in oklab, var(--color-primary) 22%, transparent);
  }
  .lexicon-ref .original {
    color: var(--color-base-content);
  }
  /* A scripture reference: a quiet link, as there are thousands. */
  .lexicon-bref {
    display: inline;
    padding: 0;
    border: 0;
    background: none;
    color: var(--color-primary);
    font: inherit;
    line-height: inherit;
    cursor: pointer;
  }
  .lexicon-bref:hover {
    text-decoration: underline;
  }
  /* BDB misprinted the number: shown as printed, linked to the verse meant. */
  .lexicon-bref-meant {
    text-decoration: underline dotted;
  }
  .lexicon-big {
    font-size: 1.25em;
  }
  .lexicon-count {
    margin-inline-start: 0.15em;
    color: color-mix(in oklab, var(--color-base-content) 60%, transparent);
  }
</style>
