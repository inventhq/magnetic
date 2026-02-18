// [P3-C1:golden-diff] Byte-for-byte compare manifests/.well-known vs tests/golden/manifests
const fs = require("fs");
const path = require("path");

const SRC = path.resolve("contracts/.well-known");
const GOLDEN = path.resolve("tests/golden/manifests");

function read(p) { return fs.readFileSync(p); }
function list(dir) {
  return fs.readdirSync(dir).filter(f => f.endsWith(".json")).sort();
}

function fail(msg) {
  console.error(msg);
  process.exit(1);
}

if (!fs.existsSync(SRC)) fail(`missing: ${SRC}`);
if (!fs.existsSync(GOLDEN)) fail(`missing: ${GOLDEN}`);

const a = list(SRC);
const b = list(GOLDEN);

if (a.join("|") !== b.join("|")) {
  fail(`file set mismatch:\n  src:    ${a.join(", ")}\n  golden: ${b.join(", ")}`);
}

for (const f of a) {
  const src = read(path.join(SRC, f));
  const gol = read(path.join(GOLDEN, f));
  if (!src.equals(gol)) {
    fail(`mismatch: ${f} (manifests/.well-known vs tests/golden/manifests)`);
  }
}

console.log("golden-manifests OK");
