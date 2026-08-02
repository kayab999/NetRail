# NetRail — Release Assurance

Un documento de confianza, no técnico. Para quien llega por primera vez y quiere
saber **qué garantías ofrece el proyecto** y **dónde están respaldadas** sin leer
el código. Cada fila apunta a la evidencia concreta (test, script o documento).

> Estado a fecha del corte **v1.6.3** (2026-08-02). Las cifras de tests son las de
> la suite local justo antes de etiquetar.

## Garantías, por área

| Área | Qué garantiza | Evidencia |
|------|---------------|-----------|
| **Seguridad** | Las URLs que se abren en el navegador pasan un filtro SSRF/rebinding agresivo (loopback codificado, IPs privadas, DNS-rebinding, redirects DDG, pin DNS en el momento de abrir — A15) | Fixture SSOT de 68 vectores `tests/fixtures/url_policy.json` (Rust + Python + parity live); `docs/AUDIT_ARCH_2026-08-01.md` (A1–A15 cerradas); `docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md` |
| **Resiliencia** | La API sobrevive a fallos inyectados de backends/red sin colapsar ni corromper estado | Chaos suite `scripts/chaos/` (`chaos_db`, `chaos_process`) ejecutada en CI; harness de estabilidad de recursos `scripts/load/` (10k peticiones, ambos stacks) → `docs/sprint3-slope.md` |
| **Concurrencia** | Escrituras concurrentes son serializadas sin datos perdidos | `HistoryStore` con `RLock` + test de 16 threads; settings con ETag/If-Match (409 en conflicto) → `tests/test_history.py`, `tests/test_api.py` |
| **Rendimiento** | Métricas dual-stack documentadas y reproducibles | Benchmarks `scripts/bench/` → `docs/bench-dual.md`: Rust ≈573 rps / p50 23 ms / 14% CPU / 10.4 MiB; Python ≈295 rps / p50 39 ms / 74% CPU / 64.1 MiB; sin knee hasta C=512 |
| **Calidad** | Suites completas en verde antes de cada release | **162** tests Python (`pytest tests/`) + **113** tests Rust (`cargo test`) + clippy `-D warnings` + smoke de parity y E2E en CI. Nota honesta: no hay linter Python (ruff/flake8) configurado en el repo |
| **Cadena de suministro** | El SBOM viaja dentro de cada artefacto y la release está firmada | E2: inventario Rust embebido en el binario (`netrail-api --sbom`, byte-idéntico al `SBOM.txt`) y `SBOM.txt` empaquetado en deb/rpm/AppImage; el CI de release lo verifica (`dpkg-deb`/`rpm -qlp`). Firma keyless cosign (`SHA256SUMS.sig` + `.pem`). `cargo audit` / `npm audit` / `pip-audit` bloquean el release |

## Disciplina de release (identidad clara por versión)

Cada release tiene **un** propósito y no mezcla endurecimiento, funcionalidad y
arquitectura:

| Versión | Identidad | Contenido |
|---------|-----------|-----------|
| 1.6.1 | Endurecimiento | DNS pin A15, webview E2E, CI firmado, FTS sync, 422s, CSP token |
| 1.6.2 | Endurecimiento + evidencia | Chaos suite, harness de estabilidad, benchmarks dual-stack |
| 1.6.3 | Reproducibilidad y cadena de suministro | E2 SBOM-in-bundle, E3 snapshot CSS, E5 fixtures golden |
| Próximos | Funcionalidad / arquitectura | Fuera del alcance de 1.6.3: DNS resolve-and-warn, RBAC, TLS pinning, Windows/macOS, LLM on-device, MCP |

## Cómo verificar por uno mismo

```bash
# Versión SSOT (5 archivos alineados)
bash scripts/check-versions.sh

# Suites
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test && cd ..

# SBOM embebido en el binario release
./src-tauri/target/release/netrail-api --sbom | head

# Smokes (rate limit off)
NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
NETRAIL_RATE_LIMIT=0 bash scripts/parity-api-smoke.sh
```

Referencias técnicas completas: `docs/ARCHITECTURE.md` (roadmap), `docs/DISTRIBUTION.md`
(packaging), `SECURITY.md` (modelo de amenazas), `docs/HANDOFF_OPENCODE_2026-08-02.md`
(plan de trabajo).
