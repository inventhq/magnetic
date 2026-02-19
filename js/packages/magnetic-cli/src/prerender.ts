// prerender.ts — Static Site Generation (SSG) for Magnetic apps
// Loads the built V8 bundle in Node, renders each route to static HTML

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import vm from 'node:vm';

export interface PrerenderOptions {
  /** Path to the built app.js bundle */
  bundlePath: string;
  /** Output directory for HTML files */
  outDir: string;
  /** Routes to prerender */
  routes: string[];
  /** Fallback page title */
  title?: string;
  /** Inline CSS to inject */
  inlineCSS?: string;
  /** Public directory (for reading style.css) */
  publicDir?: string;
  /** Logger function */
  log?: (level: string, msg: string) => void;
}

/**
 * Pre-render routes to static HTML files.
 * Loads the IIFE bundle in a Node VM context, calls MagneticApp.render(path)
 * for each route, and converts the DomNode to HTML.
 */
export async function prerenderRoutes(opts: PrerenderOptions): Promise<number> {
  const { bundlePath, outDir, routes, title, inlineCSS, publicDir, log } = opts;

  // Load the built bundle
  const bundleCode = readFileSync(bundlePath, 'utf-8');

  // Create a minimal VM context with console
  const context = vm.createContext({
    console,
    self: {},
    globalThis: {} as any,
  });

  // Execute the IIFE — it assigns to globalThis.MagneticApp (via the IIFE global name)
  const script = new vm.Script(bundleCode + '\n;globalThis.__MA = MagneticApp;', {
    filename: 'app.js',
  });
  script.runInContext(context);

  const app = (context as any).globalThis.__MA || (context as any).__MA;
  if (!app || typeof app.render !== 'function') {
    if (log) log('error', 'Bundle does not export render() — cannot prerender');
    return 0;
  }

  // Read inline CSS from public/style.css if available
  let css = inlineCSS;
  if (!css && publicDir) {
    const stylePath = join(publicDir, 'style.css');
    if (existsSync(stylePath)) {
      css = readFileSync(stylePath, 'utf-8');
    }
  }

  let count = 0;

  for (const route of routes) {
    try {
      // Use renderWithCSS if available (includes generated CSS from design.json)
      // Falls back to render(path) for backwards compatibility
      let domNode: DomNode;
      let generatedCSS: string | undefined;

      if (typeof app.renderWithCSS === 'function') {
        const result = app.renderWithCSS(route);
        if (result && result.root) {
          domNode = result.root;
          generatedCSS = result.css || undefined;
        } else {
          if (log) log('error', `  skip ${route} — renderWithCSS returned null`);
          continue;
        }
      } else {
        domNode = app.render(route);
      }

      if (!domNode || !domNode.tag) {
        if (log) log('error', `  skip ${route} — render returned null`);
        continue;
      }

      // Extract <Head> elements and render to HTML
      const { body: bodyNode, headNodes } = extractHead(domNode);
      const bodyHTML = renderNodeToHTML(bodyNode);

      let headHTML = '';
      let hasTitle = false;
      for (const node of headNodes) {
        if (node.tag === 'title') hasTitle = true;
        headHTML += renderNodeToHTML(node);
      }
      if (!hasTitle && title) {
        headHTML += `<title>${escHTML(title)}</title>`;
      }
      // Merge CSS: generated from design.json + user's style.css
      const mergedCSS = [generatedCSS, css].filter(Boolean).join('');
      if (mergedCSS) {
        headHTML += `<style>${mergedCSS}</style>`;
      }

      const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
${headHTML}
</head>
<body>
<div id="app">${bodyHTML}</div>
</body>
</html>`;

      // Determine output path: "/" → "index.html", "/about" → "about/index.html"
      const outPath = route === '/'
        ? join(outDir, 'index.html')
        : join(outDir, route.replace(/^\//, ''), 'index.html');

      mkdirSync(dirname(outPath), { recursive: true });
      writeFileSync(outPath, html, 'utf-8');

      if (log) log('debug', `  prerendered ${route} → ${outPath} (${(html.length / 1024).toFixed(1)}KB)`);
      count++;
    } catch (e: any) {
      if (log) log('error', `  prerender ${route} failed: ${e.message}`);
    }
  }

  return count;
}

// ── Minimal DomNode → HTML renderer (runs in CLI, not in V8) ────────

interface DomNode {
  tag: string;
  key?: string;
  attrs?: Record<string, string>;
  events?: Record<string, string>;
  text?: string;
  children?: DomNode[];
}

const VOID_ELEMENTS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
  'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

function renderNodeToHTML(node: DomNode): string {
  if (node.tag === 'magnetic:head') return '';

  let html = `<${node.tag}`;
  if (node.key) html += ` data-key="${escAttr(node.key)}"`;
  if (node.attrs) {
    for (const [k, v] of Object.entries(node.attrs)) {
      html += v === '' ? ` ${k}` : ` ${k}="${escAttr(v)}"`;
    }
  }
  if (node.events) {
    for (const [ev, action] of Object.entries(node.events)) {
      html += ` data-a_${ev}="${escAttr(action)}"`;
    }
  }

  if (VOID_ELEMENTS.has(node.tag)) return html + ' />';
  html += '>';
  if (node.text != null) html += escHTML(node.text);
  if (node.children) {
    for (const child of node.children) html += renderNodeToHTML(child);
  }
  return html + `</${node.tag}>`;
}

function extractHead(root: DomNode): { body: DomNode; headNodes: DomNode[] } {
  const headNodes: DomNode[] = [];
  function walk(node: DomNode): DomNode | null {
    if (node.tag === 'magnetic:head') {
      if (node.children) headNodes.push(...node.children);
      return null;
    }
    if (node.children) {
      const filtered = node.children.map(walk).filter(Boolean) as DomNode[];
      return { ...node, children: filtered.length ? filtered : undefined };
    }
    return node;
  }
  const body = walk(root) || root;
  return { body, headNodes };
}

function escHTML(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escAttr(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
