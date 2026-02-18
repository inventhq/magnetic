// @magnetic/server — Static Asset Pipeline
// Content-hashed filenames, cache headers, asset manifest

import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, readdirSync, existsSync, mkdirSync, copyFileSync } from 'node:fs';
import { join, basename, extname } from 'node:path';

// ── Asset manifest ──────────────────────────────────────────────────

export interface AssetManifest {
  /** Maps original filename → hashed filename */
  files: Record<string, string>;
  /** Reverse map: hashed filename → original filename */
  reverse: Record<string, string>;
}

/**
 * Builds a content-hashed asset manifest from a source directory.
 * Copies files to outDir with hashed names.
 *
 * e.g. style.css → style.a1b2c3d4.css
 */
export function buildAssets(opts: {
  /** Source directory with original files */
  srcDir: string;
  /** Output directory for hashed files */
  outDir: string;
  /** File extensions to hash (default: css, js, wasm) */
  extensions?: string[];
  /** Files to skip hashing (served as-is) */
  passthrough?: string[];
}): AssetManifest {
  const {
    srcDir,
    outDir,
    extensions = ['.css', '.js', '.wasm'],
    passthrough = [],
  } = opts;

  if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

  const manifest: AssetManifest = { files: {}, reverse: {} };

  if (!existsSync(srcDir)) return manifest;

  const entries = readdirSync(srcDir);

  for (const entry of entries) {
    const ext = extname(entry);
    const srcPath = join(srcDir, entry);

    // Passthrough files — copy without hashing
    if (passthrough.includes(entry)) {
      copyFileSync(srcPath, join(outDir, entry));
      manifest.files[entry] = entry;
      manifest.reverse[entry] = entry;
      continue;
    }

    // Only hash configured extensions
    if (!extensions.includes(ext)) {
      copyFileSync(srcPath, join(outDir, entry));
      manifest.files[entry] = entry;
      manifest.reverse[entry] = entry;
      continue;
    }

    // Read file and compute content hash
    const content = readFileSync(srcPath);
    const hash = createHash('md5').update(content).digest('hex').slice(0, 8);
    const name = basename(entry, ext);
    const hashedName = `${name}.${hash}${ext}`;

    copyFileSync(srcPath, join(outDir, hashedName));
    manifest.files[entry] = hashedName;
    manifest.reverse[hashedName] = entry;
  }

  return manifest;
}

/**
 * Saves manifest to a JSON file for use at runtime.
 */
export function saveManifest(manifest: AssetManifest, filePath: string): void {
  writeFileSync(filePath, JSON.stringify(manifest, null, 2));
}

/**
 * Loads manifest from a JSON file.
 */
export function loadManifest(filePath: string): AssetManifest {
  if (!existsSync(filePath)) return { files: {}, reverse: {} };
  return JSON.parse(readFileSync(filePath, 'utf-8'));
}

// ── Asset URL resolver ──────────────────────────────────────────────

/**
 * Creates an asset resolver that maps original filenames to hashed URLs.
 *
 * Usage:
 * ```ts
 * const asset = createAssetResolver(manifest, '/static');
 * asset('style.css')  // → '/static/style.a1b2c3d4.css'
 * ```
 */
export function createAssetResolver(
  manifest: AssetManifest,
  prefix: string = '',
): (filename: string) => string {
  return (filename: string) => {
    const hashed = manifest.files[filename];
    return prefix + '/' + (hashed || filename);
  };
}

// ── Serve static with cache headers ─────────────────────────────────

export interface StaticFileResult {
  found: boolean;
  content?: Buffer;
  contentType?: string;
  headers?: Record<string, string>;
}

const MIME: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
};

/**
 * Serves a static file with appropriate cache headers.
 * Content-hashed files get immutable cache (1 year).
 * Non-hashed files get short cache (5 min) with revalidation.
 */
export function serveStatic(
  dir: string,
  urlPath: string,
  manifest?: AssetManifest,
): StaticFileResult {
  // Strip leading slash
  const filename = urlPath.replace(/^\//, '');
  const filePath = join(dir, filename);

  // Security: prevent path traversal
  if (!filePath.startsWith(dir)) {
    return { found: false };
  }

  if (!existsSync(filePath)) {
    return { found: false };
  }

  const ext = extname(filename);
  const contentType = MIME[ext] || 'application/octet-stream';
  const content = readFileSync(filePath);

  // Determine cache strategy
  const isHashed = manifest?.reverse[filename] != null
    && manifest.reverse[filename] !== filename;

  const headers: Record<string, string> = {
    'Content-Type': contentType,
    'Content-Length': String(content.length),
  };

  if (isHashed) {
    // Immutable — content-hashed, safe to cache forever
    headers['Cache-Control'] = 'public, max-age=31536000, immutable';
  } else {
    // Short cache with revalidation
    headers['Cache-Control'] = 'public, max-age=300, must-revalidate';
  }

  return { found: true, content, contentType, headers };
}
