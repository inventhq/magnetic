// [P3-C4:provider:file]
// Minimal manifest provider with fetch/caching and HTTP-classified errors.
// Defaults assume the repo serves /contracts/.well-known/* at runtime (dev).
// If you serve from elsewhere, adjust BASE below (internal only).

// [P3-C4:provider:errors]
export class ManifestHttpError extends Error {
  readonly status: number;
  constructor(msg: string, status: number) { super(msg); this.name = "ManifestHttpError"; this.status = status; }
}
export class ManifestNotFoundError extends ManifestHttpError {
  constructor(msg = "manifest not found", status = 404) { super(msg, status); this.name = "ManifestNotFoundError"; }
}
export class ManifestClientError extends ManifestHttpError {
  constructor(msg = "manifest client error", status: number) { super(msg, status); this.name = "ManifestClientError"; }
}
export class ManifestServerError extends ManifestHttpError {
  constructor(msg = "manifest server error", status: number) { super(msg, status); this.name = "ManifestServerError"; }
}

// Shapes we care about (narrow)
type StreamHint = { reducer?: string; resume_param?: string; lease?: { ms?: number } };
export type StreamManifest = { topic: string; transport: "sse" | "ws"; schema?: unknown; hints?: StreamHint };
export type ActionManifest = { name: string; method: string; path: string; idempotency_header?: string; params_schema?: unknown; ack_schema?: unknown };

type StreamsDoc = { version: number; streams: StreamManifest[] };
type ActionsDoc = { version: number; actions: ActionManifest[] };

// [P3-C4:provider:fetch-cache]
const BASE = "/contracts/.well-known";
let _streams: StreamsDoc | null = null;
let _actions: ActionsDoc | null = null;
let _preload: Promise<void> | null = null;

// [P3-C4:provider:fetch-cache] (patched to avoid undefined signal with exactOptionalPropertyTypes)
async function fetchJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  const init: RequestInit = signal ? { signal } : {};
  const r = await fetch(url, init);
  if (!r.ok) {
    if (r.status === 404) throw new ManifestNotFoundError(`${url} not found`, 404);
    if (r.status >= 400 && r.status < 500) throw new ManifestClientError(`${url} ${r.status}`, r.status);
    throw new ManifestServerError(`${url} ${r.status}`, r.status);
  }
  return (await r.json()) as T;
}


async function loadAll(): Promise<void> {
  // small timeout to avoid hanging tests
  const ac = new AbortController();
  const t = setTimeout(() => ac.abort(), 5000);
  try {
    const [streams, actions] = await Promise.all([
      fetchJson<StreamsDoc>(`${BASE}/streams.json`, ac.signal),
      fetchJson<ActionsDoc>(`${BASE}/actions.json`, ac.signal),
    ]);
    _streams = streams;
    _actions = actions;
  } finally {
    clearTimeout(t);
  }
}

// Kick off background preload at module import (non-blocking)
_preload = loadAll().catch(() => { /* leave caches null; callers can await get* */ });

// [P3-C4:provider:guards]
function findStreamLocal(topic: string): StreamManifest | null {
  if (!_streams) return null;
  return _streams.streams.find(s => s.topic === topic) ?? null;
}
function findActionLocal(name: string): ActionManifest | null {
  if (!_actions) return null;
  return _actions.actions.find(a => a.name === name) ?? null;
}

// [P3-C4:provider:getStream]
export function getStreamSync(topic: string): StreamManifest | null {
  return findStreamLocal(topic);
}
export async function getStream(topic: string): Promise<StreamManifest | null> {
  if (!_streams) { try { await (_preload ?? loadAll()); } catch { /* bubble by returning null */ } }
  return findStreamLocal(topic);
}

// [P3-C4:provider:getAction]
export function getActionSync(name: string): ActionManifest | null {
  return findActionLocal(name);
}
export async function getAction(name: string): Promise<ActionManifest | null> {
  if (!_actions) { try { await (_preload ?? loadAll()); } catch { /* bubble by returning null */ } }
  return findActionLocal(name);
}
