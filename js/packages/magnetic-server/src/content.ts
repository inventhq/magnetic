// @magneticjs/server/content — Content pipeline runtime
// Provides getContent() and listContent() for state.ts to import.
// The __magnetic_content global is set by the CLI-generated bridge code at build time.

declare var globalThis: any;

export interface ContentEntry {
  meta: Record<string, any>;
  html: string;
}

export interface ContentListItem {
  slug: string;
  meta: Record<string, any>;
}

const _store = (): Record<string, ContentEntry> =>
  (typeof globalThis !== 'undefined' && globalThis.__magnetic_content) || {};

/**
 * Get a single content entry by slug.
 * Returns { meta, html } or null if not found.
 */
export function getContent(slug: string): ContentEntry | null {
  return _store()[slug] || null;
}

/**
 * List all content entries, optionally filtered by slug prefix.
 * Returns [{ slug, meta }] sorted by slug.
 */
export function listContent(prefix?: string): ContentListItem[] {
  const store = _store();
  const results: ContentListItem[] = [];
  for (const slug of Object.keys(store)) {
    if (!prefix || slug.indexOf(prefix) === 0) {
      results.push({ slug, meta: store[slug].meta });
    }
  }
  return results;
}
