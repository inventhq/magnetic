#!/usr/bin/env node
/**
 * magnetic-compile CLI
 *
 * Usage:
 *   magnetic-compile <input.magnetic.html> [--out <output.json>] [--name ComponentName]
 *   magnetic-compile --dir <folder> [--out-dir <output-folder>]
 */
import { readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from 'fs';
import { resolve, basename, extname, join, relative } from 'path';
import { compile } from '../src/index.js';

const args = process.argv.slice(2);

function flag(name) {
  const i = args.indexOf(name);
  if (i === -1) return null;
  return args[i + 1] || null;
}

function hasFlag(name) {
  return args.indexOf(name) !== -1;
}

const dir = flag('--dir');
const outDir = flag('--out-dir');

if (dir) {
  // Batch mode: compile all .magnetic.html files in directory
  const targetDir = outDir || dir;
  mkdirSync(targetDir, { recursive: true });
  const files = findTemplates(resolve(dir));
  let count = 0;
  for (const f of files) {
    const src = readFileSync(f, 'utf8');
    const name = basename(f, '.magnetic.html');
    const result = compile(src, { name });
    const outPath = join(targetDir, name + '.compiled.json');
    writeFileSync(outPath, JSON.stringify(result, null, 2));
    console.log(`  ${relative(process.cwd(), f)} → ${relative(process.cwd(), outPath)}`);
    count++;
  }
  console.log(`\n✓ Compiled ${count} template(s)`);
} else {
  // Single file mode
  const input = args.find(a => !a.startsWith('--'));
  if (!input) {
    console.error('Usage: magnetic-compile <file.magnetic.html> [--out output.json] [--name Name]');
    console.error('       magnetic-compile --dir <folder> [--out-dir <output-folder>]');
    process.exit(1);
  }

  const src = readFileSync(resolve(input), 'utf8');
  const name = flag('--name') || basename(input, '.magnetic.html');
  const result = compile(src, { name });

  const out = flag('--out');
  if (out) {
    writeFileSync(resolve(out), JSON.stringify(result, null, 2));
    console.log(`✓ ${input} → ${out}`);
  } else {
    console.log(JSON.stringify(result, null, 2));
  }
}

function findTemplates(dir) {
  const results = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      results.push(...findTemplates(full));
    } else if (entry.endsWith('.magnetic.html')) {
      results.push(full);
    }
  }
  return results;
}
