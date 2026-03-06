---
title: Hybrid Pre-render
description: Pre-render content pages at deploy time while keeping interactive pages fully dynamic.
layout: docs
order: 7
---

# Hybrid Pre-render

For apps that mix static content and dynamic pages — like a news site with blog posts AND live dashboards — you can **pre-render specific routes** at deploy time while keeping the rest fully dynamic.

Pre-rendered pages are served instantly from disk. Everything else renders on demand with full interactivity (actions, SSE, real-time data).

## Configuration

Add a `prerender` array to `magnetic.json`:

```json
{
  "name": "my-app",
  "prerender": ["/", "/about", "/blog/*"]
}
```

When you run `magnetic push`, the CLI:
1. Builds the SSR bundle as normal
2. Pre-renders the listed routes to static HTML
3. Deploys both the bundle AND the pre-rendered pages

The platform checks for a pre-rendered file first. If one exists, it's served instantly. If not, the server renders on demand as usual.

## Glob Patterns

Use `/*` to pre-render all content under a prefix:

| Pattern | Expands to |
|---------|-----------|
| `"/"` | Just the homepage |
| `"/about"` | Single route |
| `"/blog/*"` | All content slugs under `blog/` |
| `"/docs/*"` | All content slugs under `docs/` |

Globs expand against your `content/` directory slugs and static page routes. Dynamic routes (`:param`) and catch-all routes are skipped.

## Example: News Site

```json
{
  "name": "my-news-site",
  "server": "https://api.fujs.dev",
  "prerender": ["/", "/about", "/articles/*", "/categories/*"],
  "data": {
    "live-feed": {
      "url": "https://api.example.com/live",
      "type": "sse",
      "page": "/live"
    }
  }
}
```

| Route | Behavior |
|-------|----------|
| `/`, `/about` | Served as static HTML (instant, no compute) |
| `/articles/*`, `/categories/*` | Pre-rendered from `content/` markdown (instant) |
| `/live` | Rendered on demand with real-time SSE updates (dynamic) |

## When to Use What

| Mode | Command | Use case |
|------|---------|----------|
| **Pure SSR** | `magnetic push` | All pages dynamic, no pre-rendering |
| **Hybrid** | `magnetic push` + `prerender` in config | Mix of static content + live interactive pages |
| **Pure SSG** | `magnetic push --static` | All pages static, zero JS, no server needed |

### Choose Hybrid when:
- Your site has hundreds or thousands of content pages (blog posts, docs, articles)
- Some pages need real-time data, user sessions, or interactivity
- You want content pages to load instantly without server compute
- You want to deploy everything as a single app

### Choose Pure SSG when:
- Every page is static (docs site, marketing site, blog)
- You don't need any server-side interactivity
- You want zero server cost

### Choose Pure SSR when:
- Every page needs dynamic rendering
- You have few pages and server compute is negligible
- All pages use real-time data or user-specific content

## How It Works Under the Hood

```
magnetic push (with prerender config)
  │
  ├── Build SSR bundle (app.js)
  ├── Pre-render listed routes to HTML
  └── Deploy bundle + pre-rendered pages
        │
        └── Platform receives request
              │
              ├── Pre-rendered file exists? → Serve from disk (instant)
              └── No pre-rendered file? → Render on demand (SSR)
```

Pre-rendered pages are full HTML with inlined CSS — identical to what the server would render dynamically. The only difference is they're generated once at deploy time instead of per-request.

## Content Updates

Pre-rendered pages are generated at **deploy time**. To update content:

1. Edit your `content/*.md` files
2. Run `magnetic push` again
3. The CLI re-renders all pre-render routes with the new content

This is the same workflow as updating any other part of your app. The pre-render step adds only milliseconds to the deploy process.

## Navigation

Pre-rendered pages use the same navigation as SSR pages. The `<Link>` component works in both contexts:

- On **pre-rendered pages**: renders as `<a href="/path">` — standard browser navigation
- On **dynamic pages**: renders as `<a onClick="navigate:/path">` — client-side navigation via the runtime

Users navigating between pre-rendered and dynamic pages experience a full page load (standard browser navigation). This is seamless and expected — the pre-rendered HTML loads fast.
