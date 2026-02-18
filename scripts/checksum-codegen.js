// [P3-C7:checksum] verify SHA-256 of generated files matches goldens
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

const OUT = path.resolve("tools/codegen/out");
const GOLDEN = path.resolve("tests/golden/codegen");

const files = [
  ["typed.ts", "typed.ts.sha256"],
  ["Typed.swift", "Typed.swift.sha256"],
  ["Typed.kt", "Typed.kt.sha256"],
];

function sha256(buf) { return crypto.createHash("sha256").update(buf).digest("hex"); }
function read(p) { return fs.readFileSync(p); }
function fail(msg) { console.error(msg); process.exit(1); }

for (const [gen, gold] of files) {
  const g = path.join(OUT, gen);
  const h = path.join(GOLDEN, gold);
  if (!fs.existsSync(g)) fail(`missing generated: ${g}`);
  if (!fs.existsSync(h)) fail(`missing golden: ${h}`);
  const digest = sha256(read(g));
  const expected = read(h).toString().trim();
  if (digest !== expected) fail(`${gen} checksum mismatch:\n  got:      ${digest}\n  expected: ${expected}`);
}
console.log("checksum-codegen OK");
