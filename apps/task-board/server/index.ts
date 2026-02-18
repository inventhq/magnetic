// server/index.ts — Task Board HTTP server
// Compiles .magnetic.html templates at startup, serves SSE + POST + static

import { createServer, IncomingMessage, ServerResponse } from 'node:http';
import { readFileSync, existsSync } from 'node:fs';
import { join, extname, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compile } from '../../../js/packages/magnetic-compiler/src/index.js';
import { render } from './render.ts';
import type { DomNode } from './render.ts';
import { initialState, reduce, toViewModel } from './state.ts';
import type { AppState } from './state.ts';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const PUBLIC = join(ROOT, 'public');
const COMPONENTS = join(ROOT, 'components');

// ── Compile templates at startup ────────────────────────────────────

function loadTemplate(name: string): any[] {
  const src = readFileSync(join(COMPONENTS, `${name}.magnetic.html`), 'utf8');
  const result = compile(src, { name });
  return result.ops;
}

const appOps = loadTemplate('app');
const components = new Map<string, any[]>();
// Register sub-components here if needed:
// components.set('TaskCard', loadTemplate('task-card'));

console.log(`[task-board] compiled app.magnetic.html (${appOps.length} ops)`);

// ── App state ───────────────────────────────────────────────────────

let state: AppState = initialState();

function snapshot(): string {
  const vm = toViewModel(state);
  const dom = render(appOps, vm, components);
  return JSON.stringify(dom);
}

// ── SSE clients ─────────────────────────────────────────────────────

const sseClients: Set<ServerResponse> = new Set();

function broadcast(data: string): void {
  const msg = `event: message\ndata: ${data}\n\n`;
  for (const res of sseClients) {
    try {
      res.write(msg);
    } catch {
      sseClients.delete(res);
    }
  }
}

// ── HTTP server ─────────────────────────────────────────────────────

const PORT = parseInt(process.argv[2] || '3003', 10);

const MIME: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
};

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve) => {
    let body = '';
    req.on('data', (chunk: Buffer) => { body += chunk.toString(); });
    req.on('end', () => resolve(body));
  });
}

const server = createServer(async (req: IncomingMessage, res: ServerResponse) => {
  const url = req.url || '/';
  const method = req.method || 'GET';

  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');

  if (method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  // SSE endpoint
  if (method === 'GET' && url === '/sse') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    });

    // Send initial snapshot
    const snap = snapshot();
    res.write(`event: message\ndata: ${snap}\n\n`);

    sseClients.add(res);
    req.on('close', () => sseClients.delete(res));
    return;
  }

  // POST /actions/:action — process action, return snapshot
  if (method === 'POST' && url!.startsWith('/actions/')) {
    const action = decodeURIComponent(url!.slice('/actions/'.length));
    const body = await readBody(req);
    let payload: any = {};
    try {
      const parsed = JSON.parse(body);
      payload = parsed.payload || parsed;
    } catch {}

    // Reduce
    state = reduce(state, action, payload);

    // Render snapshot
    const snap = snapshot();

    // Return snapshot in POST response (single round-trip)
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(snap);

    // Broadcast to other SSE clients
    broadcast(snap);
    return;
  }

  // Static files
  if (method === 'GET') {
    let filePath = url === '/' ? '/index.html' : url!;

    // Security: prevent path traversal
    const resolved = resolve(PUBLIC + filePath);
    if (!resolved.startsWith(resolve(PUBLIC))) {
      res.writeHead(403);
      res.end();
      return;
    }

    if (existsSync(resolved)) {
      const ext = extname(resolved);
      const contentType = MIME[ext] || 'application/octet-stream';
      const data = readFileSync(resolved);
      res.writeHead(200, { 'Content-Type': contentType });
      res.end(data);
      return;
    }
  }

  res.writeHead(404);
  res.end('Not Found');
});

server.listen(PORT, () => {
  console.log(`[task-board] http://localhost:${PORT}`);
  console.log(`[task-board] ${state.tasks.length} tasks loaded`);
});
