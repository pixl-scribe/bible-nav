<script lang="ts">
  /**
   * Draws the inline content of a lexicon entry
   * (bible-nav-modules/docs/lexicon-format.md, "Entry content"). Text is
   * joined exactly as stored, so the template has no whitespace between
   * elements: line breaks fall inside tags, as Prettier writes them
   * (lexicon-rendering.test.ts checks the drawn text). A link to another entry opens it in
   * the same tab (`onOpen`); a link by headword (`lemma`) opens every entry
   * with that headword. A translation (`tr`) is in italics, as Liddell and
   * Scott print it. Styles not listed here show their content.
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
  }: {
    items: LexiconItem[];
    onOpen: (place: LexiconPlace) => void;
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
      ><LexiconInline items={item.children} {onOpen} /></em
    >{:else if item.style === 'tr'}<em class="lexicon-tr"
      ><LexiconInline items={item.children} {onOpen} /></em
    >{:else if item.style === 'strong'}<strong class="font-semibold"
      ><LexiconInline items={item.children} {onOpen} /></strong
    >{:else if item.style === 'sup'}<sup
      ><LexiconInline items={item.children} {onOpen} /></sup
    >{:else if item.style === 'sub'}<sub
      ><LexiconInline items={item.children} {onOpen} /></sub
    >{:else if item.style === 'usg'}<abbr
      class="text-base-content/70 no-underline"
      title={attr(item, 'title')}
      ><LexiconInline items={item.children} {onOpen} /></abbr
    >{:else if item.style === 'foreign'}<span
      lang={attr(item, 'lang')}
      dir={isRtl(attr(item, 'lang')) ? 'rtl' : undefined}
      class="original"><LexiconInline items={item.children} {onOpen} /></span
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
    >{#if sounds(item)}{item.children.length ? ' ' : ''}<span
        class="text-base-content/70">{sounds(item)}</span
      >{/if}{:else}<LexiconInline items={item.children} {onOpen} />{/if}{/each}

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
</style>
