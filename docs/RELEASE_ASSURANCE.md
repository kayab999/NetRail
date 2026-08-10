# NetRail — Release Assurance

Un documento de confianza, no técnico. Para quien llega por primera vez y quiere
saber **qué garantías ofrece el proyecto** y **dónde están respaldadas** sin leer
el código. Cada fila apunta a la evidencia concreta (test, script o documento).

> Estado a fecha del corte **v1.6.5 (RC)** (2026-08-10). Las cifras de tests son las
> de la suite local del ciclo de remediación + verificación R2 desde checkout limpio
> (pytest 207, cargo 130) — reproducidas sin desviación.

## Garantías, por área

| Área | Qué garantiza | Evidencia |
|------|---------------|-----------|
| **Seguridad** | Las URLs que se abren en el navegador pasan un filtro SSRF/rebinding agresivo (loopback codificado, IPs privadas, DNS-rebinding, redirects DDG, pin DNS en el momento de abrir — A15) | Fixture SSOT de 68 vectores `tests/fixtures/url_policy.json` (Rust + Python + parity live); `docs/AUDIT_ARCH_2026-08-01.md` (A1–A15 cerradas); `docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md` |
| **Resiliencia** | La API sobrevive a fallos inyectados de backends/red sin colapsar ni corromper estado | Chaos suite `scripts/chaos/` (`chaos_db`, `chaos_process`) ejecutada en CI; harness de estabilidad de recursos `scripts/load/` (10k peticiones, ambos stacks) → `docs/sprint3-slope.md` |
| **Concurrencia** | Escrituras concurrentes son serializadas sin datos perdidos | `HistoryStore` con `RLock` + test de 16 threads; settings con ETag/If-Match (409 en conflicto) → `tests/test_history.py`, `tests/test_api.py` |
| **Rendimiento** | Métricas dual-stack documentadas y reproducibles | Benchmarks `scripts/bench/` → `docs/bench-dual.md`: Rust ≈573 rps / p50 23 ms / 14% CPU / 10.4 MiB; Python ≈295 rps / p50 39 ms / 74% CPU / 64.1 MiB; sin knee hasta C=512 |
| **Calidad** | Suites completas en verde antes de cada release | **207** tests Python (`pytest tests/`, incl. parity de browsers, fanout-deadline y link-integrity) + **130** tests Rust (`cargo test`, incl. 10 de browsers) + clippy `-D warnings` + gate de fuzz diferencial (`fuzz-parity --ci`, code_diff=0) + smoke de parity E2E en CI. Cobertura visible sin gate (QA-04/T5): Python 77% (branch) / 80% stmts, Rust 57.5% líneas `--lib` — reported en cada run de CI; el umbral es decisión de Baseline #2. Nota honesta: no hay linter Python (ruff/flake8) configurado en el repo |
| **Cadena de suministro** | El SBOM viaja dentro de cada artefacto y la release está firmada | E2: inventario Rust embebido en el binario (`netrail-api --sbom`, byte-idéntico al `SBOM.txt`) y `SBOM.txt` empaquetado en deb/rpm/AppImage; el CI de release lo verifica (`dpkg-deb`/`rpm -qlp`). Firma keyless cosign (`SHA256SUMS.sig` + `.pem`). `cargo audit` / `npm audit` / `pip-audit` bloquean el release |

## Disciplina de release (identidad clara por versión)

Cada release tiene **un** propósito y no mezcla endurecimiento, funcionalidad y
arquitectura:

| Versión | Identidad | Contenido |
|---------|-----------|-----------|
| 1.6.1 | Endurecimiento | DNS pin A15, webview E2E, CI firmado, FTS sync, 422s, CSP token |
| 1.6.2 | Endurecimiento + evidencia | Chaos suite, harness de estabilidad, benchmarks dual-stack |
| 1.6.3 | Reproducibilidad y cadena de suministro | E2 SBOM-in-bundle, E3 snapshot CSS, E5 fixtures golden |
| 1.6.5 | Release-readiness (RC) | Post-1.6.4 remediation: clippy P0 green, fuzz diferencial CI-gated, parity de browsers, fanout simétrico, cobertura visible (sin gate), link-integrity, docs SSOT; Baseline #2 = **4.65 SHIP-GRADE** como gate de readiness. Verificación R2 desde checkout limpio |
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

## Webview E2E — gate manual de release (política T6/QA-05)

El webview Tauri/WebKitGTK se valida con **gate manual antes de etiquetar**, no en
CI. Decisión de política, no carencia: el E2E requiere una sesión X/Wayland real
(el app abre ventana), `WebKitWebDriver` + `tauri-driver` + selenium y puerto 7421
libre (single-instance); bajo `xvfb` el pipeline de global-shortcut (XGrabKey) y
el foco son frágiles, así que un gate CI sería intermitente e injustamente
bloqueante. El procedimiento es reproducible y su evidencia se adjunta a las notas
del release:

```bash
# 1) Precondiciones (una máquina con display):
#    - cargo install tauri-driver --locked
#    - webkit2gtk-driver (Debian/Ubuntu: sudo apt install webkit2gtk-driver)
#    - .venv con selenium; xdotool (opcional, pipeline shortcut)
#    - Ningún NetRail corriendo (7421 libre, 4444 libre)
# 2) Construir y ejecutar:
cargo build --manifest-path src-tauri/Cargo.toml --bin netrail
bash scripts/webview-e2e.sh src-tauri/target/debug/netrail
# 3) Resultado aceptado: "WEBVIEW E2E: 6/6 passed" (5/6 admisible solo si el
#    check de shortcut se marcó "(skipped: xdotool not present)").
# 4) Evidencia: adjuntar la salida completa a las release notes del tag.
```

Cubre: carga de UI + puente `netrailFocusSearch` (foco + selección), pipeline de
global-shortcut (XGrabKey → eval), puente de docs (`netrailOpenDoc`, éxito y error)
y guard de foco con modal abierto. Los puentes se verifican además en CI a nivel
API (`/api/docs/*`, smoke de parity); lo que no cubre el CI es la capa webview real,
y por eso el gate manual existe.

Referencias técnicas completas: `docs/ARCHITECTURE.md` (roadmap), `docs/DISTRIBUTION.md`
(packaging), `SECURITY.md` (modelo de amenazas), `docs/HANDOFF_OPENCODE_2026-08-02.md`
(plan de trabajo).
