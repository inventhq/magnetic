---
title: What is Magnetic
description: Server-driven UI framework — one codebase for web, iOS, and Android with zero client-side JavaScript.
layout: docs
order: 0
---

# What is Magnetic

Magnetic is a **server-driven UI framework**. You write TSX pages and business logic — the server renders everything. The browser receives pre-rendered HTML and a 2.3 KB client runtime that patches the DOM. There is no React, no virtual DOM, no hydration, no client-side JavaScript.

Your app runs on **web, iOS, and Android** from a single codebase. The same server powers all platforms — native SDKs render server-driven UI as SwiftUI/Jetpack Compose widgets.

## How It Works

```
You write:                    The server handles:
  pages/*.tsx           →     Routing, rendering, HTML generation
  components/*.tsx      →     Bundling, reusable UI
  server/state.ts       →     State management, real-time updates
  design.json           →     CSS generation, theme tokens
  magnetic.json         →     Data fetching, auth, deploy config
```

Every user interaction (click, form submit, navigation) is a single HTTP round-trip:

```
Browser                          Server
  │                                │
  │  click "Add Task"              │
  │  ───── POST /actions/add ────→ │
  │                                │  reduce(state, "add", payload)
  │                                │  re-render page
  │                                │
  │  ←── new DOM snapshot ──────── │
  │                                │
  │  patch DOM in-place            │
  │                                │
  │  ←── SSE: update other tabs ── │
```

No client-side state. No API layer. No hydration. The server is the single source of truth.

## Key Concepts

- **Pages** — TSX files in `pages/` that map to routes by filename. `IndexPage.tsx` → `/`, `AboutPage.tsx` → `/about`, `[id].tsx` → `/:id`
- **State** — A reducer in `server/state.ts`. All state lives on the server. `reduce(state, action, payload)` handles every user action.
- **Actions** — String-based event handlers. `onClick="delete_42"` sends a POST to the server, which runs the reducer and returns a new DOM snapshot.
- **Components** — Pure functions that take props and return DOM descriptors via TSX. No hooks, no state, no effects.
- **Data Sources** — External APIs configured in `magnetic.json`. The server fetches data and injects it as page props. Supports REST, polling, SSE, and WebSocket.
- **Design Tokens** — `design.json` defines colors, spacing, typography. The framework generates utility classes (like Tailwind, but server-side).
- **Content** — Markdown files in `content/` with frontmatter. Accessed via `getContent(slug)` and `listContent()` in your state.

## Who Is It For

- **Frontend developers** who want to build interactive apps without client-side complexity
- **Full-stack developers** who want one codebase for web + mobile
- **Teams** who want real-time collaborative UIs without WebSocket boilerplate
- **Content sites** that need fast static HTML with optional interactivity

## What You Ship

| Asset | Size |
|-------|------|
| HTML (complete page) | ~4 KB Brotli |
| Client runtime (total) | **2.3 KB** Brotli |
| Framework JavaScript | **0 KB** — there is none |

The client runtime is fixed at 2.3 KB regardless of app complexity. Your app can have 5 pages or 500 — the browser downloads the same tiny runtime.

## Chapter Guide

This documentation is organized as a progressive tutorial. Start from the top and work down:

| Chapter | What You'll Learn |
|---------|-------------------|
| [Getting Started](/getting-started) | Install the CLI, scaffold an app, run the dev server |
| [App Development](/app-development) | Pages, routing, state, actions, events, data sources, configuration |
| [Component Patterns](/components) | Building reusable components, built-in components, composition |
| [CSS & Styling](/css-styling) | Design tokens, utility classes, responsive design, custom CSS |
| [Deployment](/deployment) | SSR deploy, static site generation, hybrid pre-render |
| [Benchmarks](/benchmarks) | Performance data vs Next.js, Remix, HTMX, and others |

---

← Home · **Introduction** · [Next: Getting Started →](/getting-started)
