# @magnetic/server

Server-driven UI framework for Magnetic. Provides the JSX runtime that transforms TSX into JSON DOM descriptors, plus routing and SSR utilities.

## Installation

```bash
npm install @magnetic/server
```

## Usage

### JSX Runtime (pages & components)

```tsx
// tsconfig.json: { "jsx": "react-jsx", "jsxImportSource": "@magnetic/server" }
import { Head, Link } from '@magnetic/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div key="app">
      <Head><title>My App</title></Head>
      <h1>{props.title}</h1>
      <Link href="/about">About</Link>
      <button onClick="do_something">Click me</button>
    </div>
  );
}
```

### Router

```ts
import { createRouter } from '@magnetic/server/router';

const router = createRouter([
  { path: '/', page: IndexPage },
  { path: '/about', page: AboutPage },
  { path: '*', page: NotFoundPage },
]);

const result = router.resolve('/about', viewModel);
// → { kind: 'render', dom: DomNode }
```

## Key Concepts

- **DomNode**: JSON DOM descriptor `{ tag, key?, attrs?, events?, text?, children? }`
- **Events**: `onClick`, `onSubmit`, `onInput` → action names (strings, not callbacks)
- **Head**: Declares `<title>` and `<meta>` tags for SSR
- **Link**: Client-side navigation without page reload
- **Fragment**: Groups children without a wrapper element

## Exports

| Path | Description |
|------|-------------|
| `@magnetic/server` | Core index |
| `@magnetic/server/jsx-runtime` | JSX factory, Head, Link, Fragment, DomNode |
| `@magnetic/server/router` | createRouter, route matching |
| `@magnetic/server/ssr` | render_page, PageOptions |

## License

MIT
