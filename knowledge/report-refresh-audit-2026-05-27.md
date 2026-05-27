---
title: "REPORT: Refreshed Project Audit — State, Gaps, Issues, Improvements"
version: 1.0.0
status: draft
type: report
created: 2026-05-27
author: Overseer
superseded_by: null
---

# REPORT: MGE Project Audit (Refreshed)

## Executive Summary

This refreshed audit confirms and **sharpens** the prior assessment. MGE remains a well-architected cross-language game engine with a strong ECS core, comprehensive Lua/Python bindings, and clean code quality (clippy-clean, 204+ test files).

**However, the refreshed deep-dive reveals higher-precision counts and several new findings:**

- **🔴 Critical**: **371 unwrap/expect calls** in production code (not ~1465 — the prior count included test code). Still too many — every one is a potential panic.
- **🔴 Critical**: **ZERO unit tests in engine_core/src** across 96 source files. Confirmed. Only 1 placeholder `assert_eq!(2+2, 4)` exists.
- **🔴 Critical**: **engine_core unconditionally links LuaJIT via mlua** — feature flags are empty no-ops. Every build requires LuaJIT headers.
- **🟠 High**: **10 undocumented unsafe blocks** in UI widgets (WidgetId transmute). No `// SAFETY:` comments.
- **🟠 High**: **~67 unsafe blocks total** across all crates — ~43 documented (plugin loader), ~24 undocumented.
- **🟠 High**: **Plugin ABI has no versioning** — incompatible `.so` files cause UB at load time.
- **🟠 High**: **Lua scripts run with StdLib::ALL** — full system access via `os.execute()`.
- **🟠 High**: **WASM at ~10% API parity** with Lua. Python also missing `input` and `callback_registry` modules.
- **🟡 Medium**: Job system has ~100 unwrap calls, highest density of any subsystem.
- **🟡 Medium**: Codegen tool exists but is unwired from the build pipeline.

### Prior Findings Re-Validated

| Prior Finding | Status | Note |
|---------------|--------|------|
| Zero unit tests in engine_core | ✅ Confirmed | Still no unit tests |
| engine_core depends on mlua | ✅ Confirmed | Unconditional, feature flags no-op |
| ~1465 unwrap calls | 🔄 **Refined** | Actual production count: **371** (prior included test code) |
| ~10 unjustified unsafe blocks | ✅ Confirmed | Precisely 10 in UI widgets, no SAFETY comments |
| WASM backend incomplete | ✅ Confirmed | ~10% API parity |
| Stale nested Cargo.lock | ✅ Still present | `engine/core/Cargo.lock` exists |
| Vestigial feature flags | ✅ Confirmed | Empty `lua = [] python = [] wasm = []` |
| No devcontainer | ✅ Confirmed | Not yet created |
| No sandboxing | ✅ Confirmed | Lua StdLib::ALL confirmed |
| No ADRs/CONTRIBUTING | ✅ Confirmed | Not yet created |

---

## 1. Project State Summary

### MGE At a Glance

| Dimension | Assessment |
|-----------|-----------|
| **Architecture** | Strong ECS foundation, clean module boundaries. Drift: mlua hard-dependency contradicts stated architecture. |
| **Build** | Builds cleanly on nightly Rust. 5+ system deps. Stale nested Cargo.lock still present. |
| **Code Quality** | High — clippy-clean, well-formatted. Concern: 371 unwrap calls in production, ~24 undocumented unsafe blocks. |
| **Testing** | 204+ test files but ZERO unit tests in engine_core/src. All integration-level. UI, mod loader, schema validator untested. |
| **Documentation** | Good AGENTS.md. Missing: ADRs, CONTRIBUTING.md, memory model docs, plugin dev guide, CHANGELOG. |
| **Security** | StdLib::ALL for Lua. Plugin ABI has no versioning (UB risk). ~24 undocumented unsafe blocks. |
| **Performance** | JSON round-trips on every cross-language component access remains #1 concern. |
| **Cross-Language** | Lua: 30 API modules. Python: 27 (missing input, callback_registry). WASM: ~10% parity. |
| **DX** | High setup friction — nightly Rust + LuaJIT + gcc + maturin + wasm target. No devcontainer. |

### Codebase Metrics (Refreshed)

| Metric | Value |
|--------|-------|
| Rust source files in engine_core | ~96 `.rs` files |
| Binary targets | 6 |
| Component schemas | 25 JSON files |
| Test files | 108 Rust + 44 Python + 47 Lua + 6 misc |
| Job system | 9 subdirectories, 13-state FSM, ~100 unwrap calls |
| UI widgets | 10 types, 3 layout engines |
| Plugin count | 5 (1 Rust cdylib, 4 C) |
| TODO markers | 1 (province centroid) |
| Production unwrap/expect calls | **371** (across all crates) |
| unsafe blocks | ~67 total (~43 documented, ~24 undocumented) |

---

## 2. Gap Register (Refreshed)

### 🔴 Critical

| Gap | Detail | Impact |
|-----|--------|--------|
| **No unit tests in engine_core/src** | 96 source files, 0 `#[cfg(test)] mod tests {}`. Only 1 placeholder test (`assert_eq!(2+2, 4)`). All 108 Rust tests are integration-level requiring full system init. | Bug discovery slow; refactoring risks high; no isolated module verification. |
| **engine_core unconditionally depends on mlua** | mlua (LuaJIT) is a hard dependency. Feature flags `lua = [] python = [] wasm = []` are empty no-ops — no `#[cfg]` gates exist anywhere. | Prevents standalone engine_core use; forces LuaJIT install for all builds. |
| **371 unwrap/expect calls in production code** | Job system alone has ~100. File I/O, schema loading, component access all have unwraps. | Unexpected states cause panics instead of graceful error reporting. |

### 🟠 High

| Gap | Detail | Impact |
|-----|--------|--------|
| **10 undocumented unsafe blocks in UI** | WidgetId created via `ptr as u64` transmute. Zero `// SAFETY:` comments. | Memory safety risk; no documented invariants. |
| **~24 undocumented unsafe blocks total** | Across UI, FFI, and plugin loader. No safety docs. | Hidden UB risk. |
| **Plugin ABI has no versioning** | No version field in PluginVTable. Incompatible `.so` silently loaded. | Undefined behavior at load time. |
| **Lua StdLib::ALL** | Scripts can call `os.execute()`, `io.*`, `dofile()`, `loadfile()`. | Any mod has full system access. |
| **WASM at ~10% API parity** | Only entity API (spawn, get/set component). No map, jobs, combat, movement, UI. | WASM effectively unusable. |
| **Python missing 2 API modules** | No `input` or `callback_registry` modules that Lua has. | Feature gap between scripting runtimes. |
| **Vestigial feature flags** | `[features] lua = [] python = [] wasm = []` — no conditional compilation. | Misleading to contributors; dead configuration. |

### 🟡 Medium

| Gap | Detail | Impact |
|-----|--------|--------|
| **UI not tested** | 10 widget types, 3 layouts, event system — zero automated tests. | UI regressions undetectable. |
| **No devcontainer/Docker** | No reproducible dev environment for a project with 5+ system deps. | Setup friction for new contributors. |
| **Mod loader untested** | Mod manifest parsing, schema loading, script injection — no tests. | Silent failures on invalid mods. |
| **Stale nested Cargo.lock** | `engine/core/Cargo.lock` exists, is stale, conflicts with root workspace lock. | Confusion, potential build issues. |
| **Schema validator untested** | The tool that validates all schemas has no tests. | Schema validation errors could go undetected. |
| **No ADRs / CONTRIBUTING / CHANGELOG** | No architecture decisions recorded. No contribution guide. No changelog. | Onboarding friction; design rationale lost. |
| **Codegen tool unwired** | `engine/tools/codegen/` exists but not integrated into any build step. | Generated code may be stale. |
| **C ABI memory ownership undocumented** | `engine/engine_plugin_abi.h` has no ownership docs. | Plugin developers risk memory bugs. |

### 🟢 Low / Nice-to-Have

| Gap | Detail |
|-----|--------|
| **Single TODO marker** | One in province centroid code |
| **No property-based/fuzz testing** | Core ECS would benefit from proptest |
| **No dirty-region rendering** | Terminal output redraws everything |
| **JSON serialization overhead** | Round-trip on every cross-language component access |

---

## 3. Top Improvement Opportunities

### Quick Wins (Hours)

| # | Improvement | Effort | Impact | Area |
|---|-------------|--------|--------|------|
| 1 | Delete stale `engine/core/Cargo.lock` | 1 min | Medium | Build |
| 2 | Add SAFETY comments to 10 undocumented unsafe blocks | 1-2 hrs | High | Safety |
| 3 | Write CONTRIBUTING.md | 2 hrs | High | Documentation |
| 4 | Create ADR-0001 (Architecture Overview) | 2 hrs | High | Documentation |
| 5 | Add unit test module in 1-2 core modules (ecs/schema, map/pathfinding) | 4 hrs | Critical | Testing |
| 6 | Add plugin ABI version field | 4 hrs | High | Cross-Language |

### Strategic Investments (Days)

| # | Improvement | Effort | Impact | Area |
|---|-------------|--------|--------|------|
| 7 | Make mlua optional behind feature flag | 1-2 days | Critical | Architecture |
| 8 | Implement or remove feature flags with cfg gates | 1 day | High | Architecture |
| 9 | Add devcontainer.json + Dockerfile | 1 day | High | DX |
| 10 | Systematic unwrap reduction (focus: job system, file I/O) | 1-2 weeks | High | Code Quality |
| 11 | Add Lua sandbox mode (remove StdLib::ALL) | 3-5 days | High | Security |
| 12 | UI test suite (widget rendering + layout + events) | 1 week | High | Testing |
| 13 | Add unit test suite for engine_core modules | 2 weeks | Critical | Testing |
| 14 | Audit and document all ~67 unsafe blocks | 3-5 days | High | Safety |
| 15 | Complete Python API parity (add input, callback_registry) | 2-3 days | Medium | Cross-Language |

### Long-term (Weeks-Months)

| # | Improvement | Effort | Impact |
|---|-------------|--------|--------|
| 16 | Complete WASM API to Lua/Python parity | Months | High |
| 17 | Replace JSON with binary serialization for cross-language ops | Months | High |
| 18 | Add property-based/fuzz testing for ECS core | Weeks | Medium |
| 19 | Add dirty-region terminal rendering | Weeks | Medium |

---

## 4. Issues Requiring Immediate Attention

These issues should be addressed before they cause production problems or developer friction:

1. **Stale `engine/core/Cargo.lock`** — Delete it. The root workspace lockfile is authoritative. This will prevent confusion. *(1 minute)*

2. **10 undocumented unsafe blocks in UI widgets** — Add `// SAFETY:` comments explaining why `ptr as u64` is sound for WidgetId. Without invariants, future refactoring could introduce UB. *(1-2 hours)*

3. **Plugin ABI unversioned** — Add a `version` field to `PluginVTable` and check it at load time. Currently incompatible `.so` files cause silent UB. *(4 hours)*

4. **Lua StdLib::ALL sandboxing gap** — Scripts have full system access. Mod authors can execute arbitrary commands. *(3-5 days)*

5. **Vestigial feature flags** — Either remove `[features] lua/python/wasm = []` or implement proper `#[cfg(feature)]` gates. Currently dead configuration misleads contributors. *(1 day)*

6. **engine_core mlua dependency** — Make it optional behind a feature flag. Currently contradicts architecture docs and forces LuaJIT install on all builds. *(1-2 days)*

---

## 5. Recommended Actions (Next Steps)

### 🔥 Immediate (this session)
1. Delete `engine/core/Cargo.lock`
2. Add SAFETY comments to 10 undocumented unsafe blocks in UI
3. Start CONTRIBUTING.md

### 📅 Short-term (this week)
4. Create devcontainer.json + Dockerfile
5. Write ADR-0001 (Architecture Overview)
6. Add plugin ABI version field
7. Add unit tests for 1-2 core modules to set precedent

### 📆 Medium-term (this month)
8. Make mlua optional in engine_core
9. Implement or remove feature flags
10. Systematic unwrap reduction — start with job system
11. Lua sandbox implementation
12. UI test suite

---

## 6. References

- `knowledge/intent-refresh-audit-2026-05-27.md` — This audit's intent
- `knowledge/analysis-codebase-map-refresh-2026-05-27.md` — Explorer: refreshed codebase map
- `knowledge/analysis-deep-dive-refresh-2026-05-27.md` — Analyzer: refreshed deep-dive
- `knowledge/report-project-audit-2026-05-27.md` — Prior report (superseded by this one)
- `AGENTS.md` — Project identity and conventions
- `Cargo.toml` — Workspace manifest
- `Makefile` — Build orchestration
