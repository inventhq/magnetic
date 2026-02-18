# @magnetic/sdk-web

Public surface (P1):

- `Source`, `SSESource`, `WSSource`
- `createClient(source)` — returns `{ subscribe(topic, { since?, onDelta?, reducer? }) }`
- `derived(deps[], compute, onChange)` — batched recompute
- `status()` — `{ connected, lagMs?, retries }`
- `action(nameOrUrl, payload, { idempotencyKey?, headers? })` — POST echo/ack

P1 behavior: minimal SSE/WS lifecycles (no retries), gap detection only (flag internally), derived batching (~1 frame), actions via `/actions/*` echo.
