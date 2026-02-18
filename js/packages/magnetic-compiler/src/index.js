/**
 * @magnetic/compiler
 *
 * Compiles .magnetic.html templates into optimized render instruction sets.
 * Templates use:
 *   {{field}}              — bind state field (text interpolation)
 *   {{state.nested.field}} — dot-path binding
 *   @click="action"        — event binding → data-a_click="action"
 *   @submit="action"       — event binding → data-a_submit="action"
 *   {{#each items as item}} ... {{/each}}  — loop
 *   {{#if condition}} ... {{else}} ... {{/if}} — conditional
 *   <ComponentName />      — component inclusion (PascalCase tag)
 */

// ── Tokenizer ──────────────────────────────────────────────────────────

const VOID_TAGS = new Set([
  'area','base','br','col','embed','hr','img','input',
  'link','meta','param','source','track','wbr'
]);

/**
 * Tokenize a .magnetic.html template into a flat token stream.
 */
export function tokenize(src) {
  const tokens = [];
  let i = 0;
  const len = src.length;

  while (i < len) {
    // Handlebars block: {{#each}}, {{/each}}, {{#if}}, {{/if}}, {{else}}, {{field}}
    if (src[i] === '{' && i + 1 < len && src[i + 1] === '{') {
      const end = src.indexOf('}}', i + 2);
      if (end === -1) throw new Error(`Unclosed {{ at position ${i}`);
      const expr = src.slice(i + 2, end).trim();
      if (expr.startsWith('#each ')) {
        // {{#each items as item}}
        const m = expr.match(/^#each\s+(\S+)\s+as\s+(\S+)$/);
        if (!m) throw new Error(`Invalid #each syntax: {{${expr}}}`);
        tokens.push({ type: 'each_open', collection: m[1], item: m[2] });
      } else if (expr === '/each') {
        tokens.push({ type: 'each_close' });
      } else if (expr.startsWith('#if ')) {
        tokens.push({ type: 'if_open', condition: expr.slice(4).trim() });
      } else if (expr === 'else') {
        tokens.push({ type: 'else' });
      } else if (expr === '/if') {
        tokens.push({ type: 'if_close' });
      } else {
        tokens.push({ type: 'binding', field: expr });
      }
      i = end + 2;
      continue;
    }

    // HTML tag (open or close)
    if (src[i] === '<') {
      // Comment
      if (src.startsWith('<!--', i)) {
        const end = src.indexOf('-->', i + 4);
        i = end === -1 ? len : end + 3;
        continue;
      }

      // Closing tag
      if (i + 1 < len && src[i + 1] === '/') {
        const end = src.indexOf('>', i + 2);
        if (end === -1) throw new Error(`Unclosed closing tag at ${i}`);
        const tag = src.slice(i + 2, end).trim();
        tokens.push({ type: 'close', tag });
        i = end + 1;
        continue;
      }

      // Opening tag
      const tagMatch = src.slice(i).match(/^<([a-zA-Z][a-zA-Z0-9-]*)/);
      if (!tagMatch) {
        // Not a tag, treat as text
        tokens.push({ type: 'text', value: '<' });
        i++;
        continue;
      }
      const tag = tagMatch[1];
      i += tagMatch[0].length;

      // Parse attributes
      const attrs = {};
      const events = {};
      let key = null;
      let selfClose = false;

      while (i < len) {
        // Skip whitespace
        while (i < len && /\s/.test(src[i])) i++;
        // Self-close />
        if (src[i] === '/' && i + 1 < len && src[i + 1] === '>') {
          selfClose = true;
          i += 2;
          break;
        }
        // Close >
        if (src[i] === '>') {
          i++;
          break;
        }
        // Parse attribute name
        const attrStart = i;
        while (i < len && src[i] !== '=' && src[i] !== '>' && !/\s/.test(src[i]) && !(src[i] === '/' && src[i+1] === '>')) i++;
        const attrName = src.slice(attrStart, i);
        if (!attrName) break;

        // Check for = value
        let attrVal = true; // boolean attr
        while (i < len && /\s/.test(src[i])) i++;
        if (i < len && src[i] === '=') {
          i++; // skip =
          while (i < len && /\s/.test(src[i])) i++;
          if (src[i] === '"' || src[i] === "'") {
            const q = src[i];
            i++;
            const valStart = i;
            while (i < len && src[i] !== q) i++;
            attrVal = src.slice(valStart, i);
            i++; // skip closing quote
          } else {
            // Unquoted value
            const valStart = i;
            while (i < len && !/[\s>]/.test(src[i])) i++;
            attrVal = src.slice(valStart, i);
          }
        }

        // Classify attribute
        if (attrName.startsWith('@')) {
          events[attrName.slice(1)] = attrVal;
        } else if (attrName === 'data-key' || attrName === 'key') {
          key = attrVal;
        } else {
          attrs[attrName] = attrVal;
        }
      }

      const token = { type: 'open', tag };
      if (Object.keys(attrs).length) token.attrs = attrs;
      if (Object.keys(events).length) token.events = events;
      if (key) token.key = key;
      if (selfClose || VOID_TAGS.has(tag)) token.selfClose = true;
      tokens.push(token);
      continue;
    }

    // Plain text
    const textStart = i;
    while (i < len && src[i] !== '<' && !(src[i] === '{' && i + 1 < len && src[i + 1] === '{')) i++;
    const text = src.slice(textStart, i);
    if (text.trim()) {
      tokens.push({ type: 'text', value: text });
    }
  }

  return tokens;
}

// ── Parser: tokens → instruction tree ──────────────────────────────────

/**
 * Parse token stream into a compiled instruction list (AST).
 */
export function parse(tokens) {
  const ops = [];
  let i = 0;

  function walk() {
    while (i < tokens.length) {
      const t = tokens[i];

      if (t.type === 'text') {
        ops.push({ op: 'text', value: t.value.trim() });
        i++;
      } else if (t.type === 'binding') {
        ops.push({ op: 'bind', field: t.field });
        i++;
      } else if (t.type === 'open') {
        const node = { op: 'open', tag: t.tag };
        if (t.attrs) node.attrs = processAttrs(t.attrs);
        if (t.events) node.events = t.events;
        if (t.key) node.key = t.key;
        if (t.selfClose) node.selfClose = true;
        ops.push(node);
        i++;
        if (!t.selfClose) {
          walk(); // children
          // Expect close tag
          if (i < tokens.length && tokens[i].type === 'close') i++;
        }
        ops.push({ op: 'close' });
      } else if (t.type === 'close') {
        // Reached a close tag — return to parent walk
        return;
      } else if (t.type === 'each_open') {
        ops.push({ op: 'each_open', collection: t.collection, item: t.item });
        i++;
        walk(); // loop body
        if (i < tokens.length && tokens[i].type === 'each_close') i++;
        ops.push({ op: 'each_close' });
      } else if (t.type === 'each_close') {
        return;
      } else if (t.type === 'if_open') {
        ops.push({ op: 'if_open', condition: t.condition });
        i++;
        walk(); // if body
        if (i < tokens.length && tokens[i].type === 'else') {
          i++;
          ops.push({ op: 'else' });
          walk(); // else body
        }
        if (i < tokens.length && tokens[i].type === 'if_close') i++;
        ops.push({ op: 'if_close' });
      } else if (t.type === 'else' || t.type === 'if_close') {
        return;
      } else {
        i++;
      }
    }
  }

  walk();
  return ops;
}

/**
 * Process attrs — detect binding expressions inside attribute values.
 * e.g. class="msg {{active}}" → [{ text: "msg " }, { bind: "active" }]
 */
function processAttrs(attrs) {
  const result = {};
  for (const [k, v] of Object.entries(attrs)) {
    if (typeof v === 'string' && v.includes('{{')) {
      // Attribute with bindings
      const parts = [];
      let j = 0;
      const vlen = v.length;
      while (j < vlen) {
        const bs = v.indexOf('{{', j);
        if (bs === -1) {
          parts.push({ text: v.slice(j) });
          break;
        }
        if (bs > j) parts.push({ text: v.slice(j, bs) });
        const be = v.indexOf('}}', bs + 2);
        if (be === -1) {
          parts.push({ text: v.slice(j) });
          break;
        }
        parts.push({ bind: v.slice(bs + 2, be).trim() });
        j = be + 2;
      }
      result[k] = parts;
    } else {
      result[k] = v;
    }
  }
  return result;
}

// ── Compiler: full pipeline ────────────────────────────────────────────

/**
 * Compile a .magnetic.html template source to a compiled instruction set.
 *
 * @param {string} src  — template source
 * @param {object} [opts] — options
 * @param {string} [opts.name] — component name
 * @returns {{ name: string, ops: Array }}
 */
export function compile(src, opts = {}) {
  const tokens = tokenize(src);
  const ops = parse(tokens);
  return {
    version: 1,
    name: opts.name || 'anonymous',
    ops
  };
}
