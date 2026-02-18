#!/usr/bin/env node
// cli.ts — Magnetic CLI entry point
// Commands: dev, build, push

import { resolve, join } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';
import { scanApp, generateBridge } from './generator.ts';
import { bundleApp, buildForDeploy } from './bundler.ts';
import { startDev } from './dev.ts';

const args = process.argv.slice(2);
const command = args[0];

// Structured logging
function log(level: 'info' | 'warn' | 'error' | 'debug', msg: string) {
  const ts = new Date().toISOString().slice(11, 23);
  const prefix = level === 'error' ? '✗' : level === 'warn' ? '⚠' : level === 'debug' ? '·' : '→';
  const stream = level === 'error' ? process.stderr : process.stdout;
  stream.write(`[${ts}] ${prefix} ${msg}\n`);
}

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

function getArg(flag: string): string | undefined {
  const idx = args.indexOf(flag);
  return idx >= 0 ? args[idx + 1] : undefined;
}

function usage() {
  console.log(`
  @magnetic/cli — Build and deploy server-driven UI apps

  Usage:
    magnetic dev              Start dev mode (watch + rebuild + serve)
    magnetic build            Build the app bundle for deployment
    magnetic push             Build and deploy to a Magnetic platform server

  Options:
    --port <n>                Dev server port (default: 3003)
    --dir <path>              App directory (default: current directory)
    --server <url>            Platform server URL for push
    --name <name>             App name for push (default: from magnetic.json)
    --minify                  Minify the output bundle

  Developer workflow:
    1. Write pages in pages/*.tsx
    2. Write business logic in state.ts (optional)
    3. Run \`magnetic dev\` to develop locally
    4. Run \`magnetic push\` to deploy
  `);
}

async function main() {
  if (!command || command === '--help' || command === '-h') {
    usage();
    process.exit(0);
  }

  const appDir = resolve(getArg('--dir') || '.');
  const monorepoRoot = findMonorepoRoot(appDir);
  const port = parseInt(getArg('--port') || '3003', 10);

  // Load magnetic.json if it exists
  let config: any = {};
  const configPath = join(appDir, 'magnetic.json');
  if (existsSync(configPath)) {
    config = JSON.parse(readFileSync(configPath, 'utf-8'));
  }

  switch (command) {
    case 'dev': {
      await startDev({
        appDir,
        port,
        monorepoRoot: monorepoRoot || undefined,
      });
      break;
    }

    case 'build': {
      log('info', `Building ${appDir}`);
      const buildStart = Date.now();
      const scan = scanApp(appDir, monorepoRoot || undefined);
      log('info', `Scanned: ${scan.pages.length} pages, state: ${scan.statePath || 'none (using defaults)'}`);

      for (const page of scan.pages) {
        log('debug', `  route ${page.routePath.padEnd(15)} ← ${page.filePath}`);
      }

      const bridgeCode = generateBridge(scan);
      log('debug', `Bridge generated: ${bridgeCode.split('\n').length} lines`);

      if (args.includes('--verbose')) {
        console.log('\n--- Generated bridge ---');
        console.log(bridgeCode);
        console.log('--- End bridge ---\n');
      }

      const result = await bundleApp({
        appDir,
        bridgeCode,
        minify: args.includes('--minify'),
        monorepoRoot: monorepoRoot || undefined,
      });

      const kb = (result.sizeBytes / 1024).toFixed(1);
      const elapsed = Date.now() - buildStart;
      log('info', `✓ Built ${result.outPath} (${kb}KB) in ${elapsed}ms`);
      break;
    }

    case 'push': {
      const serverUrl = getArg('--server') || config.server;
      const appName = getArg('--name') || config.name;

      if (!serverUrl) {
        console.error('[magnetic] No server URL. Use --server <url> or set "server" in magnetic.json');
        process.exit(1);
      }
      if (!appName) {
        console.error('[magnetic] No app name. Use --name <name> or set "name" in magnetic.json');
        process.exit(1);
      }

      log('info', `Building for deploy...`);
      const scan = scanApp(appDir, monorepoRoot || undefined);
      log('info', `Scanned: ${scan.pages.length} pages, state: ${scan.statePath || 'none'}`);
      const bridgeCode = generateBridge(scan);
      const deploy = await buildForDeploy({ appDir, bridgeCode, monorepoRoot: monorepoRoot || undefined });

      log('info', `Bundle: ${(deploy.bundleSize / 1024).toFixed(1)}KB (minified)`);
      log('info', `Assets: ${Object.keys(deploy.assets).length} files`);
      for (const [name, content] of Object.entries(deploy.assets)) {
        log('debug', `  asset: ${name} (${(content.length / 1024).toFixed(1)}KB)`);
      }
      log('info', `Pushing to ${serverUrl}/api/apps/${appName}/deploy...`);

      const bundleContent = readFileSync(deploy.bundlePath, 'utf-8');

      const resp = await fetch(`${serverUrl}/api/apps/${appName}/deploy`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          bundle: bundleContent,
          assets: deploy.assets,
        }),
      });

      if (resp.ok) {
        const data = await resp.json() as any;
        log('info', `✓ Deployed! ${data.url || serverUrl + '/apps/' + appName + '/'}`);
        log('info', `  Live at: ${serverUrl}/apps/${appName}/`);
      } else {
        const text = await resp.text();
        log('error', `Deploy failed (${resp.status}): ${text}`);
        process.exit(1);
      }
      break;
    }

    default:
      console.error(`[magnetic] Unknown command: ${command}`);
      usage();
      process.exit(1);
  }
}

main().catch((err) => {
  console.error(`[magnetic] Fatal: ${err.message}`);
  process.exit(1);
});
