<script lang="ts">
  /**
   * The one footnote tooltip for a reading pane. It moves to whichever marker
   * is hovered or focused, and Floating UI keeps it inside the pane: `flip`
   * puts it above the marker when there's no room below, `shift` slides it
   * sideways just enough to stay within the pane, and `hide` hides it if the
   * marker scrolls out of view. It's display: none while hidden, so it can't
   * widen the pane, and it only tracks scrolling while it's shown.
   */
  import {
    arrow,
    autoUpdate,
    computePosition,
    flip,
    hide,
    offset,
    shift,
  } from '@floating-ui/dom';
  import {
    FOOTNOTE_TOOLTIP_EVENT,
    type FootnoteTooltipRequest,
  } from '$lib/services/footnote-tooltip';

  let { pane }: { pane: HTMLElement | undefined } = $props();

  let tooltip = $state<HTMLElement>();
  let pointer = $state<HTMLElement>();
  let anchor = $state<HTMLElement | undefined>();
  let text = $state('');

  /** Keep this much space between the tooltip and the pane's edges. */
  const EDGE_PADDING = 8;

  async function position() {
    if (!anchor || !tooltip || !pointer || !pane) return;
    const placed = await computePosition(anchor, tooltip, {
      strategy: 'fixed',
      placement: 'bottom',
      middleware: [
        offset(8),
        flip({ boundary: pane, padding: EDGE_PADDING }),
        shift({ boundary: pane, padding: EDGE_PADDING }),
        arrow({ element: pointer, padding: 6 }),
        hide({ boundary: pane }),
      ],
    });
    const below = placed.placement.startsWith('bottom');
    tooltip.style.left = `${placed.x}px`;
    tooltip.style.top = `${placed.y}px`;
    tooltip.style.visibility = placed.middlewareData.hide?.referenceHidden
      ? 'hidden'
      : 'visible';
    pointer.style.left = `${placed.middlewareData.arrow?.x ?? 0}px`;
    pointer.style.top = below ? '-4px' : '';
    pointer.style.bottom = below ? '' : '-4px';
  }

  // Answer show and hide requests from markers inside this pane.
  $effect(() => {
    if (!pane) return;
    const onRequest = (event: Event) => {
      const request = (event as CustomEvent<FootnoteTooltipRequest>).detail;
      if (request.text !== null) {
        anchor = request.anchor;
        text = request.text;
      } else if (request.anchor === anchor) {
        anchor = undefined;
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') anchor = undefined;
    };
    pane.addEventListener(FOOTNOTE_TOOLTIP_EVENT, onRequest);
    pane.addEventListener('keydown', onKey);
    return () => {
      pane.removeEventListener(FOOTNOTE_TOOLTIP_EVENT, onRequest);
      pane.removeEventListener('keydown', onKey);
    };
  });

  // While shown, keep it positioned as the pane scrolls or resizes.
  $effect(() => {
    if (!anchor || !tooltip) return;
    // Start hidden at the new spot until it's measured, so it doesn't flash
    // where the previous note was.
    tooltip.style.visibility = 'hidden';
    return autoUpdate(anchor, tooltip, position);
  });
</script>

<div
  bind:this={tooltip}
  role="tooltip"
  class="pointer-events-none fixed top-0 left-0 z-50 w-max max-w-xs rounded bg-neutral px-2 py-1 text-sm leading-snug text-neutral-content shadow-lg"
  class:hidden={!anchor}
>
  {text}
  <div
    bind:this={pointer}
    class="absolute h-2 w-2 rotate-45 bg-neutral"
    aria-hidden="true"
  ></div>
</div>
