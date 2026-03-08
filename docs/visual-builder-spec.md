# Visual Builder — Product Spec

> A Magnetic SSR app for visually configuring design systems, laying out pages, wiring data integrations, and deploying Magnetic apps.

## Location

`apps/visual-builder/` — standard Magnetic SSR app structure.

## Architecture

The builder is a Magnetic app that builds other Magnetic apps. It runs in SSR mode (not SSG) because it needs:
- Live preview via SSE
- Interactive state (selected element, AST, undo history)
- Server-side V8 rendering of the preview canvas

### Key files it generates for target apps

| File | Purpose |
|------|---------|
| `design.json` | Design tokens (colors, spacing, typography, radius, shadows) |
| `pages/*.tsx` | TSX page components |
| `server/state.ts` | State management, data source wiring, view model |
| `magnetic.json` | App config including data source declarations |
| `public/style.css` | Custom overrides (if any) |

### Core data model

The builder manipulates a **Component AST** — a JSON tree that maps 1:1 to TSX output:

```typescript
interface ComponentNode {
  tag: string;              // "div", "aside", "nav", "h1", "span", "a", ...
  key: string;              // required for Magnetic reconciliation
  classes: string[];        // ["stack", "gap-md", "p-lg", "border-b"]
  props: Record<string, string>;
  children: (ComponentNode | string)[];
  dataBinding?: string;     // e.g. "props.data.events.price"
}
```

Editing = mutating this AST. Preview = rendering AST through V8. Export = serializing AST to TSX source files.

---

## Phased Plan

### Phase 1: Design System Configurator

The entry point. Before building pages, establish the design system.

**What it does:**
- Edit typography: font family, size scale, line heights, weights
- Edit colors: bg, fg, accent, muted, semantic (success/warning/error)
- Edit spacing scale (xs through 3xl)
- Edit border radius scale
- Edit shadows / elevation levels
- Live preview of all tokens as you adjust them (swatches, type specimens, spacing rulers)
- Export: generates a valid `design.json`

**Why start here:**
- Self-contained, immediately useful for existing apps
- Establishes the SSR app structure that later phases build on
- The output (`design.json`) is already consumed by every Magnetic app

**Reference:** See `apps/docs/design.json` for the current token format.

### Phase 2: Page Layout Editor

**What it does:**
- Component palette (stack, row, grid, heading, text, card, nav, table, form)
- Drag components to a structured canvas
- Property panel: design token dropdowns, class toggles, text editing
- Component tree view (collapsible AST)
- Export: generates TSX page files

**Important constraints:**
- This is a structured layout editor, NOT a pixel-perfect drawing tool
- Components use Magnetic CSS utility classes, not arbitrary positioning
- Every element must have a `key` attribute (Magnetic reconciliation requirement)

### Phase 3: Data Integration Wiring

**What it does:**
- Data Sources panel: add URL + type (REST / SSE / WS)
- Auto-discover response shape (hit endpoint, infer fields)
- Bind data fields to components visually
- Live data flows through preview via SSE/WS
- Export: generates `magnetic.json` data source config + state.ts bindings

**Why this matters:**
Magnetic already standardizes data source declaration in `magnetic.json`:
```json
{
  "data": {
    "events": { "url": "https://...", "type": "sse", "buffer": 20, "target": "feed" },
    "prices": { "url": "wss://...", "type": "ws", "buffer": 50 }
  }
}
```
The builder makes this visual. Most editors punt on data integration — this is the differentiator.

### Phase 4: Multi-Page & Deploy

- Page management (add/remove/reorder pages, define routes)
- Navigation wiring between pages
- `magnetic push` directly from the builder UI
- Cross-platform preview: web + simulated iOS/Android device frames

### Phase 5: AI Copilot

- AST-aware layout suggestions
- Auto-wire integrations when API URLs are provided
- Dashboard generation from API schema
- Component recommendations based on data shape

---

## Technical References

| What | Where |
|------|-------|
| Design token format | `apps/docs/design.json` |
| CSS utility classes | `js/packages/magnetic-css/` |
| SSR app example | `apps/shipment-tracker/` |
| Data source config | `magnetic.json` `data` field |
| State management pattern | `apps/docs/server/state.ts` |
| Native SDK (iOS) | `native/ios/MagneticSDK/` |
| Native SDK (Android) | `native/android/magnetic-sdk/` |
| CLI deploy command | `magnetic push --name <app> --server https://api.fujs.dev` |
| Public docs (deployment) | `apps/docs/content/deployment.md` |
| Public docs (components) | `apps/docs/content/components.md` |
| Public docs (CSS) | `apps/docs/content/css-styling.md` |
| Public docs (app dev) | `apps/docs/content/app-development.md` |

## Not in scope

- Pixel-perfect drawing / vector tools (not Figma)
- Animation timeline
- Code editor inside the builder (export to files, edit in IDE)
- Mobile-specific gesture design
