// generator.ts — Auto-bridge generator
// Scans pages/ directory, detects state.ts, generates v8-bridge code
// so developers never write boilerplate wiring

import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, relative, extname, basename, dirname } from 'node:path';
import type { MagneticAppConfig } from './config.ts';

// ── Page scanning ───────────────────────────────────────────────────

export interface PageEntry {
  /** Relative path from app dir: "pages/TasksPage.tsx" */
  filePath: string;
  /** Import name: "TasksPage" */
  importName: string;
  /** Route path: "/" or "/about" or "*" */
  routePath: string;
  /** Whether this is the catch-all 404 page */
  isCatchAll: boolean;
}

export interface LayoutEntry {
  /** Relative path from app dir: "pages/layout.tsx" or "pages/dashboard/layout.tsx" */
  filePath: string;
  /** Import name: "RootLayout" or "DashboardLayout" */
  importName: string;
  /** Route prefix this layout applies to: "/" or "/dashboard" */
  routePrefix: string;
}

export interface ApiRouteEntry {
  /** Relative path from app dir: "server/api/users.ts" */
  filePath: string;
  /** Import name: "api_users" */
  importName: string;
  /** API route path: "/api/users" */
  routePath: string;
}

export interface AppScan {
  pages: PageEntry[];
  /** Layout files found at various directory levels */
  layouts: LayoutEntry[];
  /** API route files in server/api/ */
  apiRoutes: ApiRouteEntry[];
  /** Path to state module relative to app dir, or null */
  statePath: string | null;
  /** Whether state module exports toViewModel */
  hasViewModel: boolean;
  /** Path to magnetic-server package relative to app dir */
  serverPkgPath: string;
}

const PAGE_EXTENSIONS = ['.tsx', '.jsx', '.ts', '.js'];

const CATCH_ALL_NAMES = ['notfound', '404', 'notfoundpage', '_404', 'error'];
const INDEX_NAMES = ['index', 'indexpage', 'home', 'homepage', 'tasks', 'taskspage', 'main', 'mainpage'];

/**
 * Scan an app directory for pages and state.
 */
export function scanApp(appDir: string, monorepoRoot?: string): AppScan {
  const pagesDir = join(appDir, 'pages');
  const pages: PageEntry[] = [];
  const layouts: LayoutEntry[] = [];

  if (existsSync(pagesDir)) {
    scanDir(pagesDir, '', pages, undefined, layouts);
  }

  // Detect state module
  const stateCandidates = [
    'state.ts', 'state.tsx',
    'server/state.ts', 'server/state.tsx',
    'store.ts', 'store.tsx',
  ];
  let statePath: string | null = null;
  let hasViewModel = false;

  for (const candidate of stateCandidates) {
    const full = join(appDir, candidate);
    if (existsSync(full)) {
      statePath = './' + candidate;
      // Check if it exports toViewModel (simple text search)
      const content = readFileSync(full, 'utf-8');
      hasViewModel = /export\s+(function|const)\s+toViewModel/.test(content);
      break;
    }
  }

  // Resolve path to @magneticjs/server
  let serverPkgPath: string;
  if (monorepoRoot) {
    const relPath = relative(appDir, join(monorepoRoot, 'js/packages/magnetic-server/src'));
    serverPkgPath = relPath.startsWith('.') ? relPath : './' + relPath;
  } else {
    serverPkgPath = '@magneticjs/server';
  }

  // Scan API routes
  const apiRoutes: ApiRouteEntry[] = [];
  const apiDir = join(appDir, 'server', 'api');
  if (existsSync(apiDir)) {
    const apiEntries = readdirSync(apiDir).sort();
    for (const entry of apiEntries) {
      const ext = extname(entry);
      if (!PAGE_EXTENSIONS.includes(ext)) continue;
      const nameNoExt = basename(entry, ext);
      const importName = 'api_' + nameNoExt.replace(/[^a-zA-Z0-9]/g, '_');
      const routePath = '/api/' + nameNoExt.toLowerCase();
      const relPath = 'server/api/' + entry;
      apiRoutes.push({ filePath: relPath, importName, routePath });
    }
  }

  return { pages, layouts, apiRoutes, statePath, hasViewModel, serverPkgPath };
}

function scanDir(dir: string, pathPrefix: string, pages: PageEntry[], rootPagesDir?: string, layouts?: LayoutEntry[]) {
  const pagesRoot = rootPagesDir || dir;
  const entries = readdirSync(dir).sort();

  // Check for layout file in this directory
  if (layouts) {
    for (const ext of PAGE_EXTENSIONS) {
      const layoutPath = join(dir, 'layout' + ext);
      if (existsSync(layoutPath)) {
        const prefix = pathPrefix || '/';
        // Generate a unique import name from the directory path
        const dirName = pathPrefix
          ? pathPrefix.split('/').filter(Boolean).map(s =>
              s.startsWith(':') ? s.slice(1) : s
            ).map(s => s.charAt(0).toUpperCase() + s.slice(1)).join('')
          : 'Root';
        const importName = dirName + 'Layout';
        const relFromPagesRoot = relative(pagesRoot, layoutPath).replace(/\\/g, '/');
        layouts.push({
          filePath: 'pages/' + relFromPagesRoot,
          importName,
          routePrefix: prefix,
        });
        break;
      }
    }
  }

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      // Nested directory → nested route
      let segment = entry;
      if (entry.startsWith('[') && entry.endsWith(']')) {
        segment = ':' + entry.slice(1, -1);
      }
      scanDir(fullPath, pathPrefix + '/' + segment, pages, pagesRoot, layouts);
      continue;
    }

    const ext = extname(entry);
    if (!PAGE_EXTENSIONS.includes(ext)) continue;

    // Skip special files
    const nameNoExt = basename(entry, ext);
    if (nameNoExt.startsWith('_') && !CATCH_ALL_NAMES.includes(nameNoExt.toLowerCase())) continue;
    if (nameNoExt === 'layout') continue;

    const importName = nameNoExt;
    const nameLower = nameNoExt.toLowerCase().replace(/page$/, '');

    let routePath: string;
    let isCatchAll = false;

    if (CATCH_ALL_NAMES.includes(nameLower)) {
      routePath = '*';
      isCatchAll = true;
    } else if (INDEX_NAMES.includes(nameLower)) {
      routePath = pathPrefix || '/';
    } else if (nameNoExt.startsWith('[') && nameNoExt.endsWith(']')) {
      // Dynamic param: [id].tsx → /:id
      const param = nameNoExt.slice(1, -1);
      routePath = pathPrefix + '/:' + param;
    } else {
      routePath = pathPrefix + '/' + nameLower;
    }

    // filePath relative to the app dir (e.g. "pages/AboutPage.tsx")
    const relFromPagesRoot = relative(pagesRoot, fullPath).replace(/\\/g, '/');
    const filePath = 'pages/' + relFromPagesRoot;

    pages.push({ filePath, importName, routePath, isCatchAll });
  }
}

// ── Bridge code generation ──────────────────────────────────────────

/**
 * Generate the v8-bridge.tsx source code from scan results.
 * This code is fed to esbuild (never written to disk as a permanent file).
 */
export function generateBridge(scan: AppScan, config?: MagneticAppConfig, designJson?: string): string {
  const lines: string[] = [];
  lines.push('// AUTO-GENERATED by @magnetic/cli — do not edit');
  lines.push(`import { createRouter } from '${scan.serverPkgPath}/router';`);

  // CSS framework: import generateAllCSS if design config is available
  if (designJson) {
    lines.push("import { generateAllCSS } from '@magneticjs/css';");
  }

  // Import pages
  const catchAllPage = scan.pages.find(p => p.isCatchAll);
  for (const page of scan.pages) {
    lines.push(`import { ${page.importName} } from './${page.filePath}';`);
  }

  // Import layouts
  for (const layout of scan.layouts) {
    lines.push(`import ${layout.importName} from './${layout.filePath}';`);
  }

  // Import state (or generate default)
  if (scan.statePath) {
    lines.push(`import { initialState, reduce as _reduce${scan.hasViewModel ? ', toViewModel' : ''} } from '${scan.statePath}';`);
  } else {
    lines.push('');
    lines.push('// No state.ts found — using minimal default state');
    lines.push('function initialState() { return {}; }');
    lines.push('function _reduce(state, action, payload) { return state; }');
  }

  if (!scan.hasViewModel && scan.statePath) {
    lines.push('function toViewModel(s) { return s; }');
  } else if (!scan.statePath) {
    lines.push('function toViewModel(s) { return s; }');
  }

  // Router — build route tree with layouts
  lines.push('');
  if (scan.layouts.length > 0) {
    lines.push('const router = createRouter(');
    lines.push(buildRouteTree(scan.pages, scan.layouts));
    lines.push(');');
  } else {
    lines.push('const router = createRouter([');
    for (const page of scan.pages) {
      if (!page.isCatchAll) {
        lines.push(`  { path: '${page.routePath}', page: ${page.importName} },`);
      }
    }
    if (catchAllPage) {
      lines.push(`  { path: '*', page: ${catchAllPage.importName} },`);
    }
    lines.push(']);');
  }

  // Data source page-scope metadata (tells server which sources to fetch per route)
  const hasData = config && config.data.length > 0;
  if (hasData) {
    lines.push('');
    lines.push('// Declarative data sources — page scope metadata');
    lines.push(`var __dataScopes = ${JSON.stringify(config.data.map(d => ({ key: d.key, page: d.page })))};`);
    lines.push('');
    lines.push('// Action mappings known to the bridge (server handles forwarding)');
    lines.push(`var __actionNames = ${JSON.stringify(config.actions.map(a => a.name))};`);
  }

  // State + render/reduce
  lines.push('');
  lines.push('let state = initialState();');
  lines.push('');
  lines.push('// Server injects fetched data here before render');
  lines.push('var __magneticData = {};');
  lines.push('');
  lines.push('export function setData(data) { __magneticData = data || {}; }');
  lines.push('');
  lines.push('export function getDataScopes() { return typeof __dataScopes !== "undefined" ? __dataScopes : []; }');
  lines.push('');
  lines.push('export function getActionNames() { return typeof __actionNames !== "undefined" ? __actionNames : []; }');
  lines.push('');
  lines.push('export function render(path) {');
  lines.push('  const merged = Object.assign({}, __magneticData, state);');
  lines.push('  const vm = toViewModel(merged);');
  lines.push('  const result = router.resolve(path, vm);');
  if (catchAllPage) {
    lines.push(`  if (!result) return ${catchAllPage.importName}({ params: {} });`);
  } else {
    lines.push('  if (!result) return { tag: "div", text: "Not Found" };');
  }
  lines.push('  if (result.kind === \'redirect\') {');
  lines.push('    const r2 = router.resolve(result.to, vm);');
  lines.push('    if (r2 && r2.kind === \'render\') return r2.dom;');
  if (catchAllPage) {
    lines.push(`    return ${catchAllPage.importName}({ params: {} });`);
  } else {
    lines.push('    return { tag: "div", text: "Not Found" };');
  }
  lines.push('  }');
  lines.push('  return result.dom;');
  lines.push('}');
  lines.push('');
  lines.push('export function reduce(ap) {');
  lines.push('  const { action, payload = {}, path = \'/\' } = ap;');
  lines.push('  state = _reduce(state, action, payload);');
  lines.push('  return render(path);');
  lines.push('}');

  // CSS framework: renderWithCSS() — new export for SSR paths only
  // render() and reduce() are untouched — SSE/action flows are unaffected
  if (designJson) {
    lines.push('');
    lines.push('// ── Magnetic CSS ────────────────────────────────────────────────');
    lines.push(`var __designConfig = ${designJson};`);
    lines.push('var __cssAllUtilities = generateAllCSS(__designConfig);');
    lines.push('');
    lines.push('export function renderWithCSS(path) {');
    lines.push('  const dom = render(path);');
    lines.push('  return { root: dom, css: __cssAllUtilities };');
    lines.push('}');
  }
  lines.push('');
  lines.push('// Check if an action should be forwarded to a backend API');
  lines.push('export function isExternalAction(name) {');
  lines.push('  return typeof __actionNames !== "undefined" && __actionNames.indexOf(name) >= 0;');
  lines.push('}');

  // API routes
  if (scan.apiRoutes.length > 0) {
    lines.push('');
    lines.push('// ── API Routes ──────────────────────────────────────────────────');
    lines.push('var __apiRoutes = {};');
    for (const api of scan.apiRoutes) {
      lines.push(`import * as ${api.importName} from './${api.filePath}';`);
      lines.push(`__apiRoutes['${api.routePath}'] = ${api.importName};`);
    }
    lines.push('');
    lines.push('export function handleApi(method, path, body) {');
    lines.push('  var handler = __apiRoutes[path];');
    lines.push('  if (!handler) return JSON.stringify({ __error: "Not found: " + path, __status: 404 });');
    lines.push('  var fn = handler[method] || handler[method.toLowerCase()];');
    lines.push('  if (!fn) return JSON.stringify({ __error: "Method not allowed: " + method, __status: 405 });');
    lines.push('  try {');
    lines.push('    var parsed = typeof body === "string" && body ? JSON.parse(body) : body || {};');
    lines.push('    var result = fn({ body: parsed, method: method, path: path });');
    lines.push('    if (typeof result === "string") return result;');
    lines.push('    return JSON.stringify(result);');
    lines.push('  } catch(e) {');
    lines.push('    return JSON.stringify({ __error: e.message || String(e), __status: 500 });');
    lines.push('  }');
    lines.push('}');
    lines.push('');
    lines.push('export function getApiRoutes() { return Object.keys(__apiRoutes); }');
  }

  return lines.join('\n') + '\n';
}

// ── Route tree builder (layouts → nested route definitions) ──────────

/**
 * Build a nested route definition array as a JS code string.
 * Groups pages under their matching layout routes.
 *
 * Example structure:
 *   pages/layout.tsx          → RootLayout wraps all routes at "/"
 *   pages/IndexPage.tsx       → { path: '/', page: IndexPage }
 *   pages/dashboard/layout.tsx → DashboardLayout wraps "/dashboard/*"
 *   pages/dashboard/Home.tsx  → { path: '/dashboard', page: Home }
 *
 * Produces:
 *   [{ path: '/', layout: RootLayout, children: [
 *     { path: '/', page: IndexPage },
 *     { path: '/dashboard', layout: DashboardLayout, children: [
 *       { path: '/dashboard', page: Home },
 *     ]},
 *   ]}]
 */
function buildRouteTree(pages: PageEntry[], layouts: LayoutEntry[]): string {
  // Sort layouts by prefix depth (root first, then nested)
  const sorted = [...layouts].sort((a, b) => {
    const da = a.routePrefix === '/' ? 0 : a.routePrefix.split('/').filter(Boolean).length;
    const db = b.routePrefix === '/' ? 0 : b.routePrefix.split('/').filter(Boolean).length;
    return da - db;
  });

  // Find the root layout (prefix === '/')
  const rootLayout = sorted.find(l => l.routePrefix === '/');
  const nestedLayouts = sorted.filter(l => l.routePrefix !== '/');

  // Determine which pages belong to which layout
  // A page belongs to the deepest matching layout prefix
  function findLayout(routePath: string): LayoutEntry | undefined {
    let best: LayoutEntry | undefined;
    for (const layout of nestedLayouts) {
      const prefix = layout.routePrefix;
      if (routePath === prefix || routePath.startsWith(prefix + '/')) {
        if (!best || prefix.length > best.routePrefix.length) {
          best = layout;
        }
      }
    }
    return best;
  }

  // Group pages by their layout
  const layoutChildren = new Map<string, PageEntry[]>();
  const topLevel: PageEntry[] = [];

  for (const page of pages) {
    const layout = findLayout(page.routePath);
    if (layout) {
      const key = layout.routePrefix;
      if (!layoutChildren.has(key)) layoutChildren.set(key, []);
      layoutChildren.get(key)!.push(page);
    } else {
      topLevel.push(page);
    }
  }

  // Build route definition strings
  function pageRoute(p: PageEntry): string {
    return `{ path: '${p.routePath}', page: ${p.importName} }`;
  }

  function layoutRoute(layout: LayoutEntry): string {
    const children = layoutChildren.get(layout.routePrefix) || [];
    // Check if any deeper nested layouts belong under this one
    const subLayouts = nestedLayouts.filter(l =>
      l.routePrefix !== layout.routePrefix &&
      (l.routePrefix.startsWith(layout.routePrefix + '/') || layout.routePrefix === '/')
    );

    const childRoutes: string[] = [];

    // Add pages that belong directly to this layout (not to a sub-layout)
    for (const page of children) {
      const subLayout = subLayouts.find(sl => {
        const prefix = sl.routePrefix;
        return page.routePath === prefix || page.routePath.startsWith(prefix + '/');
      });
      if (!subLayout) {
        childRoutes.push(pageRoute(page));
      }
    }

    // Add sub-layout routes (recursive)
    for (const sub of subLayouts) {
      // Only include direct sub-layouts (not nested under another sub-layout)
      const isDirectChild = !subLayouts.some(other =>
        other !== sub &&
        sub.routePrefix.startsWith(other.routePrefix + '/') &&
        other.routePrefix.length > layout.routePrefix.length
      );
      if (isDirectChild) {
        childRoutes.push(layoutRoute(sub));
      }
    }

    return `{ path: '${layout.routePrefix}', layout: ${layout.importName}, children: [\n    ${childRoutes.join(',\n    ')}\n  ] }`;
  }

  // Build the top-level array
  const routes: string[] = [];

  if (rootLayout) {
    // Root layout wraps everything — all top-level pages + nested layout groups
    const allChildren: string[] = [];

    for (const page of topLevel) {
      if (!page.isCatchAll) allChildren.push(pageRoute(page));
    }

    for (const layout of nestedLayouts) {
      // Only include top-level nested layouts (not nested under another nested layout)
      const isTopNested = !nestedLayouts.some(other =>
        other !== layout &&
        layout.routePrefix.startsWith(other.routePrefix + '/')
      );
      if (isTopNested) {
        allChildren.push(layoutRoute(layout));
      }
    }

    const catchAll = pages.find(p => p.isCatchAll);
    if (catchAll) allChildren.push(pageRoute(catchAll));

    routes.push(`{ path: '/', layout: ${rootLayout.importName}, children: [\n    ${allChildren.join(',\n    ')}\n  ] }`);
  } else {
    // No root layout — flat list with nested layouts as groups
    for (const page of topLevel) {
      if (!page.isCatchAll) routes.push(pageRoute(page));
    }

    for (const layout of nestedLayouts) {
      const isTopNested = !nestedLayouts.some(other =>
        other !== layout &&
        layout.routePrefix.startsWith(other.routePrefix + '/')
      );
      if (isTopNested) {
        routes.push(layoutRoute(layout));
      }
    }

    const catchAll = pages.find(p => p.isCatchAll);
    if (catchAll) routes.push(pageRoute(catchAll));
  }

  return `[\n  ${routes.join(',\n  ')}\n]`;
}
