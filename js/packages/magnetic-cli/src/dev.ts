// dev.ts — Dev mode: watch pages/, auto-rebuild, start V8 server
// Gives developers instant feedback as they edit TSX pages

import { watch } from 'node:fs';
import { join, resolve } from 'node:path';
import { existsSync } from 'node:fs';
import { spawn, ChildProcess } from 'node:child_process';
import { scanApp, generateBridge } from './generator.ts';
import { bundleApp } from './bundler.ts';

export interface DevOptions {
  /** Absolute path to the app directory */
  appDir: string;
  /** Port for the V8 server (default: 3003) */
  port?: number;
  /** Path to the magnetic-v8-server binary */
  serverBin?: string;
  /** Monorepo root (for resolving @magneticjs/server) */
  monorepoRoot?: string;
}

/**
 * Start dev mode:
 * 1. Scan pages/ and generate bridge
 * 2. Bundle with esbuild
 * 3. Start the Rust V8 server
 * 4. Watch for file changes → rebuild → restart server
 */
export async function startDev(opts: DevOptions): Promise<void> {
  const {
    appDir,
    port = 3003,
    monorepoRoot,
  } = opts;

  const staticDir = join(appDir, 'public');
  const outDir = join(appDir, 'dist');

  // Find the V8 server binary
  const serverBin = opts.serverBin || findServerBinary(monorepoRoot || appDir);
  if (!serverBin) {
    console.error('[magnetic] Cannot find magnetic-v8-server binary.');
    console.error('  Build it with: cargo build --release -p magnetic-v8-server');
    process.exit(1);
  }

  let serverProcess: ChildProcess | null = null;

  async function rebuild(): Promise<string | null> {
    const start = Date.now();
    try {
      const scan = scanApp(appDir, monorepoRoot);
      console.log(`[magnetic] Scanned ${scan.pages.length} pages, state: ${scan.statePath || 'none'}`);

      for (const page of scan.pages) {
        console.log(`  ${page.routePath} → ${page.filePath} (${page.importName})`);
      }

      const bridgeCode = generateBridge(scan);
      const result = await bundleApp({
        appDir,
        bridgeCode,
        outDir,
        monorepoRoot,
      });

      const ms = Date.now() - start;
      const kb = (result.sizeBytes / 1024).toFixed(1);
      console.log(`[magnetic] Built ${result.outPath} (${kb}KB) in ${ms}ms`);
      return result.outPath;
    } catch (err: any) {
      console.error(`[magnetic] Build failed: ${err.message}`);
      return null;
    }
  }

  function startServer(bundlePath: string): ChildProcess {
    const args = [
      '--bundle', bundlePath,
      '--port', String(port),
      '--static', staticDir,
    ];

    console.log(`[magnetic] Starting V8 server on :${port}`);
    const proc = spawn(serverBin!, args, {
      stdio: ['ignore', 'inherit', 'inherit'],
    });

    proc.on('exit', (code) => {
      if (code !== null && code !== 0) {
        console.error(`[magnetic] Server exited with code ${code}`);
      }
    });

    return proc;
  }

  function stopServer() {
    if (serverProcess) {
      serverProcess.kill('SIGTERM');
      serverProcess = null;
    }
  }

  // Initial build + start
  const bundlePath = await rebuild();
  if (!bundlePath) {
    console.error('[magnetic] Initial build failed. Fix errors and save to retry.');
  } else {
    serverProcess = startServer(bundlePath);
  }

  // Watch for changes
  const watchDirs = [join(appDir, 'pages'), join(appDir, 'components')];
  const watchFiles = ['state.ts', 'state.tsx', 'server/state.ts', 'server/state.tsx']
    .map(f => join(appDir, f));

  let rebuildTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleRebuild() {
    if (rebuildTimer) clearTimeout(rebuildTimer);
    rebuildTimer = setTimeout(async () => {
      console.log('\n[magnetic] Change detected, rebuilding...');
      stopServer();
      const path = await rebuild();
      if (path) {
        serverProcess = startServer(path);
      }
    }, 200); // Debounce 200ms
  }

  for (const dir of watchDirs) {
    if (existsSync(dir)) {
      watch(dir, { recursive: true }, (event, filename) => {
        if (filename && /\.(tsx?|jsx?|css)$/.test(filename)) {
          scheduleRebuild();
        }
      });
      console.log(`[magnetic] Watching ${dir}`);
    }
  }

  for (const file of watchFiles) {
    if (existsSync(file)) {
      watch(file, () => scheduleRebuild());
      console.log(`[magnetic] Watching ${file}`);
    }
  }

  // Handle cleanup
  process.on('SIGINT', () => {
    console.log('\n[magnetic] Shutting down...');
    stopServer();
    process.exit(0);
  });

  process.on('SIGTERM', () => {
    stopServer();
    process.exit(0);
  });

  console.log(`[magnetic] Dev mode ready. Edit pages/ and save to rebuild.`);
  console.log(`[magnetic] http://localhost:${port}\n`);
}

/**
 * Look for the magnetic-v8-server binary in common locations.
 */
function findServerBinary(searchRoot: string): string | null {
  // __dirname equivalent for the CLI package
  const cliPkgBin = join(import.meta.dirname || __dirname, '..', 'bin', 'magnetic-v8-server');

  const candidates = [
    // npm-installed binary (from postinstall)
    cliPkgBin,
    // Monorepo development paths
    join(searchRoot, 'rs/crates/magnetic-v8-server/target/debug/magnetic-v8-server'),
    join(searchRoot, 'rs/crates/magnetic-v8-server/target/release/magnetic-v8-server'),
    join(searchRoot, 'target/debug/magnetic-v8-server'),
    join(searchRoot, 'target/release/magnetic-v8-server'),
  ];

  for (const path of candidates) {
    if (existsSync(path)) return path;
  }

  return null;
}
