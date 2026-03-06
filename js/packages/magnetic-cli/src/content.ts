// content.ts — Content pipeline for Magnetic apps
// Reads content/*.md files, parses frontmatter, converts markdown → HTML
// Injects content map into the V8 bundle at build time

import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, relative, extname, basename } from 'node:path';
import { marked } from 'marked';

export interface ContentEntry {
  /** URL slug derived from filename/path: "getting-started" or "blog/hello-world" */
  slug: string;
  /** Parsed frontmatter metadata */
  meta: Record<string, any>;
  /** Rendered HTML from markdown body */
  html: string;
}

export interface ContentMap {
  [slug: string]: { meta: Record<string, any>; html: string };
}

export interface ContentIndex {
  [slug: string]: { meta: Record<string, any> };
}

/**
 * Parse YAML-style frontmatter from markdown content.
 * Supports: strings, numbers, booleans, arrays (bracket syntax), dates.
 */
function parseFrontmatter(raw: string): { meta: Record<string, any>; body: string } {
  const match = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);
  if (!match) return { meta: {}, body: raw };

  const yamlBlock = match[1];
  const body = match[2];
  const meta: Record<string, any> = {};

  for (const line of yamlBlock.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    const colonIdx = trimmed.indexOf(':');
    if (colonIdx === -1) continue;

    const key = trimmed.slice(0, colonIdx).trim();
    let value: any = trimmed.slice(colonIdx + 1).trim();

    // Remove surrounding quotes
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    // Array: [item1, item2]
    else if (value.startsWith('[') && value.endsWith(']')) {
      value = value.slice(1, -1).split(',').map((s: string) => {
        s = s.trim();
        if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) {
          return s.slice(1, -1);
        }
        return s;
      });
    }
    // Boolean
    else if (value === 'true') value = true;
    else if (value === 'false') value = false;
    // Number
    else if (/^-?\d+(\.\d+)?$/.test(value)) value = Number(value);

    meta[key] = value;
  }

  return { meta, body };
}

/**
 * Scan content/ directory and convert all .md files to HTML.
 * Returns a content map keyed by slug.
 */
export function buildContentMap(appDir: string): ContentMap | null {
  const contentDir = join(appDir, 'content');
  if (!existsSync(contentDir)) return null;

  const entries = scanContentDir(contentDir, '');
  if (entries.length === 0) return null;

  // Configure marked for code blocks with language classes
  marked.setOptions({
    gfm: true,
    breaks: false,
  });

  const map: ContentMap = {};

  for (const entry of entries) {
    const raw = readFileSync(join(contentDir, entry.relativePath), 'utf-8');
    const { meta, body } = parseFrontmatter(raw);
    const html = marked.parse(body) as string;

    map[entry.slug] = { meta, html };
  }

  return map;
}

interface ContentFile {
  relativePath: string;
  slug: string;
}

export function scanContentDir(dir: string, prefix: string): ContentFile[] {
  const results: ContentFile[] = [];
  const entries = readdirSync(dir).sort();

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      const subPrefix = prefix ? prefix + '/' + entry : entry;
      results.push(...scanContentDir(fullPath, subPrefix));
      continue;
    }

    if (extname(entry) !== '.md') continue;

    const nameNoExt = basename(entry, '.md');
    const slug = prefix ? prefix + '/' + nameNoExt : nameNoExt;
    const relativePath = prefix ? prefix + '/' + entry : entry;

    results.push({ relativePath, slug });
  }

  return results;
}

/**
 * Generate a JS code string that sets globalThis.__magnetic_content.
 * This makes content accessible via @magneticjs/server/content imports.
 */
export function generateContentInjection(contentMap: ContentMap): string {
  const lines: string[] = [];
  lines.push('// ── Content Pipeline (build-time markdown → HTML) ────────────');
  lines.push(`globalThis.__magnetic_content = ${JSON.stringify(contentMap)};`);
  return lines.join('\n');
}

/**
 * Scan content/ directory and extract frontmatter metadata only (no HTML conversion).
 * Returns a lightweight index suitable for listContent() without loading full content.
 */
export function buildContentIndex(appDir: string): ContentIndex | null {
  const contentDir = join(appDir, 'content');
  if (!existsSync(contentDir)) return null;

  const entries = scanContentDir(contentDir, '');
  if (entries.length === 0) return null;

  const index: ContentIndex = {};
  for (const entry of entries) {
    const raw = readFileSync(join(contentDir, entry.relativePath), 'utf-8');
    const { meta } = parseFrontmatter(raw);
    index[entry.slug] = { meta };
  }
  return index;
}

/**
 * Generate JS code for on-disk content mode.
 * Injects a lightweight index (metadata only) and a content directory path.
 * The runtime calls __magnetic_content_load(slug) to load individual pages on demand.
 */
export function generateContentDiskInjection(contentIndex: ContentIndex, contentDir: string): string {
  const lines: string[] = [];
  lines.push('// ── Content Pipeline (on-disk, loaded on demand) ────────────');
  lines.push(`globalThis.__magnetic_content_index = ${JSON.stringify(contentIndex)};`);
  lines.push(`globalThis.__magnetic_content_dir = ${JSON.stringify(contentDir)};`);
  return lines.join('\n');
}

/** Exported for use by prerender.ts */
export { parseFrontmatter };
