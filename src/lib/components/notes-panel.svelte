<script lang="ts">
  /**
   * A Notes tab: every footnote in one chapter, lettered as in the text. The
   * note that was clicked is scrolled to the top and highlighted. Each note's
   * heading takes the main pane to its verse.
   */
  import ModuleMessage from './module-message.svelte';
  import { tick, untrack } from 'svelte';
  import { _ } from 'svelte-i18n';
  import type { ChapterNote } from '$lib/model/verse';
  import type BibleTextService from '$lib/services/bible-text-service.svelte';
  import { chapterOf, type NotesTab } from '$lib/services/parallel-pane.svelte';
  import { noteText } from '$lib/services/verse-content';
  import ModuleDbService from '$lib/services/module-db-service.svelte';

  let {
    tab,
    service,
    textMaxWidth,
    onFocusNote,
  }: {
    tab: NotesTab;
    /** The main pane's text, which the notes' headings take the reader to. */
    service: BibleTextService;
    textMaxWidth: string;
    onFocusNote: (letter: string) => void;
  } = $props();

  let notes = $state<ChapterNote[]>([]);
  let loadedChapter = $state<string | undefined>();
  let error = $state<unknown>();
  let list = $state<HTMLElement>();

  /** The notes come from the tab's own module, which may not be the main text's. */
  const notesDb = $derived(new ModuleDbService(tab.moduleId));
  const chapter = $derived(`${tab.moduleId} ${chapterOf(tab.sid)}`);
  const chapterName = $derived(
    service.formatChapterFromSid(tab.sid) || chapterOf(tab.sid)
  );

  // Load the chapter's notes whenever the tab moves to another chapter.
  $effect(() => {
    const wanted = chapter;
    const sid = tab.sid;
    untrack(() => {
      if (wanted === loadedChapter) return;
      error = undefined;
      notesDb
        .getChapterNotes(sid)
        .then((found) => {
          // Ignore an answer for a chapter the tab has already left.
          if (`${tab.moduleId} ${chapterOf(tab.sid)}` !== wanted) return;
          notes = found;
          loadedChapter = wanted;
        })
        .catch((err) => (error = err));
    });
  });

  // When asked (scrollRequest: a footnote was clicked, or the tab followed
  // the reader to another chapter), scroll the highlighted note to the top,
  // or the list to its start if this chapter has none highlighted. Clicking
  // a heading here only changes the highlight, so it's read untracked.
  $effect(() => {
    void tab.scrollRequest;
    if (loadedChapter !== chapter || !list) return;
    untrack(async () => {
      await tick();
      const target = list?.querySelector<HTMLElement>('[data-focused]');
      if (!list) return;
      list.scrollTo({ top: target ? target.offsetTop - 8 : 0 });
    });
  });

  function goTo(note: ChapterNote) {
    onFocusNote(note.letter);
    service.goToSid(note.sid);
  }
</script>

<div
  class="border-b border-base-content/10 px-3.5 py-1.5 text-xs text-base-content/60"
>
  {$_('notes.heading', {
    values: { abbr: service.moduleAbbreviation, chapter: chapterName },
  })}
</div>
<div
  bind:this={list}
  class="relative min-h-0 flex-1 overflow-y-auto px-3 pt-2.5 pb-6"
>
  <div class="mx-auto flex flex-col gap-1.5" style:max-width={textMaxWidth}>
    {#if error}
      <div role="alert" class="alert alert-error whitespace-normal">
        <ModuleMessage {error} />
      </div>
    {:else if loadedChapter !== chapter}
      <p class="p-2 text-sm text-base-content/60">{$_('notes.loading')}</p>
    {:else if notes.length === 0}
      <p class="p-2 text-sm text-base-content/60">
        {$_('notes.none', { values: { chapter: chapterName } })}
      </p>
    {:else}
      {#each notes as note (`${note.sid}|${note.letter}`)}
        <!-- Highlighted with its verse, so a new search clears both. -->
        {@const focused =
          note.letter === tab.focusLetter && note.sid === service.highlightSid}
        <div
          data-focused={focused || undefined}
          class={['rounded-lg px-2.5 py-2', focused && 'pair-highlight']}
        >
          <button
            type="button"
            class="link link-primary link-hover cursor-pointer text-sm font-semibold"
            onclick={() => goTo(note)}
          >
            {service.formatRefFromSid(note.sid)}
            <span class="font-medium text-base-content">{note.letter}</span>
          </button>
          <p class="mt-1 text-sm leading-normal">{noteText(note.note)}</p>
        </div>
      {/each}
    {/if}
  </div>
</div>
