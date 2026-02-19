// AboutPage — /about route

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';

export function AboutPage({ params }: { params: Record<string, string> }) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg board" key="about">
        <Head>
          <title>About | Magnetic Task Board</title>
          <meta name="description" content="Learn about the Magnetic framework — server-driven UI with TSX components" />
        </Head>

        <nav class="row gap-md justify-center" key="nav">
          <Link href="/" class="nav-link text-sm medium fg-muted transition-colors" prefetch>Tasks</Link>
          <Link href="/about" class="nav-link text-sm medium fg-primary" prefetch>About</Link>
        </nav>

        <div class="stack gap-sm leading-relaxed" key="content">
          <h1 class="text-2xl bold fg-heading" key="h">About Magnetic</h1>
          <p class="fg-subtle" key="p1">A server-driven UI framework where the server owns all state.</p>
          <p class="fg-subtle" key="p2">Components are pure TSX functions that produce JSON DOM descriptors.</p>
          <p class="fg-subtle" key="p3">The client runtime is 2KB (Brotli) — no framework, no hydration, no virtual DOM.</p>

          <h2 class="text-lg semibold fg-text mt-md" key="h2">Architecture</h2>
          <ul class="stack gap-xs pl-lg fg-subtle" key="list">
            <li key="l1">TSX components → JSON DOM snapshots</li>
            <li key="l2">POST → response (single round-trip)</li>
            <li key="l3">SSE broadcast for multi-client sync</li>
            <li key="l4">WASM transport for dedup (1,129 bytes)</li>
            <li key="l5">Deploys as a single JS file to V8 isolates</li>
          </ul>

          <p class="mt-md" key="p4">
            <Link href="/" class="about-link" prefetch>← Back to tasks</Link>
          </p>
        </div>
      </div>
    </div>
  );
}
