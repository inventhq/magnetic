// ---------------------------------------------------------------------------
// Magnetic CSS — CSS Extraction (Server-Side Tree-Shaking)
// ---------------------------------------------------------------------------
//
// extractCSS(root, config)  → full CSS string (theme + reset + used utilities)
// createExtractor(config)   → reusable (root) => css function
//
// Walks the DomNode tree, collects all class names, resolves them against the
// utility map, and emits only the CSS rules that are actually used. Responsive
// prefixed classes (e.g. "md:row") are grouped into @media blocks.
// ---------------------------------------------------------------------------

import type { DesignConfig, DomNode, UtilityMap } from './types.ts';
import { compileTheme } from './theme.ts';
import { generateReset } from './reset.ts';
import { generateUtilities, containerMediaRules } from './utilities.ts';

// ---------------------------------------------------------------------------
// Tree-walk: collect all class names from a DomNode tree
// ---------------------------------------------------------------------------

function collectClasses(node: DomNode, out: Set<string>): void {
  if (node.attrs?.['class']) {
    const classes = node.attrs['class'].split(/\s+/);
    for (const cls of classes) {
      if (cls) out.add(cls);
    }
  }
  if (node.children) {
    for (const child of node.children) {
      collectClasses(child, out);
    }
  }
}

// ---------------------------------------------------------------------------
// Responsive prefix parsing
// ---------------------------------------------------------------------------

interface ResolvedClass {
  /** The original class name as written (e.g. "md:row"). */
  original: string;
  /** The base class name to look up in the utility map (e.g. "row"). */
  base: string;
  /** The breakpoint name, or null if not responsive (e.g. "md"). */
  breakpoint: string | null;
}

function parseClassName(
  cls: string,
  breakpointNames: Set<string>,
): ResolvedClass {
  const colonIdx = cls.indexOf(':');
  if (colonIdx > 0) {
    const prefix = cls.substring(0, colonIdx);
    if (breakpointNames.has(prefix)) {
      return {
        original: cls,
        base: cls.substring(colonIdx + 1),
        breakpoint: prefix,
      };
    }
  }
  return { original: cls, base: cls, breakpoint: null };
}

// ---------------------------------------------------------------------------
// CSS emission
// ---------------------------------------------------------------------------

function emitRule(className: string, declarations: string): string {
  // Escape special characters in class names (e.g. "2xl" → needs escaping)
  const escaped = escapeClassName(className);
  return `.${escaped}{${declarations}}`;
}

function escapeClassName(name: string): string {
  // CSS class selectors: escape leading digits and special chars
  return name.replace(
    /([^\w-])/g,
    (_, ch: string) => `\\${ch}`,
  ).replace(
    /^(\d)/,
    '\\3$1 ',
  );
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Generate the complete CSS for all utilities defined by the design config.
 *
 * This is the "superset" approach: emits theme vars + reset + every utility
 * class. Typically <10KB. Used for SSR so that subsequent SSE DOM snapshots
 * (which may introduce new classes on client-side navigation) already have
 * all styles available without needing a client-side <style> update.
 */
export function generateAllCSS(config: DesignConfig): string {
  const themeCSS = compileTheme(config);
  const resetCSS = generateReset();
  const utilities = generateUtilities(config);
  const breakpoints = config.theme.breakpoints;

  const containerRules = containerMediaRules(breakpoints);

  let css = themeCSS + resetCSS;

  // Emit all base utility rules
  for (const [name, declarations] of utilities) {
    css += emitRule(name, declarations);
  }

  // Container media queries
  css += containerRules;

  return css;
}

/**
 * Extract only the CSS needed for a rendered DomNode tree.
 *
 * Returns a single CSS string containing:
 *   1. Theme custom properties (:root + [data-theme="dark"])
 *   2. CSS reset / normalize
 *   3. Used utility classes (base rules)
 *   4. Container @media rules (if "container" is used)
 *   5. Responsive @media blocks (mobile-first, ascending)
 *
 * This function rebuilds the utility map on every call. For repeated use,
 * prefer `createExtractor()` which caches the map.
 */
export function extractCSS(root: DomNode, config: DesignConfig): string {
  const extractor = createExtractor(config);
  return extractor(root);
}

/**
 * Create a reusable CSS extraction function for a given design config.
 *
 * Pre-computes the theme CSS, reset CSS, and utility map once.
 * The returned function only does the tree-walk + lookup per call.
 */
export function createExtractor(
  config: DesignConfig,
): (root: DomNode) => string {
  const themeCSS = compileTheme(config);
  const resetCSS = generateReset();
  const utilities = generateUtilities(config);
  const breakpoints = config.theme.breakpoints;
  const breakpointNames = new Set(Object.keys(breakpoints));

  // Sort breakpoints by min-width ascending for mobile-first ordering
  const sortedBreakpoints = Object.entries(breakpoints)
    .map(([name, value]) => ({ name, value, px: parseFloat(value) }))
    .sort((a, b) => a.px - b.px);

  // Pre-compute container media rules
  const containerRules = containerMediaRules(breakpoints);

  return function extract(root: DomNode): string {
    // 1. Collect all class names from the tree
    const usedClassNames = new Set<string>();
    collectClasses(root, usedClassNames);

    // 2. Resolve each class: separate base vs responsive
    const baseRules: string[] = [];
    const responsiveRules = new Map<string, string[]>(); // breakpoint → rules[]
    const emittedBases = new Set<string>();
    let needsContainer = false;

    for (const cls of usedClassNames) {
      const parsed = parseClassName(cls, breakpointNames);
      const declarations = utilities.get(parsed.base);

      if (!declarations) continue; // Unknown class — skip silently

      if (parsed.breakpoint === null) {
        // Base class
        if (!emittedBases.has(parsed.base)) {
          emittedBases.add(parsed.base);
          baseRules.push(emitRule(cls, declarations));
          if (parsed.base === 'container') needsContainer = true;
        }
      } else {
        // Responsive class (e.g. "md:row")
        if (!responsiveRules.has(parsed.breakpoint)) {
          responsiveRules.set(parsed.breakpoint, []);
        }
        responsiveRules.get(parsed.breakpoint)!.push(
          emitRule(cls, declarations),
        );
      }
    }

    // 3. Assemble final CSS
    let css = themeCSS + resetCSS;

    // Base utility rules
    css += baseRules.join('');

    // Container responsive max-widths
    if (needsContainer) {
      css += containerRules;
    }

    // Responsive @media blocks (mobile-first order)
    for (const bp of sortedBreakpoints) {
      const rules = responsiveRules.get(bp.name);
      if (rules && rules.length > 0) {
        css += `@media(min-width:${bp.value}){${rules.join('')}}`;
      }
    }

    return css;
  };
}
