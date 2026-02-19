// ---------------------------------------------------------------------------
// Magnetic CSS — Theme compiler (design.json → CSS custom properties)
// ---------------------------------------------------------------------------

import type { DesignConfig } from './types.ts';

/**
 * Compile a DesignConfig into CSS custom property declarations.
 *
 * Outputs:
 *   :root, [data-theme="light"] { --m-primary: #3b82f6; ... }
 *   [data-theme="dark"] { --m-surface: #1a1a2e; ... }
 */
export function compileTheme(config: DesignConfig): string {
  const { colors, spacing, radius, typography, shadows } = config.theme;

  const lightVars: string[] = [];
  const darkVars: string[] = [];

  // --- Colors ---
  for (const [name, value] of Object.entries(colors)) {
    if (typeof value === 'string') {
      // Flat color — same in both themes
      lightVars.push(`--m-${name}:${value}`);
    } else {
      // Light/dark pair
      lightVars.push(`--m-${name}:${value.light}`);
      darkVars.push(`--m-${name}:${value.dark}`);
    }
  }

  // --- Spacing ---
  for (const [name, value] of Object.entries(spacing)) {
    lightVars.push(`--m-space-${name}:${value}`);
  }

  // --- Radius ---
  for (const [name, value] of Object.entries(radius)) {
    lightVars.push(`--m-radius-${name}:${value}`);
  }

  // --- Typography: font families ---
  lightVars.push(`--m-font-sans:${typography.sans}`);
  lightVars.push(`--m-font-mono:${typography.mono}`);

  // --- Typography: font sizes ---
  for (const [name, value] of Object.entries(typography.sizes)) {
    lightVars.push(`--m-text-${name}:${value}`);
  }

  // --- Typography: line heights ---
  if (typography.leading) {
    for (const [name, value] of Object.entries(typography.leading)) {
      lightVars.push(`--m-leading-${name}:${value}`);
    }
  }

  // --- Shadows ---
  for (const [name, value] of Object.entries(shadows)) {
    lightVars.push(`--m-shadow-${name}:${value}`);
  }

  // --- Assemble ---
  let css = `:root,[data-theme="light"]{${lightVars.join(';')}}`;

  if (darkVars.length > 0) {
    css += `[data-theme="dark"]{${darkVars.join(';')}}`;
  }

  return css;
}
