---
title: Platform API
order: 9
---

# Chapter 9: Platform API

**Account management, authentication, databases, teams, and billing.**

Base URL: `https://machx.dev` (production) or `http://localhost:3000` (local dev)

Database instances live at `https://{subdomain}.machx.dev` — see [Database Management](#database-management) below.

---

## Authentication

BitBin uses passwordless OTP authentication via email. The flow produces a **session token** for interactive use and an **API key** for programmatic access.

### POST /auth/otp/send — Send OTP

```bash
curl -X POST https://machx.dev/auth/otp/send \
  -H 'Content-Type: application/json' \
  -d '{"email": "you@example.com"}'
```

Response:
```json
{"email_id": "email-test-xxxx", "status": "sent"}
```

### POST /auth/otp/verify — Verify OTP

```bash
curl -X POST https://machx.dev/auth/otp/verify \
  -H 'Content-Type: application/json' \
  -d '{"email_id": "email-test-xxxx", "code": "123456"}'
```

**New user** (no account yet):
```json
{
  "authenticated": true,
  "email": "you@example.com",
  "needs_signup": true
}
```

**Returning user** (account exists):
```json
{
  "authenticated": true,
  "email": "you@example.com",
  "needs_signup": false,
  "account_id": "my-company",
  "name": "My Company",
  "session_token": "ses_...",
  "expires_in": 86400
}
```

### POST /signup — Create Account

Called after OTP verify when `needs_signup: true`.

```bash
curl -X POST https://machx.dev/signup \
  -H 'Content-Type: application/json' \
  -d '{
    "account_id": "my-company",
    "name": "My Company",
    "email": "you@example.com"
  }'
```

Response:
```json
{
  "account_id": "my-company",
  "name": "My Company",
  "email": "you@example.com",
  "session_token": "ses_...",
  "api_key": "sk_my-company_...",
  "expires_in": 86400
}
```

> **Save the `api_key`** — it is only shown once.

### POST /login — API Key Login

Exchange an API key for a session token (programmatic/SDK use).

```bash
curl -X POST https://machx.dev/login \
  -H 'Content-Type: application/json' \
  -d '{"api_key": "sk_my-company_..."}'
```

Response:
```json
{
  "client_id": "my-company",
  "session_token": "ses_...",
  "role": "admin",
  "expires_in": 86400
}
```

### POST /logout — Revoke Session

```bash
curl -X POST https://machx.dev/logout \
  -H 'Authorization: Bearer ses_...'
```

### Auth Headers

All authenticated endpoints accept:

```
Authorization: Bearer ses_...   (session token)
Authorization: Bearer sk_...    (API key)
```

Session tokens expire after 24 hours. API keys do not expire but can be revoked.

---

## Account Management

### GET /accounts/:id — Get Account

```bash
curl https://machx.dev/accounts/my-company \
  -H 'Authorization: Bearer ses_...'
```

Response:
```json
{
  "account_id": "my-company",
  "name": "My Company",
  "email": "you@example.com",
  "created_at": 1709568000
}
```

### PATCH /accounts/:id — Update Account

```bash
curl -X PATCH https://machx.dev/accounts/my-company \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"name": "New Company Name"}'
```

### DELETE /accounts/:id — Delete Account

Cascade deletes all databases, members, API keys, settings, and product entitlements.

```bash
curl -X DELETE https://machx.dev/accounts/my-company \
  -H 'Authorization: Bearer ses_...'
```

Response:
```json
{"account_id": "my-company", "deleted": true}
```

---

## Database Management

Each database gets a unique **subdomain** (12-character UUID). All data operations (query, ingest, subscribe, WS) go to `https://{subdomain}.machx.dev`.

### POST /accounts/:id/databases — Create Database

```bash
curl -X POST https://machx.dev/accounts/my-company/databases \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"db_id": "analytics-db", "db_name": "Analytics", "tenant_count": 10}'
```

Response:
```json
{
  "account_id": "my-company",
  "db_id": "analytics-db",
  "db_name": "Analytics",
  "subdomain": "edba2ddef755",
  "active": true,
  "tenant_lo": 0,
  "tenant_hi": 10,
  "api_key": "sk_analytics-db_...",
  "key_id": "db-analytics-db",
  "endpoints": {
    "connection_url": "https://edba2ddef755.machx.dev",
    "rest": "https://edba2ddef755.machx.dev/query",
    "ws": "wss://edba2ddef755.machx.dev/ws",
    "sse": "https://edba2ddef755.machx.dev/subscribe",
    "ingest": "https://edba2ddef755.machx.dev/ingest",
    "pipes": "https://edba2ddef755.machx.dev/pipe"
  },
  "note": "Save the api_key — it will not be shown again.",
  "created_at": 1709568000
}
```

- **`subdomain`** — Your database's unique identifier, used in all endpoint URLs.
- **`tenant_lo` / `tenant_hi`** — The tenant range allocated to this database.
- **`api_key`** — Scoped to this database only. Save it — shown once.

### GET /accounts/:id/databases — List Databases

```bash
curl "https://machx.dev/accounts/my-company/databases?page=1&per_page=10" \
  -H 'Authorization: Bearer ses_...'
```

Response:
```json
{
  "account_id": "my-company",
  "databases": [
    {
      "db_id": "analytics-db",
      "db_name": "Analytics",
      "subdomain": "edba2ddef755",
      "active": true,
      "tenant_lo": 0,
      "tenant_hi": 10,
      "created_at": 1709568000
    }
  ],
  "total": 1,
  "page": 1,
  "total_pages": 1
}
```

### GET /accounts/:id/databases/:db_id — Get Database

Returns database details with live record count and per-DB metrics.

```bash
curl https://machx.dev/accounts/my-company/databases/analytics-db \
  -H 'Authorization: Bearer ses_...'
```

### PATCH /accounts/:id/databases/:db_id — Update Database

```bash
curl -X PATCH https://machx.dev/accounts/my-company/databases/analytics-db \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"db_name": "Renamed Analytics"}'
```

### DELETE /accounts/:id/databases/:db_id — Deactivate Database

Soft-deactivates the database (data is retained).

```bash
curl -X DELETE https://machx.dev/accounts/my-company/databases/analytics-db \
  -H 'Authorization: Bearer ses_...'
```

---

## Team Members

Invite collaborators to your account with role-based access.

### Roles

| Role | Permissions |
|------|------------|
| **admin** | Full access — manage account, databases, members, settings, billing |
| **writer** | Read + write data, create pipes/pipelines, manage own API keys |
| **reader** | Read-only access to data and pipes |

### POST /accounts/:id/members — Invite Member

```bash
curl -X POST https://machx.dev/accounts/my-company/members \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"email": "teammate@example.com", "name": "Jane Doe", "role": "writer"}'
```

Response:
```json
{
  "member_id": "mem_my-company_teammate_example_com",
  "email": "teammate@example.com",
  "name": "Jane Doe",
  "role": "writer",
  "invited_by": "my-company",
  "api_key": "mkey_...",
  "created_at": 1709568000
}
```

The invited member uses the returned `api_key` to log in via `POST /login`.

### GET /accounts/:id/members — List Members

```bash
curl "https://machx.dev/accounts/my-company/members?page=1&per_page=20" \
  -H 'Authorization: Bearer ses_...'
```

### GET /accounts/:id/members/:member_id — Get Member

```bash
curl https://machx.dev/accounts/my-company/members/mem_my-company_teammate_example_com \
  -H 'Authorization: Bearer ses_...'
```

### PATCH /accounts/:id/members/:member_id — Update Role

```bash
curl -X PATCH https://machx.dev/accounts/my-company/members/mem_my-company_teammate_example_com \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"role": "reader"}'
```

### DELETE /accounts/:id/members/:member_id — Remove Member

```bash
curl -X DELETE https://machx.dev/accounts/my-company/members/mem_my-company_teammate_example_com \
  -H 'Authorization: Bearer ses_...'
```

---

## API Keys

### POST /api-keys — Create API Key

```bash
curl -X POST https://machx.dev/api-keys \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"client_id": "my-company", "role": "writer"}'
```

### GET /api-keys — List API Keys

```bash
curl "https://machx.dev/api-keys?page=1&per_page=10" \
  -H 'Authorization: Bearer ses_...'
```

Response:
```json
{
  "keys": [
    {
      "key_id": "acct_5a3d0e2c6812",
      "client_id": "my-company",
      "role": "admin",
      "key_hash_prefix": "5a3d0e2c",
      "created_at": 1709568000
    }
  ],
  "total": 1,
  "page": 1,
  "total_pages": 1
}
```

Keys are identified by `key_id` and `key_hash_prefix`. The full key value is never stored or returned after creation.

### DELETE /api-keys/:id — Revoke API Key

```bash
curl -X DELETE https://machx.dev/api-keys/acct_5a3d0e2c6812 \
  -H 'Authorization: Bearer ses_...'
```

---

## Account Settings

### GET /accounts/:id/settings — Get Settings

```bash
curl https://machx.dev/accounts/my-company/settings \
  -H 'Authorization: Bearer ses_...'
```

### PATCH /accounts/:id/settings — Update Settings

```bash
curl -X PATCH https://machx.dev/accounts/my-company/settings \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"display_name": "My Workspace", "timezone": "America/Los_Angeles"}'
```

---

## Product Entitlements

Each account has a set of product entitlements that control which features are available.

### Default Products

| Product | Default | Description |
|---------|---------|-------------|
| `dbengine` | Enabled (free) | Database engine — query, ingest, CRUD |
| `vector` | Enabled (free) | Vector similarity search |
| `realtime` | Enabled (free) | Real-time subscriptions (SSE + WS) |
| `pipelines` | Disabled | Atomic pipelines (stored procedures) |
| `graph` | Disabled | Hypergraph engine |

### GET /accounts/:id/products — List Entitlements

```bash
curl https://machx.dev/accounts/my-company/products \
  -H 'Authorization: Bearer ses_...'
```

Response:
```json
{
  "account_id": "my-company",
  "products": [
    {
      "product_id": "dbengine",
      "product_name": "Database Engine",
      "enabled": true,
      "plan": "free",
      "limits": {
        "max_databases": 3,
        "max_records_per_db": 100000,
        "max_queries_per_day": 10000,
        "max_members": 5
      },
      "activated_at": 1709568000
    }
  ]
}
```

### PATCH /accounts/:id/products/:product_id — Update Entitlement

Enable a product or change plan:

```bash
curl -X PATCH https://machx.dev/accounts/my-company/products/pipelines \
  -H 'Authorization: Bearer ses_...' \
  -H 'Content-Type: application/json' \
  -d '{"enabled": true, "plan": "starter"}'
```

---

## Pagination

All list endpoints support pagination:

```
?page=1&per_page=50
```

- **`page`** — Page number (1-indexed, default: 1)
- **`per_page`** — Items per page (default: 50, max: 100)

Response always includes:

```json
{
  "total": 42,
  "page": 1,
  "total_pages": 1
}
```

Paginated endpoints: `/accounts`, `/api-keys`, `/accounts/:id/databases`, `/accounts/:id/members`.

---

## Error Responses

All errors return JSON with an `error` field:

```json
{"error": "description of what went wrong"}
```

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `201` | Created |
| `400` | Bad request (malformed JSON, missing fields) |
| `401` | Unauthorized (bad API key, expired OTP) |
| `403` | Forbidden (missing auth header, insufficient role) |
| `404` | Not found (account, database, member doesn't exist) |
| `409` | Conflict (duplicate account_id, duplicate db_id) |
| `422` | Unprocessable (validation error, e.g. missing required fields) |
| `429` | Rate limited |
| `500` | Internal server error |
| `502` | Upstream error (Stytch OTP service unavailable) |

### Common Error Examples

```json
// 401 — Bad OTP
{"error": "invalid OTP code"}

// 403 — No auth header
{"error": "admin required"}

// 409 — Duplicate account
{"error": "account_id already exists"}

// 422 — Missing field
{"error": "missing field `account_id`"}
```

---

## CORS

The API allows all origins (`Access-Control-Allow-Origin: *`). No preflight configuration needed for browser-based clients.

---

[← Previous: Examples & Recipes](/examples) · **Chapter 9** · [Next: API Reference →](/api-reference)
