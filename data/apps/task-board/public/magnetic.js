// @magnetic/sdk-web-runtime — server-driven UI client
// POST → apply response (single round-trip) + WASM transport for SSE dedup
(function(d) {
  var M = self.Magnetic = {};
  var root = null;      // mount element
  var es = null;        // EventSource
  var wasm = null;      // WASM exports (null = not loaded, 0 = loading)
  var status = "disconnected";
  var queue = [];        // queued actions while offline
  var keys = {};         // keyed element cache
  var deb = {};          // debounce timers
  var lastHash = "";     // hash of last applied snapshot (dedup fallback)
  var enc = new TextEncoder();

  M.status = function() { return status; };

  // --- Connect to SSE + mount ---
  M.connect = function(url, mount) {
    root = typeof mount == "string" ? d.querySelector(mount) : mount;
    es = new EventSource(url);
    es.onmessage = function(ev) {
      try {
        var raw = ev.data;
        // WASM transport: store snapshot, dedup + prediction confirm
        if (wasm && wasm.store) {
          var bytes = enc.encode(raw);
          if (bytes.length <= 16384) {
            new Uint8Array(wasm.memory.buffer).set(bytes, wasm.input_ptr());
            if (wasm.store(bytes.length) === 0) return; // dedup or prediction matched
          }
        } else {
          var h = fnv(raw);
          if (h === lastHash) return;
          lastHash = h;
        }
        apply(JSON.parse(raw));
      } catch(e) { console.error("[magnetic] SSE error:", e); }
    };
    es.onerror = function() {
      if (wasm) status = "offline";
    };
    status = "connected";
    bind();
  };

  M.disconnect = function() {
    if (es) { es.close(); es = null; }
    status = "disconnected";
  };

  // --- Apply snapshot to DOM ---
  function apply(snap) {
    if (!root || !snap || !snap.root) return;
    var n = snap.root;
    if (n.key && keys[n.key] && keys[n.key].parentNode === root) {
      patch(keys[n.key], n);
      return;
    }
    root.textContent = "";
    root.appendChild(create(n));
  }
  M._apply = apply;

  // Create a brand-new DOM tree from descriptor (first render / new keys)
  function create(n) {
    var el = d.createElement(n.tag);
    if (n.key) { el.dataset.key = n.key; keys[n.key] = el; }
    setAttrs(el, n);
    if (n.events) for (var v in n.events) el.dataset["a_" + v] = n.events[v];
    if (n.text != null) el.textContent = n.text;
    if (n.children) for (var i = 0; i < n.children.length; i++) el.appendChild(create(n.children[i]));
    return el;
  }

  // Patch an existing DOM element in-place (never detaches it from parent)
  function patch(el, n) {
    setAttrs(el, n);
    if (n.events) for (var v in n.events) el.dataset["a_" + v] = n.events[v];
    if (n.children) {
      reconcile(el, n.children);
    } else if (n.text != null && n.tag != "input" && n.tag != "textarea") {
      el.textContent = n.text;
    }
  }

  // Keyed child reconciliation — patches in-place, never detaches keyed parents
  function reconcile(parent, descs) {
    var i, c, el, k;
    var newEls = [];
    var wantKeys = {};

    // 1. Build array of target elements: reuse+patch keyed, create new
    for (i = 0; i < descs.length; i++) {
      c = descs[i];
      if (c.key && keys[c.key]) {
        el = keys[c.key];
        patch(el, c);
        wantKeys[c.key] = true;
      } else {
        el = create(c);
        if (c.key) wantKeys[c.key] = true;
      }
      newEls.push(el);
    }

    // 2. Remove stale children (keyed elements not in new set, or non-keyed)
    var ch = parent.firstChild;
    while (ch) {
      var nx = ch.nextSibling;
      k = ch.dataset ? ch.dataset.key : null;
      if (!k || !wantKeys[k]) parent.removeChild(ch);
      ch = nx;
    }

    // 3. Insert / reorder to match target order
    for (i = 0; i < newEls.length; i++) {
      if (parent.childNodes[i] !== newEls[i]) {
        parent.insertBefore(newEls[i], parent.childNodes[i] || null);
      }
    }
  }

  function setAttrs(el, n) {
    if (n.attrs) for (var k in n.attrs) el.setAttribute(k, n.attrs[k]);
  }

  // --- Event delegation ---
  function bind() {
    d.addEventListener("click", function(e) {
      var t = e.target.closest("[data-a_click]");
      if (t) { e.preventDefault(); send(t.dataset.a_click, {}); }
    });
    d.addEventListener("submit", function(e) {
      var t = e.target.closest("[data-a_submit]");
      if (t) {
        e.preventDefault();
        var p = {}, f = new FormData(t);
        f.forEach(function(v, k) { p[k] = v; });
        send(t.dataset.a_submit, p);
        t.querySelectorAll("input").forEach(function(i) { i.value = ""; });
      }
    });
    d.addEventListener("input", function(e) {
      var t = e.target.closest("[data-a_input]");
      if (t) {
        var a = t.dataset.a_input;
        clearTimeout(deb[a]);
        deb[a] = setTimeout(function() { send(a, { value: t.value }); }, 300);
      }
    });
  }

  // --- Action dispatch: POST → apply response (single round-trip) ---
  M.send = send;
  function send(action, payload) {
    // Client-side navigation: intercept navigate: prefix
    if (action.indexOf("navigate:") === 0) {
      var path = action.slice(9);
      history.pushState({}, "", path);
      action = "navigate";
      payload = { path: path };
    }

    var body = JSON.stringify({ action: action, payload: payload });

    // POST to server, apply response snapshot directly
    fetch("/actions/" + encodeURIComponent(action), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: body
    }).then(function(r) { return r.text(); })
      .then(function(raw) {
        if (!raw || raw[0] !== "{") return;
        // Store in WASM for SSE dedup (so broadcast for same action is skipped)
        if (wasm && wasm.store) {
          var bytes = enc.encode(raw);
          if (bytes.length <= 16384) {
            new Uint8Array(wasm.memory.buffer).set(bytes, wasm.input_ptr());
            wasm.store(bytes.length);
          }
        } else {
          lastHash = fnv(raw);
        }
        try { apply(JSON.parse(raw)); } catch(e) {}
      }).catch(function() {});

    // If offline, queue for later
    if (status != "connected") queue.push(body);
  }

  // --- Client-side routing: back/forward ---
  self.addEventListener("popstate", function() {
    send("navigate", { path: location.pathname + location.search });
  });

  // --- WASM loader (generic transport — snapshot cache + dedup) ---
  M.loadWasm = function(url) {
    if (wasm !== null) return;
    wasm = 0; // loading sentinel
    fetch(url)
      .then(function(r) { return r.arrayBuffer(); })
      .then(function(b) { return WebAssembly.instantiate(b, {}); })
      .then(function(result) {
        wasm = result.instance.exports;
        // Drain queued actions (replay via POST)
        while (queue.length) {
          var q = JSON.parse(queue.shift());
          send(q.action, q.payload);
        }
      })
      .catch(function() { wasm = null; });
  };

  // --- FNV-1a hash for fast snapshot dedup (avoids re-render on SSE confirm) ---
  function fnv(s) {
    var h = 0x811c9dc5;
    for (var i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 0x01000193);
    }
    return h;
  }

})(document);
