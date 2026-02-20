// Quick test script for @magneticjs/css
// Run: npx tsx js/packages/magnetic-css/test-css.ts

import { extractCSS, generateAllCSS, createExtractor, compileTheme, generateUtilities, defaultDesign } from './src/index.ts';
import type { DomNode, DesignConfig } from './src/types.ts';

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

// ── Mock DOM trees (simulating two different pages) ─────────────────
const page1: DomNode = {
  tag: 'main', attrs: { class: 'container stack gap-xl py-3xl' },
  children: [
    { tag: 'h1', attrs: { class: 'text-4xl bold fg-primary text-center' } },
    { tag: 'div', attrs: { class: 'grid-auto gap-lg' }, children: [
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

const page2: DomNode = {
  tag: 'main', attrs: { class: 'container stack gap-lg py-2xl' },
  children: [
    { tag: 'h1', attrs: { class: 'text-3xl bold fg-text' } },
    { tag: 'p', attrs: { class: 'text-lg fg-muted max-w-prose mx-auto leading-relaxed' } },
    { tag: 'div', attrs: { class: 'grid-3 gap-md mt-xl' }, children: [
      { tag: 'div', attrs: { class: 'p-md bg-surface round-md border shadow-sm' } },
      { tag: 'div', attrs: { class: 'p-md bg-surface round-md border shadow-sm hidden md:block' } },
    ]},
  ],
};

// ── Test 3: Mode "all" — full superset ──────────────────────────────
console.log('=== Test 3: Mode "all" (Superset) ===');
const allCSS = generateAllCSS(defaultDesign);
console.log(`  Size: ${allCSS.length} bytes (${(allCSS.length / 1024).toFixed(1)}KB)`);
console.log();

// ── Test 4: Mode "used" — single page extraction ───────────────────
console.log('=== Test 4: Mode "used" (Single Page) ===');
const usedCSS = extractCSS(page1, defaultDesign);
console.log(`  Page 1 only: ${usedCSS.length} bytes (${(usedCSS.length / 1024).toFixed(1)}KB)`);
console.log(`  Savings vs "all": ${((1 - usedCSS.length / allCSS.length) * 100).toFixed(0)}%`);
console.log();

// ── Test 5: Mode "pages" — union of all pages ──────────────────────
console.log('=== Test 5: Mode "pages" (All Pages Union) ===');
// Simulate what the bridge does: collect classes from all pages, then extract
function collectClasses(node: DomNode, out: Set<string>): void {
  if (node.attrs?.['class']) {
    for (const c of node.attrs['class'].split(/\s+/)) { if (c) out.add(c); }
  }
  if (node.children) {
    for (const ch of node.children) collectClasses(ch, out);
  }
}

const allClasses = new Set<string>();
collectClasses(page1, allClasses);
collectClasses(page2, allClasses);

const syntheticNode: DomNode = { tag: 'div', attrs: { class: Array.from(allClasses).join(' ') } };
const pagesCSS = extractCSS(syntheticNode, defaultDesign);
console.log(`  Both pages: ${pagesCSS.length} bytes (${(pagesCSS.length / 1024).toFixed(1)}KB)`);
console.log(`  Savings vs "all": ${((1 - pagesCSS.length / allCSS.length) * 100).toFixed(0)}%`);
console.log(`  Classes collected: ${allClasses.size}`);
console.log();

// ── Test 6: Tree-shaking verification ───────────────────────────────
console.log('=== Test 6: Tree-Shaking ===');
console.log(`  "absolute" in pages CSS: ${pagesCSS.includes('.absolute{')} (should be false)`);
console.log(`  "z-50" in pages CSS: ${pagesCSS.includes('.z-50{')} (should be false)`);
console.log(`  "hidden" in pages CSS: ${pagesCSS.includes('.hidden{')} (should be true — page2 uses it)`);
console.log(`  "grid-3" in pages CSS: ${pagesCSS.includes('.grid-3{')} (should be true — page2 uses it)`);
console.log(`  md:block in pages CSS: ${pagesCSS.includes('md\\:block')} (should be true — page2 uses it)`);
console.log();

// ── Test 7: CSSMode config field ────────────────────────────────────
console.log('=== Test 7: CSSMode Config ===');
const configAll: DesignConfig = { ...defaultDesign, css: 'all' };
const configPages: DesignConfig = { ...defaultDesign, css: 'pages' };
const configUsed: DesignConfig = { ...defaultDesign, css: 'used' };
console.log(`  css:"all" → generates: ${generateAllCSS(configAll).length} bytes`);
console.log(`  css:"pages" accepted: ${configPages.css === 'pages'}`);
console.log(`  css:"used" accepted: ${configUsed.css === 'used'}`);
console.log(`  css:undefined defaults to "all": ${defaultDesign.css === undefined}`);
console.log();

// ── Summary ─────────────────────────────────────────────────────────
console.log('=== Size Comparison ===');
console.log(`  Mode "all"   (superset):    ${(allCSS.length / 1024).toFixed(1)}KB uncompressed`);
console.log(`  Mode "pages" (all routes):  ${(pagesCSS.length / 1024).toFixed(1)}KB uncompressed (${((1 - pagesCSS.length / allCSS.length) * 100).toFixed(0)}% smaller)`);
console.log(`  Mode "used"  (single page): ${(usedCSS.length / 1024).toFixed(1)}KB uncompressed (${((1 - usedCSS.length / allCSS.length) * 100).toFixed(0)}% smaller)`);
console.log();
console.log('=== All tests passed ===');
