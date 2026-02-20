// ---------------------------------------------------------------------------
// Magnetic CSS — Type definitions for design.json and internal structures
// ---------------------------------------------------------------------------

/** A color value: either a flat hex string or a light/dark pair. */
export type ColorValue = string | { light: string; dark: string };

/** Spacing scale tokens (e.g. xs, sm, md, lg, xl, 2xl, 3xl). */
export type SpacingScale = Record<string, string>;

/** Border radius tokens. */
export type RadiusScale = Record<string, string>;

/** Shadow tokens. */
export type ShadowScale = Record<string, string>;

/** Breakpoint tokens (name → min-width value). */
export type BreakpointScale = Record<string, string>;

/** Typography configuration. */
export interface TypographyConfig {
  /** Primary font stack (e.g. "Inter, system-ui, sans-serif"). */
  sans: string;
  /** Monospace font stack. */
  mono: string;
  /** Font size tokens (name → rem value). */
  sizes: Record<string, string>;
  /** Line-height tokens. */
  leading?: Record<string, string>;
}

/** Theme section of design.json. */
export interface ThemeConfig {
  colors: Record<string, ColorValue>;
  spacing: SpacingScale;
  radius: RadiusScale;
  typography: TypographyConfig;
  shadows: ShadowScale;
  breakpoints: BreakpointScale;
}

/**
 * CSS generation mode (set in design.json → "css" field).
 *
 * - "all"   — emit every utility class (~13KB). Safest for SSE. (default)
 * - "pages" — render all routes at init, emit only classes that appear (~3-5KB).
 *             Still SSE-safe because every navigable page is scanned.
 * - "used"  — per-request extraction, emit only classes in the current page (~2-3KB).
 *             Smallest output but SSE updates may reference unstyled classes.
 */
export type CSSMode = 'all' | 'pages' | 'used';

/** Root design.json structure. */
export interface DesignConfig {
  /** CSS generation mode. Default: "all". */
  css?: CSSMode;
  theme: ThemeConfig;
}

// ---------------------------------------------------------------------------
// DomNode (mirrors @magneticjs/server — kept minimal to avoid dependency)
// ---------------------------------------------------------------------------

/** Minimal DomNode interface for CSS extraction (no dependency on @magneticjs/server). */
export interface DomNode {
  tag: string;
  key?: string;
  attrs?: Record<string, string>;
  events?: Record<string, string>;
  text?: string;
  children?: DomNode[];
}

// ---------------------------------------------------------------------------
// Utility map types
// ---------------------------------------------------------------------------

/** A map of class name → CSS declaration block (without selector). */
export type UtilityMap = Map<string, string>;

/**
 * A responsive utility entry: the base class name minus the breakpoint prefix,
 * plus the breakpoint name it belongs to.
 */
export interface ResponsiveEntry {
  breakpoint: string;
  className: string;
  declarations: string;
}
