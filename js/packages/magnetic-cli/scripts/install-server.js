#!/usr/bin/env node
/**
 * install-server.js — Downloads the prebuilt magnetic-v8-server binary
 * for the current platform during `npm install @magneticjs/cli`.
 *
 * Update strategy:
 *   - Version is read from ../package.json (matches @magneticjs/cli version)
 *   - When user runs `npm update @magneticjs/cli`, this script re-runs
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

// Read version from package.json (stays in sync with npm version)
const pkg = JSON.parse(readFileSync(join(pkgDir, 'package.json'), 'utf-8'));
const version = pkg.version;

// Check if installed binary matches current CLI version
if (existsSync(binPath) && existsSync(versionFile)) {
  const installed = readFileSync(versionFile, 'utf-8').trim();
  if (installed === version) {
    console.log(`[magnetic] Server binary v${version} already installed, skipping`);
    process.exit(0);
  }
  console.log(`[magnetic] Server binary outdated (${installed} → ${version}), updating...`);
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
  `https://github.com/inventhq/magnetic/releases/download/v${version}`;
const filename = `magnetic-v8-server-${target}.tar.gz`;
const url = `${baseUrl}/${filename}`;

console.log(`[magnetic] Downloading server binary for ${target}...`);
console.log(`[magnetic] ${url}`);

try {
  mkdirSync(binDir, { recursive: true });
  // Use curl or wget — available on all platforms
  try {
    execSync(`curl -fsSL "${url}" | tar xz -C "${binDir}"`, { stdio: 'pipe' });
  } catch {
    execSync(`wget -qO- "${url}" | tar xz -C "${binDir}"`, { stdio: 'pipe' });
  }

  if (existsSync(binPath)) {
    chmodSync(binPath, 0o755);
    writeFileSync(versionFile, version);
    console.log(`[magnetic] ✓ Server binary v${version} installed: ${binPath}`);
  } else {
    throw new Error('Binary not found after extraction');
  }
} catch (err) {
  console.warn(`[magnetic] Could not download prebuilt binary: ${err.message}`);
  console.warn('[magnetic] The CLI will still work, but you need the server binary for `magnetic dev`.');
  console.warn('[magnetic] Build from source: cargo build --release -p magnetic-v8-server');
  // Don't fail the install — just warn
  process.exit(0);
}
