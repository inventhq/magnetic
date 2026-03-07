---
title: Deployment
description: Deploy Magnetic apps — SSR, static site generation, and hybrid pre-render.
layout: docs
order: 5
---

# Deployment

Magnetic supports three deployment modes. Choose based on your app's needs:

| Mode | Command | Use case | Server required |
|------|---------|----------|-----------------|
| **SSR** | `magnetic push` | Dynamic apps with real-time data, user sessions | Yes |
| **Hybrid** | `magnetic push` + `prerender` in config | Mix of static content + live interactive pages | Yes |
| **SSG** | `magnetic push --static` | Docs, blogs, marketing pages — zero JS | No |

## SSR Deployment

The default. Your app is rendered on demand for every request with full interactivity — actions, SSE, real-time data, user sessions.

```bash
# Build and deploy in one command
magnetic push --name my-app --server https://api.fujs.dev

# Or specify a directory
magnetic push --dir apps/my-app --name my-app --server https://api.fujs.dev
```

Your app gets a subdomain: `https://my-app.fujs.dev`

What gets deployed: your pages, components, state, config, and static assets. One command, no Docker, no CI pipeline. Deploys hot-reload instantly.

## Static Site Generation (SSG)

Pre-renders your pages to **pure static HTML** at build time. No JavaScript, no SSE, no server required. Deploy to the Magnetic platform or any static host.

```bash
# Build and deploy static site
magnetic push --static --name my-app --server https://api.fujs.dev

# Or build only (outputs to dist/)
magnetic build --static
```

SSG is the right choice when your content doesn't change per-request. The same `content/*.md` files and page components work in both modes — the only difference is the output.

### How SSG Works

1. **Build**: The CLI bundles your app (pages, state, JSX runtime, CSS framework) into `app.js`
2. **Render**: Each route is rendered through the same server pipeline used for SSR
3. **Extract**: `<Head>` components become `<head>` elements, design tokens become inlined `<style>`
4. **Write**: Each page is written as `{route}/index.html`

Each route produces a self-contained HTML file with inlined CSS. Navigation uses plain `<a href>` links — standard browser navigation, no JavaScript needed.

### Content Pipeline

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

### Lazy Content Mode

For sites with hundreds or thousands of pages, baking all content into the JS bundle is wasteful. **Lazy content mode** keeps only the metadata index in the bundle and loads each `.md` file on demand during SSG:

```bash
# Default: all content baked into bundle (~161KB for 5 pages)
magnetic build --static

# Lazy: metadata index only, .md loaded on demand (~30KB bundle)
magnetic build --static --lazy-content
```

#### Performance at Scale

| Pages | Bundle mode | Lazy mode |
|-------|------------|-----------|
| 5 | 161KB bundle, 8ms render | 30KB bundle, 8ms render |
| 100 | ~500KB bundle | 30KB bundle |
| 1,000 | ~5MB bundle | 30KB bundle |
| 10,000 | impractical | **30KB bundle, ~15s total** |

Benchmark (M1 MacBook Air): **10,006 pages in ~15 seconds** (1.5ms per page). The bundle stays at 30KB regardless of content count.

The key advantage: **adding a new `.md` file requires zero rebuild** in lazy mode. The bundle never changes — only the SSG output step runs.

### Deploy SSG to External Hosts

The `dist/` directory is deployable to any static host:

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

## Hybrid Pre-render

For apps that mix static content and dynamic pages — like a news site with blog posts AND live dashboards — you can **pre-render specific routes** at deploy time while keeping the rest fully dynamic.

Pre-rendered pages are served instantly from disk. Everything else renders on demand with full interactivity.

### Configuration

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

### Glob Patterns

Use `/*` to pre-render all content under a prefix:

| Pattern | Expands to |
|---------|-----------|
| `"/"` | Just the homepage |
| `"/about"` | Single route |
| `"/blog/*"` | All content slugs under `blog/` |
| `"/docs/*"` | All content slugs under `docs/` |

Globs expand against your `content/` directory slugs and static page routes. Dynamic routes (`:param`) and catch-all routes are skipped.

### Example: News Site

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

### Content Updates

Pre-rendered pages are generated at **deploy time**. To update content:

1. Edit your `content/*.md` files
2. Run `magnetic push` again
3. The CLI re-renders all pre-render routes with the new content

The pre-render step adds only milliseconds to the deploy process.

## Navigation Across Modes

| Mode | Navigation behavior |
|------|-------------------|
| **SSR** | `<Link>` uses client-side navigation — no full page reload |
| **SSG** | Plain `<a href>` links — standard browser navigation, zero JS |
| **Hybrid** | Pre-rendered pages use `<a href>`, dynamic pages use client-side navigation |

Your page components work identically in all modes. The `href` attribute on `<Link>` and `<a>` is what matters.

## Choosing a Mode

### Choose SSR when:
- Every page needs dynamic rendering
- You have few pages and server compute is negligible
- All pages use real-time data or user-specific content

### Choose Hybrid when:
- Your site has hundreds or thousands of content pages (blog posts, docs, articles)
- Some pages need real-time data, user sessions, or interactivity
- You want content pages to load instantly without server compute
- You want to deploy everything as a single app

### Choose SSG when:
- Every page is static (docs site, marketing site, blog)
- You don't need any server-side interactivity
- You want zero server cost

The `content/` folder is the single source of truth in all modes — same markdown files, same `getContent()`/`listContent()` API, same page components. Only the serving behavior differs.

---

← [Previous: CSS & Styling](/css-styling) · **Chapter 5** · [Next: Benchmarks →](/benchmarks)
