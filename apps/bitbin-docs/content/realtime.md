---
title: Real-Time Subscriptions
order: 5
---

# Chapter 5: Real-Time Subscriptions

**Push-based reactive queries: updates stream to you only when data changes.**

---

## Overview

BitBin supports two real-time subscription mechanisms:

1. **SSE (Server-Sent Events)** — HTTP-based, works everywhere, ideal for dashboards and web apps
2. **WebSocket** — Binary protocol, ideal for SDKs and high-throughput applications

Both use fingerprint-based change detection: the server only pushes when the query result actually changes, not on every mutation.

---

## SSE Subscriptions

### Single Query

Subscribe to one query via `POST /subscribe`:

```bash
curl -X POST https://{subdomain}.machx.dev/subscribe \
  -H 'Content-Type: application/json' \
  -d '{
    "space": {"tenant": 1},
    "measure": {"type": "count"}
  }'
```

The server responds with an SSE stream:

```
event: result
data: {"event":"initial","fingerprint":"a1b2c3d4e5f6a7b8","result":{"count":42000,"query_us":9}}

event: result
data: {"event":"update","fingerprint":"b2c3d4e5f6a7b8a1","result":{"count":42001,"query_us":8}}
```

- The first event is always `"event": "initial"` with the current result.
- Subsequent events are `"event": "update"` and only arrive when the result changes.
- Each event includes a `fingerprint` for client-side deduplication.

### Multiplexed Queries

Subscribe to N queries over a **single SSE connection** via `POST /subscribe-multi`. This solves the browser 6-connection-per-host limit:

```bash
curl -X POST https://{subdomain}.machx.dev/subscribe-multi \
  -H 'Content-Type: application/json' \
  -d '{
    "queries": {
      "total-count": {"space": {"tenant": 1}, "measure": {"type": "count"}},
      "total-revenue": {"space": {"tenant": 1}, "measure": {"type": "sum"}, "of": "amount"},
      "region-3": {"space": {"tenant": 1, "region": 3}, "measure": {"type": "count"}}
    }
  }'
```

Each event includes an `id` field identifying which query changed:

```
data: {"id":"total-count","event":"initial","fingerprint":"...","result":{"count":42000,"query_us":9}}
data: {"id":"total-revenue","event":"initial","fingerprint":"...","result":{"sum":18500000,"query_us":12}}
data: {"id":"total-count","event":"update","fingerprint":"...","result":{"count":42001,"query_us":8}}
```

### Pipe Streams

Any saved pipe can be streamed via SSE:

```bash
curl https://{subdomain}.machx.dev/pipe/revenue-by-region/stream
```

The server pushes the pipe's query result whenever the underlying data changes.

---

## WebSocket Subscriptions

For higher performance, use the WebSocket protocol at `GET /ws/db`. See [WebSocket Protocol](./websocket.md) for the full specification.

### Subscribe (binary)

Send a SUBSCRIBE frame (opcode 0x06) with a query JSON payload. The `request_id` becomes the `subscription_id`.

### Receive Updates

The server pushes SUBSCRIPTION_DATA frames (opcode 0x07) whenever the result changes. The first push has `is_initial=1`.

### Unsubscribe

Send an UNSUBSCRIBE frame (opcode 0x08) with the `subscription_id`.

### JSON Mode

If using JSON mode over WebSocket:

```json
// Subscribe
{"op": "subscribe", "id": 2, "query": {"space": {"tenant": 1}, "measure": {"type": "sum"}, "of": "amount"}}

// Receive updates
{"op": "subscription_data", "id": 2, "initial": true, "fingerprint": "a1b2c3d4", "result": {"count": 42000, "sum": 18500000}}
{"op": "subscription_data", "id": 2, "initial": false, "fingerprint": "b2c3d4e5", "result": {"count": 42001, "sum": 18505000}}

// Unsubscribe
{"op": "unsubscribe", "id": 2}
```

---

## SDK Usage

### TypeScript

```typescript
import { BitBin } from "@bitbin/client";

const bb = new BitBin({ baseUrl: "https://{subdomain}.machx.dev" });

// Single subscription
const sub = bb.subscribe(
  { space: { amount: [1000, null] }, measure: { type: "count" } },
  (event) => console.log(`${event.event}: count=${event.result.count}`)
);

// Multiplexed subscriptions (ONE connection for N queries)
const multi = bb.subscribeMulti(
  {
    "total-count": { space: {}, measure: { type: "count" } },
    "high-value": { space: { amount: [5000, null] }, measure: { type: "sum" } },
  },
  (event) => console.log(`[${event.id}] ${event.event}: count=${event.result.count}`)
);

// Close when done
multi.close();
```

### Python

```python
from bitbin import BitBin

bb = BitBin("https://{subdomain}.machx.dev")

# Single subscription (runs in background thread)
sub = bb.subscribe(
    {"space": {"amount": [1000, None]}, "measure": {"type": "count"}},
    on_event=lambda evt: print(f"{evt.event}: count={evt.result.count}")
)

# Multiplexed subscriptions
multi = bb.subscribe_multi(
    {
        "total": {"space": {}, "measure": {"type": "count"}},
        "high": {"space": {"amount": [5000, None]}, "measure": {"type": "sum"}},
    },
    on_event=lambda evt: print(f"[{evt.id}] {evt.event}: count={evt.result.count}")
)

# Close when done
multi.close()
```

---

## Best Practices

- **Use multiplexed subscriptions** for dashboards with multiple widgets — one SSE connection instead of N.
- **Re-subscribe on reconnect** — if the connection drops, re-establish all subscriptions after receiving the WELCOME frame.
- **Use fingerprints for deduplication** — if your client receives duplicate events during reconnection, compare fingerprints to avoid redundant updates.
- **Prefer WebSocket for SDKs** — binary protocol has lower overhead and supports bidirectional communication.
- **Use SSE for simple integrations** — SSE works with `curl`, `EventSource`, and any HTTP client without WebSocket support.

---

[← Previous: Data Operations](./data-operations.md) · **Chapter 5** · [Next: WebSocket Protocol →](./websocket.md)
