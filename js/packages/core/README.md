# @magnetic/core

Public surface (frozen in P1):

- `Envelope` — canonical message shape
- `ReducerName` — `"replace" | "append" | "merge" | "patch" | "downsample"`
- `reduce(name, prev, next)` — apply reducer
- `register(name, fn)` — override allowed reducer

See repo root for Directionality and Phase-1 notes.
