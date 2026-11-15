import type { MenuSection } from '$lib/components/module-menu.svelte';
import type BibleTextService from './bible-text-service.svelte';
import type { ModuleSummary } from './module-folder-service.svelte';
import {
  notesTabKey,
  textTabKey,
  type ParallelPane,
} from './parallel-pane.svelte';

type Translate = (
  key: string,
  options?: { values?: Record<string, string> }
) => string;

/**
 * What can open in a search's parallel pane, for the pane's + and for the
 * Parallel button while the pane has no tabs: every Bible as a parallel
 * text, then every module with notes. One already open says so, and
 * picking it selects its tab. Picking opens the pane at the verse in view.
 */
export function parallelMenuSections(
  pane: ParallelPane,
  main: BibleTextService | undefined,
  modules: ModuleSummary[],
  t: Translate
): MenuSection[] {
  const openDetail = (key: string) =>
    pane.tabs.some((tab) => tab.key === key)
      ? t('module-menu.open')
      : undefined;
  return [
    {
      heading: t('module-menu.parallel-text'),
      items: modules
        .filter((module) => module.moduleType === 'bible')
        .map((module) => ({
          key: textTabKey(module.id),
          abbreviation: module.abbreviation,
          name: module.name,
          detail: openDetail(textTabKey(module.id)),
          pick: () => {
            const sid = main?.sidInView;
            if (sid) pane.openText(module.id, sid);
          },
        })),
    },
    {
      heading: t('module-menu.notes'),
      items: modules
        .filter((module) => module.hasNotes)
        .map((module) => ({
          key: notesTabKey(module.id),
          abbreviation: module.abbreviation,
          name: t('notes.tab', { values: { abbr: module.abbreviation } }),
          detail: openDetail(notesTabKey(module.id)),
          pick: () => {
            const sid = main?.sidInView;
            if (sid) pane.openNotes(module.id, sid);
          },
        })),
    },
  ];
}
