// Minimal WS implementation hidden behind the kernel.
// No DOM/UI imports; uses globalThis.WebSocket (polyfilled in tests).

import type { Envelope } from "../../kernel";
import type { Handler, Unsubscribe } from "../../kernel";

type Topic = string;

export class InternalWS {
  private ws: WebSocket | null = null;
  private handlers: Map<Topic, Set<Handler>> = new Map();
  private url: string;
  private init: { headers?: Record<string, string>; protocols?: string[] } = {};

  constructor(url: string, init?: { headers?: Record<string, string>; protocols?: string[] }) {
    this.url = url;
    this.init = init ?? {};
  }

  connect(): void {
    if (this.ws) return;
    const WS: typeof WebSocket | undefined = (globalThis as any).WebSocket;
    if (!WS) throw new Error("WebSocket not available — polyfill or run in browser");

    // Node polyfills (`ws`) accept protocols as 2nd arg; headers ignored here.
    this.ws = new WS(this.url, this.init.protocols);

    this.ws.onmessage = (ev: MessageEvent) => {
      try {
        const env = JSON.parse((ev.data as unknown as string)) as Envelope<any>;
        const set = this.handlers.get(env.topic);
        if (!set || set.size === 0) return;
        for (const fn of set) fn(env);
      } catch {
        // ignore malformed frames (dev simulator only)
      }
    };
  }

  on(topic: string, fn: Handler): Unsubscribe {
    if (!this.ws) this.connect();
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
    if (this.ws) {
      try {
        this.ws.close();
      } catch {
        /* noop */
      }
      this.ws = null;
    }
    this.handlers.clear();
  }
}

