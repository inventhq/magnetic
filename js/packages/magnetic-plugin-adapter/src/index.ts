// @magnetic/plugin-adapter
// Wraps a Magnetic app (TSX components + state) into an @inventhq/plugin-sdk plugin.
//
// The plugin-sdk V8 isolate provides these globals:
//   definePlugin, RuntimeHelper, defineTable
//
// After esbuild bundles this, the output is a single JS file with no imports
// that runs inside the plugin-sdk runtime.

// ── Plugin-SDK globals (declared for TypeScript, available at runtime) ───

declare const RuntimeHelper: {
  new (runtime: PluginRuntime): PluginRuntimeHelper;
};

interface PluginRuntime {
  getState(key: string): string | null;
  setState(key: string, value: string): void;
  emit(eventType: string, params?: Record<string, string>, rawPayload?: unknown): void;
  log: { info(msg: string): void; warn(msg: string): void; error(msg: string): void };
}

interface PluginRuntimeHelper {
  getStateOr(key: string, defaultValue: string): string;
  setState(key: string, value: string | number | boolean): void;
  emit(eventType: string, params?: Record<string, unknown>, rawPayload?: unknown): void;
  info(msg: string): void;
  warn(msg: string): void;
  error(msg: string): void;
}

interface PluginEvent {
  event_id: string;
  event_type: string;
  timestamp: string;
  params: Record<string, string>;
  raw_payload?: unknown;
}

// ── Magnetic Plugin Config ──────────────────────────────────────────

export interface DomNode {
  tag: string;
  key?: string;
  attrs?: Record<string, string>;
  events?: Record<string, string>;
  text?: string;
  children?: DomNode[];
}

export interface MagneticPluginConfig<S> {
  /** Plugin name (shown in plugin-sdk dashboard) */
  name: string;

  /** Factory that returns the initial app state */
  initialState: () => S;

  /** Pure reducer: (state, actionName, payload) → newState */
  reduce: (state: S, action: string, payload: Record<string, string>) => S;

  /** Render function: state → JSON DOM root node (call your TSX App component here) */
  render: (state: S) => DomNode;
}

// ── Adapter ─────────────────────────────────────────────────────────

/**
 * Creates a plugin-sdk plugin definition from a Magnetic app config.
 *
 * Usage:
 * ```ts
 * export default definePlugin(createMagneticPlugin({
 *   name: "my-app",
 *   initialState,
 *   reduce,
 *   render: (state) => App(toViewModel(state)),
 * }));
 * ```
 */
export function createMagneticPlugin<S>(config: MagneticPluginConfig<S>) {
  const STATE_KEY = `magnetic:${config.name}:state`;

  function loadState(rt: PluginRuntimeHelper): S {
    const raw = rt.getStateOr(STATE_KEY, "");
    if (!raw) return config.initialState();
    try {
      return JSON.parse(raw) as S;
    } catch {
      return config.initialState();
    }
  }

  function saveState(rt: PluginRuntimeHelper, state: S): void {
    rt.setState(STATE_KEY, JSON.stringify(state));
  }

  function renderSnapshot(state: S): { root: DomNode } {
    return { root: config.render(state) };
  }

  return {
    name: config.name,
    events: ["magnetic.action", "magnetic.connect"],

    async onEvent(event: PluginEvent, runtime: PluginRuntime) {
      const rt = new RuntimeHelper(runtime);

      // ── magnetic.connect: new client, return current snapshot ────
      if (event.event_type === "magnetic.connect") {
        const state = loadState(rt);
        const snapshot = renderSnapshot(state);
        rt.info(`[magnetic] connect → ${config.name}`);

        // Option A: return snapshot as HTTP response body
        // (user is implementing this in plugin-sdk)
        return snapshot;
      }

      // ── magnetic.action: reduce + render + broadcast ────────────
      if (event.event_type === "magnetic.action") {
        const action = event.params.action || "";
        if (!action) {
          rt.warn("[magnetic] empty action, skipping");
          return;
        }

        // Load → reduce → save
        let state = loadState(rt);
        state = config.reduce(state, action, event.params);
        saveState(rt, state);

        // Render
        const snapshot = renderSnapshot(state);
        const snapJson = JSON.stringify(snapshot);

        // Broadcast to other connected clients via SSE
        rt.emit("magnetic.snapshot", { snapshot: snapJson });

        rt.info(`[magnetic] ${action} → snapshot ${snapJson.length}b`);

        // Option A: return snapshot to acting client
        return snapshot;
      }
    },
  };
}
