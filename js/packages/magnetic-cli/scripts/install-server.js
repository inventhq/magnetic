#!/usr/bin/env node
/**
 * install-server.js — Downloads the prebuilt magnetic-v8-server binary
 * for the current platform during `npm install @magneticjs/cli`.
 *
 * Update strategy:
 *   - SERVER_VERSION is hardcoded (decoupled from npm package version)
 *   - Only bump SERVER_VERSION when a new binary is uploaded to GitHub Releases
 *   - Binary version is tracked in bin/.version
 *   - If version mismatch → re-download; if match → skip
 *
 * Binary distribution:
 *   GitHub Releases: magnetic-v8-server-{target}.tar.gz
 *   Targets: x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu
 *
 * Falls back gracefully if download fails — user can build from source.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync, chmodSync } from 'fs';
import { join, dirname } from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(__dirname, '..');
const binDir = join(pkgDir, 'bin');
const binPath = join(binDir, 'magnetic-v8-server');
const versionFile = join(binDir, '.version');

// Server binary version — decoupled from npm package version.
// Only bump this when a new binary is uploaded to GitHub Releases.
const SERVER_VERSION = '0.2.0';

// Check if installed binary matches current server version
if (existsSync(binPath) && existsSync(versionFile)) {
  const installed = readFileSync(versionFile, 'utf-8').trim();
  if (installed === SERVER_VERSION) {
    console.log(`[magnetic] Server binary v${SERVER_VERSION} already installed, skipping`);
    process.exit(0);
  }
  console.log(`[magnetic] Server binary outdated (${installed} → ${SERVER_VERSION}), updating...`);
}

// Determine platform target
function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin';
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  if (platform === 'linux' && arch === 'arm64') return 'aarch64-unknown-linux-gnu';
  return null;
}

const target = getTarget();
if (!target) {
  console.warn(`[magnetic] No prebuilt binary for ${process.platform}-${process.arch}`);
  console.warn('[magnetic] Build from source: cargo build --release -p magnetic-v8-server');
  process.exit(0);
}

const baseUrl = process.env.MAGNETIC_BINARY_URL ||
  `https://github.com/inventhq/magnetic/releases/download/v${SERVER_VERSION}`;
const filename = `magnetic-v8-server-${target}.tar.gz`;
const url = `${baseUrl}/${filename}`;

console.log(`[magnetic] Downloading server binary for ${target}...`);
console.log(`[magnetic] ${url}`);

try {
  mkdirSync(binDir, { recursive: true });

  // Download — curl with -L follows GitHub's 302 redirect
  let downloaded = false;
  try {
    execSync(`curl -fsSL "${url}" -o "${join(binDir, filename)}"`, { stdio: 'pipe' });
    execSync(`tar xzf "${join(binDir, filename)}" -C "${binDir}"`, { stdio: 'pipe' });
    downloaded = true;
  } catch (e1) {
    try {
      execSync(`wget -q "${url}" -O "${join(binDir, filename)}"`, { stdio: 'pipe' });
      execSync(`tar xzf "${join(binDir, filename)}" -C "${binDir}"`, { stdio: 'pipe' });
      downloaded = true;
    } catch (e2) {
      // Both failed
    }
  }

  // Clean up tarball
  try { execSync(`rm -f "${join(binDir, filename)}"`, { stdio: 'pipe' }); } catch {}

  if (downloaded && existsSync(binPath)) {
    chmodSync(binPath, 0o755);
    writeFileSync(versionFile, SERVER_VERSION);
    console.log(`[magnetic] ✓ Server binary v${SERVER_VERSION} installed: ${binPath}`);
  } else {
    throw new Error(`Download failed or binary not found after extraction`);
  }
} catch (err) {
  console.error('');
  console.error('  ╔══════════════════════════════════════════════════════════════╗');
  console.error('  ║  MAGNETIC: Server binary download failed                    ║');
  console.error('  ╚══════════════════════════════════════════════════════════════╝');
  console.error('');
  console.error(`  URL: ${url}`);
  console.error(`  Error: ${err.message}`);
  console.error('');
  console.error('  `magnetic dev` will NOT work without the server binary.');
  console.error('');
  console.error('  To fix, either:');
  console.error('    1. Re-run: npm rebuild @magneticjs/cli');
  console.error('    2. Build from source:');
  console.error('       git clone https://github.com/inventhq/magnetic.git');
  console.error('       cd magnetic/rs/crates/magnetic-v8-server');
  console.error('       cargo build --release');
  console.error('       cp target/release/magnetic-v8-server $(npm root -g)/@magneticjs/cli/bin/');
  console.error('');
  // Don't fail npm install — but make the error impossible to miss
  process.exit(0);
}
