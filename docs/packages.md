# Magnetic — Package Distribution Guide

## Overview

Magnetic has **3 npm packages** and **1 native binary**. Developers install the
npm packages; the binary is fetched automatically.

```
Developer installs:
  npm i -g @magneticjs/cli          → CLI tool: magnetic dev/build/push
  npm i @magneticjs/server          → JSX runtime, router, SSR (dev dependency)
  npx create-magnetic-app my-app  → scaffolder (one-time)

Auto-installed:
  magnetic-v8-server              → Rust binary (downloaded via postinstall)
```

---

## Packages

### 1. `@magneticjs/server` (npm)

**What**: JSX runtime that transforms TSX → JSON DOM descriptors. Also includes
the router, SSR utilities, and middleware helpers.

**Who uses it**: Developers import from it in their page/component files:
```tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';
```

**Exports**:
| Path | Description |
|------|-------------|
| `@magneticjs/server` | Core index (re-exports) |
| `@magneticjs/server/jsx-runtime` | JSX factory, Head, Link, Fragment |
| `@magneticjs/server/router` | `createRouter()`, route matching |
| `@magneticjs/server/ssr` | `render_page()`, `PageOptions` |

**Publish**: `cd js/packages/magnetic-server && npm publish --access public`

---

### 2. `@magneticjs/cli` (npm)

**What**: CLI tool for building, developing, and deploying Magnetic apps.

**Commands**:
| Command | Description |
|---------|-------------|
| `magnetic dev` | Watch mode + auto-rebuild + Rust V8 server |
| `magnetic build` | Scan pages, generate bridge, bundle (esbuild) |
| `magnetic push` | Build + deploy to Magnetic platform |

**Key flags**:
- `--dir <path>` — app directory (default: current dir)
- `--port <n>` — dev server port (default: 3003)
- `--server <url>` — platform server URL (for push)
- `--name <name>` — app name (for push)
- `--verbose` — show generated bridge code
- `--minify` — minify output bundle

**Publish**: `cd js/packages/magnetic-cli && npm run build && npm publish --access public`

---

### 3. `create-magnetic-app` (npm)

**What**: Scaffolder that generates a new Magnetic project.

**Usage**:
```bash
npx create-magnetic-app my-app
npx create-magnetic-app my-app --template blank
```

**Templates**:
- `todo` (default) — full todo app with state, pages, components
- `blank` — minimal app with one page

**Generated structure**:
```
my-app/
  pages/           ← TSX page components (auto-routed by filename)
  components/      ← shared TSX components
  server/state.ts  ← business logic (reducer, state, view model)
  public/          ← static assets (CSS, images, magnetic.js)
  magnetic.json    ← app config (name, server URL, port)
  tsconfig.json    ← IDE support
  README.md
```

**Publish**: `cd js/packages/create-magnetic-app && npm publish --access public`

---

### 4. `magnetic-v8-server` (Rust binary)

**What**: The Rust server that runs the app. Embeds V8 to execute the bundled
TSX, serves SSR HTML, handles actions via POST, streams updates via SSE, and
serves static files. This is the **only server** — there is no separate backend.

**Cannot be published to npm** — it's a compiled native binary, not JavaScript.

**Distribution options**:

| Method | How it works |
|--------|-------------|
| **GitHub Releases** (current) | Prebuilt binaries per platform. `@magneticjs/cli` downloads it via postinstall script. |
| **Platform npm packages** (future) | Publish `@magneticjs/server-darwin-arm64`, `@magneticjs/server-darwin-x64`, `@magneticjs/server-linux-x64` — each containing the binary. This is how esbuild, turbo, and swc distribute. |
| **crates.io** | `cargo install magnetic-v8-server` for Rust developers |
| **Docker image** | For production/CI: `docker pull magnetic/v8-server` |

**Supported platforms**:
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-apple-darwin` (macOS Intel)
- `x86_64-unknown-linux-gnu` (Linux x64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)

**Build from source**:
```bash
cd rs/crates/magnetic-v8-server
cargo build --release
# Binary: target/release/magnetic-v8-server
```

---

## URL Architecture

There is **one server** — `magnetic-v8-server`. It serves everything.

### Local Development

```
magnetic dev --port 3003
→ http://localhost:3003         ← developer opens this
→ http://localhost:3003/sse     ← SSE stream (auto-connected by magnetic.js)
→ http://localhost:3003/actions/ ← action endpoints (auto-dispatched by magnetic.js)
```

The developer only interacts with `http://localhost:3003`. The CLI starts the
Rust server, watches for file changes, and auto-rebuilds.

### Production (Platform Mode)

```
magnetic-v8-server --platform --port 3003
→ https://my-app.magnetic.app          ← Caddy reverse proxy → localhost:3003
→ https://my-app.magnetic.app/sse
→ https://my-app.magnetic.app/actions/
```

Platform mode is multi-tenant: each app gets its own V8 isolate + SSE clients
+ state, routed by subdomain or path prefix (`/apps/:name/`).

### Deploy Flow

```bash
magnetic push --server https://platform.magnetic.app --name my-app
# → Builds bundle
# → POSTs to https://platform.magnetic.app/api/apps/my-app/deploy
# → App available at https://platform.magnetic.app/apps/my-app/
```

---

## Publishing Checklist

```bash
# 1. Bump versions
# 2. Build CLI
cd js/packages/magnetic-cli && npm run build

# 3. Build client runtime
cd js/packages/sdk-web-runtime && node build.cjs

# 4. Build Rust binary (for each target)
cd rs/crates/magnetic-v8-server
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
# Cross-compile Linux: use cross or CI
cross build --release --target x86_64-unknown-linux-gnu

# 5. Create GitHub release with binaries
# 6. Publish npm packages (order matters — server first)
cd js/packages/magnetic-server && npm publish --access public
cd js/packages/magnetic-cli && npm publish --access public
cd js/packages/create-magnetic-app && npm publish --access public
```

---

## For AI Agents

When building a Magnetic app:

1. **Run**: `npx create-magnetic-app <name>` to scaffold
2. **Edit only**: `pages/*.tsx`, `components/*.tsx`, `server/state.ts`
3. **Never edit**: `public/magnetic.js`, generated bridge, bundle
4. **Run**: `magnetic dev` to start local server on port 3003
5. **Test**: Open `http://localhost:3003` in browser
6. **Deploy**: `magnetic push --server <url> --name <name>`

**Conventions**:
- Page filename → route: `IndexPage.tsx` → `/`, `AboutPage.tsx` → `/about`, `[id].tsx` → `/:id`
- State actions: `onClick="action_name"` → POST `/actions/action_name`
- Form submissions: `onSubmit="action_name"` → collects FormData as payload
- Navigation: `<Link href="/path">` → client-side routing (no reload)
- `server/state.ts` exports: `initialState()`, `reduce(state, action, payload)`, `toViewModel(state)`
