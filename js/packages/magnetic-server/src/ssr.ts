// @magnetic/server — SSR (Server-Side Rendering)
// Converts DomNode tree → HTML string for first paint + SEO

import type { DomNode } from './jsx-runtime.ts';

// ── HTML escaping ───────────────────────────────────────────────────

const ESC: Record<string, string> = {
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
};

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ESC[c]);
}

// ── Void elements (self-closing, no children) ───────────────────────

const VOID = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
  'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

// ── Head extraction ─────────────────────────────────────────────────

export interface ExtractedHead {
  /** DomNode tree with magnetic:head nodes removed */
  body: DomNode;
  /** Collected head elements from all <Head> components */
  headNodes: DomNode[];
}

/**
 * Walks the DomNode tree, extracts all `magnetic:head` nodes,
 * and returns the cleaned body + collected head elements.
 */
export function extractHead(root: DomNode): ExtractedHead {
  const headNodes: DomNode[] = [];

  function walk(node: DomNode): DomNode | null {
    // This IS a head node — collect its children and remove from body
    if (node.tag === 'magnetic:head') {
      if (node.children) headNodes.push(...node.children);
      return null;
    }

    // Recurse into children
    if (node.children) {
      const filtered: DomNode[] = [];
      for (const child of node.children) {
        const result = walk(child);
        if (result) filtered.push(result);
      }
      return { ...node, children: filtered.length ? filtered : undefined };
    }

    return node;
  }

  const body = walk(root) || root;
  return { body, headNodes };
}

// ── DomNode → HTML ──────────────────────────────────────────────────

/**
 * Renders a DomNode tree to an HTML string.
 * Events become `data-a_<event>` attributes (Magnetic's event delegation).
 * Keys become `data-key` attributes for client-side reconciliation.
 */
export function renderToHTML(node: DomNode): string {
  // Skip magnetic:head nodes in HTML output
  if (node.tag === 'magnetic:head') return '';

  let html = `<${node.tag}`;

  // Key → data-key
  if (node.key) {
    html += ` data-key="${esc(node.key)}"`;
  }

  // Attributes
  if (node.attrs) {
    for (const [k, v] of Object.entries(node.attrs)) {
      if (v === '') {
        html += ` ${k}`;
      } else {
        html += ` ${k}="${esc(v)}"`;
      }
    }
  }

  // Events → data-a_ attributes
  if (node.events) {
    for (const [event, action] of Object.entries(node.events)) {
      html += ` data-a_${event}="${esc(action)}"`;
    }
  }

  // Void element — self-close
  if (VOID.has(node.tag)) {
    return html + ' />';
  }

  html += '>';

  // Text content
  if (node.text != null) {
    html += esc(node.text);
  }

  // Children
  if (node.children) {
    for (const child of node.children) {
      html += renderToHTML(child);
    }
  }

  html += `</${node.tag}>`;
  return html;
}

// ── Full page render ────────────────────────────────────────────────

export interface PageOptions {
  /** The root DomNode from your App component */
  root: DomNode;
  /** CSS file paths to include */
  styles?: string[];
  /** JS file paths to include */
  scripts?: string[];
  /** Inline CSS to inject */
  inlineCSS?: string;
  /** Inline JS to inject at end of body */
  inlineJS?: string;
  /** SSE endpoint for Magnetic client to connect to */
  sseUrl?: string;
  /** Mount selector for Magnetic client */
  mountSelector?: string;
  /** WASM transport URL (optional) */
  wasmUrl?: string;
  /** HTML lang attribute */
  lang?: string;
  /** Fallback title if no <Head><title> found */
  title?: string;
  /** Fallback meta description */
  description?: string;
}

/**
 * Renders a complete HTML document with SSR'd body,
 * extracted <Head> elements, and Magnetic client bootstrap.
 */
export function renderPage(options: PageOptions): string {
  const {
    root,
    styles = [],
    scripts = [],
    inlineCSS,
    inlineJS,
    sseUrl = '/sse',
    mountSelector = '#app',
    wasmUrl,
    lang = 'en',
    title,
    description,
  } = options;

  // Extract <Head> elements from component tree
  const { body, headNodes } = extractHead(root);

  // Render head elements to HTML
  let headHTML = '';
  let hasTitle = false;

  for (const node of headNodes) {
    if (node.tag === 'title') hasTitle = true;
    headHTML += renderToHTML(node);
  }

  // Fallbacks
  if (!hasTitle && title) {
    headHTML += `<title>${esc(title)}</title>`;
  }
  if (description) {
    headHTML += `<meta name="description" content="${esc(description)}" />`;
  }

  // Stylesheets
  for (const href of styles) {
    headHTML += `<link rel="stylesheet" href="${esc(href)}" />`;
  }

  // Inline CSS
  if (inlineCSS) {
    headHTML += `<style>${inlineCSS}</style>`;
  }

  // Render body content
  const bodyHTML = renderToHTML(body);

  // Magnetic client bootstrap script
  let bootstrap = `Magnetic.connect("${sseUrl}", "${mountSelector}");`;
  if (wasmUrl) {
    bootstrap += `\nMagnetic.loadWasm("${wasmUrl}");`;
  }

  // Script tags
  let scriptsHTML = '';
  for (const src of scripts) {
    scriptsHTML += `<script src="${esc(src)}"></script>`;
  }

  if (inlineJS) {
    scriptsHTML += `<script>${inlineJS}</script>`;
  }

  return `<!DOCTYPE html>
<html lang="${lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
${headHTML}
</head>
<body>
<div id="${mountSelector.replace('#', '')}">${bodyHTML}</div>
${scriptsHTML}
<script>
${bootstrap}
</script>
</body>
</html>`;
}
