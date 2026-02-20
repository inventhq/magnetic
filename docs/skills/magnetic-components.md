# Magnetic Components — Claude Skill

You are building reusable UI components for the Magnetic framework. Magnetic components are **pure functions** that receive props and return JSON DOM descriptors via TSX. They run on the server in V8 — never in the browser.

## Critical Rules

1. **No React** — no hooks, no state, no effects, no context, no refs, no DOM APIs
2. **No client JS** — no `document`, `window`, `fetch`, `setTimeout`, `addEventListener`
3. **Events are action name strings** — `onClick="action_name"`, not `onClick={() => ...}`
4. **All dynamic data comes from props** — components never compute state themselves
5. **CSS class logic belongs in `toViewModel()`** — components receive pre-computed class strings
6. **Keys are required** on any element whose content or presence changes between renders
7. **Import only from `@magneticjs/server/jsx-runtime`** — `Head`, `Link`, `Fragment`
8. **Use `class` not `className`** — both work, but `class` is the convention

## Component Anatomy

```tsx
// components/Card.tsx
import type { CardProps } from './types.ts';

export function Card(props: CardProps) {
  return (
    <div class={`bg-raised border round-lg p-md ${props.containerClass}`} key={`card-${props.id}`}>
      <h3 class="text-lg bold fg-heading" key={`card-title-${props.id}`}>{props.title}</h3>
      <p class="text-sm fg-muted" key={`card-desc-${props.id}`}>{props.description}</p>
      {props.children}
    </div>
  );
}
```

Every component is:
- A **named export** function (not default export)
- **Props typed** via an interface in `components/types.ts`
- **Pure** — same props → same output, always

## Props Interface Pattern

Define all component interfaces in `components/types.ts`:

```ts
// components/types.ts

export interface Task {
  id: number;
  title: string;
  completed: boolean;
}

// View types extend domain types with display-specific fields
export interface TaskView extends Task {
  cardClass: string;      // pre-computed utility classes for the card container
  titleClass: string;     // pre-computed utility classes for the title
  checkClass: string;     // pre-computed utility classes for the checkbox
  checkmark: string;      // display character: '✓' or '○'
}

// Page-level props (what toViewModel() returns)
export interface TaskBoardProps {
  taskCount: string;
  visibleTasks: TaskView[];
  filterAllClass: string;
  filterActiveClass: string;
  filterDoneClass: string;
  isEmpty: boolean;
  emptyMessage: string;
}
```

**Key pattern**: Domain types (`Task`) hold data. View types (`TaskView`) extend them with display fields. The `toViewModel()` function in `server/state.ts` creates view types from domain types.

## Event Handling

Events in Magnetic are **action name strings**. The string is sent to the server's `reduce()` function.

### Click Actions

```tsx
// Simple action
<button onClick="reset" class="cursor-pointer">Reset</button>

// Parameterized action (encode ID in the action name)
<button onClick={`delete_${props.id}`} class="cursor-pointer">Delete</button>
<button onClick={`toggle_${props.id}`} class="cursor-pointer">Toggle</button>
```

### Form Submission

```tsx
<form onSubmit="add_item" key="add-form">
  <input type="text" name="title" placeholder="Enter title..." autocomplete="off" />
  <input type="number" name="priority" />
  <button type="submit">Add</button>
</form>
```

On submit, Magnetic collects all `<input>` values by their `name` attribute → `{ title: "...", priority: "..." }` and sends as payload to the reducer.

### Live Input (Debounced)

```tsx
<input type="text" name="q" placeholder="Search..." onInput="live_search" key="search-input" />
```

`onInput` is debounced (300ms). Payload: `{ value: "current input text" }`.

### All Supported Events

| Prop | Trigger | Payload |
|------|---------|---------|
| `onClick` | Click | `{}` |
| `onSubmit` | Form submit | `{ inputName: value, ... }` |
| `onInput` | Keystroke (300ms debounce) | `{ value: "..." }` |
| `onChange` | Input change | `{}` |
| `onFocus` | Focus | `{}` |
| `onBlur` | Blur | `{}` |
| `onKeyDown` | Key pressed | `{}` |
| `onKeyUp` | Key released | `{}` |
| `onScroll` | Scroll | `{}` |

## Keys — When and How

Keys tell the DOM differ which elements to patch vs recreate.

### Required
```tsx
// Dynamic list items — MUST have unique, stable keys
{props.items.map(item => (
  <div key={`item-${item.id}`}>{item.name}</div>
))}

// Content that changes between renders
<h1 key="title">{props.title}</h1>
<p key="count">{props.count} items</p>

// Conditionally rendered elements
{props.showBanner && <div key="banner">Welcome!</div>}

// Form inputs (preserves focus/cursor across patches)
<input key="search-input" name="q" />
```

### Not Required
```tsx
// Static content that never changes
<footer>Built with Magnetic</footer>
<label>Username:</label>
```

### Key Rules
- **Unique among siblings** (not globally unique)
- **Stable** — don't use array index for reorderable lists, use `item.id`
- **Consistent across pages** — shared elements (like nav) must use the same key on every page
- **Prefix with component context** — `task-${id}`, `filter-${name}`, `card-${id}`

## Styling with Utility Classes

Components use utility classes from `@magneticjs/css`. The theme is defined in `design.json`.

### Static Classes (hardcoded)
```tsx
<div class="stack gap-md p-lg bg-raised border round-lg shadow-md">
```

### Dynamic Classes (from props)
```tsx
// Props contain pre-computed class strings from toViewModel()
<div class={`row items-center gap-sm ${props.cardClass}`}>
  <span class={`grow text-base ${props.titleClass}`}>{props.title}</span>
</div>
```

### Interactive States (custom CSS)
Utility classes can't express `:hover`, `:focus`, etc. Use a CSS class name and define it in `public/style.css`:

```tsx
// Component uses a class name for the interactive element
<button class="del fg-muted text-lg cursor-pointer transition-colors">×</button>
```

```css
/* public/style.css — uses theme tokens */
.del:hover { color: var(--m-danger); }
```

### Common Layout Patterns

```tsx
// Full-page centered layout
<div class="stack items-center p-xl min-h-screen">

// Card container
<div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg">

// Horizontal row with spacing
<div class="row gap-sm items-center">

// Navigation bar
<nav class="row gap-md justify-center" key="nav">

// Form row (input + button)
<form class="row gap-sm" onSubmit="action_name">
```

## Built-in Components

### Link — Client-Side Navigation

```tsx
import { Link } from '@magneticjs/server/jsx-runtime';

<Link href="/about" prefetch>About</Link>
<Link href={`/users/${props.userId}`}>Profile</Link>
<Link href="/" class="nav-link fg-primary" prefetch>Home</Link>
```

- Renders `<a>` with client-side pushState navigation (no full page reload)
- `prefetch` triggers a GET on hover to pre-render the target page
- Accepts `class` and any standard `<a>` attributes

### Head — Document Metadata

```tsx
import { Head } from '@magneticjs/server/jsx-runtime';

<Head>
  <title>{`Tasks (${props.taskCount})`}</title>
  <meta name="description" content="Task management app" />
  <link rel="icon" href="/favicon.ico" />
</Head>
```

- Extracted into `<head>` during SSR
- Ignored during live SSE updates (head is static after initial render)
- Place inside any page component

## Real-World Component Examples

### Task Card (list item with actions)

```tsx
// components/TaskCard.tsx
import type { TaskView } from './types.ts';

export function TaskCard({ task }: { task: TaskView }) {
  return (
    <div key={`task-${task.id}`} class={`row items-center gap-sm bg-sunken border round-md px-md py-sm transition task-card ${task.cardClass}`}>
      <button onClick={`toggle_${task.id}`} class={`check center shrink-0 fg-muted text-sm cursor-pointer transition ${task.checkClass}`} key={`chk-${task.id}`}>
        {task.checkmark}
      </button>
      <span class={`grow text-base ${task.titleClass}`} key={`tt-${task.id}`}>{task.title}</span>
      <button onClick={`delete_${task.id}`} class="del fg-muted text-lg cursor-pointer transition-colors p-xs" key={`del-${task.id}`}>×</button>
    </div>
  );
}
```

### Form Input (stateless form)

```tsx
// components/TaskInput.tsx
export function TaskInput() {
  return (
    <form class="row gap-sm" onSubmit="add_task" key="add-form">
      <input type="text" name="title" placeholder="Add a task..." autocomplete="off"
        class="add-input grow bg-sunken border round-md px-md py-sm fg-text text-base transition" key="add-input" />
      <button type="submit"
        class="add-btn bg-primary fg-heading round-md px-lg py-sm text-base semibold cursor-pointer transition" key="add-btn">Add</button>
    </form>
  );
}
```

### Filter Buttons (state-driven styling)

```tsx
// components/Filters.tsx
export function Filters({
  allClass,
  activeClass,
  doneClass,
}: {
  allClass: string;
  activeClass: string;
  doneClass: string;
}) {
  return (
    <div class="row gap-xs justify-center" key="filters">
      <button onClick="filter_all" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${allClass}`} key="f-all">All</button>
      <button onClick="filter_active" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${activeClass}`} key="f-active">Active</button>
      <button onClick="filter_done" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${doneClass}`} key="f-done">Done</button>
    </div>
  );
}
```

### Empty State

```tsx
// components/EmptyState.tsx
export function EmptyState({ message, icon }: { message: string; icon?: string }) {
  return (
    <div class="stack items-center gap-md py-2xl" key="empty">
      {icon && <span class="text-4xl" key="empty-icon">{icon}</span>}
      <p class="fg-muted text-sm italic text-center" key="empty-msg">{message}</p>
    </div>
  );
}
```

### Navigation (shared across pages)

```tsx
// components/Nav.tsx
import { Link } from '@magneticjs/server/jsx-runtime';

export function Nav({ activePage }: { activePage: string }) {
  return (
    <nav class="row gap-md justify-center py-sm" key="nav">
      <Link href="/" class={activePage === 'tasks' ? 'nav-link fg-primary' : 'nav-link fg-muted'} prefetch>Tasks</Link>
      <Link href="/about" class={activePage === 'about' ? 'nav-link fg-primary' : 'nav-link fg-muted'} prefetch>About</Link>
    </nav>
  );
}
```

### Data Display Card

```tsx
// components/StatCard.tsx
export function StatCard({ label, value, valueClass }: { label: string; value: string; valueClass?: string }) {
  return (
    <div class="stack gap-xs bg-raised border round-md p-md" key={`stat-${label}`}>
      <span class="text-xs fg-muted uppercase tracking-wide">{label}</span>
      <span class={`text-2xl bold fg-heading ${valueClass || ''}`} key={`stat-val-${label}`}>{value}</span>
    </div>
  );
}
```

## Component Composition in Pages

```tsx
// pages/TasksPage.tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';
import { Nav } from '../components/Nav.tsx';
import { TaskInput } from '../components/TaskInput.tsx';
import { Filters } from '../components/Filters.tsx';
import { TaskCard } from '../components/TaskCard.tsx';
import { EmptyState } from '../components/EmptyState.tsx';
import type { TaskBoardProps } from '../components/types.ts';

export function TasksPage(props: TaskBoardProps & { params: Record<string, string> }) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg board" key="board">
        <Head>
          <title>{`Tasks (${props.taskCount})`}</title>
        </Head>
        <Nav activePage="tasks" />
        <h1 class="text-2xl bold fg-heading text-center" key="title">Tasks</h1>
        <TaskInput />
        <Filters allClass={props.filterAllClass} activeClass={props.filterActiveClass} doneClass={props.filterDoneClass} />
        <div class="stack gap-sm" key="task-list">
          {props.visibleTasks.map(task => <TaskCard task={task} />)}
        </div>
        {props.isEmpty && <EmptyState message={props.emptyMessage} />}
      </div>
    </div>
  );
}
```

## How toViewModel() Drives Components

Components receive pre-computed display data. The `server/state.ts` `toViewModel()` function is where you:

1. **Filter/sort** data based on state
2. **Compute CSS classes** based on item state (active, selected, completed, etc.)
3. **Format strings** for display (counts, labels, dates)
4. **Derive booleans** for conditional rendering (`isEmpty`, `showBanner`, etc.)

```ts
// server/state.ts
export function toViewModel(state: AppState) {
  const visible = state.tasks
    .filter(t => state.filter === 'active' ? !t.completed : state.filter === 'done' ? t.completed : true)
    .map(t => ({
      ...t,
      // CSS classes computed HERE, not in components
      cardClass: t.completed ? 'opacity-50' : '',
      titleClass: t.completed ? 'line-through fg-muted' : '',
      checkClass: t.completed ? 'check-done' : '',
      checkmark: t.completed ? '✓' : '○',
    }));

  return {
    visibleTasks: visible,
    taskCount: `${state.tasks.filter(t => !t.completed).length} active`,
    filterAllClass: state.filter === 'all' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    filterActiveClass: state.filter === 'active' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    filterDoneClass: state.filter === 'done' ? 'bg-primary fg-heading' : 'bg-raised fg-muted',
    isEmpty: visible.length === 0,
    emptyMessage: state.filter === 'active' ? 'All done!' : 'No tasks yet.',
  };
}
```

## Anti-Patterns — What NOT to Do

```tsx
// ❌ React hooks
const [open, setOpen] = useState(false);
useEffect(() => { ... }, []);

// ❌ Client-side APIs
document.getElementById('x');
window.scrollTo(0, 0);
fetch('/api/data');

// ❌ Event handlers as callbacks
<button onClick={() => doSomething()}>

// ✅ Event handlers as action strings
<button onClick="do_something">

// ❌ Computing CSS classes inside components
function Card({ done }) {
  const cls = done ? 'opacity-50' : '';
  return <div class={cls}>...</div>;
}

// ✅ Receiving pre-computed classes from toViewModel()
function Card({ cardClass }) {
  return <div class={cardClass}>...</div>;
}

// ❌ Missing keys on dynamic content
{items.map(item => <div>{item.name}</div>)}

// ✅ Keys on every dynamic element
{items.map(item => <div key={`item-${item.id}`}>{item.name}</div>)}

// ❌ Using array index as key
{items.map((item, i) => <div key={i}>{item.name}</div>)}

// ✅ Using stable ID as key
{items.map(item => <div key={`item-${item.id}`}>{item.name}</div>)}

// ❌ Importing React
import React from 'react';

// ✅ Importing Magnetic
import { Head, Link } from '@magneticjs/server/jsx-runtime';

// ❌ Default exports
export default function Card() { ... }

// ✅ Named exports
export function Card() { ... }

// ❌ className (works but not conventional)
<div className="card">

// ✅ class
<div class="card">
```

## File Organization

```
components/
├── types.ts           ← All interfaces (domain types + view types + page props)
├── Nav.tsx            ← Shared navigation
├── TaskCard.tsx       ← Individual task item
├── TaskInput.tsx      ← Add task form
├── Filters.tsx        ← Filter buttons
├── EmptyState.tsx     ← Empty state placeholder
├── StatCard.tsx       ← Data display card
└── ...
```

**Conventions:**
- One component per file
- PascalCase filename matching the function name
- Types in `components/types.ts` (shared) or inline for simple props
- Import with `.tsx` extension: `import { Card } from '../components/Card.tsx'`

## Available Packages

| Package | Version | Purpose |
|---------|---------|---------|
| `@magneticjs/server` | 0.1.3 | JSX runtime (`Head`, `Link`, `Fragment`), router, SSR |
| `@magneticjs/css` | 0.1.0 | CSS framework (utility classes, theme, extraction) |
| `@magneticjs/cli` | 0.1.7 | Build, dev, deploy CLI |
| `create-magnetic-app` | 0.1.5 | Project scaffolder |

Components only need to import from `@magneticjs/server/jsx-runtime`. The CSS framework is consumed via class names in TSX — no import needed.

## Reference

- **App development skill**: `docs/skills/magnetic-app-development.md`
- **CSS styling skill**: `docs/skills/magnetic-css-styling.md`
- **Real app example**: `apps/task-board/` (components, pages, state, design.json)
