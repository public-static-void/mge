---
title: "REPORT: Comprehensive Project Audit — State, Gaps, Issues, Improvements"
version: 1.0.0
status: draft
type: report
created: 2026-05-27
author: Overseer
superseded_by: null
---

# REPORT: MGE Project Audit

## Executive Summary

MGE is a sophisticated cross-language game engine with a **well-architected ECS core**, comprehensive **Lua and Python bindings**, and a **terminal-based UI framework**. The codebase is remarkably clean — only 1 TODO comment, CI-enforced clippy compliance, 204+ test files across 3 languages, and a coherent schema-driven component system.

**However**, the audit reveals several critical gaps and issues that should be addressed before the project scales further:

- **Critical**: Zero unit tests in engine_core (all testing is integration-level)
- **Critical**: Architectural contradiction — engine_core depends on mlua/LuaJIT despite claiming "no language binding dependencies"
- **High**: ~1465 unwrap() calls in production code, ~10 unjustified unsafe blocks
- **High**: No sandboxing for Lua/Python scripts (full system access)
- **High**: WASM backend is severely incomplete (only entity API)
- **High**: Vestigial feature flags that mislead contributors

---

## 1. Project State Summary

### MGE At a Glance

| Dimension | Assessment |
|-----------|-----------|
| **Architecture** | Strong ECS foundation, clean module boundaries, but architectural drift in feature flags and core dependencies |
| **Build** | Builds cleanly in CI. Requires nightly Rust + 5 system deps. No devcontainer. |
| **Code Quality** | High — clippy-clean, well-formatted, consistent error handling patterns. But production code has unwrap proliferation. |
| **Testing** | 204+ test files but ZERO unit tests in engine_core/src/. All integration-level. UI is untested. |
| **Documentation** | Good AGENTS.md. Missing: ADRs, CONTRIBUTING.md, memory model docs, plugin dev guide. |
| **Security** | No sandboxing for scripting runtimes. Moderate risk from unsafe blocks. Plugin trust model is implicit. |
| **Performance** | JSON round-trips on every cross-language component access is the #1 perf concern. |
| **Cross-Language** | Lua/Python: near parity. WASM: far behind. C ABI: memory ownership undocumented. |
| **DX** | High setup friction — nightly Rust + LuaJIT + gcc + maturin + WASM target. No Docker. |

### Codebase Metrics

| Metric | Value |
|--------|-------|
| Rust crates | 10 (workspace) |
| Rust source | ~15K+ lines |
| Component schemas | 25 JSON files |
| Test files | 108 Rust + 44 Python + 47 Lua + 5 misc |
| Total binaries | 6 |
| Job system | 9 subdirectories, most complex subsystem |
| UI widgets | 12 types, 3 layout engines |
| Plugin count | 5 (1 Rust, 4 C) |

---

## 2. Top Gaps

### Critical

| Gap | Details | Impact |
|-----|---------|--------|
| **No unit tests in engine_core** | Zero `#[cfg(test)] mod tests {}` in engine_core/src/. All 108 tests are integration tests that require full system initialization. | Bug discovery is slow; refactoring risk is high; no isolated module verification. |
| **engine_core depends on mlua** | engine_core/Cargo.toml has mlua as a regular dependency (not optional). Links LuaJIT into every build. Contradicts the stated architecture. | Prevents use of engine_core standalone; adds build complexity; enables feature gates that don't exist. |

### High

| Gap | Details | Impact |
|-----|---------|--------|
| **Vestigial feature flags** | `[features] lua = [] python = [] wasm = []` — empty, no cfg gates, misleading | Contributors think they can disable features; no conditional compilation. |
| **No Lua/Python sandbox** | Lua scripts can call `os.execute()`, Python scripts can import `os`, run arbitrary commands. | Any mod or script has full system access. |
| **Unwrap proliferation** | ~1465 unwrap() + ~201 expect() calls. Many in production code paths. | Unexpected states cause panics instead of graceful errors. |
| **Unjustified unsafe blocks** | ~10 unsafe blocks in UI/presentation code without safety docs. | Memory safety risk; no documented invariants. |
| **WASM backend incomplete** | Only entity API exposed. No components, map, jobs, movement, etc. | WASM is effectively unusable for real gameplay. |
| **No ADRs / CONTRIBUTING** | No architecture decision records exist. No contribution guide. | New contributors have no onboarding path; design rationale is lost. |

### Medium

| Gap | Details | Impact |
|-----|---------|--------|
| **UI not tested** | 12 widget types, 3 layouts, event system — zero tests. | UI regressions undetectable. |
| **No devcontainer** | No reproducible development environment. | Setup is a multi-step manual process. |
| **Mod loader untested** | Mod manifest parsing, schema loading, script injection — no tests. | Loading invalid mods can silently fail. |
| **Stale nested Cargo.lock** | engine/core/Cargo.lock exists and is stale. | Confusion for direct builds from that directory. |
| **Schema validator tool untested** | The tool that validates all schemas has no tests for itself. | Schema validation errors could go undetected. |

---

## 3. Top Improvement Opportunities

### Quick Wins (Low Effort, High Impact)

| # | Improvement | Effort | Impact | Area |
|---|-------------|--------|--------|------|
| 1 | Remove stale engine/core/Cargo.lock | Minutes | Medium | Build |
| 2 | Add devcontainer.json + Dockerfile | Hours | High | DX |
| 3 | Write CONTRIBUTING.md | Hours | High | Documentation |
| 4 | Create first ADRs (architecture, decisions made so far) | Hours | High | Documentation |
| 5 | Document C ABI memory ownership in engine_plugin_abi.h | Hours | Medium | Cross-Language |
| 6 | Add unit tests for 1-2 core modules (set precedent) | Days | High | Testing |
| 7 | Make mlua optional in engine_core [features] | Days | High | Architecture |

### Strategic Investments (Medium Effort, High Impact)

| # | Improvement | Effort | Impact | Area |
|---|-------------|--------|--------|------|
| 8 | Properly implement or remove feature flags with cfg gates | Days | High | Architecture |
| 9 | Systematic reduction of unwrap() → proper error handling | Weeks | High | Code Quality |
| 10 | Add unit test suite for engine_core modules | Weeks | Critical | Testing |
| 11 | Add UI test suite (widget rendering, layout, events) | Weeks | High | Testing |
| 12 | Add sandbox mode for Lua/Python script execution | Weeks | High | Security |
| 13 | Audit and document all 84 unsafe blocks | Days | High | Security |

### Long-term (High Effort, Variable Impact)

| # | Improvement | Effort | Impact | Area |
|---|-------------|--------|--------|------|
| 14 | Complete WASM API to parity with Lua/Python | Months | High | Cross-Language |
| 15 | Add dirty-region rendering for terminal output | Weeks | Medium | Performance |
| 16 | Add property-based / fuzz testing for core ECS | Weeks | Medium | Testing |
| 17 | Replace JSON serialization with binary format for cross-language ops | Months | High | Performance |

---

## 4. Issues Requiring Immediate Attention

These issues should be addressed before they cause production problems or developer friction:

1. **Stale nested Cargo.lock** — Delete `engine/core/Cargo.lock`. The root workspace lockfile is authoritative. This will prevent confusion.

2. **Vestigial feature flags** — Either remove them or implement proper `#[cfg(feature = "...")]` gates. Currently they're misleading dead configuration.

3. **engine_core mlua dependency** — Make it optional behind a feature flag, or document why it's required and update the AGENTS.md accordingly. This is an architecture principle violation.

4. **Unwraps in JSON schema loading** — If schema loading fails (e.g., corrupted file), the unwrap will panic. This should be proper error propagation.

5. **~10 unjustified unsafe blocks in UI** — These need safety documentation or refactoring to safe alternatives.

6. **No devcontainer/Docker** — For a project with 5+ system dependencies and nightly Rust, this is a significant onboarding blocker.

---

## 5. Recommended Actions (Next Steps)

### Immediate (this week)
1. ✅ Delete `engine/core/Cargo.lock`
2. ✅ Document the mlua dependency decision (ADR or README update)
3. ✅ Start a CONTRIBUTING.md
4. ✅ Audit the ~10 suspect unsafe blocks in UI code

### Short-term (this month)
5. Add unit tests for 2-3 core modules (ecs/schema.rs, map/pathfinding.rs, config.rs)
6. Create devcontainer.json
7. Write ADR-0001 (Architecture Overview)
8. Make mlua optional in engine_core

### Medium-term (next quarter)
9. Systematic unwrap reduction pass — focus on production (non-test) code
10. UI test suite
11. Lua/Python sandbox implementation
12. Complete feature flag implementation

---

## 6. References

- `knowledge/intent-project-audit-2026-05-27.md` — Original intent
- `knowledge/analysis-codebase-map-2026-05-27.md` — Explorer: codebase structure map
- `knowledge/analysis-deep-dive-2026-05-27.md` — Analyzer: deep-dive gaps and issues
- `AGENTS.md` — Project identity and conventions
- `Cargo.toml` — Workspace manifest
- `Makefile` — Build orchestration
- `engine/core/Cargo.toml` — Core crate (with mlua issue)
- `.github/workflows/ci.yml` — CI pipeline
