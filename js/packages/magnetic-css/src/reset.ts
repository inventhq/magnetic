// ---------------------------------------------------------------------------
// Magnetic CSS — Minimal CSS reset / normalize (~400 bytes)
// ---------------------------------------------------------------------------

/**
 * Generate a minimal CSS reset that uses theme custom properties.
 *
 * - box-sizing: border-box globally (cross-browser fix)
 * - 100dvh instead of 100vh (fixes iOS viewport issue)
 * - System font stack via CSS custom property
 * - Color + background from theme vars → dark mode works automatically
 * - Images responsive by default
 * - Form elements inherit font
 * - Headings reset (styled via utility classes)
 */
export function generateReset(): string {
  return [
    '*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}',
    'html{-webkit-text-size-adjust:100%;tab-size:4;font-family:var(--m-font-sans);line-height:1.5;color:var(--m-text);background:var(--m-surface)}',
    'body{min-height:100dvh}',
    'img,svg,video,canvas{display:block;max-width:100%}',
    'input,button,textarea,select{font:inherit}',
    'a{color:inherit;text-decoration:inherit}',
    'h1,h2,h3,h4,h5,h6{font-size:inherit;font-weight:inherit}',
    'button{cursor:pointer;background:none;border:none}',
    'ol,ul{list-style:none}',
    'table{border-collapse:collapse}',
    ':focus-visible{outline:2px solid var(--m-primary);outline-offset:2px}',
  ].join('');
}
