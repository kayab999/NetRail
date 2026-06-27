# NetRail — Auditoría técnica profunda

**Fecha:** 2026-06-27  
**Versión auditada:** v1.1.0 (`597838d` + cambios post-release)  
**Alcance:** Rust (Tauri/Axum), Python (fallback), frontend estático, CI/CD, seguridad, tests

---

## Resumen ejecutivo

NetRail es un producto **maduro para v1.x**: arquitectura Rust-primary coherente, superficie de ataque acotada (localhost-only), fanout con degradación parcial bien diseñado, y desde v1.1.0 errores API tipados con códigos estables. Los hallazgos críticos inmediatos eran **CI rojo en `main`** (clippy) y **cobertura de tests insuficiente** para los códigos de error — ambos abordados en este ciclo.

| Área | Estado | Nota |
|------|--------|------|
| Arquitectura Rust | ✅ Sólida | Axum embebido, pooling HTTP, keyring degradable |
| Seguridad URL/CSP | ✅ Fuerte | SSRF mitigado; CSP nativo en Tauri |
| Errores tipados (Rust) | ✅ v1.1.0 | `NetRailError` + `{code, detail, status}` |
| Tests Rust | ✅ Mejorado | 37 tests (30 unit + 7 HTTP integration) |
| Paridad Python | ⚠️ Parcial | Sin `code` en errores FastAPI |
| CI | ✅ Verde (local) | Clippy fix + nueva suite |
| Documentación | ⚠️ Drift menor | MANUAL/DISTRIBUTION/ARCHITECTURE aún citan 1.0.0 |
| Releases GitHub | ⚠️ Menor | v1.0.0 sigue en draft; v1.1.0 publicado |

---

## 1. Arquitectura

### 1.1 Flujo desktop

```
Tauri 2 shell → webview http://127.0.0.1:7421
                      ↓
              Axum Router (build_router)
                      ↓
         AppState { http_client, settings_fn }
                      ↓
    search / history / browsers / docs / static
```

**Fortalezas**
- Un solo proceso; sin CORS cross-origin en uso normal.
- `reqwest::Client` compartido (pooling TLS desde v1.0.1).
- `settings_fn: Arc<dyn Fn() -> Settings>` permite inyección en tests sin tocar disco.

**Deuda técnica (baja prioridad)**
- `search::search()` llama `load_settings()` directamente en lugar de recibir `Settings` del `AppState`. Esto impide tests de integración HTTP para `FANOUT_TOTAL_FAILURE` sin escribir config real. El fanout total se cubre a nivel `backends::search_with_fanout` (wiremock/dead ports).
- Router y handlers viven en un solo `server/mod.rs` (~500 líneas). Extraer submódulos (`handlers/`, `error.rs`) mejoraría mantenibilidad en v1.2+.

### 1.2 Python fallback

FastAPI en `netrail/main.py` replica la API para Docker/Flatpak. **No implementa `NetRailError`** — usa `HTTPException(detail=str)` sin campo `code`. El frontend Rust lee `payload.code` cuando existe; en Python solo muestra `detail`.

**Recomendación:** diferir paridad Python a v1.2 o documentar explícitamente como “legacy path”. ROI bajo si el binario Rust es el camino principal.

---

## 2. Seguridad

### 2.1 Validación de URLs (`security.rs`)

| Vector | Código | Mitigación |
|--------|--------|------------|
| localhost / loopback | `OPEN_URL_LOCALHOST` | Bloqueo explícito |
| DNS rebinding (nip.io) | `OPEN_URL_DNS_REBINDING` | Sufijos conocidos |
| Link-local / metadata | `OPEN_URL_LINK_LOCAL`, `BACKEND_URL_CLOUD_METADATA` | Rangos IP |
| Credenciales en URL | `OPEN_URL_CREDENTIALS` | Rechazo |
| Redirect DDG profundo | `OPEN_URL_REDIRECT_DEPTH` | Unwrap con límite |

**Backend URLs** permiten `127.0.0.1` (SearXNG local) — decisión correcta y documentada.

### 2.2 CSP y headers

- Middleware `security_headers`: CSP, `X-Content-Type-Options`, `Referrer-Policy`.
- Tauri `tauri.conf.json` CSP alineado (v1.0.1).
- `'unsafe-inline'` en `style-src` — necesario para estilos inline actuales; aceptable en app local.

### 2.3 Keyring / cifrado

- Degradación graceful a sesión sin cifrar + banner UI + evento `security:encryption-degraded`.
- Tests de interoperabilidad Fernet Rust ↔ Python en `history` y `crypto`.

**Riesgo residual:** en degradación, historial en disco sin cifrar hasta reinicio. Comportamiento comunicado al usuario.

### 2.4 Superficie de red

- API escucha solo `127.0.0.1:7421` — no expuesta por defecto a LAN.
- Fanout a backends externos; no hay proxy abierto.

---

## 3. Errores tipados (`NetRailError`)

### 3.1 Inventario de códigos (Rust)

| Dominio | Códigos |
|---------|---------|
| Query/API | `QUERY_INVALID` |
| Open URL | `OPEN_URL_*` (8 variantes) |
| Backend URL | `BACKEND_URL_*` (9 variantes) |
| Config | `CONFIG_MAX_RESULTS`, `CONFIG_HISTORY_TTL`, `CONFIG_SEARCH_STRATEGY` |
| History | `HISTORY_DISABLED`, `HISTORY_ENTRY_NOT_FOUND`, `COLLECTION_*` |
| Docs | `DOC_NOT_FOUND`, `DOC_ASSET_NOT_FOUND` |
| Search fanout | `FANOUT_TOTAL_FAILURE` (502) |
| Backends | `DDGS_*`, `BRAVE_*`, `SEARXNG_*`, `BRAVE_API_KEY_MISSING` |
| Infra | `DB_ERROR`, `JSON_PARSE_ERROR`, `NETWORK_*`, `INTERNAL` |

### 3.2 Contrato API

```json
{ "code": "QUERY_INVALID", "detail": "...", "status": 400 }
```

Frontend (`app.js` línea ~78): lee `payload.code` y `payload.detail` en errores.

### 3.3 Cobertura de tests (post-auditoría)

| Capa | Tests |
|------|-------|
| Unit `error.rs` | status mapping, `to_json`, `From` traits |
| Unit `config.rs` | 4 códigos de validación |
| Unit `security.rs` | 9 casos + código estable |
| Unit `backends` | fanout parcial (wiremock) + total failure |
| Integration HTTP | 7 endpoints → 7 códigos |

**Pendiente (v1.2, ROI medio):** tabla de códigos en docs; tests para `FANOUT_TOTAL_FAILURE` vía HTTP (requiere refactor `search`); tests Python con `code`.

---

## 4. Tests y CI

### 4.1 Rust (src-tauri)

```
cargo clippy --all-targets -- -D warnings  ✅
cargo test                                  ✅ 37 passed
```

- Nuevo: `tests/api_error_codes.rs` usando `build_router()` + `tower::ServiceExt::oneshot`.
- Fix CI: `browsers.rs` `sort_by_key` (clippy `unnecessary_sort_by` en Rust 1.96).

### 4.2 Python

33 tests en `tests/` — CI los ejecuta con Python 3.12. Localmente requieren venv.

**Gap:** `test_open_rejects_localhost` solo assert `status_code == 400`, no `code`. Coherente con backend Python sin códigos.

### 4.3 Pipeline GitHub Actions

- `ci.yml`: Rust clippy+test, Python pytest, npm ci.
- `release.yml`: AppImage/deb — verde en v1.1.0.

---

## 5. Documentación y releases

| Item | Estado | Acción |
|------|--------|--------|
| README install paths | ✅ Corregido → 1.1.0 | Este commit |
| `package-lock.json` | ✅ 1.1.0 | Este commit |
| `MANUAL.md` | ⚠️ `NetRail_1.0.0_*` | Actualizar en v1.1.1 o batch docs |
| `DISTRIBUTION.md` | ⚠️ footer v1.0.0 | Idem |
| `ARCHITECTURE.md` | ⚠️ footer v1.0.0 | Idem |
| Release v1.0.0 draft | ⚠️ Huérfano | Publicar o eliminar draft |
| CHANGELOG | ✅ 1.0.1 + 1.1.0 | OK |

---

## 6. Hallazgos priorizados (ROI)

### P0 — Resuelto en este ciclo
1. **CI clippy rojo** — `sort_by_key` en `browsers.rs`
2. **Tests por código de error** — suite integration + unit expandida
3. **Router no testeable** — `build_router(state)` extraído

### P1 — Próximo sprint (v1.1.1 / v1.2)
1. Refactor `search::search(client, query, mode, max, &settings)` para usar settings inyectadas
2. Documentar tabla de códigos en `MANUAL.md` o `docs/API_ERRORS.md`
3. Sincronizar MANUAL/DISTRIBUTION/ARCHITECTURE a 1.1.0
4. Cerrar release draft v1.0.0 (publicar histórico o borrar)

### P2 — Backlog
1. Paridad `code` en Python FastAPI (wrapper `ApiError` espejo de Rust)
2. Partir `server/mod.rs` en submódulos
3. HashMap estructurado para errores de fanout (bajo ROI según review externo)
4. Deprecación formal del path Python

### P3 — Nice to have
1. OpenAPI/Swagger generado desde códigos
2. Test E2E Tauri (tauri-driver)
3. Métricas opcionales locales (sin telemetría cloud)

---

## 7. Veredicto

**NetRail v1.1.0 está listo para uso productivo** en Linux desktop con el binario Rust. La auditoría no encontró vulnerabilidades bloqueantes; el modelo de amenazas (app local, usuario único, localhost) está bien calibrado.

El trabajo de este ciclo cierra el gap más visible post-review externo: **regresión automatizada de códigos de error** y **CI verde**. Los siguientes pasos de mayor valor son documentación sincronizada y refactor menor de inyección de settings en búsqueda.

---

*Auditoría realizada como parte del ciclo post-v1.1.0 — [kayab999/NetRail](https://github.com/kayab999/NetRail)*