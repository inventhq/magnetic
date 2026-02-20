# Magnetic App Development — Claude Skill

You are an expert Magnetic framework developer. Magnetic is a server-driven UI framework where the server owns all state and rendering. The client is a ~2KB rendering shell with zero framework JS.

## Mental Model

```
Developer writes:           Magnetic handles:
  pages/*.tsx        →      Route mapping, V8 bridge, SSR
  components/*.tsx   →      Bundling, JSON DOM generation
  server/state.ts    →      Reducer dispatch, SSE broadcast
  design.json        →      CSS generation, theme tokens
  public/style.css   →      Inline into SSR HTML
  magnetic.json      →      Data fetching, auth, deploy config
```

**Core principle**: TSX runs on the server in V8, never in the browser. Components are pure functions that return JSON DOM descriptors. The client receives rendered DOM snapshots and patches the real DOM. There is no React, no virtual DOM, no hydration, no client-side state.

## Project Structure

```
my-app/
├── pages/                 ← Page components (auto-routed by filename)
│   ├── IndexPage.tsx      → /
│   ├── AboutPage.tsx      → /about
│   ├── SettingsPage.tsx   → /settings
│   ├── [id].tsx           → /:id (dynamic param)
│   └── NotFoundPage.tsx   → * (404 catch-all)
├── components/            ← Shared components (imported by pages)
│   ├── Header.tsx
│   ├── Card.tsx
│   └── types.ts           ← Shared TypeScript interfaces
├── server/
│   └── state.ts           ← Business logic (state, reducer, viewModel)
├── public/
│   └── style.css          ← Custom CSS (interactive states only if using design.json)
├── design.json            ← Theme tokens + CSS mode (optional)
├── magnetic.json          ← App config (name, server, data sources, auth)
└── package.json
```

## Pages

Pages are TSX files in `pages/`. The filename determines the route:

| Filename | Route | Convention |
|----------|-------|------------|
| `IndexPage.tsx` | `/` | "Index" prefix → root |
| `AboutPage.tsx` | `/about` | PascalCase minus "Page" suffix → lowercase |
| `SettingsPage.tsx` | `/settings` | Same pattern |
| `[id].tsx` | `/:id` | Brackets → dynamic param |
| `NotFoundPage.tsx` | `*` | "NotFound" → catch-all 404 |
| `UsersPage.tsx` | `/users` | Same pattern |

### Page Template

```tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function MyPage(props: { params: Record<string, string> } & MyViewModelType) {
  return (
    <div key="page">
      <Head>
        <title>Page Title</title>
        <meta name="description" content="Page description" />
      </Head>

      <nav key="nav">
        <Link href="/" prefetch>Home</Link>
        <Link href="/about" prefetch>About</Link>
      </nav>

      <h1 key="title">{props.someValue}</h1>
      <p key="count">{props.itemCount} items</p>
    </div>
  );
}
```

### Layouts

Add a `layout.tsx` file in any `pages/` directory to wrap all pages at that level:

```tsx
// pages/layout.tsx — wraps ALL pages
export function RootLayout(props: { children: any; params: Record<string, string>; path: string }) {
  return (
    <div class="stack min-h-screen" key="layout">
      <nav key="nav">
        <Link href="/" prefetch>Home</Link>
        <Link href="/about" prefetch>About</Link>
      </nav>
      <main key="content">{props.children}</main>
    </div>
  );
}
```

Nested layouts are supported: `pages/dashboard/layout.tsx` wraps only `/dashboard/*` routes.

### API Routes

Files in `server/api/` become API endpoints:

```
server/api/users.ts   → /api/users
server/api/health.ts  → /api/health
```

Export named functions for each HTTP method:

```ts
// server/api/users.ts
export function GET({ body, method, path }: { body: any; method: string; path: string }) {
  return { data: [{ id: 1, name: "Alice" }] };
}

export function POST({ body }: { body: any; method: string; path: string }) {
  return { created: true, id: 42 };
}
```

### Page Rules

1. **Export a named function** matching the filename (e.g., `AboutPage` for `AboutPage.tsx`)
2. **`props`** contains the view model from `toViewModel()` plus `params` for dynamic routes
3. **`key`** attributes are required on elements whose content changes between renders
4. **`<Head>`** declares `<title>` and `<meta>` tags for SSR (extracted into the document head)
5. **`<Link>`** does client-side navigation via pushState (no full page reload)
6. **`prefetch`** on `<Link>` triggers a GET on hover so the server pre-renders the target page
7. **Use `class` not `className`** — Magnetic's JSX runtime accepts both, but `class` is conventional
8. **No imports from React** — this is NOT React. Import only from `@magneticjs/server/jsx-runtime`
9. **`Fragment`** exists but wraps multiple children in a `<div>` — prefer explicit wrapper elements

## Components

Components are pure functions in `components/`. They receive props and return JSX.

### Basic Component

```tsx
// components/Card.tsx
export function Card(props: { title: string; children?: any }) {
  return (
    <div class="card" key={`card-${props.title}`}>
      <h3>{props.title}</h3>
      <div class="card-body">{props.children}</div>
    </div>
  );
}
```

### Component with Dynamic Classes

```tsx
// components/StatusBadge.tsx
export function StatusBadge(props: { status: string; statusClass: string }) {
  return (
    <span class={`badge ${props.statusClass}`} key="status">
      {props.status}
    </span>
  );
}
```

### List Component with Parameterized Actions

```tsx
// components/ItemList.tsx
export function ItemList(props: { items: Array<{ id: number; name: string; doneClass: string }> }) {
  return (
    <ul key="items">
      {props.items.map(item => (
        <li key={`item-${item.id}`} class={item.doneClass}>
          <span key={`name-${item.id}`}>{item.name}</span>
          <button onClick={`toggle_${item.id}`} key={`tog-${item.id}`}>Toggle</button>
          <button onClick={`delete_${item.id}`} key={`del-${item.id}`}>Delete</button>
        </li>
      ))}
    </ul>
  );
}
```

### Form Component

```tsx
// components/AddForm.tsx
export function AddForm() {
  return (
    <form onSubmit="add_item" key="add-form">
      <input type="text" name="title" placeholder="Enter title..." autocomplete="off" />
      <button type="submit">Add</button>
    </form>
  );
}
```

When submitted, Magnetic collects all `<input>` values by `name` attribute → `{ title: "user typed this" }`.

### Live Input (Debounced)

```tsx
<input type="text" name="q" placeholder="Search..." onInput="live_search" key="search" />
```

`onInput` is debounced (300ms). Payload: `{ value: "current input value" }`.

### Component Rules

1. **Pure functions only** — no hooks, no state, no effects, no DOM APIs
2. **Props drive everything** — all dynamic data comes from the view model via props
3. **Compute derived CSS classes in `toViewModel()`**, not in components
4. **Keys on dynamic content** — every `.map()` item needs a unique `key`
5. **Events are action name strings**, not JavaScript callbacks
6. **No `useState`, `useEffect`, `useRef`** — these don't exist in Magnetic
7. **Import from `@magneticjs/server/jsx-runtime`** only — `Head`, `Link`, and optionally `Fragment`

## Business Logic (server/state.ts)

All business logic lives in `server/state.ts`. This file exports exactly 3 functions:

### 1. `initialState()` — Starting State

```ts
export interface AppState {
  items: Item[];
  filter: 'all' | 'active' | 'done';
  nextId: number;
}

export function initialState(): AppState {
  return {
    items: [],
    filter: 'all',
    nextId: 1,
  };
}
```

### 2. `reduce(state, action, payload)` — Action Handler

A **pure function**. Given current state + action + payload, return new state. Never mutate.

```ts
export function reduce(state: AppState, action: string, payload: any): AppState {
  switch (action) {
    // Simple action (no payload needed)
    case 'reset':
      return initialState();

    // Form submission (payload has input values by name)
    case 'add_item': {
      const title = payload?.title?.trim();
      if (!title) return state;
      return {
        ...state,
        items: [...state.items, { id: state.nextId, title, done: false }],
        nextId: state.nextId + 1,
      };
    }

    // Live input (payload.value = current input text)
    case 'live_search':
      return { ...state, searchQuery: payload?.value || '' };

    // Filter actions
    case 'filter_all':
      return { ...state, filter: 'all' };
    case 'filter_active':
      return { ...state, filter: 'active' };
    case 'filter_done':
      return { ...state, filter: 'done' };

    // Parameterized actions (ID encoded in action name)
    default: {
      const toggleMatch = action.match(/^toggle_(\d+)$/);
      if (toggleMatch) {
        const id = parseInt(toggleMatch[1], 10);
        return {
          ...state,
          items: state.items.map(i =>
            i.id === id ? { ...i, done: !i.done } : i
          ),
        };
      }

      const deleteMatch = action.match(/^delete_(\d+)$/);
      if (deleteMatch) {
        const id = parseInt(deleteMatch[1], 10);
        return { ...state, items: state.items.filter(i => i.id !== id) };
      }

      return state; // Unknown action — return unchanged
    }
  }
}
```

### 3. `toViewModel(state)` — Prepare Data for UI

Transform raw state into the exact shape pages/components need. This is where you:
- Filter/sort lists
- Compute derived values (counts, summaries)
- Generate CSS class strings based on state
- Format display strings

```ts
export function toViewModel(state: AppState) {
  const visible = state.items
    .filter(i => {
      if (state.filter === 'active') return !i.done;
      if (state.filter === 'done') return i.done;
      return true;
    })
    .map(i => ({
      ...i,
      cardClass: i.done ? 'opacity-50' : '',
      titleClass: i.done ? 'line-through fg-muted' : '',
      checkClass: i.done ? 'check-done' : '',
      checkmark: i.done ? '✓' : '○',
    }));

  const active = state.items.filter(i => !i.done).length;
  const done = state.items.filter(i => i.done).length;

  return {
    ...state,
    visibleItems: visible,
    itemCount: `${active} active, ${done} done`,
    filterAllClass: state.filter === 'all' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    filterActiveClass: state.filter === 'active' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    filterDoneClass: state.filter === 'done' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    isEmpty: visible.length === 0,
    emptyMessage: state.filter === 'active' ? 'All done!' : 'No items yet.',
  };
}
```

### State Rules

1. **`reduce()` must be pure** — no fetch(), no timers, no side effects
2. **Never mutate state** — always return new objects with spread syntax
3. **CSS class computation belongs in `toViewModel()`**, not in components
4. **The view model IS the component API** — `toViewModel()` returns exactly what pages receive as `props`
5. **State is private** — the client never sees `AppState`, only the rendered DOM

## Event System

Events in Magnetic are **action name strings**, not JavaScript callbacks.

| JSX Prop | Trigger | Payload |
|----------|---------|----------|
| `onClick="action_name"` | Click | `{}` |
| `onSubmit="action_name"` | Form submit | `{ inputName: value, ... }` |
| `onInput="action_name"` | Keystroke (300ms debounce) | `{ value: "current text" }` |
| `onChange="action_name"` | Input change | `{}` |
| `onFocus="action_name"` | Element focused | `{}` |
| `onBlur="action_name"` | Element blurred | `{}` |
| `onKeyDown="action_name"` | Key pressed | `{}` |
| `onKeyUp="action_name"` | Key released | `{}` |
| `onScroll="action_name"` | Element scrolled | `{}` |

### Action Flow

```
User clicks <button onClick="delete_42">
  → magnetic.js POSTs to /actions/delete_42
  → Rust server calls reduce(state, "delete_42", {})
  → New state → toViewModel() → page re-renders
  → New JSON DOM sent back in POST response
  → magnetic.js patches DOM in-place
  → SSE broadcasts update to all other connected clients
```

### Parameterized Actions

Encode IDs in the action name:

```tsx
<button onClick={`toggle_${item.id}`}>Toggle</button>
<button onClick={`delete_${item.id}`}>Delete</button>
```

Parse in the reducer with regex:

```ts
const m = action.match(/^delete_(\d+)$/);
if (m) {
  const id = parseInt(m[1], 10);
  return { ...state, items: state.items.filter(i => i.id !== id) };
}
```

### Navigation

```tsx
import { Link } from '@magneticjs/server/jsx-runtime';

<Link href="/about" prefetch>About</Link>
<Link href={`/users/${user.id}`}>Profile</Link>
```

Under the hood, `<Link>` renders `<a onClick="navigate:/about">`. The client intercepts this, does pushState, and requests the new page from the server.

## Keys — Critical for Performance

Every element whose content or presence changes between renders needs a `key`.

```tsx
// REQUIRED: dynamic list items
{items.map(item => (
  <div key={`item-${item.id}`}>{item.name}</div>
))}

// REQUIRED: content that updates
<h1 key="title">{props.title}</h1>
<p key="count">{props.count} items</p>

// REQUIRED: conditionally rendered elements
{props.showBanner && <div key="banner">Welcome!</div>}

// NOT NEEDED: static content that never changes
<footer>Built with Magnetic</footer>
```

**Key rules:**
- Keys must be **unique among siblings** (not globally)
- Keys must be **stable** (don't use array index for reorderable lists)
- Shared keys across pages (like a nav) must be the same key string on both pages
- Missing keys cause the DOM differ to recreate elements instead of patching them

## Styling

Magnetic supports three styling approaches:

### 1. Utility Classes (with design.json)

Create `design.json` in the app root with theme tokens:

```json
{
  "css": "pages",
  "theme": {
    "colors": {
      "primary": "#6366f1",
      "primary-hover": "#4f46e5",
      "danger": "#ef4444",
      "surface": "#0a0a0a",
      "raised": "#141414",
      "sunken": "#0d0d0d",
      "text": "#e4e4e7",
      "heading": "#ffffff",
      "muted": "#71717a",
      "subtle": "#a1a1aa",
      "border": "#252525",
      "border-hover": "#333333"
    },
    "spacing": {
      "xs": "0.25rem",
      "sm": "0.5rem",
      "md": "1rem",
      "lg": "1.5rem",
      "xl": "2rem",
      "2xl": "3rem",
      "3xl": "4rem"
    },
    "radius": {
      "sm": "0.375rem",
      "md": "0.625rem",
      "lg": "1rem",
      "full": "9999px"
    },
    "typography": {
      "sans": "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      "mono": "JetBrains Mono, ui-monospace, monospace",
      "sizes": {
        "xs": "0.75rem",
        "sm": "0.85rem",
        "base": "0.9rem",
        "lg": "1.1rem",
        "xl": "1.25rem",
        "2xl": "1.5rem",
        "3xl": "1.75rem",
        "4xl": "2.25rem",
        "5xl": "3rem"
      },
      "leading": {
        "tight": "1.25",
        "normal": "1.5",
        "relaxed": "1.6"
      }
    },
    "shadows": {
      "sm": "0 1px 2px rgb(0 0 0 / 0.05)",
      "md": "0 4px 6px rgb(0 0 0 / 0.07), 0 2px 4px rgb(0 0 0 / 0.06)",
      "lg": "0 8px 32px rgba(0, 0, 0, 0.5)"
    },
    "breakpoints": {
      "sm": "640px",
      "md": "768px",
      "lg": "1024px",
      "xl": "1280px"
    }
  }
}
```

CSS modes: `"all"` (emit all utilities, safest), `"pages"` (extract from all routes, recommended), `"used"` (per-request, smallest).

Then use utility classes in your TSX:

```tsx
<div class="stack items-center p-xl min-h-screen">
  <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg">
    <h1 class="text-2xl bold fg-heading">Title</h1>
    <p class="text-sm fg-muted">Subtitle</p>
  </div>
</div>
```

### 2. Custom CSS (public/style.css)

For hover/focus/interactive states that utility classes can't express:

```css
/* Use var(--m-*) tokens from design.json */
.card:hover { border-color: var(--m-border-hover); }
.input:focus { border-color: var(--m-primary); outline: none; }
.btn:hover { opacity: 0.85; }
.delete:hover { color: var(--m-danger); }
.link:hover { text-decoration: underline; }
```

### 3. Pure CSS (no design.json)

Without `design.json`, write all styles in `public/style.css`. It's automatically inlined into the SSR HTML (no extra network request).

### Common Utility Classes

**Layout:** `stack` (flex column), `row` (flex row), `cluster` (flex wrap), `center` (flex center both), `grid-auto`, `grid-2`..`grid-6`, `container`

**Flex:** `items-center`, `items-start`, `items-end`, `items-stretch`, `items-baseline`, `justify-center`, `justify-between`, `justify-end`, `justify-around`, `justify-evenly`, `grow`, `grow-0`, `shrink-0`, `self-center`, `self-end`, `wrap`, `no-wrap`

**Spacing:** `gap-{size}`, `p-{size}`, `px-{size}`, `py-{size}`, `pt-{size}`, `pr-{size}`, `pb-{size}`, `pl-{size}`, `m-{size}`, `mx-{size}`, `my-{size}`, `mt-{size}`, `mr-{size}`, `mb-{size}`, `ml-{size}`, `mx-auto`

**Colors:** `fg-{color}`, `bg-{color}`, `border-{color}` for every color in design.json (e.g., `fg-primary`, `bg-raised`, `bg-sunken`, `border-hover`)

**Typography:** `text-{size}` (fluid clamp() for sizes ≥ lg), `font-sans`, `font-mono`, `thin`, `light`, `normal`, `medium`, `semibold`, `bold`, `extrabold`, `italic`, `not-italic`, `leading-{tight|normal|relaxed}`, `tracking-{tighter|tight|normal|wide|wider}`, `uppercase`, `lowercase`, `capitalize`, `normal-case`, `text-left`, `text-center`, `text-right`, `text-justify`, `underline`, `line-through`, `no-underline`, `truncate`, `break-words`

**Borders:** `border`, `border-t`, `border-r`, `border-b`, `border-l`, `border-none`, `round-{sm|md|lg|full}`, `round-none`

**Sizing:** `w-full`, `w-screen`, `w-auto`, `h-full`, `h-screen`, `h-auto`, `min-h-screen`, `min-h-full`, `min-w-0`, `max-w-{sm|md|lg|xl}`, `max-w-prose`, `max-w-none`

**Display:** `hidden`, `block`, `inline`, `inline-block`, `flex`, `inline-flex`, `grid`, `inline-grid`, `contents`

**Position:** `relative`, `absolute`, `fixed`, `sticky`, `static`, `inset-0`, `top-0`, `right-0`, `bottom-0`, `left-0`, `z-0`..`z-50`, `z-auto`

**Effects:** `shadow-{sm|md|lg|xl}`, `shadow-none`, `opacity-0`, `opacity-25`, `opacity-50`, `opacity-75`, `opacity-100`, `transition`, `transition-colors`, `transition-opacity`, `transition-shadow`, `transition-transform`, `transition-none`, `cursor-pointer`, `cursor-default`, `cursor-not-allowed`

**Overflow:** `overflow-hidden`, `overflow-auto`, `overflow-scroll`, `overflow-visible`, `overflow-x-auto`, `overflow-y-auto`

**Aspect ratio:** `aspect-auto`, `aspect-square`, `aspect-video`, `aspect-photo`, `aspect-wide`

**Accessibility:** `sr-only`, `not-sr-only`, `select-none`, `select-text`, `pointer-events-none`

**Responsive prefixes:** `sm:`, `md:`, `lg:`, `xl:` (e.g., `md:row`, `lg:text-xl`, `sm:hidden`)

## Configuration (magnetic.json)

### Minimal

```json
{
  "name": "my-app",
  "server": "http://localhost:3003"
}
```

### With Data Sources

```json
{
  "name": "my-app",
  "server": "http://localhost:3003",
  "data": {
    "posts": {
      "url": "https://jsonplaceholder.typicode.com/posts",
      "page": "/"
    },
    "users": {
      "url": "https://api.example.com/users",
      "page": "/users",
      "auth": true,
      "timeout": "200ms"
    },
    "feed": {
      "url": "https://api.example.com/feed",
      "page": "*",
      "refresh": "5s"
    }
  }
}
```

Data sources are fetched server-side and injected as props:
- **`page`**: `"/"` scopes to that route, `"*"` is global (available on all pages)
- **`auth`**: `true` injects the session token into the request
- **`timeout`**: SSR timeout — if fetch takes longer, renders with `__loading: true` in props so you can show a loading state
- **`refresh`**: Polling interval (e.g., `"5s"`, `"10s"`) — automatically re-fetches and pushes updates via SSE
- **Source types**: `fetch` (default one-time), `poll` (periodic via `refresh`), `sse` (server-sent events), `ws` (WebSocket)

### With Prerender (SSG)

```json
{
  "name": "my-app",
  "prerender": ["/", "/about"]
}
```

Routes listed in `prerender` are rendered to static HTML at build time. Good for pages that don't need dynamic state.

### With Actions (API forwarding)

```json
{
  "actions": {
    "create_post": "POST https://api.example.com/posts",
    "update_post": {
      "method": "PUT",
      "url": "https://api.example.com/posts/${payload.id}",
      "target": "posts"
    }
  }
}
```

### With Auth

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

Supported providers: `oidc`, `oauth2`, `magic-link`, `otp`. Use `${env.VAR}` for secrets.

## Deployment

```bash
# Build only (outputs dist/app.js)
magnetic build --dir apps/my-app

# Build + deploy to platform
magnetic push --dir apps/my-app --name my-app --server https://api.fujs.dev
```

One command. No Docker, no CI pipeline. The platform hot-swaps the V8 isolate.

**Note**: `magnetic push` only deploys app code (pages, components, state, assets). The Rust binary and magnetic.js client runtime are part of the platform — app developers never touch them.

## Complete Example: Task Board

### server/state.ts

```ts
export interface Task { id: number; title: string; completed: boolean; }
export interface AppState { tasks: Task[]; filter: 'all' | 'active' | 'done'; nextId: number; }

export function initialState(): AppState {
  return { tasks: [], filter: 'all', nextId: 1 };
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  switch (action) {
    case 'add_task': {
      const title = payload?.title?.trim();
      if (!title) return state;
      return { ...state, tasks: [...state.tasks, { id: state.nextId, title, completed: false }], nextId: state.nextId + 1 };
    }
    case 'filter_all': return { ...state, filter: 'all' };
    case 'filter_active': return { ...state, filter: 'active' };
    case 'filter_done': return { ...state, filter: 'done' };
    default: {
      const toggle = action.match(/^toggle_(\d+)$/);
      if (toggle) { const id = +toggle[1]; return { ...state, tasks: state.tasks.map(t => t.id === id ? { ...t, completed: !t.completed } : t) }; }
      const del = action.match(/^delete_(\d+)$/);
      if (del) { const id = +del[1]; return { ...state, tasks: state.tasks.filter(t => t.id !== id) }; }
      return state;
    }
  }
}

export function toViewModel(state: AppState) {
  const visible = state.tasks
    .filter(t => state.filter === 'active' ? !t.completed : state.filter === 'done' ? t.completed : true)
    .map(t => ({
      ...t,
      cardClass: t.completed ? 'opacity-50' : '',
      titleClass: t.completed ? 'line-through fg-muted' : '',
      checkmark: t.completed ? '✓' : '○',
    }));
  const active = state.tasks.filter(t => !t.completed).length;
  const done = state.tasks.filter(t => t.completed).length;
  return {
    ...state, visibleTasks: visible,
    taskCount: `${active} active, ${done} done`,
    filterAllClass: state.filter === 'all' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    filterActiveClass: state.filter === 'active' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    filterDoneClass: state.filter === 'done' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    isEmpty: visible.length === 0,
    emptyMessage: state.filter === 'active' ? 'All done!' : 'No tasks yet.',
  };
}
```

### pages/TasksPage.tsx

```tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';
import { TaskCard } from '../components/TaskCard.tsx';

export function TasksPage(props: any) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg board" key="board">
        <Head>
          <title>{`Tasks (${props.taskCount})`}</title>
        </Head>
        <nav class="row gap-md justify-center" key="nav">
          <Link href="/" class="nav-link fg-primary" prefetch>Tasks</Link>
          <Link href="/about" class="nav-link fg-muted" prefetch>About</Link>
        </nav>
        <h1 class="text-2xl bold fg-heading text-center" key="title">Tasks</h1>
        <form class="row gap-sm" onSubmit="add_task" key="add-form">
          <input type="text" name="title" placeholder="Add a task..." autocomplete="off"
            class="add-input grow bg-sunken border round-md px-md py-sm fg-text" />
          <button type="submit" class="add-btn bg-primary fg-heading round-md px-lg py-sm semibold cursor-pointer">Add</button>
        </form>
        <div class="row gap-xs justify-center" key="filters">
          <button onClick="filter_all" class={`filter-btn round-sm px-sm py-xs text-sm cursor-pointer ${props.filterAllClass}`} key="f-all">All</button>
          <button onClick="filter_active" class={`filter-btn round-sm px-sm py-xs text-sm cursor-pointer ${props.filterActiveClass}`} key="f-active">Active</button>
          <button onClick="filter_done" class={`filter-btn round-sm px-sm py-xs text-sm cursor-pointer ${props.filterDoneClass}`} key="f-done">Done</button>
        </div>
        <div class="stack gap-sm" key="task-list">
          {props.visibleTasks.map((task: any) => <TaskCard task={task} />)}
        </div>
        {props.isEmpty && <p class="text-center fg-muted text-sm italic py-xl" key="empty">{props.emptyMessage}</p>}
      </div>
    </div>
  );
}
```

### components/TaskCard.tsx

```tsx
export function TaskCard(props: { task: any }) {
  const { task } = props;
  return (
    <div key={`task-${task.id}`} class={`row items-center gap-sm bg-sunken border round-md px-md py-sm task-card ${task.cardClass}`}>
      <button onClick={`toggle_${task.id}`} class="check fg-muted text-sm cursor-pointer" key={`chk-${task.id}`}>
        {task.checkmark}
      </button>
      <span class={`grow text-base ${task.titleClass}`} key={`tt-${task.id}`}>{task.title}</span>
      <button onClick={`delete_${task.id}`} class="del fg-muted text-lg cursor-pointer" key={`del-${task.id}`}>×</button>
    </div>
  );
}
```

## Anti-Patterns — What NOT to Do

```tsx
// WRONG: React hooks don't exist
const [count, setCount] = useState(0);    // ❌
useEffect(() => { ... }, []);              // ❌

// WRONG: Client-side APIs don't exist
document.getElementById('x');              // ❌
window.addEventListener('scroll', ...);    // ❌
fetch('/api/data').then(...);              // ❌ (in components)

// WRONG: Event handlers are not callbacks
<button onClick={() => setCount(c + 1)}>  // ❌
<button onClick="increment">              // ✅

// WRONG: Mutating state in reducer
state.items.push(newItem);                 // ❌
return { ...state, items: [...state.items, newItem] }; // ✅

// WRONG: className (works but not conventional)
<div className="card">                     // ⚠️ works but use class
<div class="card">                         // ✅

// WRONG: Missing keys on dynamic content
{items.map(item => <div>{item.name}</div>)}           // ❌
{items.map(item => <div key={`i-${item.id}`}>{item.name}</div>)} // ✅

// WRONG: Computing CSS in components
function Card({ done }) {
  const cls = done ? 'opacity-50' : '';    // ❌ do this in toViewModel()
  return <div class={cls}>...</div>;
}

// WRONG: Importing React
import React from 'react';                 // ❌ not React
import { Head, Link } from '@magneticjs/server/jsx-runtime'; // ✅
```

## Quick Reference

| Concept | In Magnetic | In React/Next.js |
|---------|-------------|-------------------|
| State | `server/state.ts` (server-only) | `useState`, Redux, Zustand |
| Actions | `onClick="action_name"` string | `onClick={() => handler()}` callback |
| Forms | `onSubmit="action"` + `name` attrs | Controlled inputs + state |
| Navigation | `<Link href="/path">` | `<Link href="/path">` (similar) |
| Styling | Utility classes + `design.json` | Tailwind, CSS modules |
| Data fetching | `magnetic.json` data sources | `useEffect`, `getServerSideProps` |
| Rendering | Server V8 → JSON DOM → SSE push | Client React → virtual DOM → reconcile |
| Bundle size | ~2KB client + 0 framework JS | 80-150KB+ |
| Hydration | None (no client framework) | Required (framework bootstrap) |
