// discover.ts — Probe URLs for OpenAPI/Swagger specs

import type { DiscoveryResult, SpecVersion } from './types.ts';

/** Common paths where OpenAPI/Swagger specs are served */
const PROBE_PATHS = [
  '/openapi.json',
  '/swagger.json',
  '/api-docs',
  '/api/openapi.json',
  '/api/swagger.json',
  '/v1/openapi.json',
  '/v2/openapi.json',
  '/v3/openapi.json',
  '/docs/openapi.json',
  '/.well-known/openapi.json',
];

/** Detect spec version from parsed JSON */
function detectVersion(spec: any): SpecVersion | null {
  if (typeof spec !== 'object' || spec === null) return null;
  if (typeof spec.openapi === 'string' && spec.openapi.startsWith('3.')) return 'openapi3';
  if (spec.swagger === '2.0') return 'swagger2';
  return null;
}

/** Extract base URL from a data source URL */
function getBaseUrl(url: string): string {
  try {
    const u = new URL(url);
    return `${u.protocol}//${u.host}`;
  } catch {
    return url;
  }
}

/**
 * Probe a single URL for an OpenAPI/Swagger spec.
 * Returns the parsed spec if found, null otherwise.
 */
async function probeUrl(url: string, timeoutMs: number = 5000): Promise<{ spec: any; version: SpecVersion } | null> {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);

    const res = await fetch(url, {
      signal: controller.signal,
      headers: { 'Accept': 'application/json' },
    });
    clearTimeout(timer);

    if (!res.ok) return null;

    const contentType = res.headers.get('content-type') || '';
    if (!contentType.includes('json') && !contentType.includes('yaml')) {
      // Try parsing as JSON anyway — some servers don't set content-type
      const text = await res.text();
      try {
        const spec = JSON.parse(text);
        const version = detectVersion(spec);
        if (version) return { spec, version };
      } catch {
        return null;
      }
      return null;
    }

    const spec = await res.json();
    const version = detectVersion(spec);
    if (version) return { spec, version };
    return null;
  } catch {
    return null;
  }
}

/**
 * Discover an OpenAPI/Swagger spec for a given data source URL.
 * Probes common paths on the same host.
 *
 * @param dataSourceUrl — A URL from magnetic.json data sources (e.g. "https://api.example.com/users")
 * @param timeoutMs — Per-probe timeout in milliseconds (default: 5000)
 */
export async function discover(dataSourceUrl: string, timeoutMs: number = 5000): Promise<DiscoveryResult> {
  const baseUrl = getBaseUrl(dataSourceUrl);

  // Probe all paths in parallel
  const probes = PROBE_PATHS.map(async (path) => {
    const url = baseUrl + path;
    const result = await probeUrl(url, timeoutMs);
    if (result) return { url, ...result };
    return null;
  });

  const results = await Promise.all(probes);

  for (const r of results) {
    if (r) {
      return {
        found: true,
        specUrl: r.url,
        version: r.version,
        spec: r.spec,
      };
    }
  }

  return { found: false, error: `No OpenAPI/Swagger spec found at ${baseUrl}` };
}

/**
 * Discover specs for multiple data source URLs.
 * Deduplicates by base URL so we don't probe the same host twice.
 */
export async function discoverAll(urls: string[], timeoutMs: number = 5000): Promise<Map<string, DiscoveryResult>> {
  const results = new Map<string, DiscoveryResult>();
  const seen = new Set<string>();

  for (const url of urls) {
    const base = getBaseUrl(url);
    if (seen.has(base)) continue;
    seen.add(base);

    const result = await discover(url, timeoutMs);
    results.set(base, result);
  }

  return results;
}
