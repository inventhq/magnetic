# Magnetic Developer Guide

> **For app developers (human and AI).** This guide covers everything you need to build, test, and deploy apps on the Magnetic framework. You do NOT need to understand the framework internals.

## What is Magnetic?

Magnetic is a **server-driven UI framework**. You write TSX pages and business logic — the server renders everything. The browser receives a pre-rendered HTML page and a 2.3 KB client runtime that patches the DOM. There is no React, no virtual DOM, no hydration, no client-side state management.

Your app runs on **web, iOS, and Android** from a single codebase. The same server powers all platforms — native SDKs render server-driven UI as SwiftUI/Jetpack Compose widgets.

## What You Touch vs. What You Don't

```
✅ YOU WRITE (app code):          ❌ DO NOT TOUCH (framework):
  pages/*.tsx                       rs/           (Rust server)
  components/*.tsx                  js/packages/  (framework packages)
  server/state.ts                   android/      (Android SDK)
  server/api/*.ts                   ios/          (iOS SDK)
  public/style.css                  deploy/       (infra)
  design.json                       scripts/      (build tooling)
  magnetic.json
```

The framework is **read-only**. App developers only work inside an app directory.

## Quick Start

```bash
# 1. Scaffold a new app
npx create-magnetic-app my-app
cd my-app

# 2. Install the CLI (if not already installed)
npm install -g @magneticjs/cli

# 3. Start developing
magnetic dev
# → http://localhost:3003

# 4. Deploy to production
magnetic push --name my-app --server https://api.fujs.dev
```

## App Structure

```
my-app/
├── pages/                 ← Page components (filename = route)
│   ├── IndexPage.tsx      → /
│   ├── AboutPage.tsx      → /about
│   ├── [id].tsx           → /:id (dynamic param)
│   ├── NotFoundPage.tsx   → * (404 catch-all)
│   ├── layout.tsx         → Root layout (wraps all pages)
│   └── dashboard/
│       ├── layout.tsx     → Nested layout (wraps /dashboard/*)
│       └── IndexPage.tsx  → /dashboard
├── components/            ← Shared TSX components
│   ├── Card.tsx
│   └── types.ts
├── server/
│   ├── state.ts           ← Business logic (state, reducer, viewModel)
│   └── api/               ← API routes (optional)
│       └── health.ts      → /api/health
├── public/
│   └── style.css          ← Custom CSS (inlined into SSR HTML)
├── design.json            ← Theme tokens → utility classes (optional)
├── magnetic.json          ← App config (name, data sources, auth)
└── tsconfig.json
```

## The Three Files That Matter

Every Magnetic app revolves around three concepts:

### 1. Pages (`pages/*.tsx`) — What the user sees

Pages are **pure functions** that receive the view model as props and return JSX. They run on the server in V8, never in the browser.

```tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div key="page">
      <Head><title>{props.title}</title></Head>
      <h1 key="heading">{props.greeting}</h1>
      <p key="count">{props.itemCount} items</p>
      <button onClick="increment" key="btn">Count: {props.count}</button>
      <Link href="/about" prefetch>About</Link>
    </div>
  );
}
```

### 2. State (`server/state.ts`) — Business logic

Three exports control your app's behavior:

```ts
// Initial state — called once when app starts
export function initialState() {
  return { count: 0, items: [] };
}

// Reducer — pure function, handles actions
export function reduce(state: any, action: string, payload: any) {
  switch (action) {
    case 'increment': return { ...state, count: state.count + 1 };
    case 'add_item': return { ...state, items: [...state.items, payload.title] };
    default: return state;
  }
}

// View model — shapes data for the UI
export function toViewModel(state: any) {
  return {
    ...state,
    greeting: `Hello! Count is ${state.count}`,
    itemCount: state.items.length,
  };
}
```

### 3. Config (`magnetic.json`) — Data sources, auth, deploy

```json
{
  "name": "my-app",
  "server": "http://localhost:3003",
  "data": {
    "posts": {
      "url": "https://api.example.com/posts",
      "page": "/"
    },
    "feed": {
      "url": "https://sse.example.com/events",
      "type": "sse",
      "buffer": 20,
      "page": "*"
    },
    "prices": {
      "url": "wss://ws.example.com/stream",
      "type": "ws",
      "buffer": 50
    }
  }
}
```

## Event System

Events are **action name strings**, not JavaScript callbacks:

| JSX Prop | Trigger | Payload |
|----------|---------|---------|
| `onClick="action_name"` | Click | `{}` |
| `onSubmit="action_name"` | Form submit | `{ inputName: value, ... }` |
| `onInput="action_name"` | Keystroke (300ms debounce) | `{ value: "text" }` |

### Action Flow

```
User clicks <button onClick="delete_42">
  → magnetic.js POSTs to /actions/delete_42
  → Server calls reduce(state, "delete_42", {})
  → New state → toViewModel() → page re-renders in V8
  → JSON DOM snapshot returned in POST response
  → magnetic.js patches the real DOM
  → SSE broadcasts update to all other connected clients
```

### Parameterized Actions

Encode IDs in the action name, parse with regex in the reducer:

```tsx
// In page/component:
<button onClick={`delete_${item.id}`}>Delete</button>

// In reducer:
const m = action.match(/^delete_(\d+)$/);
if (m) {
  const id = parseInt(m[1], 10);
  return { ...state, items: state.items.filter(i => i.id !== id) };
}
```

### Forms

```tsx
<form onSubmit="add_item" key="form">
  <input type="text" name="title" placeholder="Enter title..." autocomplete="off" />
  <button type="submit">Add</button>
</form>
```

Magnetic collects all `<input>` values by `name` attribute → `{ title: "user typed this" }`.

### Live Search (Debounced Input)

```tsx
<input type="text" name="q" placeholder="Search..." onInput="live_search" key="search" />
```

Payload after 300ms: `{ value: "current text" }`.

## Keys — Required for Performance

Every element whose content changes between renders needs a `key`:

```tsx
// ✅ Dynamic list items
{items.map(item => <div key={`item-${item.id}`}>{item.name}</div>)}

// ✅ Content that updates
<h1 key="title">{props.title}</h1>

// ✅ Conditionally rendered
{props.show && <div key="banner">Welcome!</div>}

// Not needed: static content that never changes
<footer>Built with Magnetic</footer>
```

Keys must be unique among siblings and stable across renders.

## Styling

### Option A: Utility Classes (recommended)

Create `design.json` with theme tokens:

```json
{
  "css": "pages",
  "theme": {
    "colors": {
      "primary": "#6366f1",
      "surface": "#0a0a0a",
      "raised": "#141414",
      "text": "#e4e4e7",
      "heading": "#ffffff",
      "muted": "#71717a"
    },
    "spacing": { "xs": "0.25rem", "sm": "0.5rem", "md": "1rem", "lg": "1.5rem", "xl": "2rem" },
    "radius": { "sm": "0.375rem", "md": "0.625rem", "lg": "1rem", "full": "9999px" }
  }
}
```

Then use utility classes:

```tsx
<div class="stack items-center p-xl min-h-screen">
  <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg">
    <h1 class="text-2xl bold fg-heading">Title</h1>
    <p class="text-sm fg-muted">Subtitle</p>
  </div>
</div>
```

Common utilities: `stack`, `row`, `gap-{size}`, `p-{size}`, `fg-{color}`, `bg-{color}`, `text-{size}`, `bold`, `round-{size}`, `shadow-{size}`, `w-full`, `items-center`, `justify-between`, `grow`, `hidden`. Responsive: `sm:`, `md:`, `lg:`, `xl:` prefixes.

### Option B: Custom CSS (`public/style.css`)

For hover/focus states and custom styles. Automatically inlined into SSR HTML:

```css
.card:hover { border-color: var(--m-border-hover); }
.btn:hover { opacity: 0.85; }
.input:focus { border-color: var(--m-primary); outline: none; }
```

## Data Sources

| Type | Config | Behavior |
|------|--------|----------|
| `fetch` (default) | `"url": "..."` | One-time fetch at SSR |
| `poll` | `"refresh": "5s"` | Re-fetch every N seconds, push via SSE |
| `sse` | `"type": "sse"` | Persistent SSE connection, real-time updates |
| `ws` | `"type": "ws"` | Persistent WebSocket connection, real-time updates |

Options:
- **`page`**: `"/"` (route-scoped) or `"*"` (global, all pages)
- **`buffer`**: Keep last N events as array (for SSE/WS). Default: 0 (replace mode)
- **`auth`**: `true` to forward session token
- **`timeout`**: SSR timeout (e.g., `"200ms"`) — renders with `__loading: true` if exceeded

## Authentication

```json
{
  "auth": {
    "provider": "oidc",
    "issuer": "https://accounts.google.com",
    "client_id": "${env.GOOGLE_CLIENT_ID}",
    "client_secret": "${env.GOOGLE_CLIENT_SECRET}",
    "scopes": ["openid", "email"],
    "redirect_uri": "/auth/callback"
  }
}
```

Providers: `oidc`, `oauth2`, `magic-link`, `otp`. Use `${env.VAR}` for secrets.

## API Routes

Files in `server/api/` become API endpoints:

```ts
// server/api/health.ts → /api/health
export function GET() {
  return { status: "ok", timestamp: Date.now() };
}

export function POST({ body }: { body: any }) {
  return { received: body };
}
```

## OpenAPI Integration

Auto-discover API specs and generate TypeScript types:

```bash
magnetic openapi
```

This probes your data source URLs for OpenAPI/Swagger specs, generates `server/api-types.ts`, and suggests data source configurations.

## Layouts

```tsx
// pages/layout.tsx — wraps ALL pages
export default function RootLayout({ children, path }: { children: any; path: string }) {
  return (
    <div class="stack min-h-screen" key="layout">
      <nav key="nav">
        <a href="/" class={path === '/' ? 'active' : ''}>Home</a>
        <a href="/about" class={path === '/about' ? 'active' : ''}>About</a>
      </nav>
      <main key="content">{children}</main>
    </div>
  );
}
```

Nested layouts: `pages/dashboard/layout.tsx` wraps only `/dashboard/*` routes.

## CLI Commands

| Command | Description |
|---------|-------------|
| `magnetic dev` | Watch + rebuild + local server on port 3003 |
| `magnetic build` | Generate production bundle |
| `magnetic push` | Build + deploy to Magnetic platform |
| `magnetic openapi` | Auto-discover APIs + generate TypeScript types |

## Deployment

```bash
# One command to deploy
magnetic push --name my-app --server https://api.fujs.dev
```

Your app gets a subdomain: `https://my-app.fujs.dev`

What gets deployed: your pages, components, state, config, and static assets.
What does NOT get deployed: the Rust server, magnetic.js runtime, native SDKs — these are part of the platform.

## Anti-Patterns — What NOT to Do

```tsx
// ❌ React hooks don't exist
const [count, setCount] = useState(0);
useEffect(() => { ... }, []);

// ❌ Client-side APIs don't exist in components
document.getElementById('x');
window.addEventListener('scroll', ...);
fetch('/api/data').then(...);

// ❌ Events are NOT callbacks
<button onClick={() => setCount(c + 1)}>
// ✅ Events are action name strings
<button onClick="increment">

// ❌ Don't mutate state
state.items.push(newItem);
// ✅ Return new objects
return { ...state, items: [...state.items, newItem] };

// ❌ Don't compute CSS in components
function Card({ done }) { const cls = done ? 'opacity-50' : ''; ... }
// ✅ Compute CSS classes in toViewModel()

// ❌ Don't import React
import React from 'react';
// ✅ Import from Magnetic
import { Head, Link } from '@magneticjs/server/jsx-runtime';
```

## Quick Reference

| Concept | Magnetic | React/Next.js |
|---------|----------|---------------|
| State | `server/state.ts` (server-only) | `useState`, Redux |
| Actions | `onClick="name"` (string) | `onClick={() => fn()}` (callback) |
| Forms | `onSubmit="name"` + `name` attrs | Controlled inputs + state |
| Navigation | `<Link href="/path">` | `<Link href="/path">` |
| Styling | `design.json` + utility classes | Tailwind, CSS modules |
| Data fetching | `magnetic.json` data sources | `useEffect`, `getServerSideProps` |
| Rendering | Server V8 → SSR HTML → SSE patches | Client React → VDOM → reconcile |
| Bundle size | 2.3 KB (fixed) | 80–150 KB+ (grows with app) |
| Hydration | None | Required |
| Native mobile | Built-in (same server) | Separate codebase |

## Further Reading

- [`docs/skills/magnetic-app-development.md`](docs/skills/magnetic-app-development.md) — Comprehensive skill reference
- [`docs/skills/magnetic-components.md`](docs/skills/magnetic-components.md) — Component patterns
- [`docs/skills/magnetic-css-styling.md`](docs/skills/magnetic-css-styling.md) — CSS and design.json deep dive
- [`docs/packages.md`](docs/packages.md) — Package distribution guide
- [`BENCHMARKS.md`](BENCHMARKS.md) — Performance measurements vs other frameworks
