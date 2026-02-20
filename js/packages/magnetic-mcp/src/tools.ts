// tools.ts — MCP tool definitions and handlers for Magnetic

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { join, resolve, relative, extname, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync, spawn } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ── Helpers ─────────────────────────────────────────────────────────

function findMonorepoRoot(from: string): string | null {
  let dir = from;
  for (let i = 0; i < 10; i++) {
    if (existsSync(join(dir, 'js/packages/magnetic-server'))) return dir;
    const parent = resolve(dir, '..');
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function listFilesRecursive(dir: string, base: string, maxDepth: number = 5, depth: number = 0): string[] {
  if (depth >= maxDepth || !existsSync(dir)) return [];
  const results: string[] = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name.startsWith('.') || entry.name === 'node_modules' || entry.name === 'dist') continue;
    const rel = relative(base, join(dir, entry.name));
    if (entry.isDirectory()) {
      results.push(rel + '/');
      results.push(...listFilesRecursive(join(dir, entry.name), base, maxDepth, depth + 1));
    } else {
      results.push(rel);
    }
  }
  return results;
}

function safeReadFile(path: string): string {
  if (!existsSync(path)) return `Error: File not found: ${path}`;
  return readFileSync(path, 'utf-8');
}

function runCli(args: string[], cwd: string): string {
  try {
    const cliPath = join(__dirname, '..', '..', 'magnetic-cli', 'src', 'cli.ts');
    const result = execSync(`npx tsx ${cliPath} ${args.join(' ')}`, {
      cwd,
      encoding: 'utf-8',
      timeout: 60000,
      env: { ...process.env, NODE_NO_WARNINGS: '1' },
    });
    return result;
  } catch (err: any) {
    return `Error: ${err.stderr || err.message}`;
  }
}

// Allow the MCP server to know where the monorepo is
const MONOREPO_ROOT = findMonorepoRoot(resolve(__dirname, '../../..'));

// ── Tool Definitions ────────────────────────────────────────────────

export interface ToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, any>;
}

export const TOOL_DEFINITIONS: ToolDef[] = [
  {
    name: 'magnetic_list_files',
    description: 'List all files in a Magnetic app directory. Returns the project structure.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
      },
      required: ['appDir'],
    },
  },
  {
    name: 'magnetic_read_file',
    description: 'Read a file from a Magnetic app. Use relative paths from the app root (e.g. "pages/IndexPage.tsx", "server/state.ts", "magnetic.json").',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
        filePath: { type: 'string', description: 'Relative path to the file within the app' },
      },
      required: ['appDir', 'filePath'],
    },
  },
  {
    name: 'magnetic_write_file',
    description: 'Write or update a file in a Magnetic app. Creates parent directories if needed. Use for creating/editing pages, components, state, config, and styles.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
        filePath: { type: 'string', description: 'Relative path to the file within the app' },
        content: { type: 'string', description: 'File content to write' },
      },
      required: ['appDir', 'filePath', 'content'],
    },
  },
  {
    name: 'magnetic_scaffold',
    description: 'Scaffold a new Magnetic app with the standard directory structure. Creates pages/, components/, server/, public/, magnetic.json, design.json, and tsconfig.json.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path where the new app should be created' },
        name: { type: 'string', description: 'App name (used in magnetic.json and as deployment name)' },
        template: { type: 'string', enum: ['blank', 'todo'], description: 'Template: "blank" for empty app, "todo" for a todo list example. Default: blank' },
      },
      required: ['appDir', 'name'],
    },
  },
  {
    name: 'magnetic_build',
    description: 'Build a Magnetic app bundle. Scans pages, generates the V8 bridge, and bundles with esbuild. Output goes to dist/app.js.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
      },
      required: ['appDir'],
    },
  },
  {
    name: 'magnetic_push',
    description: 'Build and deploy a Magnetic app to the platform. Requires the app to have a "name" and "server" in magnetic.json or passed as arguments.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
        name: { type: 'string', description: 'App name (overrides magnetic.json)' },
        server: { type: 'string', description: 'Platform server URL (overrides magnetic.json)' },
      },
      required: ['appDir'],
    },
  },
  {
    name: 'magnetic_openapi',
    description: 'Detect OpenAPI/Swagger specs from the data sources configured in magnetic.json. If found, generates TypeScript interfaces in server/api-types.ts.',
    inputSchema: {
      type: 'object',
      properties: {
        appDir: { type: 'string', description: 'Absolute path to the Magnetic app directory' },
      },
      required: ['appDir'],
    },
  },
  {
    name: 'magnetic_read_skill',
    description: 'Read a Magnetic skill document. Skills teach AI agents how to build Magnetic apps. Available skills: "app-development", "components", "css-styling".',
    inputSchema: {
      type: 'object',
      properties: {
        skill: { type: 'string', enum: ['app-development', 'components', 'css-styling'], description: 'Skill name to read' },
      },
      required: ['skill'],
    },
  },
];

// ── Tool Handlers ───────────────────────────────────────────────────

export async function handleTool(name: string, args: Record<string, any>): Promise<string> {
  switch (name) {
    case 'magnetic_list_files': {
      const { appDir } = args;
      if (!existsSync(appDir)) return `Error: Directory not found: ${appDir}`;
      const files = listFilesRecursive(appDir, appDir);
      return files.join('\n') || '(empty directory)';
    }

    case 'magnetic_read_file': {
      const { appDir, filePath } = args;
      const fullPath = join(appDir, filePath);
      return safeReadFile(fullPath);
    }

    case 'magnetic_write_file': {
      const { appDir, filePath, content } = args;
      const fullPath = join(appDir, filePath);
      const dir = join(fullPath, '..');
      mkdirSync(dir, { recursive: true });
      writeFileSync(fullPath, content);
      return `Written: ${filePath} (${content.length} bytes)`;
    }

    case 'magnetic_scaffold': {
      const { appDir, name, template } = args;
      return scaffoldApp(appDir, name, template || 'blank');
    }

    case 'magnetic_build': {
      const { appDir } = args;
      const cliArgs = ['build', '--dir', appDir];
      return runCli(cliArgs, appDir);
    }

    case 'magnetic_push': {
      const { appDir, name: pushName, server } = args;
      const cliArgs = ['push', '--dir', appDir];
      if (pushName) cliArgs.push('--name', pushName);
      if (server) cliArgs.push('--server', server);
      return runCli(cliArgs, appDir);
    }

    case 'magnetic_openapi': {
      const { appDir } = args;
      const cliArgs = ['openapi', '--dir', appDir];
      return runCli(cliArgs, appDir);
    }

    case 'magnetic_read_skill': {
      const { skill } = args;
      const skillMap: Record<string, string> = {
        'app-development': 'magnetic-app-development.md',
        'components': 'magnetic-components.md',
        'css-styling': 'magnetic-css-styling.md',
      };
      const filename = skillMap[skill];
      if (!filename) return `Error: Unknown skill "${skill}". Available: ${Object.keys(skillMap).join(', ')}`;

      // Try monorepo path first, then relative to this package
      const paths = [
        MONOREPO_ROOT ? join(MONOREPO_ROOT, 'docs/skills', filename) : null,
        join(__dirname, '../../../../docs/skills', filename),
        join(__dirname, '../../../docs/skills', filename),
      ].filter(Boolean) as string[];

      for (const p of paths) {
        if (existsSync(p)) return readFileSync(p, 'utf-8');
      }
      return `Error: Skill file "${filename}" not found. Searched: ${paths.join(', ')}`;
    }

    default:
      return `Error: Unknown tool "${name}"`;
  }
}

// ── Scaffold ────────────────────────────────────────────────────────

function scaffoldApp(appDir: string, name: string, template: string): string {
  mkdirSync(join(appDir, 'pages'), { recursive: true });
  mkdirSync(join(appDir, 'components'), { recursive: true });
  mkdirSync(join(appDir, 'server'), { recursive: true });
  mkdirSync(join(appDir, 'public'), { recursive: true });

  // magnetic.json
  writeFileSync(join(appDir, 'magnetic.json'), JSON.stringify({
    name,
    server: 'https://api.fujs.dev',
  }, null, 2) + '\n');

  // design.json (dark theme)
  writeFileSync(join(appDir, 'design.json'), JSON.stringify({
    css: 'pages',
    theme: {
      colors: {
        primary: '#6366f1',
        'primary-hover': '#4f46e5',
        danger: '#ef4444',
        surface: '#0a0a0a',
        raised: '#141414',
        sunken: '#0d0d0d',
        text: '#e4e4e7',
        heading: '#ffffff',
        muted: '#71717a',
        subtle: '#a1a1aa',
        border: '#252525',
        'border-hover': '#333333',
      },
      spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem', '3xl': '4rem' },
      radius: { sm: '0.375rem', md: '0.625rem', lg: '1rem', full: '9999px' },
      typography: {
        sans: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        mono: 'JetBrains Mono, ui-monospace, monospace',
        sizes: { xs: '0.75rem', sm: '0.85rem', base: '0.9rem', lg: '1.1rem', xl: '1.25rem', '2xl': '1.5rem', '3xl': '1.75rem', '4xl': '2.25rem', '5xl': '3rem' },
        leading: { tight: '1.25', normal: '1.5', relaxed: '1.6' },
      },
      shadows: {
        sm: '0 1px 2px rgb(0 0 0 / 0.05)',
        md: '0 4px 6px rgb(0 0 0 / 0.07), 0 2px 4px rgb(0 0 0 / 0.06)',
        lg: '0 8px 32px rgba(0, 0, 0, 0.5)',
      },
      breakpoints: { sm: '640px', md: '768px', lg: '1024px', xl: '1280px' },
    },
  }, null, 2) + '\n');

  // tsconfig.json
  writeFileSync(join(appDir, 'tsconfig.json'), JSON.stringify({
    compilerOptions: {
      target: 'ES2020',
      module: 'ESNext',
      moduleResolution: 'bundler',
      jsx: 'react-jsx',
      jsxImportSource: '@magneticjs/server',
      strict: true,
      skipLibCheck: true,
      noEmit: true,
      paths: { '@magneticjs/server/*': ['./node_modules/@magneticjs/server/src/*'] },
    },
    include: ['pages', 'components', 'server'],
  }, null, 2) + '\n');

  // public/style.css
  writeFileSync(join(appDir, 'public/style.css'), `/* ${name} — interactive states (layout + theme from @magneticjs/css) */\n`);

  if (template === 'todo') {
    return scaffoldTodo(appDir, name);
  }

  // Blank template
  // server/state.ts
  writeFileSync(join(appDir, 'server/state.ts'), `export interface AppState {
  // Add your state fields here
}

export function initialState(): AppState {
  return {};
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  return state;
}

export function toViewModel(state: AppState) {
  return {};
}
`);

  // pages/IndexPage.tsx
  writeFileSync(join(appDir, 'pages/IndexPage.tsx'), `import { Head } from '@magneticjs/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div class="stack items-center justify-center min-h-screen gap-lg" key="wrapper">
      <Head><title>${name}</title></Head>
      <h1 class="text-4xl bold fg-heading" key="title">${name}</h1>
      <p class="fg-muted text-lg" key="subtitle">Built with Magnetic</p>
    </div>
  );
}
`);

  // components/types.ts
  writeFileSync(join(appDir, 'components/types.ts'), `// Shared types for ${name} components\n\nexport interface AppProps {\n  // Add view model props here\n}\n`);

  return `Scaffolded "${name}" (blank template) at ${appDir}\nFiles: magnetic.json, design.json, tsconfig.json, pages/IndexPage.tsx, server/state.ts, components/types.ts, public/style.css`;
}

function scaffoldTodo(appDir: string, name: string): string {
  // server/state.ts
  writeFileSync(join(appDir, 'server/state.ts'), `export interface Task { id: number; title: string; completed: boolean; }
export interface AppState { tasks: Task[]; filter: 'all' | 'active' | 'done'; nextId: number; }

export function initialState(): AppState {
  return { tasks: [], filter: 'all', nextId: 1 };
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  if (action === 'add_task' && payload.title?.trim()) {
    return { ...state, tasks: [...state.tasks, { id: state.nextId, title: payload.title.trim(), completed: false }], nextId: state.nextId + 1 };
  }
  if (action.startsWith('toggle_')) {
    const id = parseInt(action.split('_')[1]);
    return { ...state, tasks: state.tasks.map(t => t.id === id ? { ...t, completed: !t.completed } : t) };
  }
  if (action.startsWith('delete_')) {
    const id = parseInt(action.split('_')[1]);
    return { ...state, tasks: state.tasks.filter(t => t.id !== id) };
  }
  if (action.startsWith('filter_')) {
    return { ...state, filter: action.split('_')[1] as AppState['filter'] };
  }
  return state;
}

export function toViewModel(state: AppState) {
  const visible = state.tasks
    .filter(t => state.filter === 'active' ? !t.completed : state.filter === 'done' ? t.completed : true)
    .map(t => ({
      ...t,
      cardClass: t.completed ? 'opacity-50' : '',
      titleClass: t.completed ? 'line-through fg-muted' : '',
      checkClass: t.completed ? 'check-done' : '',
      checkmark: t.completed ? '✓' : '○',
    }));

  const activeBtn = (f: string) => state.filter === f ? 'bg-primary fg-heading' : 'bg-raised fg-muted';

  return {
    visibleTasks: visible,
    taskCount: \`\${state.tasks.filter(t => !t.completed).length} active\`,
    filterAllClass: activeBtn('all'),
    filterActiveClass: activeBtn('active'),
    filterDoneClass: activeBtn('done'),
    isEmpty: visible.length === 0,
    emptyMessage: state.filter === 'active' ? 'All done!' : 'No tasks yet.',
  };
}
`);

  // pages/IndexPage.tsx
  writeFileSync(join(appDir, 'pages/IndexPage.tsx'), `import { Head } from '@magneticjs/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <Head><title>${name}</title></Head>
      <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg" style="max-width:520px" key="board">
        <h1 class="text-2xl bold fg-heading text-center" key="title">${name}</h1>
        <p class="fg-muted text-sm text-center" key="count">{props.taskCount}</p>

        <form class="row gap-sm" onSubmit="add_task" key="add-form">
          <input type="text" name="title" placeholder="Add a task..." autocomplete="off"
            class="grow bg-sunken border round-md px-md py-sm fg-text text-base" key="add-input" />
          <button type="submit" class="bg-primary fg-heading round-md px-lg py-sm semibold cursor-pointer" key="add-btn">Add</button>
        </form>

        <div class="row gap-xs justify-center" key="filters">
          <button onClick="filter_all" class={\`border round-sm px-sm py-xs text-sm cursor-pointer \${props.filterAllClass}\`} key="f-all">All</button>
          <button onClick="filter_active" class={\`border round-sm px-sm py-xs text-sm cursor-pointer \${props.filterActiveClass}\`} key="f-active">Active</button>
          <button onClick="filter_done" class={\`border round-sm px-sm py-xs text-sm cursor-pointer \${props.filterDoneClass}\`} key="f-done">Done</button>
        </div>

        <div class="stack gap-sm" key="task-list">
          {props.visibleTasks.map((task: any) => (
            <div key={\`task-\${task.id}\`} class={\`row items-center gap-sm bg-sunken border round-md px-md py-sm \${task.cardClass}\`}>
              <button onClick={\`toggle_\${task.id}\`} class={\`center shrink-0 fg-muted text-sm cursor-pointer \${task.checkClass}\`} key={\`chk-\${task.id}\`}>{task.checkmark}</button>
              <span class={\`grow text-base \${task.titleClass}\`} key={\`tt-\${task.id}\`}>{task.title}</span>
              <button onClick={\`delete_\${task.id}\`} class="fg-muted text-lg cursor-pointer p-xs" key={\`del-\${task.id}\`}>×</button>
            </div>
          ))}
        </div>

        {props.isEmpty && <p class="fg-muted text-sm italic text-center" key="empty">{props.emptyMessage}</p>}
      </div>
    </div>
  );
}
`);

  // components/types.ts
  writeFileSync(join(appDir, 'components/types.ts'), `export interface Task { id: number; title: string; completed: boolean; }

export interface TaskView extends Task {
  cardClass: string;
  titleClass: string;
  checkClass: string;
  checkmark: string;
}

export interface AppProps {
  taskCount: string;
  visibleTasks: TaskView[];
  filterAllClass: string;
  filterActiveClass: string;
  filterDoneClass: string;
  isEmpty: boolean;
  emptyMessage: string;
}
`);

  return `Scaffolded "${name}" (todo template) at ${appDir}\nFiles: magnetic.json, design.json, tsconfig.json, pages/IndexPage.tsx, server/state.ts, components/types.ts, public/style.css`;
}
