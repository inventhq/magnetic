// @magnetic/server — Middleware
// Express-style use() chain with next() pattern for request processing

import type { DomNode } from './jsx-runtime.ts';

// ── Context ─────────────────────────────────────────────────────────

export interface MagneticContext {
  /** HTTP method */
  method: string;
  /** Request URL path */
  path: string;
  /** Parsed query params */
  query: Record<string, string>;
  /** Request headers */
  headers: Record<string, string>;
  /** Action name (for POST /actions/:action) */
  action?: string;
  /** Action payload */
  payload?: Record<string, any>;
  /** Attached user/session data (set by auth middleware) */
  user?: { id: string; [key: string]: unknown };
  /** Response status code (middleware can set this) */
  status: number;
  /** Response headers to add */
  responseHeaders: Record<string, string>;
  /** If set, short-circuit with this response body */
  body?: string;
  /** Arbitrary storage for middleware to share data */
  state: Record<string, unknown>;
}

// ── Middleware types ─────────────────────────────────────────────────

export type NextFn = () => void | Promise<void>;

export type MiddlewareFn = (
  ctx: MagneticContext,
  next: NextFn,
) => void | Promise<void>;

// ── Middleware chain ─────────────────────────────────────────────────

export interface MiddlewareStack {
  /** Add a middleware function */
  use(fn: MiddlewareFn): void;
  /** Run the middleware chain for a given context */
  run(ctx: MagneticContext): Promise<MagneticContext>;
}

/**
 * Creates a middleware stack.
 *
 * Usage:
 * ```ts
 * const mw = createMiddleware();
 * mw.use(logger);
 * mw.use(cors);
 * mw.use(auth);
 * const ctx = await mw.run(context);
 * ```
 */
export function createMiddleware(): MiddlewareStack {
  const fns: MiddlewareFn[] = [];

  return {
    use(fn: MiddlewareFn) {
      fns.push(fn);
    },

    async run(ctx: MagneticContext): Promise<MagneticContext> {
      let index = 0;

      async function next(): Promise<void> {
        if (index >= fns.length) return;
        // Short-circuit if body was set (middleware wants to respond early)
        if (ctx.body != null) return;
        const fn = fns[index++];
        await fn(ctx, next);
      }

      await next();
      return ctx;
    },
  };
}

// ── Helper: create context from raw request ─────────────────────────

export function createContext(opts: {
  method: string;
  url: string;
  headers?: Record<string, string>;
  action?: string;
  payload?: Record<string, any>;
}): MagneticContext {
  const [path, qs] = (opts.url || '/').split('?');
  const query: Record<string, string> = {};
  if (qs) {
    for (const pair of qs.split('&')) {
      const [k, v] = pair.split('=');
      if (k) query[decodeURIComponent(k)] = decodeURIComponent(v || '');
    }
  }

  return {
    method: opts.method,
    path,
    query,
    headers: opts.headers || {},
    action: opts.action,
    payload: opts.payload,
    status: 200,
    responseHeaders: {},
    state: {},
  };
}

// ── Built-in middleware ──────────────────────────────────────────────

/** Logs request method + path + timing */
export const loggerMiddleware: MiddlewareFn = async (ctx, next) => {
  const start = Date.now();
  await next();
  const ms = Date.now() - start;
  console.log(`[magnetic] ${ctx.method} ${ctx.path} → ${ctx.status} (${ms}ms)`);
};

/** CORS headers */
export function corsMiddleware(origins: string | string[] = '*'): MiddlewareFn {
  const origin = Array.isArray(origins) ? origins.join(', ') : origins;
  return async (ctx, next) => {
    ctx.responseHeaders['Access-Control-Allow-Origin'] = origin;
    ctx.responseHeaders['Access-Control-Allow-Headers'] = 'Content-Type';
    ctx.responseHeaders['Access-Control-Allow-Methods'] = 'GET, POST, OPTIONS';
    if (ctx.method === 'OPTIONS') {
      ctx.status = 204;
      ctx.body = '';
      return;
    }
    await next();
  };
}

/** Rate limiter (per-IP, sliding window) */
export function rateLimitMiddleware(opts: {
  windowMs?: number;
  max?: number;
} = {}): MiddlewareFn {
  const windowMs = opts.windowMs || 60_000;
  const max = opts.max || 100;
  const hits = new Map<string, { count: number; resetAt: number }>();

  return async (ctx, next) => {
    const ip = ctx.headers['x-forwarded-for'] || ctx.headers['x-real-ip'] || 'unknown';
    const now = Date.now();
    let entry = hits.get(ip);

    if (!entry || now > entry.resetAt) {
      entry = { count: 0, resetAt: now + windowMs };
      hits.set(ip, entry);
    }

    entry.count++;

    if (entry.count > max) {
      ctx.status = 429;
      ctx.body = JSON.stringify({ error: 'Too many requests' });
      return;
    }

    await next();
  };
}
