---
title: WebSocket Protocol
order: 6
---

# Chapter 6: WebSocket Protocol

**Binary command multiplexer over a single persistent connection.**

BitBin exposes a unified WebSocket endpoint at `GET /ws/db` that supports all database operations — query, mutate, subscribe, unsubscribe, call pipeline — over a single connection. This is the primary transport for SDKs and the highest-performance way to interact with BitBin.

---

## Connection

```javascript
const ws = new WebSocket('wss://{subdomain}.machx.dev/ws/db');
```

On connect, the server sends a **Welcome** frame:

```
Binary frame (16 bytes):
  [0]     op = 0x01 (WELCOME)
  [1]     version = 1
  [2..3]  slot (u16 LE) — connection slot ID
  [4..7]  seq (u32 LE) — current sequence number
  [8..11] epoch (u32 LE) — current heartbeat epoch
  [12..15] max_slots (u32 LE) — server capacity
```

---

## Frame Format

All frames share a common header:

```
Byte 0:     opcode (u8)
Byte 1:     flags (u8)
Byte 2..3:  request_id (u16 LE) — client-assigned, echoed in response
Byte 4+:    payload (opcode-specific)
```

### Opcodes

| Code | Name | Direction | Description |
|------|------|-----------|-------------|
| 0x01 | WELCOME | S→C | Connection established |
| 0x02 | QUERY | C→S | Execute a query |
| 0x03 | QUERY_RESULT | S→C | Query result |
| 0x04 | MUTATE | C→S | Insert/update/delete records |
| 0x05 | MUTATE_ACK | S→C | Mutation acknowledged |
| 0x06 | SUBSCRIBE | C→S | Subscribe to a query |
| 0x07 | SUBSCRIPTION_DATA | S→C | Subscription initial/update |
| 0x08 | UNSUBSCRIBE | C→S | Cancel a subscription |
| 0x09 | UNSUBSCRIBE_ACK | S→C | Unsubscription confirmed |
| 0x0A | CALL_PIPELINE | C→S | Execute a named pipeline |
| 0x0B | PIPELINE_RESULT | S→C | Pipeline execution result |
| 0x0C | ERROR | S→C | Error response |
| 0x10 | HEARTBEAT | S→C | Heartbeat pulse |
| 0x11 | HEARTBEAT_ACK | C→S | Heartbeat response |

### Flags

| Bit | Name | Description |
|-----|------|-------------|
| 0 | JSON_PAYLOAD | Payload is JSON (not binary). For debugging/compatibility. |
| 1 | COMPRESSED | Payload is zstd-compressed |
| 2..7 | Reserved | |

---

## Operations

### QUERY (0x02)

Execute a query and get the result.

**Request (binary):**
```
[0]     0x02 (QUERY)
[1]     flags
[2..3]  request_id (u16 LE)
[4..5]  query_len (u16 LE) — length of JSON query payload
[6..]   query_json (UTF-8) — query JSON
```

**Response: QUERY_RESULT (0x03)**
```
[0]     0x03 (QUERY_RESULT)
[1]     flags
[2..3]  request_id (u16 LE) — echoed from request
[4..7]  query_us (u32 LE) — execution time in µs
[8..15] count (u64 LE) — matching record count
[16..23] sum (u64 LE) — sum (0 if N/A)
[24..31] min (u64 LE) — min (u64::MAX if N/A)
[32..39] max (u64 LE) — max (0 if N/A)
[40..41] num_groups (u16 LE) — number of group entries
[42..43] num_rows (u16 LE) — number of row entries
[44..]  group_data — packed GroupEntry structs
[..]    row_data — packed RowRecord structs
```

**GroupEntry (binary, 20 bytes each):**
```
[0..3]  tenant_id (u32 LE)
[4..11] count (u64 LE)
[12..19] sum (u64 LE)
```

**RowRecord (binary, variable):**
```
[0..7]  slot (u64 LE)
[8..9]  num_fields (u16 LE)
[10..]  fields — packed (name_len: u8, name: UTF-8, value: u32 LE)
```

### MUTATE (0x04)

Insert, update, or delete records.

**Request:**
```
[0]     0x04 (MUTATE)
[1]     flags
[2..3]  request_id (u16 LE)
[4]     mutation_type (u8): 0=INSERT, 1=UPDATE, 2=DELETE
[5..6]  record_count (u16 LE)
[7..]   records — packed 16-byte wire records (for INSERT)
        OR mutation ops (for UPDATE/DELETE)
```

**Mutation op (UPDATE, 8 bytes):**
```
[0..7]  slot (u64 LE) — target slot
[8..9]  col_offset (u16 LE)
[10..11] col_width (u16 LE)
[12..15] value (u32 LE) — new value (absolute) or delta
```

**Mutation op (DELETE, 8 bytes):**
```
[0..7]  slot (u64 LE) — target slot
```

**Response: MUTATE_ACK (0x05)**
```
[0]     0x05 (MUTATE_ACK)
[1]     flags
[2..3]  request_id (u16 LE)
[4..5]  affected (u16 LE) — number of records affected
[6..9]  seq (u32 LE) — mutation sequence number
```

### SUBSCRIBE (0x06)

Subscribe to a query. The server will push updates whenever the result changes.

**Request:**
```
[0]     0x06 (SUBSCRIBE)
[1]     flags
[2..3]  request_id (u16 LE) — becomes the subscription_id
[4..5]  query_len (u16 LE)
[6..]   query_json (UTF-8)
```

**Response: SUBSCRIPTION_DATA (0x07)**
```
[0]     0x07 (SUBSCRIPTION_DATA)
[1]     flags: bit 0 = is_initial (1) or is_update (0)
[2..3]  subscription_id (u16 LE) — echoed request_id
[4..11] fingerprint (u64 LE) — fingerprint of result
[12..]  result — same format as QUERY_RESULT payload (from byte 4 onward)
```

The server pushes SUBSCRIPTION_DATA whenever the query result's fingerprint changes. The first push has `is_initial=1`.

### UNSUBSCRIBE (0x08)

Cancel a subscription.

**Request:**
```
[0]     0x08 (UNSUBSCRIBE)
[1]     flags
[2..3]  subscription_id (u16 LE) — the request_id used in SUBSCRIBE
```

**Response: UNSUBSCRIBE_ACK (0x09)**
```
[0]     0x09 (UNSUBSCRIBE_ACK)
[1]     0x00
[2..3]  subscription_id (u16 LE)
```

### CALL_PIPELINE (0x0A)

Execute a named pipeline.

**Request:**
```
[0]     0x0A (CALL_PIPELINE)
[1]     flags
[2..3]  request_id (u16 LE)
[4]     slug_len (u8) — length of pipeline slug
[5..]   slug (UTF-8)
```

**Response: PIPELINE_RESULT (0x0B)**
```
[0]     0x0B (PIPELINE_RESULT)
[1]     flags: bit 0 = success (1) or failure (0)
[2..3]  request_id (u16 LE)
[4..7]  exec_us (u32 LE) — execution time in µs
[8..9]  steps_executed (u16 LE)
[10..]  detail_json (UTF-8, optional) — error message on failure
```

### ERROR (0x0C)

Server-side error response.

```
[0]     0x0C (ERROR)
[1]     error_code (u8)
[2..3]  request_id (u16 LE) — echoed from the failing request
[4]     msg_len (u8)
[5..]   message (UTF-8)
```

**Error codes:**
| Code | Name | Description |
|------|------|-------------|
| 1 | BAD_FRAME | Malformed frame |
| 2 | UNKNOWN_OP | Unknown opcode |
| 3 | QUERY_FAILED | Query execution error |
| 4 | MUTATE_FAILED | Mutation error |
| 5 | PIPELINE_FAILED | Pipeline execution error |
| 6 | NOT_FOUND | Pipeline/subscription not found |
| 7 | CAPACITY | Server at capacity |

### HEARTBEAT (0x10) / HEARTBEAT_ACK (0x11)

```
HEARTBEAT (S→C):
[0]     0x10
[1]     0x00
[2..3]  0x0000
[4..7]  epoch (u32 LE)

HEARTBEAT_ACK (C→S):
[0]     0x11
[1]     0x00
[2..3]  0x0000
[4..7]  epoch (u32 LE) — echoed
```

Server sends heartbeat every 30 seconds. Client must respond within the next epoch or be marked stale. After 3 consecutive misses, the connection is terminated.

---

## JSON Fallback Mode

If the first message from the client is a JSON text frame, the server enters JSON mode for that connection. All subsequent communication uses JSON text frames:

```json
// Query
{"op": "query", "id": 1, "query": {"space": {"tenant": 1}, "measure": {"type": "count"}}}

// Result
{"op": "query_result", "id": 1, "result": {"count": 42000, "query_us": 9}}

// Subscribe
{"op": "subscribe", "id": 2, "query": {"space": {"tenant": 1}, "measure": {"type": "sum"}, "of": "amount"}}

// Subscription data
{"op": "subscription_data", "id": 2, "initial": true, "fingerprint": "a1b2c3d4", "result": {...}}

// Mutate (insert)
{"op": "mutate", "id": 3, "type": "insert", "records": [{"tenant": 1, "entity": 0, "key_id": 42, "amount": 5000}]}

// Call pipeline
{"op": "call_pipeline", "id": 4, "slug": "bank-transfer"}

// Unsubscribe
{"op": "unsubscribe", "id": 2}

// Error
{"op": "error", "id": 1, "code": 3, "message": "query failed: unknown column 'foo'"}
```

JSON mode is slower than binary but useful for debugging, browser consoles, and rapid prototyping.

---

## Connection Lifecycle

```
1. Client connects (WSS upgrade)
2. Server allocates slot, sends WELCOME
3. Client sends QUERY / MUTATE / SUBSCRIBE / CALL_PIPELINE frames
4. Server responds with results, pushes subscription updates
5. Server sends HEARTBEAT every 30s
6. Client responds with HEARTBEAT_ACK
7. On close: server frees slot, cancels all subscriptions
```

### Reconnection

On reconnect, the client should:
1. Open new connection
2. Receive new WELCOME with fresh slot
3. Re-subscribe to all active subscriptions
4. Use the `seq` from WELCOME to detect missed mutations

The replay buffer holds the last N mutations (configurable, default 4096). If `client_last_seq > server.min_replay_seq`, the server can replay missed events.

---

## Wire Record Format (16 bytes)

Every record transmitted over WebSocket is exactly 16 bytes, little-endian:

```
Byte   Width   Field          Type
─────  ─────   ─────          ────
0–1    2       tenant         u16 LE
2      1       entity         u8
3      1       _pad           u8
4–5    2       key_id         u16 LE
6–7    2       amount         u16 LE
8      1       region         u8
9      1       currency       u8
10     1       category       u8
11     1       status         u8
12–15  4       _reserved      [u8; 4]
```

### Encoding Example (JavaScript)

```javascript
const buf = new ArrayBuffer(16);
const view = new DataView(buf);
view.setUint16(0, 1, true);      // tenant = 1
view.setUint8(2, 0);              // entity = 0 (analytics)
view.setUint8(3, 0);              // pad
view.setUint16(4, 42, true);     // key_id = 42
view.setUint16(6, 5000, true);   // amount = 5000
view.setUint8(8, 3);              // region = 3
view.setUint8(9, 1);              // currency = 1
view.setUint8(10, 5);             // category = 5
view.setUint8(11, 0);             // status = 0
// bytes 12-15 reserved (zero)
```

---

## Performance

| Metric | Value |
|---|---|
| Frame header overhead | 4 bytes |
| Query round-trip (binary) | ~15 µs (local), ~1 ms (network) |
| Subscription update push | ~5 µs |
| Max concurrent subscriptions per connection | 256 |
| Max concurrent connections per node | 65,536 |
| Memory per connection | ~10 bytes |

---

[← Previous: Real-Time Subscriptions](/realtime) · **Chapter 6** · [Next: Pipelines →](/pipelines)
