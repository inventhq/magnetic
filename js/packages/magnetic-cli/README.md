# @magneticjs/cli

Build, develop, and deploy [Magnetic](https://github.com/inventhq/magnetic) server-driven UI apps.

## Installation

```bash
npm install -g @magneticjs/cli
```

This also downloads the `magnetic-v8-server` binary for your platform via postinstall.

## Commands

### `magnetic dev`

Start a local development server with hot rebuild on file changes.

```bash
cd my-app
magnetic dev                    # http://localhost:3003
magnetic dev --port 4000        # custom port
magnetic dev --dir ./my-app     # specify app directory
```

What happens:
1. Scans `pages/` for TSX page components
2. Detects `server/state.ts` for business logic
3. Auto-generates the V8 bridge (route map + state wiring)
4. Bundles with esbuild into a single IIFE
5. Starts `magnetic-v8-server` on the specified port
6. Watches for changes, rebuilds + restarts on save

### `magnetic build`

Generate the production bundle without starting a server.

```bash
magnetic build --dir ./my-app
magnetic build --dir ./my-app --minify --verbose
```

Output: `dist/app.js` (~15KB typical)

### `magnetic push`

Build and deploy to a Magnetic platform server.

```bash
magnetic push --dir ./my-app --server https://platform.magnetic.app --name my-app
```

Or configure in `magnetic.json`:
```json
{
  "name": "my-app",
  "server": "https://platform.magnetic.app"
}
```

Then just: `magnetic push`

## How It Works

The CLI is a **build tool**, not a runtime. It:
- Scans your `pages/` directory and maps filenames to routes
- Generates a bridge file that wires pages + state + router
- Bundles everything into a single JS file for the V8 engine
- The Rust V8 server executes this bundle to render JSON DOM descriptors

**You write**: pages, components, state
**CLI generates**: bridge, bundle
**Rust server runs**: V8 + HTTP + SSE

## File Conventions

| File | Route |
|------|-------|
| `pages/IndexPage.tsx` | `/` |
| `pages/AboutPage.tsx` | `/about` |
| `pages/SettingsPage.tsx` | `/settings` |
| `pages/[id].tsx` | `/:id` (dynamic) |
| `pages/NotFoundPage.tsx` | `*` (catch-all) |

## Server Binary

The Rust binary (`magnetic-v8-server`) is automatically downloaded during
`npm install`. If the download fails, you can build from source:

```bash
cd rs/crates/magnetic-v8-server
cargo build --release
```

Supported platforms: macOS (ARM64, x64), Linux (x64, ARM64)

## License

MIT
