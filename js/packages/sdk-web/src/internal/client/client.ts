import type { Envelope } from "../../kernel";
import type { Source, Handler, Unsubscribe } from "../../kernel";
import { getStreamSync } from "../manifest/provider";

export type Topic = string;

export interface ClientImplStatus {
  connected: boolean;
  retries: number;
  lagMs?: number;
  topics: Record<
    Topic,
    {
      lastSeq: number;
      lastTs: number;
      gap: boolean;
      since?: number;
      reducer?: string;
    }
  >;
}

// Make onChange a required field but nullable; this avoids optional-vs-required variance in Set.add
type Derivation = {
  deps: (() => unknown)[];
  compute: (...values: unknown[]) => unknown;
  onChange: ((v: unknown) => void) | null;
  lastValue?: unknown;
};

export class ClientImpl {
  private source: Source;
  private unsubs: Map<Topic, Unsubscribe> = new Map();
  private lastSeq: Map<Topic, number> = new Map();
  private lastTs: Map<Topic, number> = new Map();
  private gaps: Set<Topic> = new Set();
  // [P3-C6:lastData:field]
  private lastData: Map<Topic, unknown> = new Map();


  private derivations: Set<Derivation> = new Set();
  private batchHandle: ReturnType<typeof setTimeout> | null = null;

  public status: ClientImplStatus = { connected: false, retries: 0, topics: {} };

  constructor(source: Source) {
    this.source = source;
  }

  subscribe(
    topic: string,
    opts?: { since?: number; onDelta?: Handler; reducer?: string }
  ): Unsubscribe {
    this.status.connected = true;

    // [P3-C4:subscribe:apply-manifest-hints]
    {
      const m = getStreamSync(topic);

      // If caller omitted reducer, use manifest hint (metadata only; real reduction happens server-side in P4).
      if (!opts || !("reducer" in opts) || opts.reducer == null) {
        const hinted = m?.hints?.reducer;
        if (typeof hinted === "string" && hinted.length > 0) {
          opts = { ...(opts ?? {}), reducer: hinted as any };
        }
      }

      // If caller omitted since, set a safe numeric default when resume is supported by manifest.
      // Note: transport param key (e.g., resume_param "since") is handled in the transport layer; here we only store the value.
      if (!opts || !("since" in opts) || opts.since == null) {
        if (m?.hints?.resume_param) {
          opts = { ...(opts ?? {}), since: 0 };
        }
      }
    }

    this.ensureTopic(topic, opts?.since, opts?.reducer);

    const handler: Handler = (env: Envelope<any>) => {
      // gap detection
      const prev = this.lastSeq.get(topic) ?? 0;
      if (env.seq > prev + 1) this.gaps.add(topic);
      this.lastSeq.set(topic, env.seq);
      this.lastTs.set(topic, env.ts);
      // [P3-C6:lastData:update]
      this.lastData.set(topic, (env as any).data);
      this.updateTopicStatus(topic);

      // recompute lag
      this.status.lagMs = Math.max(0, Date.now() - env.ts);

      // consumer callback
      if (opts?.onDelta) opts.onDelta(env);

      // schedule derived recomputation
      this.scheduleDerive();
    };

    const off = this.source.on(topic, handler);
    this.unsubs.set(topic, off);

    return () => {
      const u = this.unsubs.get(topic);
      if (u) {
        u();
        this.unsubs.delete(topic);
      }
      this.lastSeq.delete(topic);
      this.lastTs.delete(topic);
      this.gaps.delete(topic);
      delete this.status.topics[topic];

      // [P3-C6:lastData:cleanup]
      this.lastData.delete(topic);

      // [P3-C6:batch:cleanup-when-idle]
      if (this.unsubs.size === 0 && this.batchHandle) {
        clearTimeout(this.batchHandle);
        this.batchHandle = null;
      }

      if (this.unsubs.size === 0) this.status.connected = false;
    };
  }

  // Non-generic to avoid variance at the kernel boundary
  registerDerived(
    deps: Array<() => unknown>,
    compute: (...values: unknown[]) => unknown,
    onChange?: (value: unknown) => void
  ): void {
    this.derivations.add({ deps, compute, onChange: onChange ?? null });
  }

  // [P3-C6:select:helper]
  // Returns a dep fn for derived(): reads last payload for `topic` and applies optional selector.
  public select(topic: Topic, selector?: (data: unknown) => unknown): () => unknown {
    return () => {
      const d = this.lastData.get(topic);
      return selector ? selector(d) : d;
    };
  }

  // Declare the exact union to appease TS (kernel maps this to Status with optional lagMs)
  getStatus(): { connected: boolean; lagMs: number | undefined; retries: number } {
    return {
      connected: this.status.connected,
      lagMs: this.status.lagMs,
      retries: this.status.retries
    };
  }

  private scheduleDerive(): void {
    if (this.batchHandle) return;
    this.batchHandle = setTimeout(() => {
      this.batchHandle = null;
      for (const d of this.derivations) {
        const values = d.deps.map((fn) => fn());
        const next = d.compute(...values);
        if (d.onChange && next !== d.lastValue) {
          d.lastValue = next;
          d.onChange(next);
        }
      }
    }, 16);
  }

  private ensureTopic(topic: string, since?: number, reducer?: string): void {
    if (!this.status.topics[topic]) {
      const base = { lastSeq: 0, lastTs: 0, gap: false as boolean };
      this.status.topics[topic] = {
        ...base,
        ...(since !== undefined ? { since } : {}),
        ...(reducer !== undefined ? { reducer } : {})
      };
    }
  }

  private updateTopicStatus(topic: string): void {
    const t = this.status.topics[topic];
    if (!t) return;
    t.lastSeq = this.lastSeq.get(topic) ?? 0;
    t.lastTs = this.lastTs.get(topic) ?? 0;
    t.gap = this.gaps.has(topic);
  }
}
