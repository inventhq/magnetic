// @magneticjs/server/content — Content pipeline runtime
// Provides getContent() and listContent() for state.ts to import.
//
// Two modes:
// 1. Bundle mode: __magnetic_content has full { meta, html } for every slug
// 2. Disk mode: __magnetic_content_index has { meta } only,
//    __magnetic_content_load(slug) loads { meta, html } on demand

declare var globalThis: any;

export interface ContentEntry {
  meta: Record<string, any>;
  html: string;
}

export interface ContentListItem {
  slug: string;
  meta: Record<string, any>;
}

// Bundle mode store (full content)
const _store = (): Record<string, ContentEntry> =>
  (typeof globalThis !== 'undefined' && globalThis.__magnetic_content) || {};

// Disk mode index (metadata only)
const _index = (): Record<string, { meta: Record<string, any> }> | null =>
  (typeof globalThis !== 'undefined' && globalThis.__magnetic_content_index) || null;

// Disk mode loader (injected by prerender or Rust host)
const _loader = (): ((slug: string) => ContentEntry | null) | null =>
  (typeof globalThis !== 'undefined' && globalThis.__magnetic_content_load) || null;

/**
 * Get a single content entry by slug.
 * Returns { meta, html } or null if not found.
 *
 * In disk mode, calls __magnetic_content_load(slug) to load on demand.
 * In bundle mode, reads from __magnetic_content directly.
 */
export function getContent(slug: string): ContentEntry | null {
  // Bundle mode: full content available
  const store = _store();
  if (store[slug]) return store[slug];

  // Disk mode: load on demand
  const loader = _loader();
  if (loader) return loader(slug);

  return null;
}

/**
 * List all content entries, optionally filtered by slug prefix.
 * Returns [{ slug, meta }] sorted by slug.
 *
 * In disk mode, uses the lightweight index (no HTML loaded).
 * In bundle mode, reads from the full content store.
 */
export function listContent(prefix?: string): ContentListItem[] {
  // Disk mode: use lightweight index
  const index = _index();
  if (index) {
    const results: ContentListItem[] = [];
    for (const slug of Object.keys(index)) {
      if (!prefix || slug.indexOf(prefix) === 0) {
        results.push({ slug, meta: index[slug].meta });
      }
    }
    return results;
  }

  // Bundle mode: derive from full content store
  const store = _store();
  const results: ContentListItem[] = [];
  for (const slug of Object.keys(store)) {
    if (!prefix || slug.indexOf(prefix) === 0) {
      results.push({ slug, meta: store[slug].meta });
    }
  }
  return results;
}
