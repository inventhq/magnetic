# Magnetic — Internal Developer Docs

> For core developers and AI agents extending the Magnetic platform.
> For building apps WITH Magnetic, see the [public docs](../apps/docs/content/).

## Architecture Overview

Magnetic is a server-driven UI framework with four layers:

```
┌─────────────────────────────────────────────────────┐
│  Developer code                                      │
│  pages/*.tsx, components/*.tsx, server/state.ts       │
│  magnetic.json, design.json, content/*.md             │
├─────────────────────────────────────────────────────┤
│  CLI (@magneticjs/cli)                               │
│  Scans app → generates bridge → bundles with esbuild │
│  Prerenders (SSG/hybrid) → deploys via HTTP POST     │
├─────────────────────────────────────────────────────┤
│  JS Runtime (@magneticjs/server, @magneticjs/css)    │
│  JSX factory → DomNode tree                          │
│  Router, content pipeline, CSS generation             │
├─────────────────────────────────────────────────────┤
│  Platform Server (magnetic-v8-server, Rust)           │
│  V8 isolates, SSR, SSE, actions, static files         │
│  Multi-tenant, subdomain routing, data threads        │
└─────────────────────────────────────────────────────┘
```

## Chapter Guide

| # | Document | What it covers |
|---|----------|---------------|
| 1 | [Architecture](architecture.md) | Platform server internals — V8, request flow, data threads, deploy handler |
| 2 | [Packages](packages.md) | npm packages, native binary, publishing checklist |
| 3 | [Deployment & Ops](deployment-ops.md) | Civo setup, Caddy, systemd, build server, production fixes |
| 4 | [Roadmap](roadmap.md) | Future work — CRDTs, R2 content storage, and other items |

## Key Source Files

### Platform Server (Rust)
| File | What |
|------|------|
| `rs/crates/magnetic-v8-server/src/platform.rs` | Multi-tenant server: request routing, SSR, SSE, actions, deploy handler, static files |
| `rs/crates/magnetic-v8-server/src/data.rs` | Data threads: SSE, WebSocket, polling, ring buffer dedup |
| `rs/crates/magnetic-v8-server/src/main.rs` | Entry point, V8 initialization, CLI args |
| `rs/crates/magnetic-v8-server/assets/` | Embedded assets: magnetic.min.js, transport.wasm |

### CLI (TypeScript)
| File | What |
|------|------|
| `js/packages/magnetic-cli/src/cli.ts` | CLI entry: dev, build, push, login commands |
| `js/packages/magnetic-cli/src/prerender.ts` | SSG/hybrid pre-rendering: loads bundle in Node VM, renders routes to HTML |
| `js/packages/magnetic-cli/src/config.ts` | magnetic.json parser |
| `js/packages/magnetic-cli/src/scanner.ts` | App scanner: discovers pages, layouts, components |
| `js/packages/magnetic-cli/src/bridge.ts` | Bridge code generator: wires pages + state + CSS into V8 bundle |

### JS Runtime
| File | What |
|------|------|
| `js/packages/magnetic-server/src/jsx-runtime.ts` | JSX factory, Head, Link, Fragment |
| `js/packages/magnetic-server/src/router.ts` | File-based router: path matching, params |
| `js/packages/magnetic-server/src/content.ts` | Content pipeline: getContent(), listContent() |
| `js/packages/magnetic-css/src/utilities.ts` | Utility class generation from design tokens |
| `js/packages/magnetic-css/src/extract.ts` | CSS extraction: extractCSS(), generateAllCSS() |

### Public Docs (deployed at blbetmes.fujs.dev)
| File | What |
|------|------|
| `apps/docs/content/introduction.md` | Ch 0: What is Magnetic |
| `apps/docs/content/getting-started.md` | Ch 1: Install, scaffold, hello world |
| `apps/docs/content/app-development.md` | Ch 2: Pages, state, actions, data, config |
| `apps/docs/content/components.md` | Ch 3: Component patterns |
| `apps/docs/content/css-styling.md` | Ch 4: Design tokens, utility classes |
| `apps/docs/content/deployment.md` | Ch 5: SSR, SSG, hybrid pre-render |
| `apps/docs/content/benchmarks.md` | Ch 6: Performance data |

## For AI Agents

When **extending the core**:
- Platform server code is in `rs/crates/magnetic-v8-server/src/`
- CLI code is in `js/packages/magnetic-cli/src/`
- Read [Architecture](architecture.md) first to understand the request flow

When **building apps** with Magnetic:
- Read the public docs in `apps/docs/content/` (start with `introduction.md`)
- The public docs are the canonical reference for app development patterns
