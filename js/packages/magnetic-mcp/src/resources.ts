// resources.ts — MCP resource definitions for Magnetic

import { readFileSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ── Find monorepo root ─────────────────────────────────────────────

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

const MONOREPO_ROOT = findMonorepoRoot(resolve(__dirname, '../../..'));

// ── Resource Definitions ────────────────────────────────────────────

export interface ResourceDef {
  uri: string;
  name: string;
  description: string;
  mimeType: string;
}

export const RESOURCE_DEFINITIONS: ResourceDef[] = [
  {
    uri: 'magnetic://skills/app-development',
    name: 'Magnetic App Development Skill',
    description: 'Complete guide for building Magnetic apps: pages, components, state, routing, events, styling, config, and deployment.',
    mimeType: 'text/markdown',
  },
  {
    uri: 'magnetic://skills/components',
    name: 'Magnetic Components Skill',
    description: 'Guide for building reusable Magnetic UI components: pure functions, props patterns, event handling, keys, styling, and anti-patterns.',
    mimeType: 'text/markdown',
  },
  {
    uri: 'magnetic://skills/css-styling',
    name: 'Magnetic CSS Styling Skill',
    description: 'Guide for styling Magnetic apps with @magneticjs/css: utility classes, design tokens, theme configuration, responsive design.',
    mimeType: 'text/markdown',
  },
  {
    uri: 'magnetic://reference/jsx-runtime',
    name: 'JSX Runtime Source',
    description: 'Source code of the Magnetic JSX runtime — DomNode interface, jsx() function, Link, Head, Fragment, event mapping.',
    mimeType: 'text/typescript',
  },
  {
    uri: 'magnetic://reference/router',
    name: 'Router Source',
    description: 'Source code of the Magnetic router — route matching, params extraction, layouts, guards.',
    mimeType: 'text/typescript',
  },
  {
    uri: 'magnetic://reference/utilities',
    name: 'CSS Utilities Source',
    description: 'Source code of the utility class generator — all 305 utility classes and how they map to CSS.',
    mimeType: 'text/typescript',
  },
];

// ── Resource Handlers ───────────────────────────────────────────────

const SKILL_FILES: Record<string, string> = {
  'magnetic://skills/app-development': 'docs/skills/magnetic-app-development.md',
  'magnetic://skills/components': 'docs/skills/magnetic-components.md',
  'magnetic://skills/css-styling': 'docs/skills/magnetic-css-styling.md',
};

const REFERENCE_FILES: Record<string, string> = {
  'magnetic://reference/jsx-runtime': 'js/packages/magnetic-server/src/jsx-runtime.ts',
  'magnetic://reference/router': 'js/packages/magnetic-server/src/router.ts',
  'magnetic://reference/utilities': 'js/packages/magnetic-css/src/utilities.ts',
};

function resolveFile(relativePath: string): string | null {
  const paths = [
    MONOREPO_ROOT ? join(MONOREPO_ROOT, relativePath) : null,
    join(__dirname, '../../../..', relativePath),
    join(__dirname, '../../../../..', relativePath),
  ].filter(Boolean) as string[];

  for (const p of paths) {
    if (existsSync(p)) return p;
  }
  return null;
}

export function readResource(uri: string): { content: string; mimeType: string } | null {
  // Skills
  if (SKILL_FILES[uri]) {
    const path = resolveFile(SKILL_FILES[uri]);
    if (path) return { content: readFileSync(path, 'utf-8'), mimeType: 'text/markdown' };
    return { content: `Error: Skill file not found for ${uri}`, mimeType: 'text/plain' };
  }

  // Reference source files
  if (REFERENCE_FILES[uri]) {
    const path = resolveFile(REFERENCE_FILES[uri]);
    if (path) return { content: readFileSync(path, 'utf-8'), mimeType: 'text/typescript' };
    return { content: `Error: Reference file not found for ${uri}`, mimeType: 'text/plain' };
  }

  return null;
}
