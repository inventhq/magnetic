#!/usr/bin/env node
// cli.ts — Magnetic CLI entry point
// Commands: dev, build, push

import { resolve, join, dirname } from 'node:path';
import { existsSync, readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { createInterface } from 'node:readline';
import { scanApp, generateBridge } from './generator.ts';
import { bundleApp, buildForDeploy } from './bundler.ts';
import { startDev } from './dev.ts';
import { parseAppConfig, serializeConfigForServer, readDesignJson } from './config.ts';
import { buildContentMap, generateContentInjection } from './content.ts';

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

// ── Config file: ~/.magnetic/config.json ─────────────────────────────

const CONFIG_DIR = join(homedir(), '.magnetic');
const CONFIG_PATH = join(CONFIG_DIR, 'config.json');

interface MagneticConfig {
  api_key?: string;
  server?: string;
}

function loadGlobalConfig(): MagneticConfig {
  if (existsSync(CONFIG_PATH)) {
    try { return JSON.parse(readFileSync(CONFIG_PATH, 'utf-8')); } catch { return {}; }
  }
  return {};
}

function saveGlobalConfig(config: MagneticConfig) {
  mkdirSync(CONFIG_DIR, { recursive: true });
  writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2) + '\n');
}

/// Resolve API key: --key flag > MAGNETIC_API_KEY env > ~/.magnetic/config.json
function resolveApiKey(): string | undefined {
  return getArg('--key') || process.env.MAGNETIC_API_KEY || loadGlobalConfig().api_key;
}

function prompt(question: string): Promise<string> {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(question, (answer) => { rl.close(); resolve(answer.trim()); });
  });
}

function usage() {
  console.log(`
  @magneticjs/cli — Build and deploy server-driven UI apps

  Usage:
    magnetic dev              Start dev mode (watch + rebuild + serve)
    magnetic build            Build the app bundle for deployment
    magnetic push             Build and deploy to a Magnetic platform server
    magnetic openapi          Detect OpenAPI specs from data sources and generate types
    magnetic login            Authenticate with Magnetic Cloud
    magnetic whoami           Show current authenticated user

  Options:
    --port <n>                Dev server port (default: 3003)
    --dir <path>              App directory (default: current directory)
    --server <url>            Platform server URL for push
    --name <name>             App name for push (default: from magnetic.json)
    --key <api_key>           API key for push (or set MAGNETIC_API_KEY)
    --minify                  Minify the output bundle

  Developer workflow:
    1. Write pages in pages/*.tsx
    2. Write business logic in state.ts (optional)
    3. Run \`magnetic dev\` to develop locally
    4. Run \`magnetic login\` to authenticate
    5. Run \`magnetic push\` to deploy
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
      const appConfig = parseAppConfig(appDir);
      log('info', `Scanned: ${scan.pages.length} pages, ${scan.layouts.length} layouts, state: ${scan.statePath || 'none (using defaults)'}`);
      if (appConfig.data.length > 0) log('info', `Data sources: ${appConfig.data.length}`);
      if (appConfig.actions.length > 0) log('info', `Action mappings: ${appConfig.actions.length}`);

      // Read design.json for CSS framework
      const designJson = readDesignJson(appDir);
      if (designJson) log('info', 'Design tokens: design.json loaded');

      for (const page of scan.pages) {
        log('debug', `  route ${page.routePath.padEnd(15)} ← ${page.filePath}`);
      }

      // Content pipeline: two modes
      // 1. Bundle mode (default): bake all content into the JS bundle
      // 2. Lazy mode (--lazy-content): only bake metadata index, load .md on demand during SSG
      const diskContent = args.includes('--lazy-content');
      const contentDir = join(appDir, 'content');
      let contentInjection: string | undefined;
      let contentSlugs: string[] = [];

      if (diskContent) {
        const { buildContentIndex, generateContentDiskInjection } = await import('./content.ts');
        const contentIndex = buildContentIndex(appDir);
        if (contentIndex) {
          contentSlugs = Object.keys(contentIndex);
          contentInjection = generateContentDiskInjection(contentIndex, contentDir);
          log('info', `Content: ${contentSlugs.length} markdown files (on-disk mode)`);
        }
      } else {
        const contentMap = buildContentMap(appDir);
        if (contentMap) {
          contentSlugs = Object.keys(contentMap);
          contentInjection = generateContentInjection(contentMap);
          log('info', `Content: ${contentSlugs.length} markdown files`);
        }
      }

      const bridgeCode = generateBridge(scan, appConfig, designJson ?? undefined, contentInjection);
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

      // SSG: prerender routes to static HTML
      // --static: auto-discover from content slugs + static pages
      // or use explicit prerender list from magnetic.json
      let prerenderList: string[] | undefined = appConfig.prerender;

      if (args.includes('--static')) {
        prerenderList = ['/'];
        // Add content-based routes
        for (const slug of contentSlugs) {
          prerenderList.push('/' + slug);
        }
        // Add static page routes (skip / since already added, skip dynamic :param routes)
        for (const page of scan.pages) {
          if (page.routePath !== '/' && !page.routePath.includes(':') && !page.isCatchAll) {
            prerenderList.push(page.routePath);
          }
        }
        // Deduplicate
        prerenderList = [...new Set(prerenderList)];
      }

      if (prerenderList && prerenderList.length > 0) {
        log('info', `Pre-rendering ${prerenderList.length} routes...`);
        const { prerenderRoutes } = await import('./prerender.ts');
        const { count: prerenderCount } = await prerenderRoutes({
          bundlePath: result.outPath,
          outDir: join(appDir, 'dist'),
          routes: prerenderList,
          title: appConfig.name || 'Magnetic App',
          inlineCSS: undefined,
          publicDir: join(appDir, 'public'),
          contentDir: diskContent ? contentDir : undefined,
          log,
        });
        log('info', `✓ Pre-rendered ${prerenderCount} static HTML pages`);
      }
      break;
    }

    case 'login': {
      const globalConfig = loadGlobalConfig();
      const serverUrl = getArg('--server') || config.server || globalConfig.server || 'https://api.magnetic.app';

      const email = await prompt('Email: ');
      if (!email || !email.includes('@')) {
        log('error', 'Invalid email');
        process.exit(1);
      }

      log('info', `Registering with ${serverUrl}...`);
      const resp = await fetch(`${serverUrl}/api/auth/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
      });

      if (!resp.ok) {
        const text = await resp.text();
        log('error', `Registration failed (${resp.status}): ${text}`);
        process.exit(1);
      }

      const data = await resp.json() as any;
      saveGlobalConfig({ ...globalConfig, api_key: data.api_key, server: serverUrl });
      log('info', `✓ Authenticated as ${email}`);
      log('info', `  API key saved to ${CONFIG_PATH}`);
      log('info', `  User ID: ${data.user_id}`);
      break;
    }

    case 'whoami': {
      const apiKey = resolveApiKey();
      const globalConfig = loadGlobalConfig();
      const serverUrl = getArg('--server') || config.server || globalConfig.server || 'https://api.magnetic.app';

      if (!apiKey) {
        log('error', 'Not logged in. Run: magnetic login');
        process.exit(1);
      }

      const resp = await fetch(`${serverUrl}/api/auth/me`, {
        headers: { 'Authorization': `Bearer ${apiKey}` },
      });

      if (resp.ok) {
        const data = await resp.json() as any;
        log('info', `Logged in as ${data.email} (${data.tier} tier)`);
        log('info', `  User ID: ${data.id}`);
      } else {
        log('error', 'Invalid API key. Run: magnetic login');
        process.exit(1);
      }
      break;
    }

    case 'openapi': {
      const appConfig = parseAppConfig(appDir);
      const urls = appConfig.data.map(d => d.url).filter(Boolean);

      if (urls.length === 0) {
        log('error', 'No data sources in magnetic.json — nothing to detect');
        process.exit(1);
      }

      log('info', `Probing ${urls.length} data source(s) for OpenAPI/Swagger specs...`);

      const { discoverAll, parse, generateTypes, suggestDataSources } = await import('@magneticjs/openapi') as any;
      const results = await discoverAll(urls);

      let foundAny = false;
      for (const [base, result] of results) {
        if (result.found) {
          foundAny = true;
          log('info', `✓ Found ${result.version} spec at ${result.specUrl}`);

          const api = parse(result.spec, result.version!);
          log('info', `  ${api.title} v${api.version} — ${api.endpoints.length} endpoints, ${api.schemas.length} schemas`);

          // Generate types
          const types = generateTypes(api);
          const outPath = join(appDir, 'server', 'api-types.ts');
          mkdirSync(join(appDir, 'server'), { recursive: true });
          writeFileSync(outPath, types);
          log('info', `  Types written to ${outPath}`);

          // Suggest data sources
          const suggested = suggestDataSources(api);
          const suggestedCount = Object.keys(suggested).length;
          if (suggestedCount > 0) {
            log('info', `  Suggested data sources (${suggestedCount}):`);
            for (const [key, src] of Object.entries(suggested) as [string, any][]) {
              log('debug', `    "${key}": { "url": "${src.url}", "auth": ${src.auth} }`);
            }
          }
        } else {
          log('warn', `No spec found at ${base}: ${result.error}`);
        }
      }

      if (!foundAny) {
        log('warn', 'No OpenAPI/Swagger specs detected. Types not generated.');
      }
      break;
    }

    case 'push': {
      const globalConfig = loadGlobalConfig();
      const serverUrl = getArg('--server') || config.server || globalConfig.server;
      const appName = getArg('--name') || config.name;
      const apiKey = resolveApiKey();
      const isStaticPush = args.includes('--static');

      if (!serverUrl) {
        console.error('[magnetic] No server URL. Use --server <url> or set "server" in magnetic.json');
        process.exit(1);
      }
      if (!appName) {
        console.error('[magnetic] No app name. Use --name <name> or set "name" in magnetic.json');
        process.exit(1);
      }

      let deployPayload: any;

      if (isStaticPush) {
        // ── Static (SSG) deployment ──────────────────────────────
        log('info', `Building static site for deploy...`);
        const scan = scanApp(appDir, monorepoRoot || undefined);
        const appConfig = parseAppConfig(appDir);
        log('info', `Scanned: ${scan.pages.length} pages, ${scan.layouts.length} layouts, state: ${scan.statePath || 'none'}`);
        const pushDesignJson = readDesignJson(appDir);
        if (pushDesignJson) log('info', 'Design tokens: design.json loaded');
        const pushContentMap = buildContentMap(appDir);
        const pushContentInjection = pushContentMap ? generateContentInjection(pushContentMap) : undefined;
        const contentSlugs = pushContentMap ? Object.keys(pushContentMap) : [];
        if (pushContentMap) log('info', `Content: ${contentSlugs.length} markdown files`);
        const bridgeCode = generateBridge(scan, appConfig, pushDesignJson ?? undefined, pushContentInjection);

        const result = await bundleApp({
          appDir,
          bridgeCode,
          minify: true,
          monorepoRoot: monorepoRoot || undefined,
        });

        // Build prerender route list
        const prerenderList: string[] = ['/'];
        for (const slug of contentSlugs) prerenderList.push('/' + slug);
        for (const page of scan.pages) {
          if (page.routePath !== '/' && !page.routePath.includes(':') && !page.isCatchAll) {
            prerenderList.push(page.routePath);
          }
        }
        const routes = [...new Set(prerenderList)];

        log('info', `Pre-rendering ${routes.length} routes...`);
        const { prerenderRoutes } = await import('./prerender.ts');
        const distDir = join(appDir, 'dist');
        await prerenderRoutes({
          bundlePath: result.outPath,
          outDir: distDir,
          routes,
          title: appConfig.name || 'Magnetic App',
          inlineCSS: undefined,
          publicDir: join(appDir, 'public'),
          contentDir: undefined,
          log,
        });

        // Collect all files from dist/ (excluding app.js bundle) + public/
        const staticFiles: Record<string, string> = {};
        const collectFiles = (dir: string, prefix: string) => {
          if (!existsSync(dir)) return;
          for (const entry of readdirSync(dir, { withFileTypes: true })) {
            if (entry.isDirectory()) {
              collectFiles(join(dir, entry.name), prefix ? `${prefix}/${entry.name}` : entry.name);
            } else if (entry.isFile() && entry.name !== 'app.js') {
              const relPath = prefix ? `${prefix}/${entry.name}` : entry.name;
              staticFiles[relPath] = readFileSync(join(dir, entry.name), 'utf-8');
            }
          }
        };
        // Public assets (CSS, images, etc.)
        collectFiles(join(appDir, 'public'), '');
        // Prerendered HTML pages (overwrites any public/ conflicts)
        collectFiles(distDir, '');

        log('info', `Static files: ${Object.keys(staticFiles).length} files`);
        for (const [name, content] of Object.entries(staticFiles)) {
          log('debug', `  ${name} (${(content.length / 1024).toFixed(1)}KB)`);
        }

        deployPayload = { name: appName, static: true, assets: staticFiles };
      } else {
        // ── SSR deployment ───────────────────────────────────────
        log('info', `Building for deploy...`);
        const scan = scanApp(appDir, monorepoRoot || undefined);
        const appConfig = parseAppConfig(appDir);
        log('info', `Scanned: ${scan.pages.length} pages, ${scan.layouts.length} layouts, state: ${scan.statePath || 'none'}`);
        if (appConfig.data.length > 0) log('info', `Data sources: ${appConfig.data.length}`);
        if (appConfig.actions.length > 0) log('info', `Action mappings: ${appConfig.actions.length}`);
        const pushDesignJson = readDesignJson(appDir);
        if (pushDesignJson) log('info', 'Design tokens: design.json loaded');
        const pushContentMap = buildContentMap(appDir);
        const pushContentInjection = pushContentMap ? generateContentInjection(pushContentMap) : undefined;
        if (pushContentMap) log('info', `Content: ${Object.keys(pushContentMap).length} markdown files`);
        const bridgeCode = generateBridge(scan, appConfig, pushDesignJson ?? undefined, pushContentInjection);
        const deploy = await buildForDeploy({ appDir, bridgeCode, monorepoRoot: monorepoRoot || undefined });
        const serverConfig = serializeConfigForServer(appConfig);

        log('info', `Bundle: ${(deploy.bundleSize / 1024).toFixed(1)}KB (minified)`);
        log('info', `Assets: ${Object.keys(deploy.assets).length} files`);
        for (const [name, content] of Object.entries(deploy.assets)) {
          log('debug', `  asset: ${name} (${(content.length / 1024).toFixed(1)}KB)`);
        }

        const bundleContent = readFileSync(deploy.bundlePath, 'utf-8');
        deployPayload = { name: appName, bundle: bundleContent, assets: deploy.assets, config: serverConfig } as any;

        // Hybrid pre-render: if magnetic.json has prerender routes, pre-render them
        const prerenderPatterns = appConfig.prerender;
        if (prerenderPatterns && prerenderPatterns.length > 0) {
          // Expand glob patterns (e.g. "/blog/*") against content slugs
          const contentSlugs = pushContentMap ? Object.keys(pushContentMap) : [];
          const expandedRoutes: string[] = [];
          for (const pattern of prerenderPatterns) {
            if (pattern.endsWith('/*')) {
              // Glob: /blog/* → match all content slugs starting with blog/
              const prefix = pattern.slice(1, -2); // "/blog/*" → "blog"
              for (const slug of contentSlugs) {
                if (slug.startsWith(prefix + '/') || slug === prefix) {
                  expandedRoutes.push('/' + slug);
                }
              }
              // Also match page routes
              for (const page of scan.pages) {
                if (page.routePath.startsWith('/' + prefix + '/') && !page.routePath.includes(':') && !page.isCatchAll) {
                  expandedRoutes.push(page.routePath);
                }
              }
            } else {
              expandedRoutes.push(pattern);
            }
          }
          const prerenderRouteList = [...new Set(expandedRoutes)];

          if (prerenderRouteList.length > 0) {
            log('info', `Pre-rendering ${prerenderRouteList.length} routes for hybrid SSR+SSG...`);
            const { prerenderRoutes } = await import('./prerender.ts');
            const { count: prCount, pages } = await prerenderRoutes({
              bundlePath: deploy.bundlePath,
              outDir: join(appDir, 'dist'),
              routes: prerenderRouteList,
              title: appConfig.name || 'Magnetic App',
              inlineCSS: undefined,
              publicDir: join(appDir, 'public'),
              log,
            });
            if (prCount > 0) {
              deployPayload.prerendered = pages;
              log('info', `✓ ${prCount} routes will be served as static HTML (hybrid mode)`);
            }
          }
        }
      }

      if (apiKey) {
        // Authenticated: deploy via control plane
        log('info', `Deploying ${isStaticPush ? 'static site' : 'app'} to ${serverUrl} (authenticated)...`);
        const deployBody = Buffer.from(JSON.stringify(deployPayload));
        const resp = await fetch(`${serverUrl}/api/deploy`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`,
          },
          body: deployBody,
        });

        if (resp.ok) {
          const data = await resp.json() as any;
          log('info', `✓ Deployed!`);
          log('info', `  Live at: ${data.url}`);
          log('info', `  App ID:  ${data.id}`);
          if (data.static) log('info', `  Mode: static (${data.files} files)`);
        } else {
          const text = await resp.text();
          log('error', `Deploy failed (${resp.status}): ${text}`);
          process.exit(1);
        }
      } else {
        // No auth: direct push to node (backward-compatible with --platform servers)
        log('info', `Pushing to ${serverUrl}/api/apps/${appName}/deploy...`);
        const directBody = Buffer.from(JSON.stringify(
          isStaticPush
            ? { static: true, assets: deployPayload.assets }
            : { bundle: deployPayload.bundle, assets: deployPayload.assets, config: deployPayload.config, prerendered: deployPayload.prerendered }
        ));
        const resp = await fetch(`${serverUrl}/api/apps/${appName}/deploy`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: directBody,
        });

        if (resp.ok) {
          const data = await resp.json() as any;
          log('info', `✓ Deployed! ${data.url || serverUrl + '/apps/' + appName + '/'}`);
          log('info', `  Live at: ${serverUrl}/apps/${appName}/`);
          if (data.static) log('info', `  Mode: static (${data.files} files)`);
        } else {
          const text = await resp.text();
          log('error', `Deploy failed (${resp.status}): ${text}`);
          process.exit(1);
        }
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
