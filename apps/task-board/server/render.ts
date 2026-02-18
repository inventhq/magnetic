// render.ts — Template render engine
// Compiles .magnetic.html ops + state → JSON DOM snapshot

export interface DomNode {
  tag: string;
  key?: string;
  attrs?: Record<string, string>;
  events?: Record<string, string>;
  text?: string;
  children?: DomNode[];
}

interface Op {
  op: string;
  tag?: string;
  key?: string;
  attrs?: any;
  events?: Record<string, string>;
  selfClose?: boolean;
  field?: string;
  value?: string;
  collection?: string;
  item?: string;
  condition?: string;
}

type Scope = Record<string, any>;

// Resolve dot-path field from scope: "task.title" → scope.task.title
function resolveField(field: string, scope: Scope): any {
  const parts = field.split('.');
  let val: any = scope;
  for (const p of parts) {
    if (val == null || typeof val !== 'object') return undefined;
    val = val[p];
  }
  return val;
}

// Resolve {{bindings}} in a string
function resolveString(s: string, scope: Scope): string {
  return s.replace(/\{\{(\w+(?:\.\w+)*)\}\}/g, (_, field) => {
    return String(resolveField(field, scope) ?? '');
  });
}

// Resolve attribute values (may contain binding arrays from compiler)
function resolveAttrValue(value: any, scope: Scope): string {
  if (typeof value === 'string') return resolveString(value, scope);
  if (value === true) return '';
  if (Array.isArray(value)) {
    return value.map((part: any) => {
      if (part.text != null) return part.text;
      if (part.bind) return String(resolveField(part.bind, scope) ?? '');
      return '';
    }).join('');
  }
  return String(value);
}

function resolveAttrs(attrs: any, scope: Scope): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [k, v] of Object.entries(attrs)) {
    result[k] = resolveAttrValue(v, scope);
  }
  return result;
}

function resolveEvents(events: Record<string, string>, scope: Scope): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [k, v] of Object.entries(events)) {
    result[k] = resolveString(v, scope);
  }
  return result;
}

function isPascalCase(s: string): boolean {
  return /^[A-Z]/.test(s);
}

// Find matching close op (respects nesting)
// Note: compiler emits close ops for self-closing elements too, so count ALL opens
function findClose(ops: Op[], start: number): number {
  let depth = 0;
  for (let i = start; i < ops.length; i++) {
    if (ops[i].op === 'open') depth++;
    if (ops[i].op === 'close') {
      if (depth === 0) return i;
      depth--;
    }
  }
  return ops.length;
}

// Find matching block close (each_open/each_close, if_open/if_close)
function findBlockClose(ops: Op[], start: number, openOp: string, closeOp: string): number {
  let depth = 0;
  for (let i = start; i < ops.length; i++) {
    if (ops[i].op === openOp) depth++;
    if (ops[i].op === closeOp) {
      if (depth === 0) return i;
      depth--;
    }
  }
  return ops.length;
}

// Find else and if_close for an if block
function findIfBranches(ops: Op[], start: number): { elseIdx: number; closeIdx: number } {
  let depth = 0;
  let elseIdx = -1;
  for (let i = start; i < ops.length; i++) {
    if (ops[i].op === 'if_open') depth++;
    if (ops[i].op === 'if_close') {
      if (depth === 0) return { elseIdx, closeIdx: i };
      depth--;
    }
    if (ops[i].op === 'else' && depth === 0) elseIdx = i;
  }
  return { elseIdx, closeIdx: ops.length };
}

// Check if a range of ops contains only text/bind (no elements)
function isTextOnly(ops: Op[], start: number, end: number): boolean {
  for (let i = start; i < end; i++) {
    const o = ops[i].op;
    if (o === 'open' || o === 'each_open' || o === 'if_open') return false;
  }
  return true;
}

// Concatenate text/bind ops into a single string
function concatText(ops: Op[], start: number, end: number, scope: Scope): string {
  let text = '';
  for (let i = start; i < end; i++) {
    if (ops[i].op === 'text') text += ops[i].value;
    else if (ops[i].op === 'bind') text += String(resolveField(ops[i].field!, scope) ?? '');
  }
  return text;
}

// Main render: walk ops, build DomNode tree
function renderBlock(
  ops: Op[],
  start: number,
  end: number,
  scope: Scope,
  components: Map<string, Op[]>
): DomNode[] {
  const nodes: DomNode[] = [];
  let i = start;

  while (i < end) {
    const op = ops[i];

    switch (op.op) {
      case 'open': {
        // Component inclusion
        if (isPascalCase(op.tag!) && components.has(op.tag!)) {
          const compOps = components.get(op.tag!)!;
          const compNodes = renderBlock(compOps, 0, compOps.length, scope, components);
          nodes.push(...compNodes);
          if (!op.selfClose) {
            i = findClose(ops, i + 1) + 1;
          } else {
            i++;
          }
          continue;
        }

        const node: DomNode = { tag: op.tag! };
        if (op.key) node.key = resolveString(op.key, scope);
        if (op.attrs) node.attrs = resolveAttrs(op.attrs, scope);
        if (op.events) node.events = resolveEvents(op.events, scope);

        if (op.selfClose) {
          nodes.push(node);
          i++;
          continue;
        }

        // Find matching close
        const closeIdx = findClose(ops, i + 1);
        const childStart = i + 1;

        // If all children are text/bind, merge into node.text
        if (isTextOnly(ops, childStart, closeIdx)) {
          const text = concatText(ops, childStart, closeIdx, scope);
          if (text) node.text = text;
        } else {
          // Mixed content — render children as nodes
          const children = renderBlock(ops, childStart, closeIdx, scope, components);
          if (children.length > 0) node.children = children;
        }

        nodes.push(node);
        i = closeIdx + 1;
        continue;
      }

      case 'text': {
        if (op.value!.trim()) {
          nodes.push({ tag: 'span', text: op.value!.trim() });
        }
        i++;
        continue;
      }

      case 'bind': {
        const val = resolveField(op.field!, scope);
        nodes.push({ tag: 'span', text: String(val ?? '') });
        i++;
        continue;
      }

      case 'each_open': {
        const collection = resolveField(op.collection!, scope);
        const bodyStart = i + 1;
        const closeIdx = findBlockClose(ops, bodyStart, 'each_open', 'each_close');

        if (Array.isArray(collection)) {
          for (let j = 0; j < collection.length; j++) {
            const itemScope = {
              ...scope,
              [op.item!]: collection[j],
              [`${op.item!}_index`]: j,
            };
            const itemNodes = renderBlock(ops, bodyStart, closeIdx, itemScope, components);
            nodes.push(...itemNodes);
          }
        }

        i = closeIdx + 1;
        continue;
      }

      case 'if_open': {
        const val = resolveField(op.condition!, scope);
        const bodyStart = i + 1;
        const { elseIdx, closeIdx } = findIfBranches(ops, bodyStart);

        if (val) {
          const trueEnd = elseIdx !== -1 ? elseIdx : closeIdx;
          const trueNodes = renderBlock(ops, bodyStart, trueEnd, scope, components);
          nodes.push(...trueNodes);
        } else if (elseIdx !== -1) {
          const falseNodes = renderBlock(ops, elseIdx + 1, closeIdx, scope, components);
          nodes.push(...falseNodes);
        }

        i = closeIdx + 1;
        continue;
      }

      default:
        i++;
        continue;
    }
  }

  return nodes;
}

// Public API: render compiled ops with state → JSON DOM snapshot
export function render(
  ops: Op[],
  state: Scope,
  components?: Map<string, Op[]>
): { root: DomNode } {
  const nodes = renderBlock(ops, 0, ops.length, state, components || new Map());
  if (nodes.length === 0) {
    return { root: { tag: 'div', text: '' } };
  }
  if (nodes.length === 1) {
    return { root: nodes[0] };
  }
  // Multiple root nodes — wrap in div
  return { root: { tag: 'div', children: nodes } };
}
