// 404 page

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';

export function NotFoundPage({ params }: { params: Record<string, string> }) {
  return (
    <div class="not-found" key="404">
      <Head>
        <title>404 Not Found | Magnetic Task Board</title>
      </Head>
      <h1 key="h">404</h1>
      <p key="p">Page not found.</p>
      <p key="link"><Link href="/">← Back to tasks</Link></p>
    </div>
  );
}
