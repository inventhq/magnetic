# Magnetic Monorepo

Phase-1 (Core & Transports) completed via C1–C10.

- Workspaces via `pnpm@8.15.4`
- Hard-mode TypeScript (`tsconfig.base.json`)
- ESLint v9 flat config; UI firewall for `@magnetic/sdk-web`
- CI: build, lint, typecheck, start simulators, run tests
- Goldens under `tests/golden/`

## Directionality (dev rails)

- **Never** tag components as *incoming/outgoing*.
- **Inbound** = `useStream` / `client.subscribe` (consume topics).
- **Outbound** = `useAction` (send verbs, expect ack).
- **Connectedness** = implicit (shared topics + derived state), not direct wiring.
- **Visibility (dev-only, optional)** = `{topic, reducer, since, last_seq, gap}` via `client/status` — no transport labels.

See Foundation Concepts for architectural laws and kernel rules.
