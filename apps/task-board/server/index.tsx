// server/index.tsx — Task Board server
// Full stack: nested routing, guards, redirects, middleware, error boundaries,
// content-hashed assets, SSR, SSE, POST→response

import { createServer, IncomingMessage, ServerResponse } from 'node:http';
import { readFileSync, existsSync } from 'node:fs';
import { extname, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  createRouter, renderPage,
  withErrorBoundary, safeReduce,
  createMiddleware, createContext, loggerMiddleware, corsMiddleware,
  buildAssets, createAssetResolver, serveStatic,
} from '../../../js/packages/magnetic-server/src/index.ts';
import type { DomNode, RouteGuard, LayoutComponent } from '../../../js/packages/magnetic-server/src/index.ts';

import { TasksPage } from '../pages/TasksPage.tsx';
import { AboutPage } from '../pages/AboutPage.tsx';
import { NotFoundPage } from '../pages/NotFoundPage.tsx';
import { initialState, reduce, toViewModel } from './state.ts';
import type { AppState } from './state.ts';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const PUBLIC = resolve(ROOT, 'public');

// ── Asset pipeline ──────────────────────────────────────────────────

const ASSETS_DIR = resolve(ROOT, 'dist/static');
const manifest = buildAssets({
  srcDir: PUBLIC,
  outDir: ASSETS_DIR,
  extensions: ['.css', '.js', '.wasm'],
  passthrough: ['magnetic.js', 'transport.wasm'],
});
const asset = createAssetResolver(manifest, '/static');
console.log('[task-board] Assets:', Object.entries(manifest.files).map(([k, v]) => k === v ? k : `${k} → ${v}`).join(', '));

// ── Route guard: demo auth guard for /admin ─────────────────────────

const requireAuth: RouteGuard = ({ path, params }) => {
  // Demo: always redirect /admin to / (no auth system yet)
  return { redirect: '/' };
};

// ── Router (nested routes, guards, redirects) ───────────────────────

const router = createRouter([
  { path: '/', page: TasksPage },
  { path: '/about', page: AboutPage },
  { path: '/admin', page: TasksPage, guard: requireAuth },
  { path: '/old-about', redirect: '/about' },
  { path: '*', page: NotFoundPage },
]);

// ── App state + error boundaries ────────────────────────────────────

let state: AppState = initialState();
let currentPath: string = '/';

const safeReducer = safeReduce(reduce);
const safeRender = withErrorBoundary((path: string): DomNode => {
  const vm = toViewModel(state);
  const result = router.resolve(path, vm);
  if (!result) return NotFoundPage({ params: {} });
  if (result.kind === 'redirect') {
    // Follow redirect (server-side)
    const r2 = router.resolve(result.to, vm);
    if (r2 && r2.kind === 'render') return r2.dom;
    return NotFoundPage({ params: {} });
  }
  return result.dom;
});

function snapshot(path?: string): string {
  const dom = safeRender(path || currentPath);
  return JSON.stringify({ root: dom });
}

// ── SSE clients ─────────────────────────────────────────────────────

const sseClients: Set<ServerResponse> = new Set();

function broadcast(data: string): void {
  const msg = `event: message\ndata: ${data}\n\n`;
  for (const res of sseClients) {
    try { res.write(msg); } catch { sseClients.delete(res); }
  }
}

// ── Middleware stack ─────────────────────────────────────────────────

const mw = createMiddleware();
mw.use(loggerMiddleware);
mw.use(corsMiddleware('*'));

// ── HTTP server ─────────────────────────────────────────────────────

const PORT = parseInt(process.argv[2] || '3003', 10);

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((res) => {
    let body = '';
    req.on('data', (chunk: Buffer) => { body += chunk.toString(); });
    req.on('end', () => res(body));
  });
}

const server = createServer(async (req: IncomingMessage, res: ServerResponse) => {
  const url = req.url || '/';
  const method = req.method || 'GET';

  // Run middleware
  const headers: Record<string, string> = {};
  req.rawHeaders.forEach((v, i, a) => { if (i % 2 === 0) headers[a[i].toLowerCase()] = a[i + 1]; });
  const ctx = createContext({ method, url, headers });
  await mw.run(ctx);

  // Apply middleware response headers
  for (const [k, v] of Object.entries(ctx.responseHeaders)) {
    res.setHeader(k, v);
  }

  // Middleware short-circuit (e.g. CORS preflight, rate limit)
  if (ctx.body != null) {
    res.writeHead(ctx.status);
    res.end(ctx.body);
    return;
  }

  // SSE endpoint
  if (method === 'GET' && url === '/sse') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    });
    const snap = snapshot();
    res.write(`event: message\ndata: ${snap}\n\n`);
    sseClients.add(res);
    req.on('close', () => sseClients.delete(res));
    return;
  }

  // POST /actions/:action
  if (method === 'POST' && url!.startsWith('/actions/')) {
    const action = decodeURIComponent(url!.slice('/actions/'.length));
    const body = await readBody(req);
    let payload: any = {};
    try { const p = JSON.parse(body); payload = p.payload || p; } catch {}

    // Navigate action → update current path, handle guards/redirects
    if (action === 'navigate') {
      const path = payload?.path || '/';
      currentPath = path;

      // Check for redirect
      const result = router.resolve(path, toViewModel(state));
      if (result && result.kind === 'redirect') {
        // Tell client to navigate to redirect target
        const rSnap = snapshot(result.to);
        currentPath = result.to;
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(rSnap);
        return;
      }

      const snap = snapshot(path);
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(snap);
      return;
    }

    state = safeReducer(state, action, payload);
    const snap = snapshot();

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(snap);
    broadcast(snap);
    return;
  }

  // Static files with content-hash cache headers
  if (method === 'GET' && url.startsWith('/static/')) {
    const staticPath = url.slice('/static/'.length);
    const result = serveStatic(ASSETS_DIR, staticPath, manifest);
    if (result.found) {
      res.writeHead(200, result.headers);
      res.end(result.content);
      return;
    }
    res.writeHead(404); res.end(); return;
  }

  // Legacy static paths (magnetic.js, transport.wasm — unversioned)
  if (method === 'GET') {
    const ext = extname(url);
    if (ext && ext !== '.html') {
      const result = serveStatic(PUBLIC, url);
      if (result.found) {
        res.writeHead(200, result.headers);
        res.end(result.content);
        return;
      }
      res.writeHead(404); res.end(); return;
    }

    // SSR: any GET to a route path → full HTML page
    currentPath = url.split('?')[0];
    const dom = safeRender(currentPath);

    // Read CSS
    let inlineCSS = '';
    const cssPath = resolve(PUBLIC, 'style.css');
    if (existsSync(cssPath)) {
      inlineCSS = readFileSync(cssPath, 'utf-8');
    }

    const html = renderPage({
      root: dom,
      scripts: [asset('magnetic.js')],
      inlineCSS,
      sseUrl: '/sse',
      mountSelector: '#app',
      wasmUrl: asset('transport.wasm'),
      title: 'Magnetic Task Board',
      description: 'A server-driven task board built with Magnetic TSX components',
    });

    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(html);
    return;
  }

  res.writeHead(404);
  res.end('Not Found');
});

server.listen(PORT, () => {
  console.log(`[task-board] http://localhost:${PORT}`);
  console.log(`[task-board] SSR + nested routing + guards + middleware + error boundaries`);
  console.log(`[task-board] Routes: /, /about, /admin (guarded→redirect), /old-about (→/about)`);
});
