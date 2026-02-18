// @magnetic/server — File-based Router
// Scans a pages/ directory and generates route definitions from file conventions.
//
// Convention:
//   pages/
//     index.tsx        → /
//     about.tsx        → /about
//     layout.tsx       → layout wrapping all routes at this level
//     _guard.ts        → guard for all routes at this level
//     tasks/
//       index.tsx      → /tasks
//       [id].tsx       → /tasks/:id
//       layout.tsx     → layout wrapping /tasks/* routes
//       _guard.ts      → guard for /tasks/* routes
//
// Exports `scanPages()` which returns a RouteDefinition[] tree.
// This runs at build time or server startup — not at request time.

import { readdirSync, statSync, existsSync } from 'node:fs';
import { join, basename, extname } from 'node:path';
import type { RouteDefinition, PageComponent, LayoutComponent, RouteGuard } from './router.ts';

export interface FileRouterOptions {
  /** Absolute path to the pages/ directory */
  pagesDir: string;
  /** Function to import a module given its file path. Default: dynamic import */
  importFn?: (filePath: string) => any;
}

interface ScannedModule {
  default?: PageComponent | LayoutComponent | RouteGuard;
  guard?: RouteGuard;
  layout?: LayoutComponent;
  redirect?: string;
}

/**
 * Scans a pages/ directory and returns route definitions.
 *
 * Each .tsx file becomes a route. The default export is the page component.
 * Special files:
 *   - `layout.tsx` → layout component (default export)
 *   - `_guard.ts`  → route guard (default or named `guard` export)
 *
 * Dynamic params use bracket syntax: `[id].tsx` → `:id`
 */
export function scanPages(
  pagesDir: string,
  modules: Record<string, ScannedModule>,
): RouteDefinition[] {
  if (!existsSync(pagesDir)) return [];
  return scanDir(pagesDir, '', modules);
}

function scanDir(
  dir: string,
  pathPrefix: string,
  modules: Record<string, ScannedModule>,
): RouteDefinition[] {
  const entries = readdirSync(dir).sort();
  const routes: RouteDefinition[] = [];

  // Check for layout and guard at this level
  let layout: LayoutComponent | undefined;
  let guard: RouteGuard | undefined;

  const layoutFile = findFile(dir, 'layout');
  if (layoutFile) {
    const mod = modules[layoutFile];
    if (mod) layout = (mod.default || mod.layout) as LayoutComponent;
  }

  const guardFile = findFile(dir, '_guard');
  if (guardFile) {
    const mod = modules[guardFile];
    if (mod) guard = (mod.default || mod.guard) as RouteGuard;
  }

  // Scan files and subdirectories
  const files: string[] = [];
  const dirs: string[] = [];

  for (const entry of entries) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      dirs.push(entry);
    } else if (/\.(tsx?|jsx?)$/.test(entry)) {
      const name = basename(entry, extname(entry));
      // Skip special files
      if (name === 'layout' || name === '_guard') continue;
      files.push(entry);
    }
  }

  // Process page files
  for (const file of files) {
    const name = basename(file, extname(file));
    const filePath = join(dir, file);
    const mod = modules[filePath];
    if (!mod || !mod.default) continue;

    let segment: string;
    if (name === 'index') {
      segment = pathPrefix || '/';
    } else if (name.startsWith('[') && name.endsWith(']')) {
      // Dynamic param: [id].tsx → :id
      const param = name.slice(1, -1);
      segment = pathPrefix + '/:' + param;
    } else {
      segment = pathPrefix + '/' + name;
    }

    const route: RouteDefinition = {
      path: segment,
      page: mod.default as PageComponent,
    };

    if (mod.redirect) route.redirect = mod.redirect;

    routes.push(route);
  }

  // Process subdirectories (nested routes)
  for (const dirName of dirs) {
    if (dirName.startsWith('.') || dirName === 'node_modules') continue;

    const subDir = join(dir, dirName);
    let segment: string;
    if (dirName.startsWith('[') && dirName.endsWith(']')) {
      const param = dirName.slice(1, -1);
      segment = pathPrefix + '/:' + param;
    } else {
      segment = pathPrefix + '/' + dirName;
    }

    const children = scanDir(subDir, segment, modules);

    if (children.length > 0) {
      // Check if any child is the index for this path
      const indexIdx = children.findIndex((c) => c.path === segment);

      if (indexIdx >= 0 || layout || guard) {
        // Create a parent route node with children
        const parentRoute: RouteDefinition = {
          path: segment,
          children,
        };
        if (layout) parentRoute.layout = layout;
        if (guard) parentRoute.guard = guard;

        // If there's an index page, lift it to be the parent's page
        if (indexIdx >= 0) {
          parentRoute.page = children[indexIdx].page;
          children.splice(indexIdx, 1);
        }

        routes.push(parentRoute);
      } else {
        // No layout/guard — flatten children into current level
        routes.push(...children);
      }
    }
  }

  // If this is the root level and we have a layout/guard, wrap everything
  if (pathPrefix === '' && (layout || guard)) {
    return [{
      path: '/',
      layout,
      guard,
      children: routes,
    }];
  }

  return routes;
}

function findFile(dir: string, name: string): string | null {
  for (const ext of ['.tsx', '.ts', '.jsx', '.js']) {
    const full = join(dir, name + ext);
    if (existsSync(full)) return full;
  }
  return null;
}

/**
 * Helper: converts a pages/ directory path to a route path segment.
 * e.g. "[id]" → ":id", "about" → "about"
 */
export function fileNameToSegment(name: string): string {
  if (name.startsWith('[') && name.endsWith(']')) {
    return ':' + name.slice(1, -1);
  }
  return name;
}
