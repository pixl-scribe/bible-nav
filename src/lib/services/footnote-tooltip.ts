/**
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
