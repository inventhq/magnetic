import { Head } from '@magneticjs/server/jsx-runtime';

export function IndexPage(props: any) {
  return (
    <div class="row min-h-screen" key="page">
      <Head>
        <title>{props.title} — BitBin Docs</title>
        <meta name="description" content={props.description} />
      </Head>

      <aside class="docs-sidebar" key="sidebar">
        <div class="stack gap-xs p-lg border-b" key="logo">
          <h1 class="text-xl bold fg-heading" key="brand">BitBin</h1>
          <span class="text-xs fg-muted" key="tagline">Real-Time Database Platform</span>
        </div>
        <nav class="stack gap-xs p-md" key="nav">
          {props.sidebar.map((item: any) => (
            <a
              href={item.href}
              class={item.active ? 'nav-link active' : 'nav-link'}
              onClick={`navigate:/${item.slug}`}
              key={`nav-${item.slug}`}
            >
              {item.title}
            </a>
          ))}
        </nav>
        <div class="stack gap-sm p-md border-t" key="links">
          <span class="text-xs fg-muted bold" key="ext-label">Links</span>
          <a href="https://github.com/inventhq/magnetic" class="nav-link" key="github">GitHub</a>
        </div>
      </aside>

      <main class="docs-main" key="main">
        <div
          class="docs-content"
          key="content"
          dangerouslySetInnerHTML={{ __html: props.contentHtml }}
        />
      </main>
    </div>
  );
}
