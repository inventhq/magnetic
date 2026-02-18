// Minimal WS dev server — broadcasts JSONL envelopes with increasing seq.
// Endpoint: ws://localhost:7070 (auto-broadcast; no explicit subscribe)
const fs = require("fs");
const path = require("path");
const { WebSocketServer } = require("ws");

const PORT = 7070;
const SEED = path.join(__dirname, "seeds", "stream.jsonl");

function readLines(file) {
  return fs.readFileSync(file, "utf8").trim().split("\n").map((l) => JSON.parse(l));
}
const seed = readLines(SEED);

// [P3-C2:sim:ws:chat.events:state]
const wsSeq = new Map(); // topic → seq counter
function nextSeq(topic) {
  const n = (wsSeq.get(topic) || 0) + 1;
  wsSeq.set(topic, n);
  return n;
}

const wss = new WebSocketServer({ port: PORT });

wss.on("connection", (ws, req) => {
  console.log("[sim] WS client connected");

  // [P3-C2:sim:ws:chat.events:start]
  // Parse query string like: ws://localhost:7070?topic=chat.{room_id}.events&room_id=general
  const url = new URL(req.url, "http://localhost");
  const topicTmpl = url.searchParams.get("topic");
  const roomId = url.searchParams.get("room_id") || "general";

  if (topicTmpl === "chat.{room_id}.events") {
    const topic = `chat.${roomId}.events`;

    const sendJoined = () => {
      const env = {
        topic,
        seq: nextSeq(topic),
        ts: Date.now(),
        data: { type: "user.joined", room_id: roomId, user: "bot" }
      };
      ws.send(JSON.stringify(env));
    };

    const sendMessage = () => {
      const env = {
        topic,
        seq: nextSeq(topic),
        ts: Date.now(),
        data: {
          type: "message.sent",
          room_id: roomId,
          message_id: "m_" + nextSeq(topic),
          author: "bot",
          text: "ping " + new Date().toISOString()
        }
      };
      ws.send(JSON.stringify(env));
    };

    sendJoined();
    const t = setInterval(sendMessage, 1500);
    ws.on("close", () => clearInterval(t));
    return;
  }
  // [P3-C2:sim:ws:chat.events:end]

  // Default: replay seed file as before
  let i = 0;
  const tick = () => {
    const e = seed[i % seed.length];
    if (ws.readyState === ws.OPEN) {
      ws.send(JSON.stringify(e));
    }
    i += 1;
  };
  tick();
  const id = setInterval(tick, 600);
  ws.on("close", () => clearInterval(id));
});

wss.on("listening", () => {
  console.log(`[sim] WS listening ws://localhost:${PORT}`);
});
