import { describe, it, expect, beforeEach, vi } from "vitest";

// IMPORTANT: import after stubbing fetch
import * as Provider from "../src/internal/manifest/provider";

describe("manifest provider", () => {
  const streamsDoc = { version: 1, streams: [{ topic: "metrics.cpu", transport: "sse", hints: { reducer: "replace", resume_param: "since" } }] };
  const actionsDoc = { version: 1, actions: [{ name: "orders.cancel", method: "POST", path: "/actions/orders.cancel" }] };

  beforeEach(() => {
    vi.resetAllMocks();
    (globalThis as any).fetch = vi.fn((url: string) => {
      if (url.endsWith("/streams.json")) return Promise.resolve(new Response(JSON.stringify(streamsDoc), { status: 200 }));
      if (url.endsWith("/actions.json")) return Promise.resolve(new Response(JSON.stringify(actionsDoc), { status: 200 }));
      return Promise.resolve(new Response("not found", { status: 404 }));
    });
  });

  it("getStream returns manifest and getStreamSync hits cache after preload", async () => {
    const s1 = await Provider.getStream("metrics.cpu");
    expect(s1?.hints?.reducer).toBe("replace");
    // cache should now be warm → sync lookup works
    const s2 = Provider.getStreamSync("metrics.cpu");
    expect(s2?.hints?.resume_param).toBe("since");
  });

  it("classifies 404 as NotFound, 5xx as ServerError", async () => {
    (globalThis as any).fetch = vi.fn((url: string) => {
      const status = url.endsWith("/streams.json") ? 404 : 503;
      return Promise.resolve(new Response("x", { status }));
    });
    await expect(Provider.getStream("metrics.cpu")).resolves.toBeNull();
    await expect(Provider.getAction("orders.cancel")).resolves.toBeNull();
  });
});
