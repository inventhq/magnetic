#!/usr/bin/env node
/**
 * create-magnetic-app CLI
 *
 * Usage:
 *   create-magnetic-app <project-name> [--template todo|blank] [--dir <target-dir>]
 */
import { scaffold } from '../src/index.js';
import { resolve } from 'path';
import { existsSync } from 'fs';

const args = process.argv.slice(2);
const name = args.find(a => !a.startsWith('--'));

if (!name || name === '--help' || name === '-h') {
  console.log(`
  create-magnetic-app — scaffold a new Magnetic project

  Usage:
    create-magnetic-app <project-name> [options]

  Options:
    --template <name>   Template: "todo" (default) or "blank"
    --dir <path>        Target directory (default: current directory)
  `);
  process.exit(name ? 0 : 1);
}

const dirFlag = args.indexOf('--dir');
const targetDir = dirFlag !== -1 && args[dirFlag + 1]
  ? resolve(args[dirFlag + 1], name)
  : resolve(process.cwd(), name);

const tplFlag = args.indexOf('--template');
const template = tplFlag !== -1 && args[tplFlag + 1] ? args[tplFlag + 1] : 'todo';

// Try to find the built client runtime
const runtimePaths = [
  resolve(import.meta.dirname, '../../../sdk-web-runtime/dist/magnetic.min.js'),
  resolve(import.meta.dirname, '../../../../apps/task-board/public/magnetic.js'),
  resolve(process.cwd(), 'js/packages/sdk-web-runtime/dist/magnetic.min.js'),
  resolve(process.cwd(), 'apps/task-board/public/magnetic.js'),
];
let runtimeSrc = null;
for (const p of runtimePaths) {
  if (existsSync(p)) { runtimeSrc = p; break; }
}

// Try to find transport.wasm
const wasmPaths = [
  resolve(import.meta.dirname, '../../../magnetic-server/wasm/transport.wasm'),
  resolve(import.meta.dirname, '../../../magnetic-cli/wasm/transport.wasm'),
  resolve(process.cwd(), 'js/packages/magnetic-server/wasm/transport.wasm'),
];
let wasmSrc = null;
for (const p of wasmPaths) {
  if (existsSync(p)) { wasmSrc = p; break; }
}

const dir = scaffold(targetDir, { name, template, runtimeSrc, wasmSrc });
console.log(`\n✓ Created Magnetic app: ${name}`);
console.log(`  ${dir}\n`);
console.log(`  Next steps:`);
console.log(`    cd ${name}`);
console.log(`    magnetic dev\n`);
if (!runtimeSrc) {
  console.log(`  ⚠ Client runtime (magnetic.js) not found — copy it to public/magnetic.js`);
}
