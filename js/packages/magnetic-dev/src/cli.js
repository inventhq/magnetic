#!/usr/bin/env node
// @magnetic/dev — Development CLI
// Watches source files, rebuilds with esbuild, restarts the server automatically.
// Client auto-reconnects via SSE (EventSource default behavior).
//
// Usage:
//   magnetic-dev --entry apps/task-board/server/index.tsx --port 3003
//   magnetic-dev --entry apps/task-board/plugin.ts --mode plugin

import { spawn } from 'node:child_process';
import { watch } from 'node:fs';
import { resolve, dirname, basename } from 'node:path';
import { existsSync } from 'node:fs';

// ── Parse args ──────────────────────────────────────────────────────

const args = process.argv.slice(2);
function arg(name, fallback) {
  const i = args.indexOf('--' + name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const hasFlag = (name) => args.includes('--' + name);

const entry = arg('entry', null);
const port = arg('port', '3003');
const mode = arg('mode', 'server'); // 'server' or 'plugin'
const projectRoot = process.cwd();

if (!entry) {
  console.error('Usage: magnetic-dev --entry <file.tsx> [--port 3003] [--mode server|plugin]');
  process.exit(1);
}

const entryPath = resolve(projectRoot, entry);
const entryDir = dirname(entryPath);
const outFile = mode === 'plugin'
  ? resolve(entryDir, 'dist/plugin.js')
  : resolve(entryDir, 'dist/server.mjs');

// ── Resolve @magnetic/server package ────────────────────────────────

function findMagneticServer() {
  // Walk up from project root looking for js/packages/magnetic-server/src
  let dir = projectRoot;
  for (let i = 0; i < 10; i++) {
    const candidate = resolve(dir, 'js/packages/magnetic-server/src');
    if (existsSync(candidate)) return candidate;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

const magneticServerPath = findMagneticServer();
if (!magneticServerPath) {
  console.error('[magnetic-dev] Could not find @magnetic/server package');
  process.exit(1);
}

// ── esbuild command ─────────────────────────────────────────────────

const esbuildArgs = [
  'esbuild', entry,
  '--bundle',
  '--format=esm',
  `--outfile=${outFile}`,
  '--jsx=automatic',
  '--jsx-import-source=@magnetic/server',
  `--alias:@magnetic/server=${magneticServerPath}`,
];

if (mode === 'server') {
  esbuildArgs.push('--platform=node', '--packages=external');
}

// ── State ───────────────────────────────────────────────────────────

let serverProc = null;
let buildTimer = null;
let building = false;
const DEBOUNCE_MS = 200;

// ── Build ───────────────────────────────────────────────────────────

function build() {
  return new Promise((res, rej) => {
    building = true;
    const start = Date.now();
    const proc = spawn('npx', esbuildArgs, {
      cwd: projectRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: true,
    });
    let stderr = '';
    proc.stderr.on('data', (d) => { stderr += d; });
    proc.on('close', (code) => {
      building = false;
      if (code === 0) {
        console.log(`\x1b[32m✓\x1b[0m Built in ${Date.now() - start}ms → ${basename(outFile)}`);
        res();
      } else {
        console.error(`\x1b[31m✗\x1b[0m Build failed:\n${stderr}`);
        rej(new Error('Build failed'));
      }
    });
  });
}

// ── Server management (mode=server only) ────────────────────────────

function startServer() {
  if (mode !== 'server') return;

  serverProc = spawn('node', [outFile, port], {
    cwd: projectRoot,
    stdio: 'inherit',
  });

  serverProc.on('exit', (code) => {
    if (code !== null && code !== 0) {
      console.error(`\x1b[33m⚠\x1b[0m Server exited with code ${code}`);
    }
    serverProc = null;
  });
}

function stopServer() {
  if (serverProc) {
    serverProc.kill('SIGTERM');
    serverProc = null;
  }
}

function restartServer() {
  stopServer();
  startServer();
}

// ── File watcher ────────────────────────────────────────────────────

function watchDir(dir) {
  try {
    watch(dir, { recursive: true }, (eventType, filename) => {
      if (!filename) return;
      // Skip dist/, node_modules, dotfiles
      if (filename.includes('dist/') || filename.includes('node_modules') || filename.startsWith('.')) return;
      // Only watch ts/tsx/js/jsx/css/html
      if (!/\.(tsx?|jsx?|css|html)$/.test(filename)) return;

      // Debounce rapid file changes
      clearTimeout(buildTimer);
      buildTimer = setTimeout(async () => {
        console.log(`\x1b[36m↻\x1b[0m ${filename} changed`);
        try {
          await build();
          if (mode === 'server') restartServer();
          else console.log(`\x1b[32m✓\x1b[0m Plugin rebuilt → ${basename(outFile)}`);
        } catch { /* build error already logged */ }
      }, DEBOUNCE_MS);
    });
  } catch (e) {
    console.error(`[magnetic-dev] Cannot watch ${dir}:`, e.message);
  }
}

// ── Main ────────────────────────────────────────────────────────────

async function main() {
  console.log(`\x1b[1m@magnetic/dev\x1b[0m`);
  console.log(`  entry:  ${entry}`);
  console.log(`  mode:   ${mode}`);
  if (mode === 'server') console.log(`  port:   ${port}`);
  console.log(`  output: ${outFile}`);
  console.log('');

  try {
    await build();
  } catch {
    console.error('Initial build failed. Watching for changes...');
  }

  if (mode === 'server') {
    startServer();
  }

  // Watch the entry directory and the magnetic-server package
  watchDir(entryDir);
  if (magneticServerPath !== entryDir) {
    watchDir(magneticServerPath);
  }

  console.log('\n\x1b[2mWatching for changes... (Ctrl+C to stop)\x1b[0m\n');

  // Graceful shutdown
  process.on('SIGINT', () => {
    console.log('\n\x1b[2mShutting down...\x1b[0m');
    stopServer();
    process.exit(0);
  });
  process.on('SIGTERM', () => {
    stopServer();
    process.exit(0);
  });
}

main();
