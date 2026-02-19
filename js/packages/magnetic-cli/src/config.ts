// config.ts — Magnetic app configuration parser
// Parses magnetic.json: data sources, action mappings, auth config

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// ── Data source types ───────────────────────────────────────────────

export type DataSourceType = 'fetch' | 'poll' | 'sse' | 'ws';

export interface DataSource {
  /** Key name (used as props.<key>) */
  key: string;
  /** Remote URL to fetch from */
  url: string;
  /** Source type: fetch (default), poll, sse, ws */
  type: DataSourceType;
  /** Poll interval (e.g. "5s", "10s") — only for poll type */
  refresh?: string;
  /** Page scope: "*" for global, "/path" for page-specific */
  page: string;
  /** Whether to inject auth token from session */
  auth: boolean;
}

// ── Action mapping types ────────────────────────────────────────────

export interface ActionMapping {
  /** Action name (matches action dispatched from UI) */
  name: string;
  /** HTTP method */
  method: string;
  /** URL (may contain ${payload.xxx} interpolation) */
  url: string;
  /** Which data key to update with the response (optional) */
  target?: string;
  /** Debounce in ms (optional) */
  debounce?: number;
}

// ── Auth config ─────────────────────────────────────────────────────

export interface AuthConfig {
  /** Provider type: oauth2, oidc, magic-link, otp */
  provider: 'oauth2' | 'oidc' | 'magic-link' | 'otp';
  /** Issuer URL (for oauth2/oidc) */
  issuer?: string;
  /** OAuth client ID (also project ID for Stytch etc.) */
  client_id?: string;
  /** OAuth client secret */
  client_secret?: string;
  /** OAuth scopes */
  scopes?: string[];
  /** Redirect URI for OAuth callback */
  redirect_uri?: string;
  /** Login URL for magic-link/OTP providers (where the initial send request goes) */
  login_url?: string;
  /** Verify URL for magic-link/OTP providers (where tokens/codes are verified) */
  verify_url?: string;
  /** JSON field name containing the session token in verify response (default: "session_token") */
  token_field?: string;
  /** Token lifetime in seconds if provider doesn't return expires_in (default: 3600) */
  token_expires_in?: number;
  /** Session config */
  session?: {
    cookie?: string;
    ttl?: string;
  };
}

// ── Full app config ─────────────────────────────────────────────────

export interface MagneticAppConfig {
  name?: string;
  server?: string;
  auth?: AuthConfig;
  data: DataSource[];
  actions: ActionMapping[];
}

// ── Parser ──────────────────────────────────────────────────────────

function parseRefreshToType(refresh?: string): DataSourceType {
  if (refresh) return 'poll';
  return 'fetch';
}

function parseActionShorthand(name: string, value: string | Record<string, any>): ActionMapping {
  if (typeof value === 'string') {
    // Shorthand: "POST https://api.example.com/todos"
    const parts = value.trim().split(/\s+/);
    if (parts.length < 2) {
      throw new Error(`Invalid action mapping for '${name}': expected "METHOD URL"`);
    }
    return {
      name,
      method: parts[0].toUpperCase(),
      url: parts.slice(1).join(' '),
    };
  }
  // Explicit object config
  return {
    name,
    method: (value.method || 'POST').toUpperCase(),
    url: value.url,
    target: value.target,
    debounce: value.debounce,
  };
}

/**
 * Parse magnetic.json from an app directory.
 * Returns structured config with data sources and action mappings.
 */
export function parseAppConfig(appDir: string): MagneticAppConfig {
  const configPath = join(appDir, 'magnetic.json');

  const result: MagneticAppConfig = {
    data: [],
    actions: [],
  };

  if (!existsSync(configPath)) {
    return result;
  }

  const raw = JSON.parse(readFileSync(configPath, 'utf-8'));

  result.name = raw.name;
  result.server = raw.server;

  // Parse auth
  if (raw.auth) {
    result.auth = {
      provider: raw.auth.provider || 'oidc',
      issuer: raw.auth.issuer,
      client_id: raw.auth.client_id,
      client_secret: raw.auth.client_secret,
      scopes: raw.auth.scopes,
      redirect_uri: raw.auth.redirect_uri || '/auth/callback',
      login_url: raw.auth.login_url,
      verify_url: raw.auth.verify_url,
      token_field: raw.auth.token_field,
      token_expires_in: raw.auth.token_expires_in,
      session: raw.auth.session || { cookie: 'magnetic_session', ttl: '24h' },
    };
  }

  // Parse data sources
  if (raw.data && typeof raw.data === 'object') {
    for (const [key, value] of Object.entries(raw.data)) {
      const src = value as Record<string, any>;
      const explicitType = src.type as DataSourceType | undefined;
      const type = explicitType || parseRefreshToType(src.refresh);

      result.data.push({
        key,
        url: src.url,
        type,
        refresh: src.refresh,
        page: src.page || '*',
        auth: src.auth === true,
      });
    }
  }

  // Parse action mappings
  if (raw.actions && typeof raw.actions === 'object') {
    for (const [name, value] of Object.entries(raw.actions)) {
      result.actions.push(parseActionShorthand(name, value as any));
    }
  }

  return result;
}

/**
 * Serialize config to JSON for embedding in the bridge bundle.
 * Only includes data sources and action mappings (not secrets).
 */
export function serializeConfigForBridge(config: MagneticAppConfig): string {
  return JSON.stringify({
    data: config.data.map(d => ({
      key: d.key,
      page: d.page,
    })),
    actions: config.actions.map(a => a.name),
  });
}

/**
 * Serialize full config for the Rust server.
 * Includes URLs, types, auth flags — everything the server needs to fetch data.
 * Env vars (${env.XXX}) are NOT resolved here — the server resolves them at runtime.
 */
export function serializeConfigForServer(config: MagneticAppConfig): string {
  return JSON.stringify({
    auth: config.auth || null,
    data: config.data,
    actions: config.actions,
  });
}
