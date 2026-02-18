// @magnetic/server — Router
// Nested routes, dynamic params, layouts, guards, redirects, file-based conventions

import type { DomNode } from './jsx-runtime.ts';

// ── Types ───────────────────────────────────────────────────────────

export type PageComponent = (props: {
  params: Record<string, string>;
  children?: DomNode;
  [key: string]: unknown;
}) => DomNode;

export type LayoutComponent = (props: {
  children: DomNode;
  params: Record<string, string>;
  path: string;
}) => DomNode;

/** Return true to allow, a string to redirect, or { redirect } */
export type RouteGuard = (ctx: {
  path: string;
  params: Record<string, string>;
}) => true | string | { redirect: string };

export interface RouteDefinition {
  /** Path pattern: "/tasks/:id", "/about", "*" */
  path: string;
  /** Page component (leaf) */
  page?: PageComponent;
  /** Layout wrapping this route and its children */
  layout?: LayoutComponent;
  /** Guard — runs before render. Return true to allow, string to redirect */
  guard?: RouteGuard;
  /** Redirect target — if set, this route always redirects */
  redirect?: string;
  /** Nested child routes */
  children?: RouteDefinition[];
}

export interface RouteMatch {
  /** Matched page component */
  page: PageComponent;
  /** Extracted URL params */
  params: Record<string, string>;
  /** Layout chain from outermost to innermost */
  layouts: LayoutComponent[];
  /** Guards to run in order (outermost first) */
  guards: RouteGuard[];
  /** If set, this route should redirect instead of render */
  redirect?: string;
}

/** Result of resolving a route — either a DomNode or a redirect */
export type RouteResult =
  | { kind: 'render'; dom: DomNode }
  | { kind: 'redirect'; to: string };

// ── Compiled route node ─────────────────────────────────────────────

interface CompiledNode {
  def: RouteDefinition;
  regex: RegExp;
  paramNames: string[];
  children: CompiledNode[];
}

function compileNode(def: RouteDefinition, prefix: string): CompiledNode {
  let pattern: string;
  const paramNames: string[] = [];

  if (def.path === '*') {
    pattern = '.*';
  } else {
    const parts = def.path.split('/').filter(Boolean);
    const regParts = parts.map((p) => {
      if (p.startsWith(':')) {
        paramNames.push(p.slice(1));
        return '([^/]+)';
      }
      return p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    });
    pattern = regParts.length ? regParts.join('/') : '';
  }

  const fullPattern = prefix + (pattern ? '/' + pattern : '');
  const hasChildren = def.children && def.children.length > 0;

  // If has children, match prefix (allow more segments after)
  // If leaf, match exactly
  const regexStr = hasChildren
    ? `^${fullPattern || ''}(?:/|$)`
    : `^${fullPattern || '/'}$`;

  const children = (def.children || []).map((c) =>
    compileNode(c, fullPattern)
  );

  return {
    def,
    regex: new RegExp(regexStr),
    paramNames,
    children,
  };
}

// ── Router ──────────────────────────────────────────────────────────

export interface Router {
  /** Match a URL path. Returns matched page + layout chain + guards */
  match(path: string): RouteMatch | null;
  /** Resolve: run guards, apply redirects, nest layouts → DomNode or redirect */
  resolve(path: string, appProps?: Record<string, unknown>): RouteResult | null;
  /** All top-level route definitions */
  routes: RouteDefinition[];
}

/**
 * Creates a router from (potentially nested) route definitions.
 *
 * Supports:
 *   - Static paths: `/about`
 *   - Dynamic params: `/tasks/:id`
 *   - Wildcard: `*`
 *   - Nested routes with `children`
 *   - Layouts at any level
 *   - Guards at any level (run outermost first)
 *   - Redirects (route-level or guard-returned)
 *   - First match wins (definition order)
 */
export function createRouter(routes: RouteDefinition[]): Router {
  // Compile the root as a virtual node with empty path
  const compiled: CompiledNode[] = routes.map((r) => compileNode(r, ''));

  function matchPath(
    nodes: CompiledNode[],
    path: string,
    params: Record<string, string>,
    layouts: LayoutComponent[],
    guards: RouteGuard[],
  ): RouteMatch | null {
    const normalized = path === '/' ? '/' : path.replace(/\/+$/, '');

    for (const node of nodes) {
      const m = normalized.match(node.regex);
      if (!m) continue;

      // Extract params from this level
      const levelParams = { ...params };
      for (let i = 0; i < node.paramNames.length; i++) {
        levelParams[node.paramNames[i]] = decodeURIComponent(m[i + 1]);
      }

      // Collect layout + guard from this level
      const levelLayouts = node.def.layout ? [...layouts, node.def.layout] : [...layouts];
      const levelGuards = node.def.guard ? [...guards, node.def.guard] : [...guards];

      // Redirect at route level
      if (node.def.redirect) {
        return {
          page: () => ({ tag: 'div' }),
          params: levelParams,
          layouts: levelLayouts,
          guards: levelGuards,
          redirect: node.def.redirect,
        };
      }

      // Try children first (depth-first)
      if (node.children.length > 0) {
        const childMatch = matchPath(
          node.children, normalized, levelParams, levelLayouts, levelGuards,
        );
        if (childMatch) return childMatch;
      }

      // Leaf match — must have a page component
      if (node.def.page) {
        return {
          page: node.def.page,
          params: levelParams,
          layouts: levelLayouts,
          guards: levelGuards,
        };
      }
    }
    return null;
  }

  return {
    routes,

    match(path: string): RouteMatch | null {
      return matchPath(compiled, path, {}, [], []);
    },

    resolve(path: string, appProps?: Record<string, unknown>): RouteResult | null {
      const match = this.match(path);
      if (!match) return null;

      // Run guards in order
      for (const guard of match.guards) {
        const result = guard({ path, params: match.params });
        if (result === true) continue;
        const to = typeof result === 'string' ? result : result.redirect;
        return { kind: 'redirect', to };
      }

      // Route-level redirect
      if (match.redirect) {
        return { kind: 'redirect', to: match.redirect };
      }

      // Render page
      const pageProps = { params: match.params, ...appProps };
      let dom = match.page(pageProps);

      // Wrap in layouts (innermost first → outermost wraps)
      for (let i = match.layouts.length - 1; i >= 0; i--) {
        dom = match.layouts[i]({ children: dom, params: match.params, path });
      }

      return { kind: 'render', dom };
    },
  };
}

// ── Convenience: renderRoute (backward compat) ─────────────────────

export function renderRoute(
  router: Router,
  path: string,
  appProps?: Record<string, unknown>,
): DomNode | null {
  const result = router.resolve(path, appProps);
  if (!result) return null;
  if (result.kind === 'redirect') {
    // For backward compat, follow one redirect level
    const r2 = router.resolve(result.to, appProps);
    if (r2 && r2.kind === 'render') return r2.dom;
    return null;
  }
  return result.dom;
}

// ── Navigate action string ──────────────────────────────────────────

export function navigateAction(path: string): string {
  return `navigate:${path}`;
}
