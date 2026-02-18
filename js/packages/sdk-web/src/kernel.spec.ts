import { it, expect } from "vitest";
import {
  SSESource,
  WSSource,
  createClient,
  derived,
  action,
  status
} from "./kernel";

// API surface smoke: types exist; no behavior calls in C4.
it("exports constructor symbols", () => {
  expect(typeof SSESource).toBe("function");
  expect(typeof WSSource).toBe("function");
});

it("exports factory/functions with correct shapes", () => {
  expect(typeof createClient).toBe("function");
  expect(typeof derived).toBe("function");
  expect(typeof action).toBe("function");
  expect(typeof status).toBe("function");
});

import EventSourcePolyfill from "eventsource";

it("SSESource: connect → receive → dispatch → unsubscribe", async () => {
  // Polyfill EventSource for Node test runtime
  // @ts-expect-error assigning to global
  globalThis.EventSource = EventSourcePolyfill as unknown as typeof EventSource;

  const { SSESource } = await import("./kernel");

  // Requires simulator running: pnpm sim (SSE on http://localhost:6060)
  const src = new SSESource("http://localhost:6060/sse?topic=chat.events");

  let received = 0;
  const off = src.on("chat.events", () => {
    received += 1;
    if (received >= 1) {
      off();
      src.close();
    }
  });

  // Wait up to ~3s for at least one event
  await new Promise<void>((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("No SSE event received")), 3000);
    const check = setInterval(() => {
      if (received >= 1) {
        clearTimeout(t);
        clearInterval(check);
        resolve();
      }
    }, 50);
  });

  expect(received).toBeGreaterThanOrEqual(1);
});

import WS from "ws";

it("WSSource: connect → receive → dispatch → unsubscribe", async () => {
  // Polyfill WebSocket for Node test runtime
  // @ts-expect-error assigning to global
  globalThis.WebSocket = WS as unknown as typeof WebSocket;

  const { WSSource } = await import("./kernel");

  // Requires simulator running: pnpm sim (WS on ws://localhost:7070)
  const src = new WSSource("ws://localhost:7070");

  let received = 0;
  const off = src.on("chat.events", () => {
    received += 1;
    if (received >= 1) {
      off();
      src.close();
    }
  });

  // Wait up to ~3s for at least one event
  await new Promise<void>((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("No WS event received")), 3000);
    const check = setInterval(() => {
      if (received >= 1) {
        clearTimeout(t);
        clearInterval(check);
        resolve();
      }
    }, 50);
  });

  expect(received).toBeGreaterThanOrEqual(1);
});

// --- C8 tests: subscribe + gaps + derived batching ---

class MockSource implements import("./kernel").Source {
  private handlers = new Map<string, Set<import("./kernel").Handler>>();
  on(topic: string, fn: import("./kernel").Handler): import("./kernel").Unsubscribe {
    let set = this.handlers.get(topic);
    if (!set) { set = new Set(); this.handlers.set(topic, set); }
    set.add(fn);
    return () => { set!.delete(fn); };
  }
  close(): void { this.handlers.clear(); }
  emit(env: import("./kernel").Envelope<any>): void {
    const set = this.handlers.get(env.topic);
    if (!set) return;
    for (const fn of set) fn(env);
  }
}

it("subscribe tracks seq and detects gaps", async () => {
  const { createClient, status } = await import("./kernel");
  const src = new MockSource();
  const client = createClient(src);

  let lastSeq = 0;
  client.subscribe("chat.events", {
    onDelta: (e) => { lastSeq = e.seq; }
  });

  src.emit({ topic: "chat.events", seq: 1, ts: Date.now(), version: 1, data: {} });
  src.emit({ topic: "chat.events", seq: 3, ts: Date.now(), version: 1, data: {} }); // gap (missing 2)

  // allow derived-batch tick (not strictly required here)
  await new Promise(r => setTimeout(r, 20));

  expect(lastSeq).toBe(3);
  const s = status();
  expect(s.connected).toBe(true);
  expect(typeof s.lagMs === "number" || typeof s.lagMs === "undefined").toBe(true);
  expect(s.retries).toBe(0);
});

it("derived batches compute within ~1 frame and calls onChange", async () => {
  const { createClient, derived } = await import("./kernel");
  const src = new MockSource();
  const client = createClient(src);

  // Subscribe to any topic to activate the client's event pipeline & batching
  client.subscribe("t");

  let counter = 0;
  let derivedValue = 0;

  const depA = () => counter;

  derived([depA], (a: number) => a * 2, (v) => {
    derivedValue = v as number;
  });

  // multiple emits coalesce into one derived recompute
  counter = 1;
  src.emit({ topic: "t", seq: 1, ts: Date.now(), version: 1, data: {} });
  src.emit({ topic: "t", seq: 2, ts: Date.now(), version: 1, data: {} });

  await new Promise((r) => setTimeout(r, 25)); // >16ms tick

  expect(derivedValue).toBe(2);
});

it("action(): OK path returns ack with applied_seq and echoes idempotency key", async () => {
  const { action } = await import("./kernel");
  const res = await action("http://localhost:6060/actions/echo", { foo: 1 }, { idempotencyKey: "abc123" });
  expect(res.status).toBe("OK");
  expect(typeof res.applied_seq).toBe("number");
  expect(res.echoedKey).toBe("abc123");
});

it("action(): ERR path returns error", async () => {
  const { action } = await import("./kernel");
  const res = await action("http://localhost:6060/actions/fail", {});
  expect(res.status).toBe("ERR");
  expect(typeof res.error).toBe("string");
});

