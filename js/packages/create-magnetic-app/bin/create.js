#!/usr/bin/env node
/**
 * create-magnetic-app CLI
 *
 * Usage:
 *   create-magnetic-app <project-name> [--template todo|blank] [--dir <target-dir>]
 */
import { scaffold } from '../src/index.js';
import { resolve } from 'path';

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

const dir = scaffold(targetDir, { name, template });
console.log(`\n✓ Created Magnetic app: ${name}`);
console.log(`  ${dir}\n`);
console.log(`  Next steps:`);
console.log(`    cd ${name}`);
console.log(`    npm install`);
console.log(`    npx magnetic dev\n`);
