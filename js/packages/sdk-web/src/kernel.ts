// Public kernel surface for @magnetic/sdk-web (C4)
// Signatures only — zero transport/DOM behavior. Runtime calls throw.

// Reuse Envelope from core without importing to avoid cross-pkg dep at C4.
import { InternalSSE } from "./internal/sse/impl";
import { InternalWS } from "./internal/ws/impl";
import { ClientImpl } from "./internal/client/client";

export interface Envelope<T = unknown> {
  topic: string;
  seq: number;
  ts: number;
  version: number;
  data: T;
}

export type Unsubscribe = () => void;
export type Handler = (env: Envelope<any>) => void;

export interface Source {
  on(topic: string, fn: Handler): Unsubscribe;
  close(): void;
}

export interface SourceInit {
  headers?: Record<string, string>;
  protocols?: string[];
}

// Stubs: constructors do nothing; methods throw if called.
// No network, no DOM, no side effects in C4.
export class SSESource implements Source {
  private impl: InternalSSE;
  constructor(url: string, init?: SourceInit) {
    this.impl = new InternalSSE(url, init);
  }
  on(topic: string, fn: Handler): Unsubscribe {
    return this.impl.on(topic, fn);
  }
  close(): void {
    this.impl.close();
  }
}

export class WSSource implements Source {
  private impl: InternalWS;
  constructor(url: string, init?: SourceInit) {
    this.impl = new InternalWS(url, init);
  }
  on(topic: string, fn: Handler): Unsubscribe {
    return this.impl.on(topic, fn);
  }
  close(): void {
    this.impl.close();
  }
}

export interface Client {
  subscribe(
    topic: string,
    opts?: { since?: number; onDelta?: Handler; reducer?: string }
  ): Unsubscribe;
}

let _clientImpl: ClientImpl | null = null;

export function createClient(source: Source): Client {
  _clientImpl = new ClientImpl(source);
  return {
    subscribe(topic, opts) {
      if (!_clientImpl) throw new Error("client not initialized");
      return _clientImpl.subscribe(topic, opts);
    }
  };
}

// Minimal derived batching tied to the current client instance.
// deps are functions returning current values; we recompute on incoming deltas.
export function derived(
  deps: Array<() => unknown>,
  compute: (...values: unknown[]) => unknown,
  onChange?: (value: unknown) => void
): void {
  if (!_clientImpl) throw new Error("client not initialized");
  _clientImpl.registerDerived(deps, compute, onChange);
}

export interface Status {
  connected: boolean;
  lagMs?: number;
  retries: number;
}

export function status(): Status {
  if (!_clientImpl) return { connected: false, retries: 0 };
  const s = _clientImpl.getStatus();
  // Map { lagMs: number | undefined } to optional lagMs
  return s.lagMs === undefined
    ? { connected: s.connected, retries: s.retries }
    : { connected: s.connected, retries: s.retries, lagMs: s.lagMs };
}

export async function action(
  name: string,
  payload: unknown,
  opts?: { idempotencyKey?: string; headers?: Record<string, string> }
): Promise<{ status: "OK" | "ERR"; applied_seq?: number; error?: string; echoedKey?: string }> {
  // Accept absolute URLs (e.g., 'http://localhost:6060/actions/echo') or verb names (-> '/actions/:name')
  const url = /^https?:\/\//i.test(name) ? name : `/actions/${encodeURIComponent(name)}`;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(opts?.headers ?? {})
  };
  if (opts?.idempotencyKey) headers["Idempotency-Key"] = opts.idempotencyKey;

  const resp = await fetch(url, {
    method: "POST",
    headers,
    body: JSON.stringify(payload)
  });

  // 2xx/4xx both return JSON body in our dev echo; parse and map
  let body: any = null;
  try {
    body = await resp.json();
  } catch {
    body = null;
  }

  // Validate minimal shape
  const statusVal = body?.status === "OK" ? "OK" : body?.status === "ERR" ? "ERR" : "ERR";
  return {
    status: statusVal,
    applied_seq: typeof body?.applied_seq === "number" ? body.applied_seq : undefined,
    error: typeof body?.error === "string" ? body.error : statusVal === "ERR" ? "unknown_error" : undefined,
    echoedKey: typeof body?.echoedKey === "string" ? body.echoedKey : undefined
  };
}

