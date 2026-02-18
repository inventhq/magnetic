import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { compile, tokenize, parse } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fix = (name) => readFileSync(resolve(__dirname, 'fixtures', name), 'utf8');

let passed = 0;
let failed = 0;

function assert(cond, msg) {
  if (cond) {
    passed++;
  } else {
    failed++;
    console.error(`  FAIL: ${msg}`);
  }
}

function test(name, fn) {
  console.log(`  ${name}`);
  fn();
}

// ── Counter ──
console.log('\n=== counter.magnetic.html ===');
test('tokenizes correctly', () => {
  const tokens = tokenize(fix('counter.magnetic.html'));
  assert(tokens.length > 0, 'should produce tokens');
  assert(tokens.some(t => t.type === 'binding' && t.field === 'count'), 'should find {{count}} binding');
  assert(tokens.some(t => t.type === 'open' && t.events && t.events.click === 'increment'), 'should find @click="increment"');
  assert(tokens.some(t => t.type === 'open' && t.events && t.events.click === 'decrement'), 'should find @click="decrement"');
});

test('compiles to instruction set', () => {
  const result = compile(fix('counter.magnetic.html'), { name: 'counter' });
  assert(result.version === 1, 'version should be 1');
  assert(result.name === 'counter', 'name should be counter');
  assert(result.ops.length > 0, 'should produce ops');
  assert(result.ops.some(o => o.op === 'bind' && o.field === 'count'), 'should have bind op for count');
  assert(result.ops.some(o => o.op === 'open' && o.events && o.events.click === 'increment'), 'should have increment event');
  assert(result.ops.filter(o => o.op === 'open').length === 5, 'should have 5 open ops (div, h1, div.controls, button, button)');
});

// ── Chat ──
console.log('\n=== chat.magnetic.html ===');
test('compiles with #each loop', () => {
  const result = compile(fix('chat.magnetic.html'), { name: 'chat' });
  assert(result.ops.some(o => o.op === 'each_open' && o.collection === 'messages'), 'should have each_open for messages');
  assert(result.ops.some(o => o.op === 'each_close'), 'should have each_close');
  assert(result.ops.some(o => o.op === 'bind' && o.field === 'msg.author'), 'should bind msg.author');
  assert(result.ops.some(o => o.op === 'bind' && o.field === 'msg.text'), 'should bind msg.text');
  assert(result.ops.some(o => o.op === 'open' && o.events && o.events.submit === 'send_message'), 'should have submit event');
  assert(result.ops.some(o => o.op === 'open' && o.tag === 'input' && o.selfClose), 'input should be self-closing');
});

// ── App (full) ──
console.log('\n=== app.magnetic.html ===');
test('compiles with #each + #if/else', () => {
  const result = compile(fix('app.magnetic.html'), { name: 'app' });
  assert(result.ops.some(o => o.op === 'each_open' && o.collection === 'messages'), 'should have each loop');
  assert(result.ops.some(o => o.op === 'if_open' && o.condition === 'hasMessages'), 'should have if_open');
  assert(result.ops.some(o => o.op === 'else'), 'should have else');
  assert(result.ops.some(o => o.op === 'if_close'), 'should have if_close');
  assert(result.ops.some(o => o.op === 'open' && o.key === 'title'), 'h1 should have key=title');
  assert(result.ops.some(o => o.op === 'open' && o.key === 'msg-form'), 'form should have key=msg-form');
});

test('ops roundtrip to JSON', () => {
  const result = compile(fix('app.magnetic.html'), { name: 'app' });
  const json = JSON.stringify(result);
  const parsed = JSON.parse(json);
  assert(parsed.ops.length === result.ops.length, 'JSON roundtrip preserves op count');
  assert(parsed.name === 'app', 'JSON roundtrip preserves name');
});

// ── Summary ──
console.log(`\n${passed} passed, ${failed} failed\n`);
if (failed > 0) process.exit(1);
