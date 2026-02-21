// ---------------------------------------------------------------------------
// Magnetic CSS — Utility class generator
// ---------------------------------------------------------------------------
//
// generateUtilities(config) → Map<className, cssDeclarationBlock>
//
// Each entry maps a class name (e.g. "stack", "gap-md", "fg-primary") to its
// CSS declaration block (without the selector wrapper). The extractor uses this
// map to emit only the rules actually referenced in a DomNode tree.
// ---------------------------------------------------------------------------

import type { DesignConfig, UtilityMap } from './types.ts';

// ---------------------------------------------------------------------------
// Fluid clamp helpers
// ---------------------------------------------------------------------------

/**
 * Build a clamp() value for fluid typography.
 * min/max in rem, preferred uses vw so it scales smoothly.
 */
function fluidSize(minRem: number, maxRem: number): string {
  // preferred = minRem + (maxRem - minRem) scaled by viewport
  const delta = maxRem - minRem;
  const vwCoeff = +(delta * 2).toFixed(3);  // ~2vw per rem of range
  const base = +(minRem - vwCoeff * 0.01 * 320 / 16).toFixed(3);
  return `clamp(${minRem}rem, ${base}rem + ${vwCoeff}vw, ${maxRem}rem)`;
}

// ---------------------------------------------------------------------------
// Main generator
// ---------------------------------------------------------------------------

/**
 * Generate the full utility class map from a DesignConfig.
 *
 * Returns a Map where:
 *   key   = class name (e.g. "stack", "p-md", "fg-primary")
 *   value = CSS declaration block (e.g. "display:flex;flex-direction:column")
 *
 * Responsive variants ({bp}:{class}) are NOT in this map — they are resolved
 * at extraction time by looking up the base class and wrapping in @media.
 */
export function generateUtilities(config: DesignConfig): UtilityMap {
  const map: UtilityMap = new Map();
  const { colors, spacing, radius, typography, shadows, breakpoints = {} } = config.theme;

  // -----------------------------------------------------------------------
  // Layout primitives
  // -----------------------------------------------------------------------
  map.set('stack', 'display:flex;flex-direction:column');
  map.set('row', 'display:flex;flex-direction:row');
  map.set('cluster', 'display:flex;flex-wrap:wrap;align-items:center');
  map.set('center', 'display:flex;align-items:center;justify-content:center');
  map.set('wrap', 'flex-wrap:wrap');
  map.set('no-wrap', 'flex-wrap:nowrap');

  // Grid — auto-responsive (configurable via --min-w CSS variable)
  map.set('grid-auto', 'display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,var(--min-w,16rem)),1fr))');

  // Grid — fixed columns
  for (let n = 2; n <= 6; n++) {
    map.set(`grid-${n}`, `display:grid;grid-template-columns:repeat(${n},1fr)`);
  }

  // -----------------------------------------------------------------------
  // Gap
  // -----------------------------------------------------------------------
  for (const name of Object.keys(spacing)) {
    map.set(`gap-${name}`, `gap:var(--m-space-${name})`);
  }

  // -----------------------------------------------------------------------
  // Padding
  // -----------------------------------------------------------------------
  for (const name of Object.keys(spacing)) {
    map.set(`p-${name}`, `padding:var(--m-space-${name})`);
    map.set(`px-${name}`, `padding-inline:var(--m-space-${name})`);
    map.set(`py-${name}`, `padding-block:var(--m-space-${name})`);
    map.set(`pt-${name}`, `padding-top:var(--m-space-${name})`);
    map.set(`pr-${name}`, `padding-right:var(--m-space-${name})`);
    map.set(`pb-${name}`, `padding-bottom:var(--m-space-${name})`);
    map.set(`pl-${name}`, `padding-left:var(--m-space-${name})`);
  }

  // -----------------------------------------------------------------------
  // Margin
  // -----------------------------------------------------------------------
  for (const name of Object.keys(spacing)) {
    map.set(`m-${name}`, `margin:var(--m-space-${name})`);
    map.set(`mx-${name}`, `margin-inline:var(--m-space-${name})`);
    map.set(`my-${name}`, `margin-block:var(--m-space-${name})`);
    map.set(`mt-${name}`, `margin-top:var(--m-space-${name})`);
    map.set(`mr-${name}`, `margin-right:var(--m-space-${name})`);
    map.set(`mb-${name}`, `margin-bottom:var(--m-space-${name})`);
    map.set(`ml-${name}`, `margin-left:var(--m-space-${name})`);
  }
  map.set('mx-auto', 'margin-inline:auto');

  // -----------------------------------------------------------------------
  // Typography — font sizes (fluid for lg+)
  // -----------------------------------------------------------------------
  const fluidThreshold = 1.125; // rem — sizes >= this get clamp()
  for (const [name, value] of Object.entries(typography.sizes)) {
    const rem = parseFloat(value);
    if (rem >= fluidThreshold) {
      // Fluid: scales between ~85% of value and full value
      const minRem = +(rem * 0.85).toFixed(3);
      map.set(`text-${name}`, `font-size:${fluidSize(minRem, rem)}`);
    } else {
      // Static small sizes
      map.set(`text-${name}`, `font-size:var(--m-text-${name})`);
    }
  }

  // Typography — font families
  map.set('font-sans', 'font-family:var(--m-font-sans)');
  map.set('font-mono', 'font-family:var(--m-font-mono)');

  // Typography — font weights
  map.set('thin', 'font-weight:100');
  map.set('light', 'font-weight:300');
  map.set('normal', 'font-weight:400');
  map.set('medium', 'font-weight:500');
  map.set('semibold', 'font-weight:600');
  map.set('bold', 'font-weight:700');
  map.set('extrabold', 'font-weight:800');

  // Typography — style
  map.set('italic', 'font-style:italic');
  map.set('not-italic', 'font-style:normal');

  // Typography — line height
  if (typography.leading) {
    for (const [name, value] of Object.entries(typography.leading)) {
      map.set(`leading-${name}`, `line-height:${value}`);
    }
  }

  // Typography — letter spacing
  map.set('tracking-tighter', 'letter-spacing:-0.05em');
  map.set('tracking-tight', 'letter-spacing:-0.025em');
  map.set('tracking-normal', 'letter-spacing:0');
  map.set('tracking-wide', 'letter-spacing:0.025em');
  map.set('tracking-wider', 'letter-spacing:0.05em');

  // Typography — text transform
  map.set('uppercase', 'text-transform:uppercase');
  map.set('lowercase', 'text-transform:lowercase');
  map.set('capitalize', 'text-transform:capitalize');
  map.set('normal-case', 'text-transform:none');

  // Typography — text alignment
  map.set('text-left', 'text-align:left');
  map.set('text-center', 'text-align:center');
  map.set('text-right', 'text-align:right');
  map.set('text-justify', 'text-align:justify');

  // Typography — text decoration
  map.set('underline', 'text-decoration:underline');
  map.set('line-through', 'text-decoration:line-through');
  map.set('no-underline', 'text-decoration:none');

  // Typography — text overflow
  map.set('truncate', 'overflow:hidden;text-overflow:ellipsis;white-space:nowrap');
  map.set('break-words', 'overflow-wrap:break-word');

  // -----------------------------------------------------------------------
  // Colors
  // -----------------------------------------------------------------------
  for (const name of Object.keys(colors)) {
    map.set(`fg-${name}`, `color:var(--m-${name})`);
    map.set(`bg-${name}`, `background-color:var(--m-${name})`);
    map.set(`border-${name}`, `border-color:var(--m-${name})`);
  }

  // -----------------------------------------------------------------------
  // Borders
  // -----------------------------------------------------------------------
  map.set('border', 'border:1px solid var(--m-border)');
  map.set('border-t', 'border-top:1px solid var(--m-border)');
  map.set('border-r', 'border-right:1px solid var(--m-border)');
  map.set('border-b', 'border-bottom:1px solid var(--m-border)');
  map.set('border-l', 'border-left:1px solid var(--m-border)');
  map.set('border-none', 'border:none');

  // -----------------------------------------------------------------------
  // Border radius
  // -----------------------------------------------------------------------
  for (const [name, _value] of Object.entries(radius)) {
    map.set(`round-${name}`, `border-radius:var(--m-radius-${name})`);
  }
  map.set('round-none', 'border-radius:0');

  // -----------------------------------------------------------------------
  // Shadows
  // -----------------------------------------------------------------------
  for (const [name, _value] of Object.entries(shadows)) {
    map.set(`shadow-${name}`, `box-shadow:var(--m-shadow-${name})`);
  }
  map.set('shadow-none', 'box-shadow:none');

  // -----------------------------------------------------------------------
  // Sizing
  // -----------------------------------------------------------------------
  map.set('w-full', 'width:100%');
  map.set('w-screen', 'width:100vw');
  map.set('w-auto', 'width:auto');
  map.set('h-full', 'height:100%');
  map.set('h-screen', 'height:100dvh');
  map.set('h-auto', 'height:auto');
  map.set('min-h-screen', 'min-height:100dvh');
  map.set('min-h-full', 'min-height:100%');
  map.set('min-w-0', 'min-width:0');

  // Max-width from breakpoints
  for (const [name, value] of Object.entries(breakpoints)) {
    map.set(`max-w-${name}`, `max-width:${value}`);
  }
  map.set('max-w-none', 'max-width:none');
  map.set('max-w-prose', 'max-width:65ch');

  // -----------------------------------------------------------------------
  // Aspect ratio
  // -----------------------------------------------------------------------
  map.set('aspect-auto', 'aspect-ratio:auto');
  map.set('aspect-square', 'aspect-ratio:1');
  map.set('aspect-video', 'aspect-ratio:16/9');
  map.set('aspect-photo', 'aspect-ratio:4/3');
  map.set('aspect-wide', 'aspect-ratio:21/9');

  // -----------------------------------------------------------------------
  // Flexbox utilities
  // -----------------------------------------------------------------------
  map.set('grow', 'flex-grow:1');
  map.set('grow-0', 'flex-grow:0');
  map.set('shrink', 'flex-shrink:1');
  map.set('shrink-0', 'flex-shrink:0');

  // Align items
  map.set('items-start', 'align-items:flex-start');
  map.set('items-center', 'align-items:center');
  map.set('items-end', 'align-items:flex-end');
  map.set('items-stretch', 'align-items:stretch');
  map.set('items-baseline', 'align-items:baseline');

  // Justify content
  map.set('justify-start', 'justify-content:flex-start');
  map.set('justify-center', 'justify-content:center');
  map.set('justify-end', 'justify-content:flex-end');
  map.set('justify-between', 'justify-content:space-between');
  map.set('justify-around', 'justify-content:space-around');
  map.set('justify-evenly', 'justify-content:space-evenly');

  // Align self
  map.set('self-auto', 'align-self:auto');
  map.set('self-start', 'align-self:flex-start');
  map.set('self-center', 'align-self:center');
  map.set('self-end', 'align-self:flex-end');
  map.set('self-stretch', 'align-self:stretch');

  // -----------------------------------------------------------------------
  // Container (auto-responsive with embedded media queries)
  // Emitted as a special case — the declaration here is the base only.
  // The extractor handles adding the @media rules for max-width.
  // -----------------------------------------------------------------------
  map.set('container', 'width:100%;margin-inline:auto;padding-inline:var(--m-space-md)');

  // -----------------------------------------------------------------------
  // Display
  // -----------------------------------------------------------------------
  map.set('hidden', 'display:none');
  map.set('block', 'display:block');
  map.set('inline', 'display:inline');
  map.set('inline-block', 'display:inline-block');
  map.set('flex', 'display:flex');
  map.set('inline-flex', 'display:inline-flex');
  map.set('grid', 'display:grid');
  map.set('inline-grid', 'display:inline-grid');
  map.set('contents', 'display:contents');

  // -----------------------------------------------------------------------
  // Position
  // -----------------------------------------------------------------------
  map.set('relative', 'position:relative');
  map.set('absolute', 'position:absolute');
  map.set('fixed', 'position:fixed');
  map.set('sticky', 'position:sticky');
  map.set('static', 'position:static');

  // Inset
  map.set('inset-0', 'inset:0');
  map.set('top-0', 'top:0');
  map.set('right-0', 'right:0');
  map.set('bottom-0', 'bottom:0');
  map.set('left-0', 'left:0');

  // -----------------------------------------------------------------------
  // Overflow
  // -----------------------------------------------------------------------
  map.set('overflow-hidden', 'overflow:hidden');
  map.set('overflow-auto', 'overflow:auto');
  map.set('overflow-scroll', 'overflow:scroll');
  map.set('overflow-visible', 'overflow:visible');
  map.set('overflow-x-auto', 'overflow-x:auto');
  map.set('overflow-y-auto', 'overflow-y:auto');
  map.set('overflow-x-hidden', 'overflow-x:hidden');
  map.set('overflow-y-hidden', 'overflow-y:hidden');

  // -----------------------------------------------------------------------
  // Z-index
  // -----------------------------------------------------------------------
  map.set('z-0', 'z-index:0');
  map.set('z-10', 'z-index:10');
  map.set('z-20', 'z-index:20');
  map.set('z-30', 'z-index:30');
  map.set('z-40', 'z-index:40');
  map.set('z-50', 'z-index:50');
  map.set('z-auto', 'z-index:auto');

  // -----------------------------------------------------------------------
  // Cursor
  // -----------------------------------------------------------------------
  map.set('cursor-pointer', 'cursor:pointer');
  map.set('cursor-default', 'cursor:default');
  map.set('cursor-not-allowed', 'cursor:not-allowed');
  map.set('cursor-wait', 'cursor:wait');
  map.set('cursor-text', 'cursor:text');
  map.set('cursor-grab', 'cursor:grab');

  // -----------------------------------------------------------------------
  // Opacity
  // -----------------------------------------------------------------------
  map.set('opacity-0', 'opacity:0');
  map.set('opacity-25', 'opacity:0.25');
  map.set('opacity-50', 'opacity:0.5');
  map.set('opacity-75', 'opacity:0.75');
  map.set('opacity-100', 'opacity:1');

  // -----------------------------------------------------------------------
  // Pointer events
  // -----------------------------------------------------------------------
  map.set('pointer-events-none', 'pointer-events:none');
  map.set('pointer-events-auto', 'pointer-events:auto');

  // -----------------------------------------------------------------------
  // User select
  // -----------------------------------------------------------------------
  map.set('select-none', 'user-select:none');
  map.set('select-text', 'user-select:text');
  map.set('select-all', 'user-select:all');
  map.set('select-auto', 'user-select:auto');

  // -----------------------------------------------------------------------
  // Accessibility
  // -----------------------------------------------------------------------
  map.set('sr-only', 'position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0');
  map.set('not-sr-only', 'position:static;width:auto;height:auto;padding:0;margin:0;overflow:visible;clip:auto;white-space:normal');

  // -----------------------------------------------------------------------
  // Transitions (opt-in, lightweight)
  // -----------------------------------------------------------------------
  map.set('transition', 'transition:all 150ms ease');
  map.set('transition-colors', 'transition:color,background-color,border-color 150ms ease');
  map.set('transition-opacity', 'transition:opacity 150ms ease');
  map.set('transition-shadow', 'transition:box-shadow 150ms ease');
  map.set('transition-transform', 'transition:transform 150ms ease');
  map.set('transition-none', 'transition:none');

  return map;
}

// ---------------------------------------------------------------------------
// Container media query helper (used by the extractor)
// ---------------------------------------------------------------------------

/**
 * Generate the @media container rules for a given set of breakpoints.
 * Called by the extractor when "container" is in the used classes.
 */
export function containerMediaRules(breakpoints: Record<string, string>): string {
  let css = '';
  // Sort breakpoints by value ascending
  const sorted = Object.entries(breakpoints)
    .map(([name, value]) => [name, value, parseFloat(value)] as const)
    .sort((a, b) => a[2] - b[2]);

  for (const [_name, value] of sorted) {
    css += `@media(min-width:${value}){.container{max-width:${value}}}`;
  }
  return css;
}
