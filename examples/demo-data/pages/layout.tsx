import { Head } from '@magneticjs/server';

export default function RootLayout({ children, path }: { children: any; path: string }) {
  return (
    <div class="app-shell">
      <Head>
        <title>Demo Data App</title>
        <meta name="description" content="Magnetic demo with layouts" />
      </Head>
      <nav class="main-nav">
        <a href="/" class={path === '/' ? 'active' : ''}>Home</a>
        <a href="/users" class={path === '/users' ? 'active' : ''}>Users</a>
      </nav>
      <main class="content">
        {children}
      </main>
      <footer>Powered by Magnetic</footer>
    </div>
  );
}
