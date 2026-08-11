# NetRail API error codes

All API errors return JSON with three fields:

```json
{
  "code": "QUERY_INVALID",
  "detail": "Query must be 1-500 characters.",
  "status": 400
}
```

The frontend branches on `code`; `detail` is human-readable. Rust (Tauri / `netrail-api`) and Python (Docker / Flatpak fallback) both use this shape as of v1.1.0.

## Client errors (4xx)

| Code | HTTP | When |
|------|------|------|
| `QUERY_INVALID` | 400 | Search query empty or longer than 500 characters |
| `OPEN_URL_INVALID` | 400 | URL is not http/https |
| `OPEN_URL_INVALID_SCHEME` | 400 | Blocked scheme (javascript, data, file, …) |
| `OPEN_URL_CREDENTIALS` | 400 | Embedded username/password in URL |
| `OPEN_URL_NO_HOST` | 400 | Missing hostname |
| `OPEN_URL_LOCALHOST` | 400 | Loopback / localhost open blocked (incl. decimal/hex/octal/short forms) |
| `OPEN_URL_DNS_REBINDING` | 400 | nip.io / sslip.io / xip.io / localtest.me hostname |
| `OPEN_URL_LINK_LOCAL` | 400 | Link-local IP |
| `OPEN_URL_PRIVATE` | 400 | Private / non-public IP (RFC1918, ULA, multicast, …) |
| `OPEN_URL_DNS_UNRESOLVABLE` | 400 | Hostname does not resolve (fails closed before spawn) |
| `OPEN_URL_CLOUD_METADATA` | 400 | Cloud metadata hostname (e.g. `metadata.google.internal`) |
| `COLLECTION_ITEM_NOTES_INVALID` | 400 | Collection item notes longer than 2000 characters |
| `REQUEST_INVALID` | 400 | Generic request validation failure (malformed body/query params) — both stacks |
| `RATE_LIMITED` | 429 | Too many search/open/mutation calls in a 60s window (disable with `NETRAIL_RATE_LIMIT=0`) |
| `SETTINGS_CONFLICT` | 409 | `If-Match` on `PUT /api/settings` doesn't match the current settings `ETag` (settings changed since read; re-fetch and retry) |
| `READONLY_MODE` | 403 | `NETRAIL_READONLY=1` — mutations (settings PUT, history delete/purge, collection create/add) rejected. Note that search/visit history logging is still performed in this mode to preserve audit logs. |
| `AUTH_REQUIRED` | 401 | `NETRAIL_API_TOKEN` set but Bearer / `X-NetRail-Token` missing or wrong |
| `BACKEND_URL_STRICT_PRIVATE` | 400 | `strict_backend_urls` rejected private/loopback backend host |
| `OPEN_URL_REDIRECT_DEPTH` | 400 | Too many DDG redirect unwraps |
| `CONFIG_MAX_RESULTS` | 400 | `max_results` not in 1–50 |
| `CONFIG_HISTORY_TTL` | 400 | `history_ttl_days` over 3650 |
| `CONFIG_SEARCH_STRATEGY` | 400 | Strategy not `fanout` or `fallback` |
| `CONFIG_SAVE_FAILED` | 500 | Settings file I/O failed (temp write / rename); not a client validation error |
| `BACKEND_URL_EMPTY` | 400 | Empty backend URL |
| `BACKEND_URL_INVALID` | 400 | Unparseable backend URL |
| `BACKEND_URL_INVALID_SCHEME` | 400 | Backend URL not http/https |
| `BACKEND_URL_CREDENTIALS` | 400 | Credentials in backend URL |
| `BACKEND_URL_NO_HOST` | 400 | Backend URL missing host |
| `BACKEND_URL_DNS_REBINDING` | 400 | Rebinding hostname in backend URL |
| `BACKEND_URL_CLOUD_METADATA` | 400 | 169.254.169.254, fd00:ec2::254, or known metadata hostnames |
| `BACKEND_URL_LINK_LOCAL` | 400 | Link-local backend address |
| `BACKEND_URL_DNS_UNRESOLVABLE` | 400 | Fetch-time guard: backend hostname resolved to no addresses (fails closed) |
| `HISTORY_DISABLED` | 400 | History endpoints with history off |
| `COLLECTION_NAME_INVALID` | 400 | Collection name empty or too long |
| `COLLECTION_ITEM_TITLE_INVALID` | 400 | Item title empty or too long |
| `COLLECTION_EXISTS` | 400 | Duplicate collection name |
| `HISTORY_ENTRY_NOT_FOUND` | 404 | Unknown history query id |
| `DOC_NOT_FOUND` | 404 | Unknown `/api/docs/{slug}` |
| `DOC_ASSET_NOT_FOUND` | 404 | Missing doc asset file |
| `COLLECTION_NOT_FOUND` | 404 | Unknown collection id |

## Upstream / gateway errors (502)

| Code | HTTP | When |
|------|------|------|
| `FANOUT_TOTAL_FAILURE` | 502 | All backends failed; no results |
| `BRAVE_HTTP_ERROR` | 502 | Brave API non-success status |
| `SEARXNG_HTTP_ERROR` | 502 | SearXNG non-success status |
| `DDGS_*` / `BRAVE_API_KEY_MISSING` | 502 | Backend-specific failures (see logs) |

Partial fanout (some backends fail) still returns **200** with `results` and an `errors[]` string list — not a typed error response.

## Server errors (5xx)

| Code | HTTP | When |
|------|------|------|
| `DB_ERROR` | 500 | SQLite failure |
| `JSON_PARSE_ERROR` | 500 | Invalid JSON in persistence layer |
| `NETWORK_TIMEOUT` | 500 | Outbound HTTP timeout |
| `NETWORK_CONNECT` | 500 | Outbound connection refused |
| `NETWORK_ERROR` | 500 | Other reqwest errors |
| `SEARCH_PAYLOAD` | 500 | Internal search response shape bug |
| `INTERNAL` / `BROWSER_NOT_FOUND` | 500 | Unexpected or browser spawn failure |

## Regression tests

Rust integration tests: `src-tauri/tests/api_error_codes.rs`  
Python API tests: `tests/test_api.py`, `tests/test_security.py`

---

*NetRail v1.6.5 — backend fetch-time codes included (A-05) — maintained by [kayab999](https://github.com/kayab999)*