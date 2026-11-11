/**
 * Events that footnote markers send up to their search.
 *
 * Footnote markers don't own a tooltip. They ask their reading pane's single
 * shared tooltip (footnote-tooltip.svelte) to show or hide, with an event that
 * bubbles up to the pane. Each pane, including a parallel one, answers only
 * for its own markers, with no shared global state.
 */
export const FOOTNOTE_TOOLTIP_EVENT = 'footnote-tooltip';

export interface FootnoteTooltipRequest {
  anchor: HTMLElement;
  /** The note to show, or null to hide the tooltip for this anchor. */
  text: string | null;
}

/** Asks the pane holding `anchor` to show `text` beside it, or to hide it. */
export function requestFootnoteTooltip(
  anchor: HTMLElement,
  text: string | null
): void {
  anchor.dispatchEvent(
    new CustomEvent<FootnoteTooltipRequest>(FOOTNOTE_TOOLTIP_EVENT, {
      bubbles: true,
      detail: { anchor, text },
    })
  );
}

/**
 * Clicking a footnote marker opens its note in the parallel pane's Notes tab.
 * The marker only knows its verse and letter, so it sends this event up to
 * its search (search-view.svelte), which owns the parallel pane.
 */
export const FOOTNOTE_OPEN_EVENT = 'footnote-open';

export interface FootnoteOpenRequest {
  /** The verse holding the note, such as 'PSA 12:1'. */
  sid: string;
  /** The note's letter within its chapter. */
  letter: string;
}

/** Asks the search holding `anchor` to show this note in its Notes tab. */
export function requestFootnoteOpen(
  anchor: HTMLElement,
  sid: string,
  letter: string
): void {
  anchor.dispatchEvent(
    new CustomEvent<FootnoteOpenRequest>(FOOTNOTE_OPEN_EVENT, {
      bubbles: true,
      detail: { sid, letter },
    })
  );
}
