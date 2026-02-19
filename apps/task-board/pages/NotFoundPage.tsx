// 404 page

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';

export function NotFoundPage({ params }: { params: Record<string, string> }) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <div class="stack gap-sm items-center w-full bg-raised border round-lg p-2xl shadow-lg text-center board" key="404">
        <Head>
          <title>404 Not Found | Magnetic Task Board</title>
        </Head>
        <h1 class="text-5xl extrabold fg-primary" key="h">404</h1>
        <p class="fg-muted" key="p">Page not found.</p>
        <p key="link"><Link href="/" class="about-link" prefetch>← Back to tasks</Link></p>
      </div>
    </div>
  );
}
