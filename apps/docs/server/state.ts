// state.ts — Docs app state management
// Uses the content pipeline: __content map is injected at build time

import { getContent, listContent } from '@magneticjs/server/content';

export interface DocsState {
  currentSlug: string;
}

export function initialState(): DocsState {
  return { currentSlug: 'getting-started' };
}

export function reduce(state: DocsState, action: string, payload: any): DocsState {
  if (action === 'navigate' && payload?.path) {
    const path = payload.path as string;
    const slug = path === '/' || path === '' ? 'getting-started' : path.replace(/^\//, '').replace(/\/$/, '');
    return { ...state, currentSlug: slug };
  }

  return state;
}

export function toViewModel(state: DocsState, path?: string) {
  // Derive slug from the request path (server updates session path on navigate)
  const slug = path && path !== '/'
    ? path.replace(/^\//, '').replace(/\/$/, '')
    : state.currentSlug;

  const allDocs = listContent();
  const sorted = allDocs.sort((a, b) => (a.meta.order || 99) - (b.meta.order || 99));

  const current = getContent(slug);

  const sidebar = sorted.map(doc => ({
    slug: doc.slug,
    title: doc.meta.title || doc.slug,
    active: doc.slug === slug,
    href: doc.slug,
  }));

  return {
    sidebar,
    currentSlug: slug,
    title: current?.meta?.title || 'Magnetic Docs',
    description: current?.meta?.description || '',
    contentHtml: current?.html || '<p>Page not found.</p>',
  };
}
