// @magnetic/server JSX runtime
// Transforms TSX → DomNode (JSON DOM descriptors for Magnetic)

export interface DomNode {
  tag: string;
  key?: string;
  attrs?: Record<string, string>;
  events?: Record<string, string>;
  text?: string;
  children?: DomNode[];
}

type Child = DomNode | string | number | boolean | null | undefined | Child[];

interface Props {
  key?: string;
  children?: Child | Child[];
  [prop: string]: unknown;
}

type Component = (props: any) => DomNode;

// ── Built-in components ─────────────────────────────────────────────

/**
 * Client-side navigation link. Renders an <a> that magnetic.js intercepts
 * to do pushState + send navigate action (no full page reload).
 */
export function Link(props: { href: string; prefetch?: boolean; children?: Child | Child[]; class?: string; [k: string]: unknown }): DomNode {
  const { href, prefetch, children, ...rest } = props;
  const extra: Record<string, unknown> = { ...rest, href, onClick: `navigate:${href}` };
  if (prefetch) extra['data-prefetch'] = href;
  return jsx('a', { ...extra, children }, undefined);
}

/**
 * Declares <head> elements (title, meta, link, etc.) from within a page component.
 * During SSR, these are extracted and placed into the document <head>.
 * During live updates, they are ignored (head is static after SSR).
 *
 * Usage:
 *   <Head><title>My Page</title><meta name="description" content="..." /></Head>
 */
export function Head({ children }: { children?: Child | Child[] }): DomNode {
  const flat = flattenChildren(children);
  return { tag: 'magnetic:head', children: flat };
}

// Event prop prefix → event name mapping
const EVENT_MAP: Record<string, string> = {
  onClick: 'click',
  onSubmit: 'submit',
  onInput: 'input',
  onChange: 'change',
  onFocus: 'focus',
  onBlur: 'blur',
  onKeyDown: 'keydown',
  onKeyUp: 'keyup',
  onScroll: 'scroll',
};

function flattenChildren(raw: Child | Child[]): DomNode[] {
  if (raw == null || raw === false || raw === true) return [];
  if (typeof raw === 'string' || typeof raw === 'number') {
    return [{ tag: 'span', text: String(raw) }];
  }
  if (Array.isArray(raw)) {
    const out: DomNode[] = [];
    for (const c of raw) {
      out.push(...flattenChildren(c));
    }
    return out;
  }
  return [raw as DomNode];
}

export function jsx(tag: string | Component, props: Props, key?: string | number): DomNode {
  const { children, ...rest } = props;

  // Component function — call it with props (including children)
  if (typeof tag === 'function') {
    return tag({ ...rest, key, children });
  }

  // HTML element
  const node: DomNode = { tag };
  if (key != null) node.key = String(key);

  const attrs: Record<string, string> = {};
  const events: Record<string, string> = {};

  for (const [k, v] of Object.entries(rest)) {
    if (v == null || v === false) continue;

    // Event props
    if (EVENT_MAP[k]) {
      events[EVENT_MAP[k]] = String(v);
      continue;
    }

    // class prop → attrs.class
    if (k === 'class' || k === 'className') {
      const cls = String(v).trim();
      if (cls) attrs['class'] = cls;
      continue;
    }

    // Boolean attributes
    if (v === true) {
      attrs[k] = '';
      continue;
    }

    attrs[k] = String(v);
  }

  if (Object.keys(attrs).length) node.attrs = attrs;
  if (Object.keys(events).length) node.events = events;

  // Children
  if (children != null) {
    // Single string/number child → text property (no wrapper span)
    if (typeof children === 'string' || typeof children === 'number') {
      node.text = String(children);
    } else {
      const flat = flattenChildren(children);
      // If all children are text spans, merge into single text
      if (flat.length === 1 && flat[0].tag === 'span' && flat[0].text != null && !flat[0].key) {
        node.text = flat[0].text;
      } else if (flat.length > 0) {
        node.children = flat;
      }
    }
  }

  return node;
}

// jsxs = jsx with static children array (same implementation, key is 3rd arg)
export const jsxs = jsx;

// Fragment — returns children as-is (for use inside other elements)
export function Fragment({ children }: { children?: Child | Child[] }): DomNode {
  const flat = flattenChildren(children);
  if (flat.length === 1) return flat[0];
  return { tag: 'div', children: flat };
}

// ── JSX namespace for TypeScript ────────────────────────────────────

type Booleanish = boolean | 'true' | 'false';

interface HtmlAttributes {
  key?: string | number;
  class?: string;
  className?: string;
  id?: string;
  style?: string;
  tabIndex?: number;
  role?: string;
  title?: string;
  hidden?: Booleanish;
  'data-key'?: string;

  // Events (action names, not callbacks)
  onClick?: string;
  onSubmit?: string;
  onInput?: string;
  onChange?: string;
  onFocus?: string;
  onBlur?: string;
  onKeyDown?: string;
  onKeyUp?: string;
  onScroll?: string;

  children?: Child | Child[];
  [attr: string]: unknown;
}

interface InputAttributes extends HtmlAttributes {
  type?: string;
  name?: string;
  value?: string;
  placeholder?: string;
  disabled?: boolean;
  readonly?: boolean;
  required?: boolean;
  autocomplete?: string;
  autofocus?: boolean;
  checked?: boolean;
}

interface FormAttributes extends HtmlAttributes {
  action?: string;
  method?: string;
  novalidate?: boolean;
}

export declare namespace JSX {
  type Element = DomNode;
  interface IntrinsicElements {
    div: HtmlAttributes;
    span: HtmlAttributes;
    p: HtmlAttributes;
    h1: HtmlAttributes;
    h2: HtmlAttributes;
    h3: HtmlAttributes;
    h4: HtmlAttributes;
    h5: HtmlAttributes;
    h6: HtmlAttributes;
    a: HtmlAttributes & { href?: string; target?: string; rel?: string };
    button: HtmlAttributes & { type?: string; disabled?: boolean };
    form: FormAttributes;
    input: InputAttributes;
    textarea: HtmlAttributes & { name?: string; placeholder?: string; rows?: number };
    select: HtmlAttributes & { name?: string };
    option: HtmlAttributes & { value?: string; selected?: boolean };
    label: HtmlAttributes & { for?: string };
    img: HtmlAttributes & { src?: string; alt?: string; width?: number; height?: number; loading?: string };
    meta: HtmlAttributes & { name?: string; content?: string; property?: string; charset?: string; 'http-equiv'?: string };
    title: HtmlAttributes;
    link: HtmlAttributes & { rel?: string; href?: string; type?: string; sizes?: string; media?: string };
    ul: HtmlAttributes;
    ol: HtmlAttributes;
    li: HtmlAttributes;
    nav: HtmlAttributes;
    header: HtmlAttributes;
    footer: HtmlAttributes;
    main: HtmlAttributes;
    section: HtmlAttributes;
    article: HtmlAttributes;
    aside: HtmlAttributes;
    strong: HtmlAttributes;
    em: HtmlAttributes;
    code: HtmlAttributes;
    pre: HtmlAttributes;
    hr: HtmlAttributes;
    br: HtmlAttributes;
    table: HtmlAttributes;
    thead: HtmlAttributes;
    tbody: HtmlAttributes;
    tr: HtmlAttributes;
    th: HtmlAttributes;
    td: HtmlAttributes;
  }
}
