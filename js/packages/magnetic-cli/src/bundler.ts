// bundler.ts — esbuild wrapper for Magnetic apps
// Takes generated bridge code and bundles it into an IIFE for V8

import { build } from 'esbuild';
import { join, resolve } from 'node:path';
import { mkdirSync, existsSync, writeFileSync, statSync, readdirSync, readFileSync } from 'node:fs';

export interface BundleOptions {
  /** Absolute path to the app directory */
  appDir: string;
  /** Generated bridge source code (from generator) */
  bridgeCode: string;
  /** Output directory (default: dist/) */
  outDir?: string;
  /** Output filename (default: app.js) */
  outFile?: string;
  /** Minify output */
  minify?: boolean;
  /** Monorepo root (for resolving @magneticjs/server) */
  monorepoRoot?: string;
}

export interface BundleResult {
  outPath: string;
  sizeBytes: number;
}

/**
 * Bundle the generated bridge code into an IIFE for V8 consumption.
 * Uses esbuild with stdin so no temp file is needed.
 */
export async function bundleApp(opts: BundleOptions): Promise<BundleResult> {
  const outDir = opts.outDir || join(opts.appDir, 'dist');
  const outFile = opts.outFile || 'app.js';
  const outPath = join(outDir, outFile);

  if (!existsSync(outDir)) {
    mkdirSync(outDir, { recursive: true });
  }

  // Resolve @magneticjs/server — in monorepo use actual path, otherwise npm package
  const alias: Record<string, string> = {};
  if (opts.monorepoRoot) {
    const serverPkg = join(opts.monorepoRoot, 'js/packages/magnetic-server/src');
    alias['@magneticjs/server'] = serverPkg;
    alias['@magneticjs/server/jsx-runtime'] = join(serverPkg, 'jsx-runtime.ts');
  }

  const result = await build({
    stdin: {
      contents: opts.bridgeCode,
      resolveDir: opts.appDir,
      loader: 'tsx',
    },
    bundle: true,
    format: 'iife',
    globalName: 'MagneticApp',
    outfile: outPath,
    minify: opts.minify || false,
    sourcemap: false,
    target: 'es2020',
    jsx: 'automatic',
    jsxImportSource: '@magneticjs/server',
    alias,
    logLevel: 'warning',
  });

  const stat = statSync(outPath);

  return {
    outPath,
    sizeBytes: stat.size,
  };
}

/**
 * Bundle and also prepare the assets manifest for deployment.
 * Returns the bundle path + a map of public/ files.
 */
export async function buildForDeploy(opts: BundleOptions): Promise<{
  bundlePath: string;
  bundleSize: number;
  assets: Record<string, string>;
}> {
  const bundle = await bundleApp({ ...opts, minify: true });

  // Collect public/ files as a map for upload
  const publicDir = join(opts.appDir, 'public');
  const assets: Record<string, string> = {};

  if (existsSync(publicDir)) {
    const entries = readdirSync(publicDir);
    for (const entry of entries) {
      const fullPath = join(publicDir, entry);
      if (statSync(fullPath).isFile()) {
        // For text files, include as string; for binary, base64 encode
        const ext = entry.split('.').pop() || '';
        const textExts = ['css', 'js', 'json', 'html', 'svg', 'txt', 'xml'];
        if (textExts.includes(ext)) {
          assets[entry] = readFileSync(fullPath, 'utf-8');
        } else {
          assets[entry] = readFileSync(fullPath).toString('base64');
        }
      }
    }
  }

  return {
    bundlePath: bundle.outPath,
    bundleSize: bundle.sizeBytes,
    assets,
  };
}
