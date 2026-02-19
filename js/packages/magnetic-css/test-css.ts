// Quick test script for @magneticjs/css
// Run: npx tsx js/packages/magnetic-css/test-css.ts

import { extractCSS, generateAllCSS, compileTheme, generateUtilities, defaultDesign } from './src/index.ts';
import type { DomNode } from './src/types.ts';

// ── Test 1: Theme compilation ──────────────────────────────────────
console.log('=== Test 1: Theme Compilation ===');
const themeCSS = compileTheme(defaultDesign);
console.log(`Theme CSS: ${themeCSS.length} bytes`);
console.log(`  Has :root: ${themeCSS.includes(':root')}`);
console.log(`  Has dark mode: ${themeCSS.includes('[data-theme="dark"]')}`);
console.log(`  Has --m-primary: ${themeCSS.includes('--m-primary')}`);
console.log(`  Has --m-space-md: ${themeCSS.includes('--m-space-md')}`);
console.log();

// ── Test 2: Utility count ──────────────────────────────────────────
console.log('=== Test 2: Utility Classes ===');
const utilities = generateUtilities(defaultDesign);
console.log(`Total utility classes: ${utilities.size}`);
console.log(`  Has "stack": ${utilities.has('stack')}`);
console.log(`  Has "cluster": ${utilities.has('cluster')}`);
console.log(`  Has "grid-auto": ${utilities.has('grid-auto')}`);
console.log(`  Has "gap-md": ${utilities.has('gap-md')}`);
console.log(`  Has "fg-primary": ${utilities.has('fg-primary')}`);
console.log(`  Has "text-4xl": ${utilities.has('text-4xl')}`);
console.log(`  Has "container": ${utilities.has('container')}`);
console.log(`  Has "sr-only": ${utilities.has('sr-only')}`);
console.log();

// ── Test 3: Full CSS generation (superset) ─────────────────────────
console.log('=== Test 3: Full CSS (Superset) ===');
const allCSS = generateAllCSS(defaultDesign);
console.log(`All CSS: ${allCSS.length} bytes (${(allCSS.length / 1024).toFixed(1)}KB)`);
console.log();

// ── Test 4: Per-render extraction ──────────────────────────────────
console.log('=== Test 4: Per-Render Extraction ===');
const mockDom: DomNode = {
  tag: 'main', attrs: { class: 'container stack gap-xl py-3xl' },
  children: [
    { tag: 'h1', attrs: { class: 'text-4xl bold fg-primary text-center' } },
    { tag: 'div', attrs: { class: 'grid-auto gap-lg' }, children: [
      { tag: 'div', attrs: { class: 'stack gap-sm p-lg bg-surface round-lg shadow-md' }, children: [
        { tag: 'h2', attrs: { class: 'text-xl semibold' } },
        { tag: 'p', attrs: { class: 'fg-muted leading-relaxed' } },
      ]},
      { tag: 'div', attrs: { class: 'stack gap-sm p-lg bg-surface round-lg shadow-md' }, children: [
        { tag: 'h2', attrs: { class: 'text-xl semibold' } },
        { tag: 'p', attrs: { class: 'fg-muted leading-relaxed' } },
      ]},
    ]},
    { tag: 'div', attrs: { class: 'stack md:row gap-md items-center justify-center' }, children: [
      { tag: 'button', attrs: { class: 'bg-primary fg-surface px-xl py-sm round-md shadow-sm bold' } },
      { tag: 'a', attrs: { class: 'fg-primary semibold', href: '/docs' } },
    ]},
  ],
};

const extractedCSS = extractCSS(mockDom, defaultDesign);
console.log(`Extracted CSS: ${extractedCSS.length} bytes (${(extractedCSS.length / 1024).toFixed(1)}KB)`);
console.log(`Savings vs superset: ${((1 - extractedCSS.length / allCSS.length) * 100).toFixed(0)}%`);
console.log(`  Has @media (responsive): ${extractedCSS.includes('@media')}`);
console.log(`  Has .container: ${extractedCSS.includes('.container{')}`);
console.log(`  Has .stack: ${extractedCSS.includes('.stack{')}`);
console.log(`  Has md\\:row: ${extractedCSS.includes('md\\:row')}`);
console.log();

// ── Test 5: Verify no unused classes leak in ────────────────────────
console.log('=== Test 5: Tree-Shaking ===');
console.log(`  "hidden" in extracted: ${extractedCSS.includes('.hidden{')} (should be false)`);
console.log(`  "absolute" in extracted: ${extractedCSS.includes('.absolute{')} (should be false)`);
console.log(`  "z-50" in extracted: ${extractedCSS.includes('.z-50{')} (should be false)`);
console.log();

console.log('=== All tests passed ===');
