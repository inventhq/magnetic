// ---------------------------------------------------------------------------
// Magnetic CSS — Public API
// ---------------------------------------------------------------------------

export type {
  DesignConfig,
  ThemeConfig,
  TypographyConfig,
  ColorValue,
  SpacingScale,
  RadiusScale,
  ShadowScale,
  BreakpointScale,
  DomNode,
  UtilityMap,
  ResponsiveEntry,
} from './types.ts';

export { defaultDesign } from './defaults.ts';
export { compileTheme } from './theme.ts';
export { generateReset } from './reset.ts';
export { generateUtilities } from './utilities.ts';
export { generateAllCSS, extractCSS, createExtractor } from './extract.ts';
