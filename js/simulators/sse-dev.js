// Minimal SSE dev server — streams JSONL envelopes with increasing seq.
// Endpoint: GET http://localhost:6060/sse?topic=<name>
const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 6060;
const SEED = path.join(__dirname, "seeds", "stream.jsonl");

function readLines(file) {
  return fs.readFileSync(file, "utf8").trim().split("\n").map((l) => JSON.parse(l));
}

const seed = readLines(SEED); // [{topic, seq, ts, version, data}, …]

// [P3-C2:sim:sse:metrics.cpu:state]
const cpuState = { seq: 0 };
function nextCpuPercent() {
  const base = 30 + Math.round(20 * Math.sin(Date.now() / 1000));
  return Math.max(0, Math.min(100, base));
}

const server = http.createServer(async (req, res) => {
  const urlObj = new URL(req.url, `http://${req.headers.host}`);

  // CORS (dev only)
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Idempotency-Key");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  if (req.method === "OPTIONS") { res.writeHead(204); res.end(); return; }

// [P3-C3:sim:actions:echo:start]
// --- Actions echo: POST /actions/:name
if (req.method === "POST" && urlObj.pathname.startsWith("/actions/")) {
  const name = urlObj.pathname.replace("/actions/", "");
  let body = "";
  req.on("data", (chunk) => (body += chunk));
  req.on("end", () => {
    let parsed = {};
    try { parsed = JSON.parse(body || "{}"); } catch { parsed = {}; }
    if (name === "fail") {
      res.writeHead(400, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ status: "ERR", error: "simulated_failure" }));
      return;
    }
    const applied = typeof parsed.applied_seq === "number" ? parsed.applied_seq : 42;
    const echoedKey = req.headers["idempotency-key"] || null;
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "OK", applied_seq: applied, echoedKey }));
  });
  return;
}
// [P3-C3:sim:actions:echo:end]

  // --- SSE stream: GET /sse?topic=...
  if (req.method === "GET" && urlObj.pathname === "/sse") {
    const topic = urlObj.searchParams.get("topic"); // optional filter

    res.writeHead(200, {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive"
    });

    // [P3-C2:sim:sse:metrics.cpu:branch:start]
    if (topic === "metrics.cpu") {
      const interval = setInterval(() => {
        cpuState.seq += 1; // strictly monotonic per topic
        const envelope = {
          topic: "metrics.cpu",
          seq: cpuState.seq,
          ts: Date.now(),
          data: nextCpuPercent() // schema: number
        };
        res.write(`event: message\n`);
        res.write(`data: ${JSON.stringify(envelope)}\n\n`);
      }, 1000);

      req.on("close", () => clearInterval(interval));
      return; // don't fall through to seed replay
    }
    // [P3-C2:sim:sse:metrics.cpu:branch:end]

    let i = 0;
    const tick = () => {
      const e = seed[i % seed.length];
      if (!topic || e.topic === topic) {
        res.write(`event: message\n`);
        res.write(`data: ${JSON.stringify(e)}\n\n`);
      }
      i += 1;
    };
    tick();
    const id = setInterval(tick, 600);
    req.on("close", () => clearInterval(id));
    return;
  }

  // Fallback
  res.writeHead(404);
  res.end();
});

server.listen(PORT, () => {
  console.log(`[sim] SSE listening http://localhost:${PORT}/sse?topic=<name>`);
});
