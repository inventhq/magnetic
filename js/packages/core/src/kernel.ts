// Public kernel surface for @magnetic/core
// C3: exports envelope types + reducer registry API only.

export type ReducerName = "replace" | "append" | "merge" | "patch" | "downsample";

/**
 * Canonical envelope shape exchanged over transports.
 */
export interface Envelope<T = unknown> {
  topic: string;
  seq: number;
  ts: number;
  version: number;
  data: T;
}

/**
 * Registry of reducers, hidden behind allowlist.
 */
const reducers: Record<ReducerName, (a: any, b: any) => any> = {
  // Note: `void a` satisfies noUnusedParameters without changing behavior.
  replace: (a, b) => { void a; return b; },
  append: (a, b) => [...(a ?? []), ...(b ?? [])],
  merge: (a, b) => ({ ...(a ?? {}), ...(b ?? {}) }),
  patch: (a, b) => {
    if (typeof a !== "object" || typeof b !== "object") return b;
    return { ...a, ...b };
  },
  downsample: (a, b) => { void a; return b; } // placeholder
};

/**
 * Reduce two payloads using a named reducer.
 */
export function reduce(name: ReducerName, prev: unknown, next: unknown): unknown {
  const fn = reducers[name];
  if (!fn) throw new Error(`Unknown reducer: ${name}`);
  return fn(prev, next);
}

/**
 * Register a custom reducer under an allowed name.
 */
export function register(name: ReducerName, fn: (a: any, b: any) => any): void {
  reducers[name] = fn;
}
