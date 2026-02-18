// Minimal build: strip comments, collapse whitespace, shorten names
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const src = fs.readFileSync(path.join(__dirname, "src", "magnetic.js"), "utf8");

// Strip single-line comments (both full-line and inline)
let min = src.replace(/\/\/[^\n]*/g, "");
// Strip multi-line comments
min = min.replace(/\/\*[\s\S]*?\*\//g, "");
// Remove leading whitespace per line
min = min.split("\n").map(l => l.trim()).filter(l => l.length > 0).join("\n");
// Remove newlines around braces/parens/operators
min = min.replace(/\n(?=[{}()\[\];,.])/g, "");
min = min.replace(/([{}()\[\];,])\n/g, "$1");
// Collapse remaining newlines to single
min = min.replace(/\n+/g, "\n");

const dist = path.join(__dirname, "dist");
fs.mkdirSync(dist, { recursive: true });
fs.writeFileSync(path.join(dist, "magnetic.min.js"), min);

const raw = Buffer.byteLength(min, "utf8");
// Check gzip size
try {
  const gz = execSync(`gzip -c "${path.join(dist, "magnetic.min.js")}" | wc -c`).toString().trim();
  console.log(`magnetic.min.js: ${raw} bytes raw, ${gz} bytes gzipped`);
} catch {
  console.log(`magnetic.min.js: ${raw} bytes raw`);
}
