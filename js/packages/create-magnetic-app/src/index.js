/**
 * create-magnetic-app — scaffolds a new Magnetic project.
 *
 * Generates:
 *   <name>/
 *   ├── pages/
 *   │   ├── IndexPage.tsx       (main page component)
 *   │   └── NotFoundPage.tsx    (404 page)
 *   ├── components/
 *   │   └── (shared components)
 *   ├── server/
 *   │   └── state.ts            (business logic — state, reducer, viewModel)
 *   ├── public/
 *   │   ├── magnetic.js         (client runtime)
 *   │   └── style.css           (app styles)
 *   ├── magnetic.json           (project config)
 *   └── README.md
 *
 * Developer writes ONLY pages/components + server/state.ts.
 * The @magnetic/cli handles bundling, bridge generation, and server management.
 */
import { mkdirSync, writeFileSync, copyFileSync, existsSync } from 'fs';
import { join, resolve } from 'path';

/**
 * @param {string} dest — target directory
 * @param {object} opts
 * @param {string} opts.name — project name
 * @param {string} [opts.runtimeSrc] — path to magnetic.js to copy
 * @param {string} [opts.template] — template name: "blank" | "todo" (default: "todo")
 * @param {object} [opts.files] — extra files to write: { relativePath: content }
 */
export function scaffold(dest, opts = {}) {
  const name = opts.name || 'my-magnetic-app';
  const template = opts.template || 'todo';
  const dir = resolve(dest);

  // Create directories
  for (const d of ['pages', 'components', 'server', 'public']) {
    mkdirSync(join(dir, d), { recursive: true });
  }

  // magnetic.json
  writeFileSync(join(dir, 'magnetic.json'), JSON.stringify({
    name,
    server: 'http://localhost:3003',
  }, null, 2) + '\n');

  // tsconfig.json for IDE support
  writeFileSync(join(dir, 'tsconfig.json'), JSON.stringify({
    compilerOptions: {
      target: 'ES2020',
      module: 'ESNext',
      moduleResolution: 'bundler',
      jsx: 'react-jsx',
      jsxImportSource: '@magneticjs/server',
      strict: true,
      noEmit: true,
      allowImportingTsExtensions: true,
    },
    include: ['pages', 'components', 'server'],
  }, null, 2) + '\n');

  // Generate template files
  if (template === 'todo') {
    writeTodoTemplate(dir, name);
  } else {
    writeBlankTemplate(dir, name);
  }

  // README + GUIDE
  writeFileSync(join(dir, 'README.md'), readme(name));
  writeFileSync(join(dir, 'GUIDE.md'), guide(name));

  // Copy client runtime if available
  if (opts.runtimeSrc && existsSync(opts.runtimeSrc)) {
    copyFileSync(opts.runtimeSrc, join(dir, 'public/magnetic.js'));
  }

  // Write any extra files
  if (opts.files) {
    for (const [rel, content] of Object.entries(opts.files)) {
      const p = join(dir, rel);
      mkdirSync(join(p, '..'), { recursive: true });
      writeFileSync(p, content);
    }
  }

  return dir;
}

// ── Todo template ──────────────────────────────────────────────────

function writeTodoTemplate(dir, name) {
  // server/state.ts — business logic
  writeFileSync(join(dir, 'server/state.ts'), `// ${name} — Server-side state and reducer
// This is your business logic. The Magnetic platform runs this on the server.
// State is never exposed to the client — only the rendered UI (JSON DOM) is sent.

export interface Todo {
  id: number;
  title: string;
  completed: boolean;
}

export interface AppState {
  todos: Todo[];
  nextId: number;
  filter: 'all' | 'active' | 'done';
}

export function initialState(): AppState {
  return {
    todos: [],
    nextId: 1,
    filter: 'all',
  };
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  switch (action) {
    case 'add_todo': {
      const title = payload?.title?.trim();
      if (!title) return state;
      return {
        ...state,
        todos: [...state.todos, { id: state.nextId, title, completed: false }],
        nextId: state.nextId + 1,
      };
    }

    case 'filter_all':
      return { ...state, filter: 'all' };
    case 'filter_active':
      return { ...state, filter: 'active' };
    case 'filter_done':
      return { ...state, filter: 'done' };

    case 'clear_completed':
      return { ...state, todos: state.todos.filter(t => !t.completed) };

    default: {
      // Parameterized actions: toggle_1, delete_1
      const toggleMatch = action.match(/^toggle_(\\d+)$/);
      if (toggleMatch) {
        const id = parseInt(toggleMatch[1], 10);
        return {
          ...state,
          todos: state.todos.map(t =>
            t.id === id ? { ...t, completed: !t.completed } : t
          ),
        };
      }

      const deleteMatch = action.match(/^delete_(\\d+)$/);
      if (deleteMatch) {
        const id = parseInt(deleteMatch[1], 10);
        return { ...state, todos: state.todos.filter(t => t.id !== id) };
      }

      return state;
    }
  }
}

export function toViewModel(state: AppState) {
  const visibleTodos = state.todos
    .filter(t => {
      if (state.filter === 'active') return !t.completed;
      if (state.filter === 'done') return t.completed;
      return true;
    })
    .map(t => ({
      ...t,
      completedClass: t.completed ? 'completed' : '',
      checkmark: t.completed ? '✓' : '○',
    }));

  const active = state.todos.filter(t => !t.completed).length;
  const done = state.todos.filter(t => t.completed).length;
  const total = state.todos.length;

  return {
    ...state,
    visibleTodos,
    activeCount: active,
    doneCount: done,
    totalCount: total,
    summary: total === 0 ? 'No todos yet' : \`\${active} active, \${done} done\`,
    filterAllClass: state.filter === 'all' ? 'active' : '',
    filterActiveClass: state.filter === 'active' ? 'active' : '',
    filterDoneClass: state.filter === 'done' ? 'active' : '',
    isEmpty: visibleTodos.length === 0,
    emptyMessage:
      state.filter === 'active'
        ? 'All done! Nothing active.'
        : state.filter === 'done'
          ? 'No completed todos yet.'
          : 'Add your first todo above!',
  };
}
`);

  // pages/IndexPage.tsx — main page
  writeFileSync(join(dir, 'pages/IndexPage.tsx'), `import { Head, Link } from '@magneticjs/server/jsx-runtime';
import { TodoInput } from '../components/TodoInput.tsx';
import { TodoFilters } from '../components/TodoFilters.tsx';
import { TodoItem } from '../components/TodoItem.tsx';

export function IndexPage(props: any) {
  return (
    <div class="todo-app" key="app">
      <Head>
        <title>{\`\${props.summary} | ${name}\`}</title>
        <meta name="description" content="A todo app built with Magnetic" />
      </Head>

      <nav class="topnav" key="nav">
        <Link href="/" class="nav-link active">Todos</Link>
        <Link href="/about" class="nav-link">About</Link>
      </nav>

      <div class="header" key="header">
        <h1 key="title">${name}</h1>
        <p class="subtitle" key="subtitle">{props.summary}</p>
      </div>

      <TodoInput />

      <TodoFilters
        allClass={props.filterAllClass}
        activeClass={props.filterActiveClass}
        doneClass={props.filterDoneClass}
      />

      <div class="todo-list" key="todo-list">
        {props.visibleTodos.map((todo: any) => (
          <TodoItem todo={todo} />
        ))}
      </div>

      {props.isEmpty && <p class="empty" key="empty">{props.emptyMessage}</p>}

      {props.doneCount > 0 && (
        <div class="footer" key="footer">
          <button class="clear-btn" onClick="clear_completed" key="clear">
            Clear completed ({props.doneCount})
          </button>
        </div>
      )}
    </div>
  );
}
`);

  // pages/AboutPage.tsx
  writeFileSync(join(dir, 'pages/AboutPage.tsx'), `import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function AboutPage(props: any) {
  return (
    <div class="about-page" key="about">
      <Head>
        <title>About | ${name}</title>
      </Head>

      <nav class="topnav" key="nav">
        <Link href="/" class="nav-link">Todos</Link>
        <Link href="/about" class="nav-link active">About</Link>
      </nav>

      <div class="content" key="content">
        <h1>About</h1>
        <p>This app was built with Magnetic — a server-driven UI framework.</p>

        <h2>How it works</h2>
        <ul>
          <li>All state lives on the server (Rust + V8)</li>
          <li>UI is rendered as JSON DOM descriptors on the server</li>
          <li>The client is a thin rendering shell (~1.5KB)</li>
          <li>Real-time updates via Server-Sent Events</li>
          <li>Actions are sent to the server, which re-renders and pushes updates</li>
        </ul>

        <h2>What the developer writes</h2>
        <ul>
          <li>TSX page components (this page!)</li>
          <li>Business logic in server/state.ts</li>
          <li>That's it. No client-side JS, no state management, no build config.</li>
        </ul>

        <p><Link href="/">← Back to todos</Link></p>
      </div>
    </div>
  );
}
`);

  // pages/NotFoundPage.tsx
  writeFileSync(join(dir, 'pages/NotFoundPage.tsx'), `import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function NotFoundPage(props: any) {
  return (
    <div class="not-found" key="404">
      <Head><title>404 | ${name}</title></Head>
      <h1>404</h1>
      <p>Page not found</p>
      <p><Link href="/">← Back home</Link></p>
    </div>
  );
}
`);

  // components/TodoInput.tsx
  writeFileSync(join(dir, 'components/TodoInput.tsx'), `export function TodoInput() {
  return (
    <form class="add-form" onSubmit="add_todo" key="input">
      <input
        type="text"
        name="title"
        placeholder="What needs to be done?"
        autocomplete="off"
        autofocus
      />
      <button type="submit">Add</button>
    </form>
  );
}
`);

  // components/TodoFilters.tsx
  writeFileSync(join(dir, 'components/TodoFilters.tsx'), `export function TodoFilters(props: {
  allClass: string;
  activeClass: string;
  doneClass: string;
}) {
  return (
    <div class="filters" key="filters">
      <button class={\`filter-btn \${props.allClass}\`} onClick="filter_all" key="f-all">All</button>
      <button class={\`filter-btn \${props.activeClass}\`} onClick="filter_active" key="f-active">Active</button>
      <button class={\`filter-btn \${props.doneClass}\`} onClick="filter_done" key="f-done">Done</button>
    </div>
  );
}
`);

  // components/TodoItem.tsx
  writeFileSync(join(dir, 'components/TodoItem.tsx'), `export function TodoItem(props: { todo: any }) {
  const { todo } = props;
  return (
    <div class={\`todo-card \${todo.completedClass}\`} key={\`todo-\${todo.id}\`}>
      <button class="check" onClick={\`toggle_\${todo.id}\`} key={\`chk-\${todo.id}\`}>
        {todo.checkmark}
      </button>
      <span class="todo-title" key={\`title-\${todo.id}\`}>{todo.title}</span>
      <button class="delete" onClick={\`delete_\${todo.id}\`} key={\`del-\${todo.id}\`}>
        ×
      </button>
    </div>
  );
}
`);

  // public/style.css
  writeFileSync(join(dir, 'public/style.css'), todoStyles(name));
}

// ── Blank template ─────────────────────────────────────────────────

function writeBlankTemplate(dir, name) {
  writeFileSync(join(dir, 'server/state.ts'), `export interface AppState {}

export function initialState(): AppState {
  return {};
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  return state;
}

export function toViewModel(state: AppState) {
  return { ...state };
}
`);

  writeFileSync(join(dir, 'pages/IndexPage.tsx'), `import { Head } from '@magneticjs/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div class="app" key="app">
      <Head><title>${name}</title></Head>
      <h1 key="title">Welcome to ${name}</h1>
      <p key="desc">Edit pages/IndexPage.tsx to get started.</p>
    </div>
  );
}
`);

  writeFileSync(join(dir, 'pages/NotFoundPage.tsx'), `import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function NotFoundPage(props: any) {
  return (
    <div class="not-found" key="404">
      <Head><title>404 | ${name}</title></Head>
      <h1>404</h1>
      <p>Page not found</p>
      <p><Link href="/">← Back home</Link></p>
    </div>
  );
}
`);

  writeFileSync(join(dir, 'public/style.css'), `*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: system-ui, -apple-system, sans-serif;
  background: #0a0a0a; color: #e4e4e7;
  display: flex; justify-content: center;
  min-height: 100vh; padding: 2rem;
}
.app {
  max-width: 520px; width: 100%;
  background: #141414; border: 1px solid #252525; border-radius: 16px;
  padding: 2rem; text-align: center;
}
h1 { font-size: 1.75rem; color: #fff; margin-bottom: .5rem; }
p { color: #a1a1aa; }
.not-found { text-align: center; padding: 3rem; }
.not-found h1 { font-size: 3rem; color: #6366f1; }
.not-found a { color: #6366f1; text-decoration: none; }
`);
}

// ── Styles ─────────────────────────────────────────────────────────

function todoStyles(name) {
  return `*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  background: #0a0a0a; color: #e4e4e7;
  display: flex; justify-content: center;
  min-height: 100vh; padding: 2rem;
}

/* Navigation */
.topnav {
  display: flex; gap: 1rem; margin-bottom: 1.25rem;
  justify-content: center;
}
.nav-link {
  color: #71717a; text-decoration: none; font-size: .85rem;
  font-weight: 500; padding: .25rem .5rem; border-radius: 6px;
  transition: all .15s;
}
.nav-link:hover { color: #e4e4e7; }
.nav-link.active { color: #6366f1; }

/* App container */
.todo-app, .about-page, .not-found {
  max-width: 520px; width: 100%;
  background: #141414; border: 1px solid #252525; border-radius: 16px;
  padding: 1.5rem; box-shadow: 0 8px 32px rgba(0,0,0,.5);
}
.header { text-align: center; margin-bottom: 1.25rem; }
.header h1 { font-size: 1.75rem; font-weight: 700; color: #fff; margin-bottom: .25rem; }
.subtitle { font-size: .85rem; color: #71717a; }

/* Input form */
.add-form {
  display: flex; gap: .5rem; margin-bottom: 1rem;
}
.add-form input {
  flex: 1; background: #0d0d0d; border: 1px solid #333;
  border-radius: 10px; padding: .6rem .875rem; color: #e4e4e7;
  font-size: .9rem; outline: none; transition: border-color .15s;
}
.add-form input:focus { border-color: #6366f1; }
.add-form button {
  background: #6366f1; color: #fff; border: none;
  border-radius: 10px; padding: .6rem 1.25rem; font-size: .9rem;
  font-weight: 600; cursor: pointer; transition: background .15s;
}
.add-form button:hover { background: #4f46e5; }

/* Filters */
.filters {
  display: flex; gap: .375rem; margin-bottom: 1rem;
  justify-content: center;
}
.filter-btn {
  background: #1a1a2e; color: #71717a; border: 1px solid #252525;
  border-radius: 8px; padding: .375rem .875rem; font-size: .8rem;
  cursor: pointer; transition: all .15s;
}
.filter-btn:hover { color: #e4e4e7; border-color: #333; }
.filter-btn.active {
  background: #6366f1; color: #fff; border-color: #6366f1;
}

/* Todo list */
.todo-list { display: flex; flex-direction: column; gap: .5rem; }
.todo-card {
  display: flex; align-items: center; gap: .625rem;
  background: #0d0d0d; border: 1px solid #252525; border-radius: 10px;
  padding: .625rem .875rem; transition: all .15s;
}
.todo-card:hover { border-color: #333; }
.todo-card.completed { opacity: .5; }
.todo-card.completed .todo-title { text-decoration: line-through; color: #71717a; }

.check {
  width: 28px; height: 28px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  background: transparent; border: 2px solid #333;
  border-radius: 50%; color: #71717a; font-size: .8rem;
  cursor: pointer; transition: all .15s;
}
.check:hover { border-color: #6366f1; color: #6366f1; }
.todo-card.completed .check {
  background: #6366f1; border-color: #6366f1; color: #fff;
}

.todo-title { flex: 1; font-size: .9rem; }

.delete {
  background: transparent; border: none; color: #71717a;
  font-size: 1.1rem; cursor: pointer; padding: .25rem;
  line-height: 1; transition: color .15s;
}
.delete:hover { color: #ef4444; }

.empty {
  text-align: center; color: #71717a; font-size: .85rem;
  padding: 2rem 0; font-style: italic;
}

.footer {
  margin-top: 1rem; text-align: center;
}
.clear-btn {
  background: transparent; border: 1px solid #333; color: #71717a;
  border-radius: 8px; padding: .375rem .875rem; font-size: .8rem;
  cursor: pointer; transition: all .15s;
}
.clear-btn:hover { color: #ef4444; border-color: #ef4444; }

/* About page */
.about-page .content { line-height: 1.6; }
.about-page h1 { font-size: 1.5rem; font-weight: 700; color: #fff; margin-bottom: .75rem; }
.about-page h2 { font-size: 1.1rem; font-weight: 600; color: #e4e4e7; margin: 1.25rem 0 .5rem; }
.about-page p { color: #a1a1aa; margin-bottom: .5rem; }
.about-page ul { padding-left: 1.25rem; color: #a1a1aa; }
.about-page li { margin-bottom: .375rem; }
.about-page a { color: #6366f1; text-decoration: none; }
.about-page a:hover { text-decoration: underline; }

/* 404 */
.not-found { text-align: center; padding: 3rem 1.5rem; }
.not-found h1 { font-size: 3rem; font-weight: 800; color: #6366f1; margin-bottom: .5rem; }
.not-found p { color: #71717a; margin-bottom: .5rem; }
.not-found a { color: #6366f1; text-decoration: none; }
.not-found a:hover { text-decoration: underline; }
`;
}

// ── GUIDE ──────────────────────────────────────────────────────────

function guide(name) {
  return `# Magnetic Developer Guide

A complete reference for building components, pages, and business logic.

## Architecture Overview

\`\`\`
Developer writes:           Magnetic handles:
  pages/*.tsx        →      Auto-bridge generation
  components/*.tsx   →      Bundling (esbuild)
  server/state.ts    →      Rust V8 server execution
  public/style.css   →      SSR + SSE + action dispatch
\`\`\`

**Key principle**: All state lives on the server. The client is a ~1.5KB
rendering shell. Your TSX runs in V8 on the server — never in the browser.

---

## Pages

Pages are TSX files in \`pages/\`. The filename determines the route:

| File                     | Route       |
|--------------------------|-------------|
| \`pages/IndexPage.tsx\`    | \`/\`         |
| \`pages/AboutPage.tsx\`    | \`/about\`    |
| \`pages/SettingsPage.tsx\` | \`/settings\` |
| \`pages/[id].tsx\`         | \`/:id\`      |
| \`pages/NotFoundPage.tsx\` | \`*\` (404)   |

### Page template

\`\`\`tsx
import { Head, Link } from '@magneticjs/server/jsx-runtime';

export function MyPage(props: any) {
  return (
    <div key="page">
      <Head>
        <title>Page Title</title>
        <meta name="description" content="Page description" />
      </Head>

      <h1 key="heading">{props.someValue}</h1>
      <Link href="/other">Go somewhere</Link>
    </div>
  );
}
\`\`\`

### Rules for pages
- **Export a named function** matching the filename (e.g. \`MyPage\`)
- **\`props\`** = the view model from \`toViewModel()\` in \`server/state.ts\`
- **\`key\`** attributes help the client efficiently patch the DOM — add them to elements that change
- **\`<Head>\`** sets \`<title>\` and \`<meta>\` during SSR
- **\`<Link>\`** does client-side navigation (no full page reload)

---

## Components

Components are TSX files in \`components/\`. They are regular functions that
receive props and return JSX. Import them into pages or other components.

### Example: Button component

\`\`\`tsx
// components/Button.tsx
export function Button(props: {
  label: string;
  action: string;
  variant?: 'primary' | 'danger' | 'ghost';
}) {
  const cls = \\\`btn btn-\\\${props.variant || 'primary'}\\\`;
  return (
    <button class={cls} onClick={props.action} key={\\\`btn-\\\${props.action}\\\`}>
      {props.label}
    </button>
  );
}
\`\`\`

### Example: Card component

\`\`\`tsx
// components/Card.tsx
export function Card(props: { title: string; children?: any }) {
  return (
    <div class="card" key={\\\`card-\\\${props.title}\\\`}>
      <h3 class="card-title">{props.title}</h3>
      <div class="card-body">{props.children}</div>
    </div>
  );
}
\`\`\`

### Example: List component with iteration

\`\`\`tsx
// components/ItemList.tsx
export function ItemList(props: { items: Array<{ id: number; name: string }> }) {
  return (
    <ul class="item-list" key="items">
      {props.items.map(item => (
        <li key={\\\`item-\\\${item.id}\\\`}>
          <span>{item.name}</span>
          <button onClick={\\\`delete_\\\${item.id}\\\`}>Remove</button>
        </li>
      ))}
    </ul>
  );
}
\`\`\`

### Example: Conditional rendering

\`\`\`tsx
// components/StatusBadge.tsx
export function StatusBadge(props: { isOnline: boolean }) {
  return (
    <span class={\\\`badge \\\${props.isOnline ? 'online' : 'offline'}\\\`} key="status">
      {props.isOnline ? 'Online' : 'Offline'}
    </span>
  );
}
\`\`\`

### Example: Form component

\`\`\`tsx
// components/SearchForm.tsx
export function SearchForm() {
  return (
    <form onSubmit="search" key="search-form">
      <input type="text" name="query" placeholder="Search..." />
      <button type="submit">Search</button>
    </form>
  );
}
\`\`\`

When the form is submitted, Magnetic collects all \`<input>\` values by their
\`name\` attribute and sends them as the action payload:
\`{ query: "user typed this" }\`

### Example: Live input (debounced)

\`\`\`tsx
// components/LiveSearch.tsx
export function LiveSearch() {
  return (
    <input
      type="text"
      name="q"
      placeholder="Type to search..."
      onInput="live_search"
      key="live-search"
    />
  );
}
\`\`\`

\`onInput\` is debounced (300ms). Payload: \`{ value: "current input value" }\`

---

## Business Logic (server/state.ts)

All business logic lives in \`server/state.ts\`. This file exports 3 functions:

### 1. \`initialState()\` — starting state

\`\`\`ts
export interface AppState {
  items: Item[];
  searchQuery: string;
  currentUser: string | null;
}

export function initialState(): AppState {
  return {
    items: [],
    searchQuery: '',
    currentUser: null,
  };
}
\`\`\`

### 2. \`reduce(state, action, payload)\` — handles actions

This is a **pure function**. Given the current state, an action name, and a
payload, return the new state. Never mutate — always return a new object.

\`\`\`ts
export function reduce(state: AppState, action: string, payload: any): AppState {
  switch (action) {
    // Simple action (no payload)
    case 'reset':
      return initialState();

    // Action with payload (from form submission)
    case 'add_item': {
      const name = payload?.name?.trim();
      if (!name) return state;
      return {
        ...state,
        items: [...state.items, { id: Date.now(), name, done: false }],
      };
    }

    // Action with payload (from onInput)
    case 'live_search':
      return { ...state, searchQuery: payload?.value || '' };

    // Parameterized actions (action name encodes the ID)
    default: {
      const m = action.match(/^toggle_(\\\\d+)$/);
      if (m) {
        const id = parseInt(m[1], 10);
        return {
          ...state,
          items: state.items.map(i =>
            i.id === id ? { ...i, done: !i.done } : i
          ),
        };
      }
      return state;
    }
  }
}
\`\`\`

### 3. \`toViewModel(state)\` — prepares data for the UI

Transform raw state into the shape your pages and components need.
This is where you compute derived values, filter lists, format strings, etc.

\`\`\`ts
export function toViewModel(state: AppState) {
  const filtered = state.items.filter(i =>
    i.name.toLowerCase().includes(state.searchQuery.toLowerCase())
  );

  return {
    items: filtered.map(i => ({
      ...i,
      statusClass: i.done ? 'done' : '',
      statusIcon: i.done ? '✓' : '○',
    })),
    totalCount: state.items.length,
    doneCount: state.items.filter(i => i.done).length,
    hasSearch: state.searchQuery.length > 0,
  };
}
\`\`\`

The object returned by \`toViewModel()\` is passed as \`props\` to every page component.

---

## Event Reference

Events in Magnetic are **action names** (strings), not JavaScript callbacks.

| JSX Prop       | HTML Behavior                           | Payload                    |
|----------------|-----------------------------------------|----------------------------|
| \`onClick\`      | Click → POST to server                  | \`{}\`                       |
| \`onSubmit\`     | Form submit → collect inputs by name    | \`{ name: value, ... }\`     |
| \`onInput\`      | Keystroke (300ms debounce)              | \`{ value: "current text" }\` |

### How actions flow

\`\`\`
User clicks button (onClick="do_thing")
  → magnetic.js POSTs to /actions/do_thing
  → Rust server calls reduce(state, "do_thing", {})
  → New state → toViewModel() → page re-renders
  → New JSON DOM sent back to client
  → magnetic.js patches the DOM in-place
  → SSE broadcasts update to all connected clients
\`\`\`

### Parameterized actions

Encode IDs in the action name:

\`\`\`tsx
<button onClick={\\\`delete_\\\${item.id}\\\`}>Delete</button>
\`\`\`

Then parse in the reducer:
\`\`\`ts
const m = action.match(/^delete_(\\\\d+)$/);
if (m) {
  const id = parseInt(m[1], 10);
  return { ...state, items: state.items.filter(i => i.id !== id) };
}
\`\`\`

---

## Navigation

Use \`<Link>\` for client-side navigation (no page reload):

\`\`\`tsx
import { Link } from '@magneticjs/server/jsx-runtime';

<Link href="/about">About</Link>
<Link href="/users/42">User Profile</Link>
\`\`\`

Under the hood, \`<Link>\` renders an \`<a>\` with \`onClick="navigate:/about"\`.
The client runtime intercepts this, does \`pushState\`, and requests the new
page from the server.

---

## Keys — Important!

Every element that **changes between renders** should have a \`key\` attribute.
Keys help the client runtime efficiently patch the DOM instead of re-creating it.

\`\`\`tsx
// Good: stable keys on dynamic content
{items.map(item => (
  <div key={\\\`item-\\\${item.id}\\\`}>{item.name}</div>
))}

// Good: keys on sections that update
<h1 key="title">{props.title}</h1>
<p key="count">{props.count} items</p>

// Not needed: static content that never changes
<footer>Made with Magnetic</footer>
\`\`\`

---

## Styling

Put CSS in \`public/style.css\`. It's automatically inlined into the SSR HTML
(no extra network request). Use standard CSS — no build step needed.

---

## Tips

1. **Think server-first**: Your TSX never runs in the browser. No \`useState\`, no \`useEffect\`, no DOM APIs.
2. **State is private**: The client never sees your AppState. Only the JSON DOM is sent.
3. **Keep reducers pure**: No side effects, no \`fetch()\`, no timers. Just state → new state.
4. **View model is your API**: \`toViewModel()\` shapes exactly what the UI sees.
5. **Small components**: Break UI into small components with clear props.
6. **Keys matter**: Add \`key\` to any element whose content or presence changes.
`;
}

// ── README ─────────────────────────────────────────────────────────

function readme(name) {
  return `# ${name}

A server-driven UI app built with [Magnetic](https://github.com/inventhq/magnetic).

## Quick Start

\`\`\`bash
# Start development server (auto-rebuilds on file changes)
magnetic dev

# Or: build + deploy to a Magnetic platform server
magnetic build
magnetic push --server http://your-platform:3003
\`\`\`

Then open http://localhost:3003

## Project Structure

\`\`\`
${name}/
├── pages/              ← Page components (auto-routed by filename)
│   ├── IndexPage.tsx   → /
│   ├── AboutPage.tsx   → /about
│   └── NotFoundPage.tsx→ * (404)
├── components/         ← Shared components
├── server/
│   └── state.ts        ← Business logic (state, reducer, viewModel)
├── public/             ← Static files (CSS, images, client runtime)
└── magnetic.json       ← Project config
\`\`\`

## How It Works

1. **You write** pages (TSX) + business logic (state.ts)
2. **Magnetic CLI** auto-generates the bridge, bundles, and runs the Rust V8 server
3. **The server** renders your TSX to JSON DOM descriptors and pushes updates via SSE
4. **The client** (~1.5KB) is a thin rendering shell — no React, no virtual DOM

All state lives on the server. The client never sees your business logic.
`;
}
