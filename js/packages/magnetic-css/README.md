# @magneticjs/css

Server-side CSS framework for Magnetic. Zero client-side JS for styling.

Generates CSS custom properties from `design.json` tokens, provides 305 semantic utility classes, and extracts only the CSS actually needed — all at build/render time on the server.

## Quick Start

1. Create `design.json` in your app directory (next to `magnetic.json`):

```json
{
  "css": "pages",
  "theme": {
    "colors": {
      "primary": "#3b82f6",
      "secondary": "#8b5cf6",
      "accent": "#f59e0b",
      "success": "#10b981",
      "warning": "#f59e0b",
      "error": "#ef4444",
      "surface": { "light": "#ffffff", "dark": "#1a1a2e" },
      "text": { "light": "#111827", "dark": "#f9fafb" },
      "muted": { "light": "#6b7280", "dark": "#9ca3af" },
      "border": { "light": "#e5e7eb", "dark": "#374151" }
    },
    "spacing": {
      "xs": "0.25rem", "sm": "0.5rem", "md": "1rem",
      "lg": "1.5rem", "xl": "2rem", "2xl": "3rem", "3xl": "4rem"
    },
    "radius": {
      "sm": "0.25rem", "md": "0.5rem", "lg": "1rem", "full": "9999px"
    },
    "typography": {
      "sans": "Inter, system-ui, -apple-system, sans-serif",
      "mono": "JetBrains Mono, ui-monospace, monospace",
      "sizes": {
        "xs": "0.75rem", "sm": "0.875rem", "base": "1rem",
        "lg": "1.125rem", "xl": "1.25rem", "2xl": "1.5rem",
        "3xl": "1.875rem", "4xl": "2.25rem", "5xl": "3rem"
      },
      "leading": { "tight": "1.25", "normal": "1.5", "relaxed": "1.75" }
    },
    "shadows": {
      "sm": "0 1px 2px rgb(0 0 0 / 0.05)",
      "md": "0 4px 6px rgb(0 0 0 / 0.07), 0 2px 4px rgb(0 0 0 / 0.06)",
      "lg": "0 10px 15px rgb(0 0 0 / 0.1), 0 4px 6px rgb(0 0 0 / 0.05)",
      "xl": "0 20px 25px rgb(0 0 0 / 0.1), 0 8px 10px rgb(0 0 0 / 0.04)"
    },
    "breakpoints": {
      "sm": "640px", "md": "768px", "lg": "1024px", "xl": "1280px"
    }
  }
}
```

2. Use utility classes in your TSX pages:

```tsx
export function TasksPage() {
  return (
    <main class="container stack gap-xl py-3xl">
      <h1 class="text-4xl bold fg-primary text-center">My Tasks</h1>
      <div class="grid-auto gap-lg">
        <div class="stack gap-sm p-lg bg-surface round-lg shadow-md">
          <h2 class="text-xl semibold">Task Card</h2>
          <p class="fg-muted leading-relaxed">Description here</p>
        </div>
      </div>
    </main>
  );
}
```

3. Build and deploy:

```bash
magnetic build --dir apps/my-app
magnetic push --dir apps/my-app --server https://your-server --name my-app
```

The CLI reads `design.json`, generates CSS, and the Rust server injects it as inline `<style>` in the SSR HTML response. No build step. No PostCSS. No client JS.

---

## CSS Generation Modes

Set the `"css"` field in `design.json`:

| Mode | `design.json` | Output Size | SSE Safe | Description |
|------|---------------|-------------|----------|-------------|
| **`"all"`** | `"css": "all"` | ~12.8KB | Yes | Emits every utility class. Safe default. |
| **`"pages"`** | `"css": "pages"` | ~3-5KB | Yes | Renders all routes at init, emits only used classes. **Recommended.** |
| **`"used"`** | `"css": "used"` | ~2-3KB | No* | Per-request extraction. Smallest, but SSE may miss classes. |

Default is `"all"` if omitted.

**SSE safety**: Magnetic uses Server-Sent Events to push DOM updates. When a user navigates, the server sends a JSON DOM snapshot. If that snapshot contains a CSS class that wasn't in the initial `<style>` block, it renders unstyled. `"all"` and `"pages"` prevent this.

> \* `"used"` is safe for single-page apps or apps where all pages use the same classes.

### When to use each mode

- **`"pages"`** — Best for most apps. Scans all routes at V8 init, so every class from any page is included. SSE-safe. 72% smaller than `"all"`.
- **`"all"`** — Use if your app generates classes dynamically (e.g., from database values) that aren't in the static page renders.
- **`"used"`** — Use for static site generation (SSG) or when you don't use SSE navigation.

---

## What Gets Generated

The inline `<style>` contains three layers:

1. **Theme variables** (~1.2KB) — CSS custom properties on `:root` and `[data-theme="dark"]`
2. **Reset/normalize** (~0.4KB) — Minimal reset using theme variables
3. **Utility classes** (varies by mode) — Only the classes your pages actually use

### Theme Variables

Every token in `design.json` becomes a CSS custom property:

```css
:root, [data-theme="light"] {
  --m-primary: #3b82f6;
  --m-space-md: 1rem;
  --m-radius-lg: 1rem;
  --m-font-sans: Inter, system-ui, -apple-system, sans-serif;
  --m-text-xl: 1.25rem;
  --m-shadow-md: 0 4px 6px rgb(0 0 0 / 0.07), 0 2px 4px rgb(0 0 0 / 0.06);
  /* ... */
}

[data-theme="dark"] {
  --m-surface: #1a1a2e;
  --m-text: #f9fafb;
  --m-muted: #9ca3af;
  --m-border: #374151;
}
```

Use these in your custom `style.css` for interactive states:

```css
.card:hover { border-color: var(--m-primary); }
.btn:focus  { outline: 2px solid var(--m-primary); }
```

---

## Utility Class Reference

### Layout

| Class | CSS | Description |
|-------|-----|-------------|
| `stack` | `display:flex;flex-direction:column` | Vertical flex container |
| `row` | `display:flex;flex-direction:row` | Horizontal flex container |
| `cluster` | `display:flex;flex-wrap:wrap;align-items:center` | Wrapping inline cluster |
| `center` | `display:flex;align-items:center;justify-content:center` | Center children both axes |
| `grid-auto` | `display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,var(--min-w,16rem)),1fr))` | Auto-responsive grid |
| `grid-2`..`grid-6` | `display:grid;grid-template-columns:repeat(N,1fr)` | Fixed N-column grid |
| `container` | `width:100%;margin-inline:auto;padding-inline:var(--m-space-md)` | Centered max-width container with responsive breakpoints |
| `wrap` | `flex-wrap:wrap` | Enable wrapping |

### Spacing

Generated from `theme.spacing` tokens (xs, sm, md, lg, xl, 2xl, 3xl):

| Pattern | Example | CSS |
|---------|---------|-----|
| `gap-{size}` | `gap-md` | `gap:var(--m-space-md)` |
| `p-{size}` | `p-lg` | `padding:var(--m-space-lg)` |
| `px-{size}` | `px-xl` | `padding-inline:var(--m-space-xl)` |
| `py-{size}` | `py-sm` | `padding-block:var(--m-space-sm)` |
| `pt-` `pr-` `pb-` `pl-` | `pt-md` | Individual sides |
| `m-{size}` | `m-md` | `margin:var(--m-space-md)` |
| `mx-{size}` `my-{size}` | `mx-auto` | `margin-inline:auto` |
| `mt-` `mr-` `mb-` `ml-` | `mt-xl` | Individual sides |

### Typography

| Class | CSS |
|-------|-----|
| `text-xs`..`text-5xl` | Font size (fluid `clamp()` for lg+) |
| `font-sans` `font-mono` | Font family from theme |
| `thin` `light` `normal` `medium` `semibold` `bold` `extrabold` | Font weight (100–800) |
| `italic` `not-italic` | Font style |
| `leading-tight` `leading-normal` `leading-relaxed` | Line height |
| `tracking-tighter`..`tracking-wider` | Letter spacing |
| `uppercase` `lowercase` `capitalize` `normal-case` | Text transform |
| `text-left` `text-center` `text-right` `text-justify` | Text alignment |
| `underline` `line-through` `no-underline` | Text decoration |
| `truncate` | Ellipsis overflow |
| `break-words` | Word break |

### Colors

Generated from `theme.colors` (primary, secondary, accent, success, warning, error, surface, text, muted, border):

| Pattern | Example | CSS |
|---------|---------|-----|
| `fg-{color}` | `fg-primary` | `color:var(--m-primary)` |
| `bg-{color}` | `bg-surface` | `background-color:var(--m-surface)` |
| `border-{color}` | `border-primary` | `border-color:var(--m-primary)` |

### Borders & Radius

| Class | CSS |
|-------|-----|
| `border` | `border:1px solid var(--m-border)` |
| `border-t` `border-r` `border-b` `border-l` | Single side |
| `border-none` | `border:none` |
| `round-sm` `round-md` `round-lg` `round-full` | Border radius from theme |
| `round-none` | `border-radius:0` |

### Shadows

| Class | CSS |
|-------|-----|
| `shadow-sm` `shadow-md` `shadow-lg` `shadow-xl` | Box shadow from theme |
| `shadow-none` | `box-shadow:none` |

### Sizing

| Class | CSS |
|-------|-----|
| `w-full` `w-screen` `w-auto` | Width |
| `h-full` `h-screen` `h-auto` | Height |
| `min-h-screen` `min-h-full` `min-w-0` | Min sizes |
| `max-w-sm`..`max-w-xl` | Max-width from breakpoints |
| `max-w-prose` | `max-width:65ch` |
| `max-w-none` | No max-width |

### Flexbox

| Class | CSS |
|-------|-----|
| `grow` `grow-0` | Flex grow |
| `shrink` `shrink-0` | Flex shrink |
| `items-start` `items-center` `items-end` `items-stretch` `items-baseline` | Align items |
| `justify-start` `justify-center` `justify-end` `justify-between` `justify-around` `justify-evenly` | Justify content |
| `self-auto` `self-start` `self-center` `self-end` `self-stretch` | Align self |

### Display & Position

| Class | CSS |
|-------|-----|
| `hidden` `block` `inline` `inline-block` `flex` `inline-flex` `grid` `inline-grid` `contents` | Display |
| `relative` `absolute` `fixed` `sticky` `static` | Position |
| `inset-0` `top-0` `right-0` `bottom-0` `left-0` | Inset |
| `z-0` `z-10` `z-20` `z-30` `z-40` `z-50` `z-auto` | Z-index |

### Overflow, Cursor, Opacity

| Class | CSS |
|-------|-----|
| `overflow-hidden` `overflow-auto` `overflow-scroll` `overflow-visible` | Overflow |
| `overflow-x-auto` `overflow-y-auto` `overflow-x-hidden` `overflow-y-hidden` | Axis overflow |
| `cursor-pointer` `cursor-default` `cursor-not-allowed` `cursor-wait` `cursor-text` `cursor-grab` | Cursor |
| `opacity-0` `opacity-25` `opacity-50` `opacity-75` `opacity-100` | Opacity |

### Interactions

| Class | CSS |
|-------|-----|
| `pointer-events-none` `pointer-events-auto` | Pointer events |
| `select-none` `select-text` `select-all` `select-auto` | User select |
| `transition` | `transition:all 150ms ease` |
| `transition-colors` `transition-opacity` `transition-shadow` `transition-transform` | Targeted transitions |
| `transition-none` | Disable transitions |

### Aspect Ratio

| Class | CSS |
|-------|-----|
| `aspect-auto` `aspect-square` `aspect-video` `aspect-photo` `aspect-wide` | Aspect ratio |

### Accessibility

| Class | CSS |
|-------|-----|
| `sr-only` | Visually hidden, screen reader accessible |
| `not-sr-only` | Undo `sr-only` |

---

## Responsive Prefixes

Every utility class can be prefixed with a breakpoint name for mobile-first responsive design:

| Prefix | Breakpoint | CSS |
|--------|------------|-----|
| `sm:` | ≥ 640px | `@media (min-width: 640px)` |
| `md:` | ≥ 768px | `@media (min-width: 768px)` |
| `lg:` | ≥ 1024px | `@media (min-width: 1024px)` |
| `xl:` | ≥ 1280px | `@media (min-width: 1280px)` |

**Examples:**
```tsx
<div class="stack md:row gap-md">        // Stack on mobile, row on md+
<div class="hidden md:block">            // Hidden on mobile, block on md+
<h1 class="text-2xl lg:text-4xl">       // Smaller on mobile, larger on lg+
<div class="grid-2 lg:grid-4 gap-md">   // 2 cols on mobile, 4 on lg+
```

---

## Dark Mode

Colors with `{ light, dark }` values automatically get dark mode support via `[data-theme="dark"]`.

To enable dark mode, add this attribute to your HTML:

```html
<html data-theme="dark">
```

Or toggle it dynamically based on user preference. A typical approach:

```html
<script>
  const t = localStorage.getItem('theme') ||
    (matchMedia('(prefers-color-scheme:dark)').matches ? 'dark' : 'light');
  document.documentElement.setAttribute('data-theme', t);
</script>
```

---

## Combining Utilities with Custom CSS

Utility classes handle layout and composition. Use `style.css` for interactive states (`:hover`, `:focus`) and component-specific styles:

```css
/* style.css — interactive states only */
.card:hover   { border-color: var(--m-primary); }
.btn:hover    { opacity: 0.85; }
.input:focus  { border-color: var(--m-primary); outline: none; }
.link:hover   { text-decoration: underline; }
```

Both sources are merged automatically: generated CSS + your `style.css` → single `<style>` block.

---

## How It Works (Architecture)

```
design.json
     │
     ▼
┌──────────────┐    ┌──────────────┐    ┌────────────────┐
│ CLI reads    │───▶│ Bridge code  │───▶│ esbuild bundle │
│ design.json  │    │ + CSS import │    │ (IIFE for V8)  │
└──────────────┘    └──────────────┘    └────────┬───────┘
                                                 │
                                                 ▼
                                        ┌────────────────┐
                                        │  V8 isolate    │
                                        │  renderWithCSS │
                                        │  → {root, css} │
                                        └────────┬───────┘
                                                 │
                                                 ▼
                                        ┌────────────────┐
                                        │  Rust server   │
                                        │  merge CSS +   │
                                        │  render HTML   │
                                        └────────┬───────┘
                                                 │
                                                 ▼
                                        ┌────────────────┐
                                        │  <style>       │
                                        │  theme vars    │
                                        │  + reset       │
                                        │  + utilities   │
                                        │  + style.css   │
                                        └────────────────┘
```

1. **CLI** reads `design.json` and passes it as a JSON string to the bridge generator
2. **Bridge** imports from `@magneticjs/css` and generates a `renderWithCSS(path)` export
3. **V8** executes the bundle. `renderWithCSS` returns `{ root: DomNode, css: string }`
4. **Rust server** parses the result, merges generated CSS with user's `style.css`, and injects it into the SSR HTML `<head>` as an inline `<style>` block
5. **SSE/action paths** are unaffected — `render()` and `reduce()` still return bare `DomNode`

---

## File Structure

```
js/packages/magnetic-css/
├── src/
│   ├── types.ts       — DesignConfig, DomNode, CSSMode interfaces
│   ├── defaults.ts    — Default theme tokens (used if design.json omits values)
│   ├── theme.ts       — compileTheme() → CSS custom properties
│   ├── reset.ts       — generateReset() → minimal normalize (~400 bytes)
│   ├── utilities.ts   — generateUtilities() → Map<className, declarations>
│   ├── extract.ts     — extractCSS(), generateAllCSS(), createExtractor()
│   └── index.ts       — Public exports
├── package.json
├── tsconfig.json
└── README.md
```

---

## Measured Performance (Production)

From the deployed task-board app at `zf9at7gr.fujs.dev`:

| Asset | Transfer (Brotli) | Content |
|-------|-------------------|---------|
| Document | 4.9 KB | Full SSR HTML + inline `<style>` + rendered DOM |
| magnetic.js | 2.1 KB | Client runtime |
| transport.wasm | 0.8 KB | WASM transport buffer |
| SSE | 0.1 KB | EventSource headers |
| **Total** | **7.9 KB** | **Entire interactive app** |

Time to first paint: ~95ms (paint on first response, no hydration).
