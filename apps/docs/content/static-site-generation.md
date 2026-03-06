---
title: Static Site Generation
description: Pre-render content pages to static HTML at build time. Zero JavaScript, deployable anywhere.
layout: docs
order: 6
---

# Static Site Generation (SSG)

Magnetic can pre-render your pages to **pure static HTML** at build time. No JavaScript, no SSE, no server required. Deploy to the Magnetic platform with `magnetic push --static`, or to any static host — Netlify, Cloudflare Pages, GitHub Pages, S3, or a plain nginx server.

## When to Use SSG

| Mode | Best for | JavaScript shipped | Server required |
|------|----------|-------------------|-----------------|
| **SSR** (default) | Dynamic apps with real-time data, user sessions | 2KB client runtime | Yes (magnetic-v8-server) |
| **SSG** (`--static`) | Docs, blogs, marketing pages, content sites | **0KB — none** | No |

SSG is the right choice when your content doesn't change per-request. The same `content/*.md` files and page components work in both modes — the only difference is the output.

## Quick Start

```bash
# Build static HTML from your content pages
magnetic build --static

# Output goes to dist/
#   dist/index.html
#   dist/getting-started/index.html
#   dist/components/index.html
#   ...
```

Each route produces a self-contained HTML file with inlined CSS. Sidebar navigation uses plain `<a href>` links — standard browser navigation, no JavaScript needed.

## How It Works

1. **Build**: The CLI bundles your app (pages, state, JSX runtime, CSS framework) into `app.js`
2. **Render**: Each route is rendered through the same V8 pipeline used for SSR
3. **Extract**: `<Head>` components become `<head>` elements, design tokens become inlined `<style>`
4. **Write**: Each page is written as `{route}/index.html`

The same `toViewModel` → JSX → DomNode → HTML pipeline runs for both SSR and SSG. Your pages don't need any changes.

## Content Pipeline

SSG works with the `content/` folder — the same markdown files used in SSR mode:

```
my-app/
  content/
    getting-started.md
    components.md
    api-reference.md
  pages/
    IndexPage.tsx
    [slug].tsx          ← dynamic route for /:slug
  server/
    state.ts
  magnetic.json
  design.json
```

Each `.md` file with frontmatter becomes a route:

```markdown
---
title: Getting Started
description: Build your first Magnetic app
order: 1
---

# Getting Started

Your markdown content here...
```

The `[slug].tsx` page file creates a `/:slug` route, so visiting `/getting-started` renders that content.

## Lazy Content Mode

For sites with hundreds or thousands of pages, baking all content into the JS bundle is wasteful. **Lazy content mode** keeps only the metadata index in the bundle and loads each `.md` file on demand during SSG:

```bash
# Default: all content baked into bundle (~161KB for 5 pages)
magnetic build --static

# Lazy: metadata index only, .md loaded on demand (~30KB bundle)
magnetic build --static --lazy-content
```

### Performance at Scale

| Pages | Bundle mode | Lazy mode |
|-------|------------|-----------|
| 5 | 161KB bundle, 8ms render | 30KB bundle, 8ms render |
| 100 | ~500KB bundle | 30KB bundle |
| 1,000 | ~5MB bundle | 30KB bundle |
| 10,000 | impractical | **30KB bundle, ~15s total** |

Benchmark (M1 MacBook Air): **10,006 pages in ~15 seconds** (1.5ms per page). The bundle stays at 30KB regardless of content count because only the frontmatter metadata index is included.

### How Lazy Mode Works

1. **Build time**: Scan all `.md` files, extract frontmatter only (no HTML conversion)
2. **Inject**: Bundle contains `__magnetic_content_index` (lightweight metadata map)
3. **SSG**: For each route, the renderer calls `getContent(slug)` which loads ONE `.md` file from disk, converts to HTML, renders the page, writes the file, and moves on
4. **Memory**: Constant — only one page's content is in memory at a time

### Comparison with Other Frameworks

| Framework | 10K pages | Per page | Content rebuild |
|-----------|-----------|----------|-----------------|
| **Magnetic (lazy)** | **~15s** | 1.5ms | **0ms** (no rebuild needed) |
| Hugo | ~10s | ~1ms | Full rebuild |
| Gatsby | 2-30min | 12-180ms | Full rebuild |
| Next.js (SSG) | 1-10min | 6-60ms | Full rebuild |

The key advantage: **adding a new `.md` file requires zero rebuild** in lazy mode. The bundle never changes — only the SSG output step runs. Drop a file in `content/`, run `magnetic build --static --lazy-content`, done.

## Deploying Static Sites

### Deploy to Magnetic Platform

```bash
# Build SSG + deploy in one command
magnetic push --static
```

This pre-renders all routes, collects the HTML + CSS + public assets, and deploys to the Magnetic platform as a static site. The platform serves files directly — no V8 isolate, no SSE, just fast static file serving with proper content-type headers and caching.

### Deploy to External Hosts

The `dist/` directory is also deployable to any static host:

```bash
# Netlify
netlify deploy --dir=dist --prod

# Cloudflare Pages
npx wrangler pages deploy dist

# GitHub Pages
# Copy dist/ to your gh-pages branch

# Any static server
npx http-server dist -p 3000
```

## Navigation in SSG vs SSR

In **SSR mode**, the `<Link>` component renders `<a onClick="navigate:/path">` — the client runtime intercepts clicks, does `pushState`, and fetches the new page from the server without a full reload.

In **SSG mode**, there is no client runtime. Navigation links are plain `<a href="/path">` — standard browser navigation. Each click loads a new static HTML file. This is why SSG pages ship zero JavaScript.

Your page components work identically in both modes. The `href` attribute on `<Link>` and `<a>` is what matters for SSG.

## SSR vs SSG — Choose One Per Deployment

An app is deployed as **either SSR or SSG** — not both at the same time. They are different deployment targets for the same source code:

| Command | Output | Server | Interactivity |
|---------|--------|--------|---------------|
| `magnetic push` | V8 bundle | Required (magnetic-v8-server) | Full — actions, SSE, real-time data |
| `magnetic push --static` | Static HTML files | Not required | None — pure HTML, zero JS |

If you need *some pages* interactive and *some pages* static, **deploy as SSR**. The V8 server can render any page, and SSR is a superset of SSG. The only reason to choose SSG is when you want zero server cost for content-only sites like docs or blogs.

The `content/` folder is the single source of truth in both modes — same markdown files, same `getContent()`/`listContent()` API, same page components. Only the output differs.
