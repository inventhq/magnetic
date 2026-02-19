# Magnetic

Server-driven UI framework. Write TSX pages and business logic — Magnetic handles everything else.

## What is Magnetic?

Magnetic is a **server-driven UI framework** where all state and rendering logic runs on the server (Rust + V8). The browser receives pre-rendered JSON DOM descriptors and a ~1.5KB client runtime that patches the DOM. No React, no virtual DOM, no client-side state management.

```
You write:                    Magnetic does:
  pages/*.tsx          →      Maps filename to route, renders in V8
  components/*.tsx     →      Imported by pages, bundled together
  server/state.ts      →      Called on every action, drives re-renders
  public/style.css     →      Inlined into SSR HTML (no extra request)
```

## Quick Start

```bash
# 1. Scaffold a new app
npx create-magnetic-app my-app
cd my-app

# 2. Install the CLI
npm install -g @magneticjs/cli

# 3. Start developing
magnetic dev
# → http://localhost:3003
```

## How It Works

1. **You write** TSX page components + business logic in `server/state.ts`
2. **The CLI** scans `pages/`, auto-generates a V8 bridge, and bundles with esbuild
3. **The Rust server** executes your bundle in V8, renders JSON DOM descriptors
4. **SSR** delivers the initial HTML — no loading spinners
5. **SSE** streams real-time updates to all connected clients
6. **Actions** (`onClick`, `onSubmit`, `onInput`) POST to the server, which re-renders and pushes the new DOM

The client runtime (~1.5KB gzipped) is a thin rendering shell. It patches the DOM using keyed reconciliation — no framework, no virtual DOM.

## Architecture

```
Browser                          Server (Rust + V8)
┌─────────────────┐              ┌──────────────────────┐
│  magnetic.js     │◄── SSE ────│  magnetic-v8-server    │
│  (~1.5KB)        │             │                        │
│  DOM patching    │── POST ───►│  V8: your TSX + state  │
│  Event delegation│             │  SSR + action dispatch │
│  Client routing  │             │  Asset serving         │
└─────────────────┘              └──────────────────────┘
```

## Project Structure

```
my-app/
├── pages/                 ← TSX page components (filename = route)
│   ├── layout.tsx           Wraps all pages (nav, footer, <Head>)
│   ├── IndexPage.tsx        → /
│   ├── AboutPage.tsx        → /about
│   ├── [id].tsx             → /:id (dynamic)
│   ├── NotFoundPage.tsx     → * (catch-all)
│   └── dashboard/
│       ├── layout.tsx       Nested layout for /dashboard/*
│       └── IndexPage.tsx    → /dashboard
├── components/            ← Shared TSX components
├── server/
│   └── state.ts           ← Business logic (state, reducer, view model)
├── public/
│   └── style.css            App styles (inlined into SSR HTML)
├── magnetic.json          ← App config
└── tsconfig.json          ← IDE support
```

## Layouts

Place a `layout.tsx` file in any `pages/` directory to wrap all pages at that level:

```tsx
// pages/layout.tsx — Root layout (wraps every page)
import { Head } from '@magneticjs/server';

export default function RootLayout({ children, path }: { children: any; path: string }) {
  return (
    <div class="app-shell">
      <Head>
        <title>My App</title>
        <meta name="description" content="Built with Magnetic" />
      </Head>
      <nav>
        <a href="/" class={path === '/' ? 'active' : ''}>Home</a>
        <a href="/about" class={path === '/about' ? 'active' : ''}>About</a>
      </nav>
      <main>{children}</main>
      <footer>Powered by Magnetic</footer>
    </div>
  );
}
```

Nested layouts work automatically:

```
pages/
├── layout.tsx              ← wraps ALL pages
├── IndexPage.tsx
├── AboutPage.tsx
└── dashboard/
    ├── layout.tsx          ← wraps /dashboard/* pages (nested inside root layout)
    └── IndexPage.tsx
```

Layout props:
- **`children`** — the rendered page (or inner layout)
- **`path`** — current URL path (for active nav highlighting)
- **`params`** — extracted URL parameters

## Head & Meta (SEO)

Use the `<Head>` component anywhere in a page or layout to inject `<head>` elements:

```tsx
import { Head } from '@magneticjs/server';

export function AboutPage() {
  return (
    <div>
      <Head>
        <title>About Us</title>
        <meta name="description" content="Learn more about us" />
        <meta property="og:title" content="About Us" />
        <link rel="canonical" href="https://example.com/about" />
      </Head>
      <h1>About</h1>
    </div>
  );
}
```

During SSR, `<Head>` children are extracted and placed into the HTML `<head>`. Page-level `<Head>` overrides layout-level `<Head>` for `<title>`.

## Writing Pages

Pages are TSX files in `pages/`. The exported function receives the view model as props:

```tsx
import { Head, Link } from '@magneticjs/server';

export function IndexPage(props: any) {
  return (
    <div key="app">
      <Head><title>{props.title}</title></Head>
      <h1 key="heading">{props.greeting}</h1>
      <button onClick="increment" key="btn">Count: {props.count}</button>
      <Link href="/about">About</Link>
    </div>
  );
}
```

## Writing Components

Components are regular functions in `components/`:

```tsx
export function TodoItem(props: { todo: any }) {
  return (
    <div class={`todo ${props.todo.doneClass}`} key={`todo-${props.todo.id}`}>
      <button onClick={`toggle_${props.todo.id}`}>{props.todo.icon}</button>
      <span>{props.todo.title}</span>
      <button onClick={`delete_${props.todo.id}`}>×</button>
    </div>
  );
}
```

## Business Logic

All state lives in `server/state.ts` — three exports:

```ts
// 1. Initial state
export function initialState() {
  return { count: 0 };
}

// 2. Reducer — pure function, handles actions
export function reduce(state, action, payload) {
  switch (action) {
    case 'increment': return { ...state, count: state.count + 1 };
    case 'add_item': return { ...state, items: [...state.items, payload.name] };
    default: return state;
  }
}

// 3. View model — shapes data for the UI
export function toViewModel(state) {
  return { ...state, greeting: `Count is ${state.count}` };
}
```

## Events

Events are **action names** (strings), not JavaScript callbacks:

| Prop | Behavior | Payload |
|------|----------|---------|
| `onClick="action"` | Click → POST to server | `{}` |
| `onSubmit="action"` | Form submit → collect inputs | `{ name: value, ... }` |
| `onInput="action"` | Keystroke (300ms debounce) | `{ value: "text" }` |

Parameterized actions encode IDs in the name: `onClick={`delete_${id}`}`

## CLI Commands

| Command | Description |
|---------|-------------|
| `magnetic dev` | Watch + rebuild + local server on port 3003 |
| `magnetic build` | Generate production bundle |
| `magnetic push` | Build + deploy to Magnetic platform |

## Packages

| Package | npm | Description |
|---------|-----|-------------|
| [`@magneticjs/server`](https://www.npmjs.com/package/@magneticjs/server) | `npm i @magneticjs/server` | JSX runtime, router, SSR |
| [`@magneticjs/cli`](https://www.npmjs.com/package/@magneticjs/cli) | `npm i -g @magneticjs/cli` | Build, dev, deploy CLI |
| [`create-magnetic-app`](https://www.npmjs.com/package/create-magnetic-app) | `npx create-magnetic-app` | Project scaffolder |

## Server Binary

The Rust V8 server is automatically downloaded when you install `@magneticjs/cli`. Supported platforms:

- macOS Apple Silicon (aarch64)
- macOS Intel (x86_64)
- Linux x64
- Linux ARM64

Build from source: `cd rs/crates/magnetic-v8-server && cargo build --release`

## Repository Structure

```
magnetic/
├── js/packages/
│   ├── magnetic-server/       @magneticjs/server — JSX runtime + router
│   ├── magnetic-cli/          @magneticjs/cli — build tool + dev server
│   ├── create-magnetic-app/   Scaffolder
│   └── sdk-web-runtime/       Client runtime (magnetic.js, ~1.5KB)
├── rs/crates/
│   └── magnetic-v8-server/    Rust V8 server (HTTP + SSE + V8)
├── apps/                      Example apps
└── docs/                      Internal docs
```

## License

MIT
