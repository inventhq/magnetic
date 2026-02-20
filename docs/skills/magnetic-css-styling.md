# Magnetic CSS Styling — Claude Skill

You are an expert at styling Magnetic apps using `@magneticjs/css`. This is a server-side utility CSS framework — zero client-side JS for styling. CSS is generated at build time from `design.json` tokens and injected as inline `<style>` in the SSR HTML.

## Mental Model

```
design.json          →  CSS custom properties + utility classes
class="stack gap-md" →  Server resolves to CSS at build/render time
public/style.css     →  :hover, :focus, animations (merged with generated CSS)
```

**You never install Tailwind, PostCSS, or any CSS build tool.** The framework handles everything.

---

## 1. Design Tokens (design.json)

Place `design.json` in the app root (same directory as `magnetic.json`).

### Full Schema

```json
{
  "css": "pages",
  "theme": {
    "colors": {
      "primary": "#6366f1",
      "danger": "#ef4444",
      "surface": "#0a0a0a",
      "raised": "#141414",
      "sunken": "#0d0d0d",
      "text": "#e4e4e7",
      "heading": "#ffffff",
      "muted": "#71717a",
      "border": "#252525"
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

### Token → CSS Custom Property Mapping

Every token becomes a CSS custom property on `:root`:

| Token path | CSS custom property | Example value |
|-----------|-------------------|---------------|
| `theme.colors.primary` | `--m-primary` | `#6366f1` |
| `theme.colors.surface` | `--m-surface` | `#0a0a0a` |
| `theme.spacing.md` | `--m-space-md` | `1rem` |
| `theme.spacing.xl` | `--m-space-xl` | `2rem` |
| `theme.radius.lg` | `--m-radius-lg` | `1rem` |
| `theme.typography.sans` | `--m-font-sans` | `-apple-system, ...` |
| `theme.typography.mono` | `--m-font-mono` | `JetBrains Mono, ...` |
| `theme.typography.sizes.xl` | `--m-text-xl` | `1.25rem` |
| `theme.typography.leading.relaxed` | `--m-leading-relaxed` | `1.6` |
| `theme.shadows.md` | `--m-shadow-md` | `0 4px 6px ...` |

**Rule**: The pattern is always `--m-{name}` for colors, `--m-space-{name}` for spacing, `--m-radius-{name}` for radius, `--m-font-{name}` for fonts, `--m-text-{name}` for font sizes, `--m-leading-{name}` for line heights, `--m-shadow-{name}` for shadows.

### Dark Mode

Colors can be flat strings or `{ light, dark }` objects:

```json
{
  "colors": {
    "primary": "#6366f1",
    "surface": { "light": "#ffffff", "dark": "#0a0a0a" },
    "text": { "light": "#111827", "dark": "#e4e4e7" }
  }
}
```

Flat colors are the same in both themes. Light/dark pairs automatically generate:
- `:root, [data-theme="light"]` → light values
- `[data-theme="dark"]` → dark values

No `dark:` prefix needed on utility classes. Switching `data-theme` on `<html>` toggles everything.

### CSS Modes

The `"css"` field controls how much CSS is generated:

| Mode | Value | Output | SSE Safe | Use when |
|------|-------|--------|----------|----------|
| **all** | `"css": "all"` | Every utility class (~13KB) | Yes | Dynamic class generation from DB values |
| **pages** | `"css": "pages"` | Union of all routes (~3-5KB) | Yes | **Default choice for most apps** |
| **used** | `"css": "used"` | Current page only (~2-3KB) | No | Static sites (SSG), no SSE navigation |

**Always use `"pages"` unless you have a specific reason not to.** It scans all routes at V8 init, so every class from any page is included. SSE navigation won't break.

---

## 2. Complete Utility Class Reference

Use the `class` attribute (not `className`) on JSX elements.

### Layout Primitives

| Class | CSS | Use for |
|-------|-----|---------|
| `stack` | `display:flex;flex-direction:column` | Vertical layouts |
| `row` | `display:flex;flex-direction:row` | Horizontal layouts |
| `cluster` | `display:flex;flex-wrap:wrap;align-items:center` | Inline tag groups, pill lists |
| `center` | `display:flex;align-items:center;justify-content:center` | Centering content |
| `wrap` | `flex-wrap:wrap` | Enable wrapping on any flex |
| `no-wrap` | `flex-wrap:nowrap` | Prevent wrapping |
| `grid-auto` | `display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,var(--min-w,16rem)),1fr))` | Auto-responsive card grids |
| `grid-2` | `display:grid;grid-template-columns:repeat(2,1fr)` | Fixed 2-column grid |
| `grid-3` | `display:grid;grid-template-columns:repeat(3,1fr)` | Fixed 3-column grid |
| `grid-4` | `display:grid;grid-template-columns:repeat(4,1fr)` | Fixed 4-column grid |
| `grid-5` | `display:grid;grid-template-columns:repeat(5,1fr)` | Fixed 5-column grid |
| `grid-6` | `display:grid;grid-template-columns:repeat(6,1fr)` | Fixed 6-column grid |
| `container` | `width:100%;margin-inline:auto;padding-inline:var(--m-space-md)` + responsive max-widths | Page wrapper |

### Flexbox

| Class | CSS |
|-------|-----|
| `grow` | `flex-grow:1` |
| `grow-0` | `flex-grow:0` |
| `shrink` | `flex-shrink:1` |
| `shrink-0` | `flex-shrink:0` |
| `items-start` | `align-items:flex-start` |
| `items-center` | `align-items:center` |
| `items-end` | `align-items:flex-end` |
| `items-stretch` | `align-items:stretch` |
| `items-baseline` | `align-items:baseline` |
| `justify-start` | `justify-content:flex-start` |
| `justify-center` | `justify-content:center` |
| `justify-end` | `justify-content:flex-end` |
| `justify-between` | `justify-content:space-between` |
| `justify-around` | `justify-content:space-around` |
| `justify-evenly` | `justify-content:space-evenly` |
| `self-auto` | `align-self:auto` |
| `self-start` | `align-self:flex-start` |
| `self-center` | `align-self:center` |
| `self-end` | `align-self:flex-end` |
| `self-stretch` | `align-self:stretch` |

### Spacing

Generated from `theme.spacing` tokens. Sizes depend on your `design.json` — typical: `xs`, `sm`, `md`, `lg`, `xl`, `2xl`, `3xl`.

| Pattern | Example | CSS |
|---------|---------|-----|
| `gap-{size}` | `gap-md` | `gap:var(--m-space-md)` |
| `p-{size}` | `p-lg` | `padding:var(--m-space-lg)` |
| `px-{size}` | `px-md` | `padding-inline:var(--m-space-md)` |
| `py-{size}` | `py-sm` | `padding-block:var(--m-space-sm)` |
| `pt-{size}` | `pt-md` | `padding-top:var(--m-space-md)` |
| `pr-{size}` | `pr-md` | `padding-right:var(--m-space-md)` |
| `pb-{size}` | `pb-md` | `padding-bottom:var(--m-space-md)` |
| `pl-{size}` | `pl-lg` | `padding-left:var(--m-space-lg)` |
| `m-{size}` | `m-md` | `margin:var(--m-space-md)` |
| `mx-{size}` | `mx-lg` | `margin-inline:var(--m-space-lg)` |
| `my-{size}` | `my-sm` | `margin-block:var(--m-space-sm)` |
| `mt-{size}` | `mt-xl` | `margin-top:var(--m-space-xl)` |
| `mr-{size}` | `mr-sm` | `margin-right:var(--m-space-sm)` |
| `mb-{size}` | `mb-md` | `margin-bottom:var(--m-space-md)` |
| `ml-{size}` | `ml-md` | `margin-left:var(--m-space-md)` |
| `mx-auto` | `mx-auto` | `margin-inline:auto` |

### Colors

Generated from `theme.colors`. Names depend on your `design.json`.

| Pattern | Example | CSS |
|---------|---------|-----|
| `fg-{color}` | `fg-primary` | `color:var(--m-primary)` |
| `bg-{color}` | `bg-surface` | `background-color:var(--m-surface)` |
| `border-{color}` | `border-primary` | `border-color:var(--m-primary)` |

With the task-board's `design.json`, available color names are: `primary`, `primary-hover`, `danger`, `surface`, `raised`, `sunken`, `text`, `heading`, `muted`, `subtle`, `border`, `border-hover`.

So you get: `fg-primary`, `fg-danger`, `fg-heading`, `fg-muted`, `fg-subtle`, `fg-text`, `bg-primary`, `bg-surface`, `bg-raised`, `bg-sunken`, `border-primary`, `border-danger`, etc.

### Typography

| Class | CSS | Notes |
|-------|-----|-------|
| `text-xs` | `font-size:var(--m-text-xs)` | Static |
| `text-sm` | `font-size:var(--m-text-sm)` | Static |
| `text-base` | `font-size:var(--m-text-base)` | Static |
| `text-lg` | `font-size:clamp(...)` | Fluid — scales with viewport |
| `text-xl` | `font-size:clamp(...)` | Fluid |
| `text-2xl` | `font-size:clamp(...)` | Fluid |
| `text-3xl` | `font-size:clamp(...)` | Fluid |
| `text-4xl` | `font-size:clamp(...)` | Fluid |
| `text-5xl` | `font-size:clamp(...)` | Fluid |
| `font-sans` | `font-family:var(--m-font-sans)` | |
| `font-mono` | `font-family:var(--m-font-mono)` | |
| `thin` | `font-weight:100` | |
| `light` | `font-weight:300` | |
| `normal` | `font-weight:400` | |
| `medium` | `font-weight:500` | |
| `semibold` | `font-weight:600` | |
| `bold` | `font-weight:700` | |
| `extrabold` | `font-weight:800` | |
| `italic` | `font-style:italic` | |
| `not-italic` | `font-style:normal` | |
| `leading-tight` | `line-height:1.25` | |
| `leading-normal` | `line-height:1.5` | |
| `leading-relaxed` | `line-height:1.6` | Value from your design.json |
| `tracking-tighter` | `letter-spacing:-0.05em` | |
| `tracking-tight` | `letter-spacing:-0.025em` | |
| `tracking-normal` | `letter-spacing:0` | |
| `tracking-wide` | `letter-spacing:0.025em` | |
| `tracking-wider` | `letter-spacing:0.05em` | |
| `uppercase` | `text-transform:uppercase` | |
| `lowercase` | `text-transform:lowercase` | |
| `capitalize` | `text-transform:capitalize` | |
| `normal-case` | `text-transform:none` | |
| `text-left` | `text-align:left` | |
| `text-center` | `text-align:center` | |
| `text-right` | `text-align:right` | |
| `text-justify` | `text-align:justify` | |
| `underline` | `text-decoration:underline` | |
| `line-through` | `text-decoration:line-through` | |
| `no-underline` | `text-decoration:none` | |
| `truncate` | `overflow:hidden;text-overflow:ellipsis;white-space:nowrap` | |
| `break-words` | `overflow-wrap:break-word` | |

Font sizes `lg` and above use `clamp()` for fluid scaling — they automatically shrink on small screens and grow on large screens. No responsive prefix needed for typography scaling.

### Borders & Radius

| Class | CSS |
|-------|-----|
| `border` | `border:1px solid var(--m-border)` |
| `border-t` | `border-top:1px solid var(--m-border)` |
| `border-r` | `border-right:1px solid var(--m-border)` |
| `border-b` | `border-bottom:1px solid var(--m-border)` |
| `border-l` | `border-left:1px solid var(--m-border)` |
| `border-none` | `border:none` |
| `round-sm` | `border-radius:var(--m-radius-sm)` |
| `round-md` | `border-radius:var(--m-radius-md)` |
| `round-lg` | `border-radius:var(--m-radius-lg)` |
| `round-full` | `border-radius:var(--m-radius-full)` |
| `round-none` | `border-radius:0` |

### Shadows

| Class | CSS |
|-------|-----|
| `shadow-sm` | `box-shadow:var(--m-shadow-sm)` |
| `shadow-md` | `box-shadow:var(--m-shadow-md)` |
| `shadow-lg` | `box-shadow:var(--m-shadow-lg)` |
| `shadow-xl` | `box-shadow:var(--m-shadow-xl)` |
| `shadow-none` | `box-shadow:none` |

Note: `shadow-xl` only exists if your `design.json` defines a `shadows.xl` token.

### Sizing

| Class | CSS |
|-------|-----|
| `w-full` | `width:100%` |
| `w-screen` | `width:100vw` |
| `w-auto` | `width:auto` |
| `h-full` | `height:100%` |
| `h-screen` | `height:100dvh` |
| `h-auto` | `height:auto` |
| `min-h-screen` | `min-height:100dvh` |
| `min-h-full` | `min-height:100%` |
| `min-w-0` | `min-width:0` |
| `max-w-sm` | `max-width:640px` |
| `max-w-md` | `max-width:768px` |
| `max-w-lg` | `max-width:1024px` |
| `max-w-xl` | `max-width:1280px` |
| `max-w-prose` | `max-width:65ch` |
| `max-w-none` | `max-width:none` |

`max-w-{name}` values come from `theme.breakpoints`.

### Aspect Ratio

| Class | CSS |
|-------|-----|
| `aspect-auto` | `aspect-ratio:auto` |
| `aspect-square` | `aspect-ratio:1` |
| `aspect-video` | `aspect-ratio:16/9` |
| `aspect-photo` | `aspect-ratio:4/3` |
| `aspect-wide` | `aspect-ratio:21/9` |

### Display

| Class | CSS |
|-------|-----|
| `hidden` | `display:none` |
| `block` | `display:block` |
| `inline` | `display:inline` |
| `inline-block` | `display:inline-block` |
| `flex` | `display:flex` |
| `inline-flex` | `display:inline-flex` |
| `grid` | `display:grid` |
| `inline-grid` | `display:inline-grid` |
| `contents` | `display:contents` |

### Position

| Class | CSS |
|-------|-----|
| `relative` | `position:relative` |
| `absolute` | `position:absolute` |
| `fixed` | `position:fixed` |
| `sticky` | `position:sticky` |
| `static` | `position:static` |
| `inset-0` | `inset:0` |
| `top-0` | `top:0` |
| `right-0` | `right:0` |
| `bottom-0` | `bottom:0` |
| `left-0` | `left:0` |

### Z-Index

`z-0`, `z-10`, `z-20`, `z-30`, `z-40`, `z-50`, `z-auto`

### Overflow

`overflow-hidden`, `overflow-auto`, `overflow-scroll`, `overflow-visible`, `overflow-x-auto`, `overflow-y-auto`, `overflow-x-hidden`, `overflow-y-hidden`

### Opacity

`opacity-0`, `opacity-25`, `opacity-50`, `opacity-75`, `opacity-100`

### Cursor

`cursor-pointer`, `cursor-default`, `cursor-not-allowed`, `cursor-wait`, `cursor-text`, `cursor-grab`

### Interactions

| Class | CSS |
|-------|-----|
| `pointer-events-none` | `pointer-events:none` |
| `pointer-events-auto` | `pointer-events:auto` |
| `select-none` | `user-select:none` |
| `select-text` | `user-select:text` |
| `select-all` | `user-select:all` |
| `select-auto` | `user-select:auto` |

### Transitions

| Class | CSS |
|-------|-----|
| `transition` | `transition:all 150ms ease` |
| `transition-colors` | `transition:color,background-color,border-color 150ms ease` |
| `transition-opacity` | `transition:opacity 150ms ease` |
| `transition-shadow` | `transition:box-shadow 150ms ease` |
| `transition-transform` | `transition:transform 150ms ease` |
| `transition-none` | `transition:none` |

### Accessibility

| Class | CSS |
|-------|-----|
| `sr-only` | Visually hidden, screen reader accessible |
| `not-sr-only` | Undo sr-only |

---

## 3. Responsive Prefixes

Every utility class can be prefixed with a breakpoint for mobile-first responsive design:

| Prefix | Min-width | Example |
|--------|-----------|---------|
| `sm:` | 640px | `sm:row` |
| `md:` | 768px | `md:row`, `md:grid-3` |
| `lg:` | 1024px | `lg:text-4xl` |
| `xl:` | 1280px | `xl:grid-4` |

```tsx
// Stack on mobile, row on tablet+
<div class="stack md:row gap-md">

// Hidden on mobile, visible on md+
<div class="hidden md:block">

// 1 column on mobile, 3 on desktop
<div class="stack lg:grid-3 gap-md">
```

---

## 4. Custom CSS (public/style.css)

Utility classes handle layout, spacing, typography, colors, borders, shadows, sizing. Use `style.css` **only** for:

- `:hover`, `:focus`, `:active` states
- `::placeholder` styling
- `@keyframes` animations
- Component-specific one-off styles (e.g., fixed max-width)

### Rules for style.css

1. **Always use `var(--m-*)` tokens** — never hardcode colors/spacing
2. **Name your selectors** — use semantic class names that you add alongside utility classes
3. **Keep it minimal** — if it can be a utility class, use the utility class

### Real Example (task-board)

```css
/* task-board — interactive states only (layout + theme from @magneticjs/css) */

.board { max-width: 520px; }

.nav-link:hover { color: var(--m-text); }
.add-input:focus { border-color: var(--m-primary); outline: none; }
.add-btn:hover { opacity: 0.85; }
.filter-btn:hover { color: var(--m-text); border-color: var(--m-border-hover); }
.task-card:hover { border-color: var(--m-border-hover); }

.check {
  width: 28px; height: 28px;
  border: 2px solid var(--m-border-hover);
  border-radius: var(--m-radius-full);
}
.check:hover { border-color: var(--m-primary); color: var(--m-primary); }
.check-done { background: var(--m-primary); border-color: var(--m-primary); color: #fff; }

.del:hover { color: var(--m-danger); }

.about-link { color: var(--m-primary); }
.about-link:hover { text-decoration: underline; }
```

Note: `--m-border-hover` and `--m-danger` exist because the app's `design.json` defines `"border-hover"` and `"danger"` as color tokens. Custom properties are generated from **your** token names.

### How Custom CSS Merges

Generated CSS + `public/style.css` → concatenated into a single inline `<style>` block in the SSR HTML. No extra network request. Order: theme vars → reset → utility classes → your style.css.

---

## 5. CSS Pipeline Architecture

```
design.json
     │
     ▼
CLI reads design.json
     │
     ▼
generateBridge() adds CSS imports to V8 bridge code
     │
     ▼
esbuild bundles bridge + @magneticjs/css into IIFE
     │
     ▼
V8 isolate executes bundle
     │
     ├─ render(path) → DomNode           (SSE/action paths — no CSS)
     └─ renderWithCSS(path) → {root, css} (SSR path — CSS included)
            │
            ▼
     Rust server parses {root, css}
     Merges generated CSS + public/style.css
     Injects into <head> as inline <style>
            │
            ▼
     Browser receives complete HTML + CSS
     No FOUC, no layout shift, no CSS file request
```

Key points:
- `render()` and `reduce()` are **untouched** — SSE and action paths return bare `DomNode`
- `renderWithCSS()` is a **new** export used only for initial SSR HTML
- CSS is generated **once** at V8 init (for `"all"` and `"pages"` modes) and cached
- SSE updates reuse the CSS already in the page — no style updates needed

---

## 6. Patterns and Examples

### Full Page Layout

Every page should follow this wrapper pattern:

```tsx
<div class="stack items-center p-xl min-h-screen" key="wrapper">
  <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg board" key="board">
    {/* page content */}
  </div>
</div>
```

- Outer: centers content vertically and horizontally, full viewport height
- Inner: card-like container with background, border, padding, shadow
- `board` is a custom class for `max-width` in `style.css`

### Navigation Bar

```tsx
<nav class="row gap-md justify-center" key="nav">
  <Link href="/" class="nav-link text-sm medium fg-primary" prefetch>Tasks</Link>
  <Link href="/about" class="nav-link text-sm medium fg-muted transition-colors" prefetch>About</Link>
</nav>
```

Active link gets `fg-primary`, inactive gets `fg-muted`. `transition-colors` for smooth hover.

### Card Component

```tsx
<div class="row items-center gap-sm bg-sunken border round-md px-md py-sm transition task-card">
  <span class="grow text-base">{title}</span>
  <button onClick="action" class="fg-muted cursor-pointer transition-colors">×</button>
</div>
```

- `bg-sunken` for recessed card look
- `transition` on the card for hover border effect (defined in `style.css`)
- `transition-colors` on the button for hover color change

### Form with Input

```tsx
<form class="row gap-sm" onSubmit="add_task" key="add-form">
  <input type="text" name="title" placeholder="Add a task..." autocomplete="off"
    class="add-input grow bg-sunken border round-md px-md py-sm fg-text text-base transition" />
  <button type="submit"
    class="add-btn bg-primary fg-heading round-md px-lg py-sm text-base semibold cursor-pointer transition">Add</button>
</form>
```

- Input: `grow` fills available space, `bg-sunken` for depth
- Button: `bg-primary fg-heading` for contrast, `cursor-pointer` for UX
- Focus state in `style.css`: `.add-input:focus { border-color: var(--m-primary); outline: none; }`

### Filter Buttons (Dynamic Classes from toViewModel)

```tsx
// In toViewModel():
filterAllClass: state.filter === 'all' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',

// In component:
<button onClick="filter_all" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${props.filterAllClass}`}>
  All
</button>
```

**Dynamic classes are computed in `toViewModel()`, never in components.** The component just receives the class string as a prop.

### Responsive Layout

```tsx
// Stack on mobile, row on tablet+
<div class="stack md:row gap-md items-center">
  <div class="grow">Content</div>
  <button class="shrink-0">Action</button>
</div>

// Hidden on mobile, visible on md+
<aside class="hidden md:block">Sidebar</aside>

// Different grid columns by breakpoint
<div class="grid-2 lg:grid-4 gap-md">
  {items.map(item => <Card item={item} />)}
</div>
```

### State-Driven Styling

```tsx
// In toViewModel():
cardClass: item.completed ? 'opacity-50' : '',
titleClass: item.completed ? 'line-through fg-muted' : '',

// In component:
<div class={`row items-center gap-sm ${task.cardClass}`}>
  <span class={`grow text-base ${task.titleClass}`}>{task.title}</span>
</div>
```

Completed items get `opacity-50` (faded) and `line-through fg-muted` (struck-through, dimmed text). All driven by state, computed in `toViewModel()`.

### About/Content Page

```tsx
<div class="stack gap-sm leading-relaxed" key="content">
  <h1 class="text-2xl bold fg-heading">About</h1>
  <p class="fg-subtle">Description paragraph.</p>
  <h2 class="text-lg semibold fg-text mt-md">Section</h2>
  <ul class="stack gap-xs pl-lg fg-subtle">
    <li>Item one</li>
    <li>Item two</li>
  </ul>
</div>
```

- `leading-relaxed` on the wrapper for comfortable reading
- `mt-md` for section spacing
- `pl-lg` for list indentation
- `fg-subtle` for secondary text (if your design.json defines `subtle`)

---

## 7. Tailwind → Magnetic CSS Mapping

| Tailwind | Magnetic CSS |
|----------|-------------|
| `flex flex-col` | `stack` |
| `flex flex-row` | `row` |
| `flex flex-wrap items-center` | `cluster` |
| `flex items-center justify-center` | `center` |
| `grid grid-cols-3` | `grid-3` |
| `grid grid-cols-[repeat(auto-fit,...)]` | `grid-auto` |
| `container mx-auto px-4` | `container` |
| `items-center` | `items-center` |
| `justify-between` | `justify-between` |
| `gap-4` | `gap-md` |
| `p-4` | `p-md` |
| `px-4` | `px-md` |
| `mx-auto` | `mx-auto` |
| `mt-4` | `mt-md` |
| `text-blue-500` | `fg-primary` |
| `bg-gray-900` | `bg-surface` |
| `border border-gray-700` | `border` (uses `--m-border` color) |
| `rounded-lg` | `round-lg` |
| `shadow-md` | `shadow-md` |
| `text-sm` | `text-sm` |
| `text-2xl` | `text-2xl` (fluid clamp() for lg+) |
| `font-bold` | `bold` |
| `font-semibold` | `semibold` |
| `italic` | `italic` |
| `line-through` | `line-through` |
| `text-center` | `text-center` |
| `uppercase` | `uppercase` |
| `truncate` | `truncate` |
| `leading-relaxed` | `leading-relaxed` |
| `w-full` | `w-full` |
| `min-h-screen` | `min-h-screen` |
| `max-w-prose` | `max-w-prose` |
| `hidden` | `hidden` |
| `block` | `block` |
| `relative` | `relative` |
| `absolute` | `absolute` |
| `z-50` | `z-50` |
| `overflow-hidden` | `overflow-hidden` |
| `opacity-50` | `opacity-50` |
| `cursor-pointer` | `cursor-pointer` |
| `transition` | `transition` |
| `transition-colors` | `transition-colors` |
| `sr-only` | `sr-only` |
| `flex-grow` | `grow` |
| `flex-shrink-0` | `shrink-0` |
| `aspect-video` | `aspect-video` |
| `sm:flex-row` | `sm:row` |
| `md:grid-cols-3` | `md:grid-3` |
| `lg:text-4xl` | `lg:text-4xl` |
| `hover:bg-blue-600` | Use `style.css`: `.btn:hover { background: var(--m-primary-hover); }` |
| `focus:ring-2` | Use `style.css`: `.input:focus { border-color: var(--m-primary); }` |
| `dark:bg-gray-900` | Automatic via `[data-theme="dark"]` — no prefix needed |

### What Tailwind has that we intentionally don't

| Tailwind feature | Why we skip it | What to do instead |
|-----------------|----------------|-------------------|
| `hover:`, `focus:`, `active:` prefixes | Server sends JSON DOM — pseudo-states are CSS-native | Put in `style.css` with `var(--m-*)` tokens |
| `w-[137px]` arbitrary values | Encourages inconsistency | Add a token to `design.json` or use `style.css` |
| `bg-blue-500/50` opacity modifier | Adds complexity | Use `style.css`: `background: rgb(99 102 241 / 0.5)` |
| `animate-spin`, `animate-pulse` | Rare in app UIs | Use `style.css` with `@keyframes` |
| `ring-2`, `ring-blue-500` | Focus rings | Use `style.css`: `outline` or `box-shadow` |
| `bg-gradient-to-r` | Gradients | Use `style.css`: `background: linear-gradient(...)` |
| 220 color shades | We use semantic names | Define your palette in `design.json` |

---

## 8. Anti-Patterns

```tsx
// WRONG: Don't use Tailwind
<div class="flex flex-col gap-4 p-4">     // ❌ Tailwind syntax
<div class="stack gap-md p-md">            // ✅ Magnetic CSS

// WRONG: Don't use className
<div className="stack gap-md">             // ⚠️ works but not conventional
<div class="stack gap-md">                 // ✅

// WRONG: Don't compute classes in components
function Card({ done }) {
  const cls = done ? 'opacity-50' : '';    // ❌ do this in toViewModel()
  return <div class={cls}>...</div>;
}

// WRONG: Don't write layout CSS in style.css
.card { display: flex; gap: 1rem; }        // ❌ use utility classes
// ✅ Only :hover, :focus, animations in style.css

// WRONG: Don't hardcode colors in style.css
.btn:hover { background: #4f46e5; }       // ❌ hardcoded
.btn:hover { background: var(--m-primary-hover); } // ✅ theme token

// WRONG: Don't use inline styles
<div style="padding: 16px">               // ❌
<div class="p-md">                         // ✅

// WRONG: Don't install Tailwind or PostCSS
npm install tailwindcss                    // ❌ not needed
```

---

## 9. Source Files

If you need to verify a class name or understand how something works:

| File | What it contains |
|------|-----------------|
| `js/packages/magnetic-css/src/types.ts` | `DesignConfig`, `CSSMode`, `DomNode` interfaces |
| `js/packages/magnetic-css/src/defaults.ts` | Default theme tokens (fallback if design.json omits values) |
| `js/packages/magnetic-css/src/theme.ts` | `compileTheme()` → CSS custom properties |
| `js/packages/magnetic-css/src/reset.ts` | `generateReset()` → minimal normalize (~400 bytes) |
| `js/packages/magnetic-css/src/utilities.ts` | `generateUtilities()` → all utility class definitions |
| `js/packages/magnetic-css/src/extract.ts` | `extractCSS()`, `generateAllCSS()`, `createExtractor()` |
| `js/packages/magnetic-css/README.md` | Full documentation |
| `apps/task-board/design.json` | Real-world design tokens example |
| `apps/task-board/public/style.css` | Real-world custom CSS example (23 lines) |
| `apps/task-board/pages/TasksPage.tsx` | Utility classes in use |
| `apps/task-board/components/TaskCard.tsx` | Dynamic classes from toViewModel |
