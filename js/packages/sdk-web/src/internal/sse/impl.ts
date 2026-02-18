// Minimal SSE implementation hidden behind the kernel.
// No DOM/UI imports; uses globalThis.EventSource (polyfilled in tests).

import type { Envelope } from "../../kernel"; // type-only (doesn't create a dep cycle)
import type { Handler, Unsubscribe } from "../../kernel";

type Topic = string;

export class InternalSSE {
  private es: EventSource | null = null;
  private handlers: Map<Topic, Set<Handler>> = new Map();
  private url: string;
  private init: { headers?: Record<string, string>; protocols?: string[] } = {};

  constructor(url: string, init?: { headers?: Record<string, string>; protocols?: string[] }) {
    this.url = url;
    this.init = init ?? {};
  }

  connect(): void {
    if (this.es) return;
    const ES: typeof EventSource | undefined = (globalThis as any).EventSource;
    if (!ES) throw new Error("EventSource not available — polyfill or run in browser");

    // Some polyfills (e.g., 'eventsource') support { headers }
    const options: any = {};
    if (this.init.headers) options.headers = this.init.headers;

    this.es = new ES(this.url, options);

    this.es.addEventListener("message", (ev: MessageEvent) => {
      try {
        const env = JSON.parse((ev as MessageEvent).data as unknown as string) as Envelope<any>;
        const set = this.handlers.get(env.topic);
        if (!set || set.size === 0) return;
        for (const fn of set) fn(env);
      } catch {
        // ignore malformed frames (dev simulator only)
      }
    });
  }

  on(topic: string, fn: Handler): Unsubscribe {
    if (!this.es) this.connect();
    let set = this.handlers.get(topic);
    if (!set) {
      set = new Set();
      this.handlers.set(topic, set);
    }
    set.add(fn);
    return () => {
      const s = this.handlers.get(topic);
      if (!s) return;
      s.delete(fn);
      if (s.size === 0) this.handlers.delete(topic);
    };
  }

  close(): void {
    if (this.es) {
      try {
        this.es.close();
      } catch {
        /* noop */
      }
      this.es = null;
    }
    this.handlers.clear();
  }
}
