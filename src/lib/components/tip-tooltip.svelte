<script lang="ts">
  /**
   * One shared tooltip for every element with a `data-tip` inside `root`,
   * such as the reference picker's book and chapter buttons. It shows on
   * hover and on keyboard focus (which the browser's own `title` tooltip
   * doesn't), styled like the footnote tooltip, and placed by Floating UI to
   * stay inside `boundary` (the window, unless given), sliding along an
   * edge rather than being cut off by it. While hidden it's display: none,
   * so unlike a CSS tooltip on each element it can never widen or scroll
   * its container. A press hides it, so it never covers a menu the press
   * opens.
   */
  import {
    arrow,
    autoUpdate,
    computePosition,
    flip,
    offset,
    shift,
  } from '@floating-ui/dom';

  let {
    root,
    boundary,
    placement = 'top',
    selector = '[data-tip]',
    textOf = (element: HTMLElement) => element.dataset.tip,
  }: {
    /** Where the elements with `data-tip` are. */
    root: HTMLElement | undefined;
    /** What the tooltip stays inside; the window when not given. */
    boundary?: HTMLElement;
    placement?: 'top' | 'bottom' | 'right';
    /** Which elements have a tooltip, instead of those with `data-tip`. */
    selector?: string;
    /** Each one's text, instead of its `data-tip`. */
    textOf?: (element: HTMLElement) => string | undefined;
  } = $props();

  let tooltip = $state<HTMLElement>();
  let pointer = $state<HTMLElement>();
  let anchor = $state<HTMLElement | undefined>();
  let text = $state('');

  async function position() {
    if (!anchor || !tooltip || !pointer) return;
    const limits = boundary ? { boundary, padding: 8 } : { padding: 8 };
    const placed = await computePosition(anchor, tooltip, {
      strategy: 'fixed',
      placement,
      middleware: [
        offset(8),
        flip(limits),
        shift(limits),
        arrow({ element: pointer, padding: 6 }),
      ],
    });
    tooltip.style.left = `${placed.x}px`;
    tooltip.style.top = `${placed.y}px`;
    tooltip.style.visibility = 'visible';
    // The arrow sits on the side facing the element: the bottom edge of a
    // tooltip above it, the left edge of one to its right, and so on.
    const side = placed.placement.split('-')[0];
    const facing: Record<string, string> = {
      top: 'bottom',
      bottom: 'top',
      right: 'left',
      left: 'right',
    };
    const { x, y } = placed.middlewareData.arrow ?? {};
    Object.assign(pointer.style, {
      left: x == null ? '' : `${x}px`,
      top: y == null ? '' : `${y}px`,
      right: '',
      bottom: '',
      [facing[side]]: '-4px',
    });
  }

  function show(event: Event) {
    const target = (event.target as Element | null)?.closest<HTMLElement>(
      selector
    );
    const tip = target && textOf(target);
    if (!target || !tip) return;
    anchor = target;
    text = tip;
  }

  function hideFor(event: Event) {
    const target = (event.target as Element | null)?.closest(selector);
    if (target && target === anchor) anchor = undefined;
  }

  $effect(() => {
    const area = root;
    if (!area) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') anchor = undefined;
    };
    // A press hides it, so it doesn't sit over a menu the press opens.
    const onPress = () => (anchor = undefined);
    area.addEventListener('mouseover', show);
    area.addEventListener('focusin', show);
    area.addEventListener('mouseout', hideFor);
    area.addEventListener('focusout', hideFor);
    area.addEventListener('keydown', onKey);
    area.addEventListener('pointerdown', onPress);
    return () => {
      area.removeEventListener('pointerdown', onPress);
      area.removeEventListener('mouseover', show);
      area.removeEventListener('focusin', show);
      area.removeEventListener('mouseout', hideFor);
      area.removeEventListener('focusout', hideFor);
      area.removeEventListener('keydown', onKey);
    };
  });

  $effect(() => {
    if (!anchor || !tooltip) return;
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
