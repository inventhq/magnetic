// AboutPage — /about route

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';

export function AboutPage({ params }: { params: Record<string, string> }) {
  return (
    <div class="about-page" key="about">
      <Head>
        <title>About | Magnetic Task Board</title>
        <meta name="description" content="Learn about the Magnetic framework — server-driven UI with TSX components" />
      </Head>

      <nav class="topnav" key="nav">
        <Link href="/" class="nav-link">Tasks</Link>
        <Link href="/about" class="nav-link active">About</Link>
      </nav>

      <div class="content" key="content">
        <h1 key="h">About Magnetic</h1>
        <p key="p1">A server-driven UI framework where the server owns all state.</p>
        <p key="p2">Components are pure TSX functions that produce JSON DOM descriptors.</p>
        <p key="p3">The client runtime is 2KB (Brotli) — no framework, no hydration, no virtual DOM.</p>

        <h2 key="h2">Architecture</h2>
        <ul key="list">
          <li key="l1">TSX components → JSON DOM snapshots</li>
          <li key="l2">POST → response (single round-trip)</li>
          <li key="l3">SSE broadcast for multi-client sync</li>
          <li key="l4">WASM transport for dedup (1,129 bytes)</li>
          <li key="l5">Deploys as a single JS file to V8 isolates</li>
        </ul>

        <p key="p4">
          <Link href="/">← Back to tasks</Link>
        </p>
      </div>
    </div>
  );
}
