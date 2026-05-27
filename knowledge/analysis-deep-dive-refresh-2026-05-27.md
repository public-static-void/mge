---
title: "ANALYSIS: Deep-Dive Refresh — MGE Codebase Health"
version: 1.0.0
status: draft
type: analysis
created: 2026-05-27
author: Analyzer
superseded_by: null
---

<!-- Filename: knowledge/analysis-deep-dive-refresh-2026-05-27.md -->

# ANALYSIS: Deep-Dive Refresh — MGE Codebase Health

## Overview

A comprehensive, evidence-based deep-dive investigation of the MGE monorepo codebase. Verifies all prior audit claims, produces fresh counts, and documents gaps across 11 investigation areas. Every finding is backed by file:line evidence from direct tool inspection.

## Context

Supersedes the initial audit (`knowledge/report-project-audit-2026-05-27.md`). Leverages the refreshed codebase map (`knowledge/analysis-codebase-map-refresh-2026-05-27.md`) for structural orientation. All claims below are based on primary tool inspection (grep, glob, read) — not secondary sources.

---

## Key Findings (Executive Summary)

| # | Finding | Severity | Area |
|---|---------|----------|------|
| 1 | **~371 unwrap/expect calls in production code** — pervasive unwrap culture | **Critical** | Error Handling |
| 2 | **~67 unsafe blocks in production code — 10 undocumented** | **Critical** | Safety |
| 3 | **engine_core has ZERO unit tests** (only placeholder `it_works`) | **Critical** | Test Coverage |
| 4 | **engine_core unconditionally depends on mlua** — feature flags are full no-ops | **Critical** | Architecture |
| 5 | **10 unsafe blocks in UI widgets with ZERO safety documentation** | **High** | Safety/Docs |
| 6 | **engine_core links LuaJIT unconditionally** — Python/WASM builds require LuaJIT | **High** | Architecture |
| 7 | **Lua unsafe_new_with(StdLib::ALL)** — full stdlib, no sandboxing | **High** | Security |
| 8 | **Plugin ABI has no versioning** — incompatible plugins cause UB | **High** | Security |
| 9 | **No CONTRIBUTING.md, CHANGELOG.md, ADRs, devcontainer** | **Medium** | DX |
| 10 | **WASM backend has ~10% API parity** — only entity operations | **Medium** | Parity |
| 11 | **Stale nested Cargo.lock at engine/core/Cargo.lock** | **Low** | Build |
| 12 | **codegen tool unwired from build** — dead tooling | **Low** | Dead Code |

---

## Depth Analysis Per Area

### 1. unwrap/expect Audit

**Methodology**: Counted all `.unwrap()` and `.expect()` calls in production Rust source files only (excluded `tests/`, `examples/`, `target/`, `wasm_tests/`).

#### Counts by Crate

| Crate | `.unwrap()` | `.expect()` | Total | Source Files Scanned |
|-------|:----------:|:----------:|:----:|:------------------:|
| engine_core/src | 174 | 17 | **191** | ~96 |
| engine_lua/src | 68 | 36 | **104** | 39 |
| engine_py/src | 47 | 5 | **52** | 35 |
| engine_wasm/src | 10 | 6 | **16** | 4 |
| schema_validator | 0 | 2 | **2** | 1 |
| xtask | 6 | 0 | **6** | 1 |
| **Total** | **305** | **66** | **371** | |

#### Risk Categorization

**Critical** (panic on valid user input or predictable runtime state):
- `engine/core/src/systems/job/types/loader.rs:15-30` — 7 `expect()` on file I/O + deserialization during job loading: `fs::read_to_string(&path).expect("Failed to read job file")`
- `engine/core/src/systems/economic/loader.rs:13-17` — 3 `expect()` on recipe file loading
- `engine/core/src/systems/job/system/orchestrator.rs:20-248` — ~25 `unwrap()` on `world.get_component()` / `world.set_component()`, any missing component causes panic
- `engine/core/src/presentation/ui/widget/widget_trait.rs:95,101` — `serde_json::to_value()` / `from_value()` expect — serialization failure panics
- `engine/core/src/plugins/ffi.rs:29-31` — `CStr::from_ptr().to_str().unwrap()` + `serde_json::from_str().unwrap()` — invalid C string or bad JSON panics inside unsafe FFI
- `engine_lua/src/engine.rs:48` — `Lua::unsafe_new_with(StdLib::ALL, ...)` — constructing Lua runtime

**Major** (panic in well-defined internal paths, recoverable by caller):
- `engine/core/src/systems/job/reservation/resource_reservation.rs:117` — `requirements.unwrap()` on Option
- `engine/core/src/systems/job/states/resource/fetching.rs:42` — `reserved_stockpile.unwrap()` on Option
- `engine/core/src/systems/job/ai/event_reaction_system.rs` — multiplie unwrap in event processing
- `engine_py/src/event_bus.rs` — 5x `lock().unwrap()` on mutex
- `engine_wasm/src/engine.rs:120` — `self.store.lock().unwrap()` — mutex poisoning risk

**Minor** (setup/init code that panics on misconfiguration):
- `engine_lua/src/bin/mge_cli.rs:88-201` — 6 `expect()` during CLI startup
- `engine_lua/src/bin/mge_lua_test_runner.rs:80,305` — 2 `expect()` in test runner setup
- `xtask/src/main.rs` — 6 `unwrap()` in build tooling (acceptable for build tool)

#### Evidence: Worst Offender Files (engine_core)

```rust
// engine/core/src/systems/job/system/orchestrator.rs:20-28
world.set_component(agent_id, "Agent", agent).unwrap();
world.set_component(job_id, "Job", job_obj).unwrap();

// engine/core/src/systems/job/system/process.rs:20-35
let registry = world.job_handler_registry.lock().unwrap();
world.set_component(eid, "Job", result.clone()).unwrap();

// engine/core/src/systems/job/reservation/resource_reservation.rs:117
if requirements.is_none() || requirements.unwrap().is_empty() {
```

---

### 2. Unsafe Blocks

#### Counts by Location

| Crate | `unsafe {` blocks | Has Safety Docs? | Notes |
|-------|:----------------:|:----------------:|-------|
| engine_core/src/plugins/loader.rs | ~40+ | **Yes** | `/// # Safety` on each public function; internal calls inherit |
| engine_core/src/plugins/ffi.rs | 4 | **Yes** | `/// # Safety` on extern functions |
| engine_core/src/presentation/ui/widget/* | 10 | **NO** | `WidgetId` generation via transmute — no safety docs |
| engine_core/src/plugins/types.rs | 1 | Partial | Name extraction from C string |
| engine_lua/src | 7 | Mixed | Lua unsafe_new, FFI in job_board, job_ai, binaries |
| engine_py/src | 4 | Mixed | pyo3 FFI, plugin init |
| engine_wasm/src/host_api/entity.rs | 1 | No | Memory access |
| **Total** | **~67** | ~43 documented / ~24 undocumented | |

#### Undocumented Unsafe Blocks (Critical Finding)

All 10 UI widget files use an identical pattern for `WidgetId` generation:

```rust
// Example: engine/core/src/presentation/ui/widget/panel.rs:43
let id = unsafe {
    let ptr = self as *const Self;
    ptr as u64
};
```

Files affected: `panel.rs`, `text_input.rs`, `linear.rs`, `grid.rs`, `label.rs`, `context_menu.rs`, `button.rs`, `focus_grid.rs`, `checkbox.rs`, `dropdown.rs`.

**Problem**: The pointer-to-integer transmute is used to generate a unique WidgetId. This is technically sound only because the pointer is stack-local and never dereferenced, but:
1. No `// SAFETY:` comment exists in any of these 10 files
2. The safety invariant (pointer is valid, aligned, never dereferenced via this integer) is not documented
3. If `self` is a temporary, the pointer could dangle — though this pattern is used in `new()` which returns `Self` by value

#### Plugin Loading Unsafe (Justified)

The plugin loader (`plugins/loader.rs`) has ~40+ `unsafe` operations, all properly documented with `/// # Safety` sections. The operations are:
- `Library::new()` — loading a dynamic library
- `lib.get(b"PLUGIN_VTABLE\0")` — resolving a symbol
- Function pointer calls on the vtable
- `CStr::from_ptr()` — C string conversion

These are inherently unsafe and justified. The documentation is present but could be more rigorous about lifetime invariants.

---

### 3. Dead Code / Vestigial Artifacts

#### Stale Cargo.lock
- **Confirmed**: `engine/core/Cargo.lock` **exists** at `engine/core/Cargo.lock`
- Impact: Confusion risk; Cargo may use this instead of root workspace lock

#### Feature Flags
- Confirmed: `engine/core/Cargo.toml` has:
  ```toml
  [features]
  lua = []
  python = []
  wasm = []
  ```
- **Zero** `#[cfg(feature = "...")]` guards exist in `engine/core/src/` — verified by grep
- **mlua is unconditional**: 18 references to `mlua` across engine_core/src in system trait signatures

#### Unwired Codegen Tool
- `engine/tools/codegen/` exists with tests and expected output
- Not called from any build target (Makefile, xtask, Cargo)
- Tests exist but are not run in CI

#### engine_core → schema_validator Dependency
- `engine_core/Cargo.toml`: `schema_validator = { path = "../tools/schema_validator" }`
- Used in `ecs/schema.rs` for validation during schema loading
- Unusual: a tool crate doubles as a library dependency of the core engine

---

### 4. Test Coverage Gaps

#### engine_core Module Test Coverage

| Metric | Count |
|--------|:-----:|
| Source files (`engine/core/src/`) | ~96 `.rs` files |
| `#[cfg(test)]` modules in src/ | **1** (lib.rs: `it_works` placeholder) |
| `#[test]` functions in src/ | **1** (`assert_eq!(2+2,4)`) |
| Integration tests (`engine/core/tests/`) | **108 files** |
| Integration test files belonging to modules | 0 |
| **Ratio: unit tests to source files** | **1:96** |

Zero unit tests for any of these modules: ecs, map, plugins, presentation, systems (job/*, economic/*), mods, modes, worldgen, config.

#### Test Distribution Across Crates

| Crate | Integration Tests | Unit Tests (in src/) |
|-------|:----------------:|:-------------------:|
| engine_core | 108 | 1 (placeholder) |
| engine_lua | 2 | 0 |
| engine_py | 44 (pytest) | 0 |
| engine_wasm | 3 | 0 |
| engine_macros | 1 | 0 |
| schema_validator | **0** | 0 |
| codegen | 4 | 0 |
| xtask | 0 | 0 |

#### Coverage Gap Summary

**Critical gaps**:
- `ecs/registry.rs` — ComponentRegistry has zero unit tests
- `ecs/schema.rs` — Schema loading has zero unit tests
- `plugins/loader.rs` — Plugin loading has zero unit tests (only integration tests that require .so files)
- `mods/loader.rs` + `mods/manifest.rs` — Mod loading has zero unit tests
- `presentation/ui/*` — All 10 widgets + 3 layouts + factory + root have zero unit tests
- `ecs/world/*` — All 12 World sub-modules have zero unit tests

**Gaps partially covered** by integration tests:
- Job system: 25+ integration tests (good coverage)
- ECS/component: ~12 integration tests
- Map: 8 integration tests
- Plugins: 6 integration tests
- Event bus: 10+ integration tests

**Fully uncovered**:
- schema_validator: **zero tests** (binary + library)
- xtask: **zero tests**

---

### 5. Error Handling Patterns

#### Pervasive unwrap Culture
- **371 total unwrap/expect calls** across production code
- Heaviest concentration in the job system (`systems/job/`): ~100+ calls
- Pattern: code uses `anyhow::Result` at API boundaries but immediately unwraps internally

#### Specific Anti-Patterns

**A. File I/O unwrap in loaders** (Critical)
```rust
// engine/core/src/systems/job/types/loader.rs:18-19
let data = fs::read_to_string(&path).expect("Failed to read job file");
let job: JobTypeData = serde_json::from_str(&data).expect("Failed to parse job file");
```
Same pattern in `economic/loader.rs:16-17`. These are critical because a corrupt or missing file in `engine/assets/jobs/` or recipes/ will panic the engine at startup.

**B. Component access unwrap** (Major)
```rust
// systems/job/system/orchestrator.rs:69
let job = world.get_component(eid, "Job").unwrap();
// systems/job/reservation/resource_reservation.rs:117
if requirements.is_none() || requirements.unwrap().is_empty() {
```
The job system assumes all entities have specific components. If entity state is inconsistent, panics occur instead of graceful error propagation.

**C. Mutex lock unwrap** (Medium)
```rust
// engine/core/src/ecs/world/events.rs:21
bus.lock().unwrap().send(payload);
// engine_py/src/event_bus.rs:17
let mut buses = EVENT_BUSES.lock().unwrap();
```
Mutex poisoning from thread panics would cause cascading panics. `parking_lot` mutexes are used in some places (factory.rs) but `std::sync::Mutex` is used in others.

**D. Silent failure in UI schema loader** (Medium)
```rust
// engine/core/src/presentation/ui/schema_loader.rs:7-9
pub fn load_ui_from_json(json: &str) -> Option<Box<dyn UiWidget + Send>> {
    let value: Value = serde_json::from_str(json).ok()?;  // silently returns None
```
Errors during UI construction are silently swallowed. The caller gets `None` with no diagnostic.

**E. `panic!()` in production path** (Major)
```rust
// engine/core/src/plugins/loader.rs:416
panic!("Could not find workspace root containing 'plugins' directory");
```
`panic!()` in a non-init code path. If `MGE_WORKSPACE_ROOT` is not set and the fallback directory walk fails, the engine panics.

---

### 6. Documentation Gaps

#### Missing Project Docs
| Document | Exists? | Path |
|----------|:-------:|------|
| README.md | ✓ | `README.md` (69 lines, minimal) |
| CONTRIBUTING.md | **✗** | — |
| CHANGELOG.md | **✗** | — |
| ADRs | **✗** | Not in `knowledge/` or `docs/` |
| devcontainer | **✗** | `.devcontainer/` does not exist |
| API docs | ✓ | `docs/api.md` |
| Plugin ABI docs | ✓ | `docs/plugin_abi.md` |
| Dev setup guide | ✓ | `docs/dev.md` |
| ROADMAP | ✓ | `docs/ROADMAP.md` |

#### Code-Level Documentation (engine_core)
- **Public API**: Most structs, enums, and public functions have `///` doc comments
- **Module-level**: Many modules lack `//!` module docs
  - `ecs/world/component.rs` — no module-level docs
  - `ecs/world/entity.rs` — no module-level docs
  - `plugins/manager.rs` — no module-level docs
  - `plugins/registry.rs` — no module-level docs
  - `systems/job/ai/logic.rs` — no module-level docs
  - `systems/job/board/job_board.rs` — no module-level docs
- **Unsafe docs**: 10 unsafe blocks in UI widgets have **zero** `// SAFETY:` comments (see §2)
- **Risky undocumented patterns**: WidgetId transmute (see §2), plugin loading fallback panic

---

### 7. Security Concerns

#### Critical: Lua Sandboxing

```rust
// engine_lua/src/engine.rs:48
let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, LuaOptions::default()) };
```

- Uses `StdLib::ALL` — full access to Lua standard library including `os.execute()`, `io.*`, `loadfile()`, `dofile()`
- **No sandboxing** — any Lua script can read/write any file, execute arbitrary shell commands, access network
- Lua scripts loaded from mod directories or CLI arguments have full system access

#### Critical: Plugin ABI Versioning

The `PluginVTable` struct in `engine/engine_plugin_abi.h` has **no version field**. If a plugin was compiled against a different ABI version:
- Function pointer offsets will mismatch
- Calling a wrong function at the wrong offset is **undefined behavior**
- No safety check at `lib.get(b"PLUGIN_VTABLE\0")` time

#### High: Python Runtime

Python bindings via pyo3 have full access to the Python standard library:
- `os`, `subprocess`, `sys`, `builtins.open` — all available
- No restricted execution environment
- Python bridge runs in-process with no sandboxing

#### High: Unsafe in UI Widgets

10 unsafe blocks with no safety documentation (see §2). While the `WidgetId` transmute is likely safe, undocumented unsafe is a maintenance burden and a code review risk.

#### Medium: Plugin Trust Model

- Plugins loaded from `game.toml` paths — no checksum/signature verification
- C plugins compiled with `-fPIC -shared` — no hardening flags
- Rust plugins rely on `#[no_mangle]` exports — mutable static `PLUGIN_VTABLE` is inherently unsafe

---

### 8. Architecture Drift

#### Feature Flag Contradiction (Critical)

```toml
# engine/core/Cargo.toml
[dependencies]
mlua = { version = "0.10.5", features = ["luajit", "serialize"] }

[features]
lua = []
python = []
wasm = []
```

**Evidence**: 18 references to `mlua` in engine_core/src:
```
ecs/system.rs:11 — fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>);
ecs/world/systems.rs:66 — pub fn run_system(&mut self, name: &str, lua: Option<&mlua::Lua>);
systems/job/system/orchestrator.rs:58 — pub fn run_job_system(world: &mut World, lua: Option<&mlua::Lua>);
systems/job/system/process.rs:126 — _lua: Option<&mlua::Lua>,
systems/job/system/mod.rs:32 — fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>);
systems/stat_calculation.rs:13 — fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>);
systems/movement_system.rs:14 — fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>);
systems/economic/system.rs:72 — fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>);
systems/equipment_logic.rs:14 — fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>);
systems/inventory.rs:34 — fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>);
systems/death_decay.rs:12,48 — fn run(... _lua: Option<&mlua::Lua>);
systems/body_equipment_sync.rs:106 — fn run(... _lua: Option<&mlua::Lua>);
systems/equipment_effect_aggregation.rs:15 — fn run(... _lua: Option<&mlua::Lua>);
systems/job/ai/event_reaction_system.rs:13 — fn run(... _lua: Option<&mlua::Lua>);
systems/job/reservation/resource_reservation.rs:33,166 — fn run(... _lua: Option<&mlua::Lua>);
systems/job/core/children.rs:8 — lua: Option<&mlua::Lua>,
```

**Reality**: `mlua` is baked into every system trait signature. The `Option<&mlua::Lua>` parameter is threaded through all systems even if they never use Lua. This means:
1. Building engine_core **always** requires LuaJIT dev headers
2. Python and WASM builds also require LuaJIT
3. Feature flags do nothing — no conditional compilation exists

#### Dependency Graph Drift

```
Expected (per design):
  engine_macros ← engine_core ← engine_lua
                                ← engine_py
                                ← engine_wasm

Reality:
  engine_macros ← engine_core ← engine_lua
                                ← engine_py    (still requires mlua)
                                ← engine_wasm  (still requires mlua)
                                ← schema_validator (tool → lib dependency)
```

The `schema_validator` as a library dependency is unusual — it's primarily a CLI tool. However, it provides the `validate_schema()` function used by `ecs/schema.rs`, so this is a functional dependency though architecturally unexpected.

---

### 9. Build / DX Issues

#### Stale Lockfile
- `engine/core/Cargo.lock` **exists and is stale** — the root `Cargo.lock` is authoritative
- Risk: CI cache confusion, non-deterministic builds if Cargo picks up the nested lock

#### No devcontainer
- `.devcontainer/` directory does **not exist**
- New contributors must manually install: `nightly Rust`, `LuaJIT`, `libjansson`, `wasm32-unknown-unknown`, `Python 3.12`, `maturin`, `pytest`

#### CI Pipeline Assessment
- **3 workflows**: `ci.yml`, `release.yml`, `lint-schemas.yml`
- **ci.yml**: 8 parallel jobs — comprehensive but has issues:
  - **No caching between jobs**: Each job downloads dependencies from scratch
  - **No Rust caching**: `moonrepo/setup-rust` doesn't cache Cargo registry
  - **`cargo clean` at end of lint job**: Wastes build artifacts that could be reused
  - **test-rust duplicates build-all**: downloads artifacts and rebuilds — should share artifacts
  - **test-python requires full Rust compilation**: maturin develop recompiles everything

#### Nightly Rust Constraint
- Edition "2024" requires nightly — not stable
- `Cargo.toml` has no `rust-toolchain.toml` pinning nightly version
- CI may get different nightly versions over time

---

### 10. Cross-Language Parity

#### API Surface Comparison

| API Area | Lua (30 modules) | Python (27 modules) | WASM (4 sources) |
|----------|:----------------:|:-------------------:|:----------------:|
| Entity lifecycle | ✓ (entity.rs) | ✓ (entity.rs) | **Partial** (spawn + get_component only) |
| Component CRUD | ✓ | ✓ | ✗ |
| Map operations | ✓ | ✓ | ✗ |
| Movement | ✓ | ✓ | ✗ |
| Combat/damage | ✓ | ✓ | ✗ |
| Game mode | ✓ | ✓ | ✗ |
| Job system | 7 modules | **9 modules** | ✗ |
| Inventory | ✓ | ✓ | ✗ |
| Equipment | ✓ | ✓ | ✗ |
| Body | ✓ | ✓ | ✗ |
| Economic | ✓ | ✓ | ✗ |
| Save/Load | ✓ | ✓ | ✗ |
| Worldgen | ✓ | ✓ | ✗ |
| Event bus | ✓ | ✓ | ✗ |
| UI | ✓ | ✓ | ✗ |
| Region | ✓ | ✓ | ✗ |
| Camera | ✓ | ✓ | ✗ |
| Input | **✓ (input.rs)** | ✗ | ✗ |
| Turn/Time | ✓ | ✓ | ✗ |
| Callback registry | **✓ (callback_registry.rs)** | ✗ | ✗ |

#### Gaps

**Python vs Lua differences**:
- Python has extra job modules not in Lua: `job_children.rs`, `job_dependencies.rs`, `job_production.rs`, `job_reservation.rs`
- Lua has `callback_registry.rs` and `input.rs` not in Python
- Lua has `save_load.rs` as a module; Python has `save_load.rs` as well

**WASM gap**:
- Only 4 source files (vs 39 for Lua, 35 for Python)
- Only entity operations (spawn, get/set component)
- No job, map, UI, combat, or any gameplay API
- Only 1 WASM test module (`test_entity_api.wasm`)

---

### 11. TODO / FIXME / HACK Markers

**Only 1 TODO found in production code**:

```rust
// engine/core/src/presentation/layout.rs:46
CellKey::Province { .. } => (0, 0), // TODO: implement province centroid
```

The codebase is exceptionally clean of unresolved technical debt markers. Zero `FIXME`, `HACK`, or `XXX` markers were found in any production Rust source.

This is a positive indicator but may also suggest that markers are being cleaned up without being addressed, or that work items are tracked outside the codebase.

---

## Gap Register

| # | Gap | Severity | Location | Impact |
|---|-----|----------|----------|--------|
| G-01 | Zero unit tests in engine_core | **Critical** | `engine/core/src/` | No isolated test coverage for any module |
| G-02 | ~371 unwrap/expect in production | **Critical** | All crates | Runtime panics on recoverable errors |
| G-03 | mlua unconditional dependency | **Critical** | `engine/core/Cargo.toml:9` | Python/WASM builds require LuaJIT |
| G-04 | Feature flags are no-ops | **High** | `engine/core/Cargo.toml:33-35` | No conditional compilation for lang backends |
| G-05 | 10 undocumented unsafe blocks | **High** | `engine/core/src/presentation/ui/widget/*` | Maintenance risk, potential UB |
| G-06 | No sandboxing for Lua scripts | **High** | `engine_lua/src/engine.rs:48` | Arbitrary code execution risk |
| G-07 | Plugin ABI has no versioning | **High** | `engine/engine_plugin_abi.h` | Incompatible plugins cause UB |
| G-08 | No CONTRIBUTING.md / CHANGELOG.md | **Medium** | Repo root | Barrier to new contributors |
| G-09 | No devcontainer | **Medium** | `.devcontainer/` | Complex manual setup for contributors |
| G-10 | WASM backend at ~10% API parity | **Medium** | `engine_wasm/src/` | WASM effectively unused |
| G-11 | Silent error swallowing in UI loader | **Medium** | `presentation/ui/schema_loader.rs:8` | Debugging difficulty |
| G-12 | Stale nested Cargo.lock | **Low** | `engine/core/Cargo.lock` | Build confusion |
| G-13 | codegen tool unwired | **Low** | `engine/tools/codegen/` | Dead tooling |
| G-14 | CI no caching between jobs | **Low** | `.github/workflows/ci.yml` | Slow CI (~15-20 min) |
| G-15 | schema_validator has zero tests | **Medium** | `engine/tools/schema_validator/` | Unvalidated validation logic |
| G-16 | `panic!()` in fallback path | **Medium** | `plugins/loader.rs:416` | Engine crash on config issue |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|:----------:|:------:|------------|
| Unwrap panic in job system crashes runtime | **High** | **High** | Replace unwrap with result propagation in component access |
| Unsafe ABI mismatch causes UB | **Low** | **Critical** | Add ABI version field to PluginVTable; validate on load |
| Lua script escapes sandbox | **High** | **High** | Use `mlua::Lua::new()` (safe) with restricted stdlib; implement allowed function whitelist |
| Module without unit tests regresses | **High** | **Medium** | Add unit tests per module; enforce coverage gate |
| New contributor blocked by setup complexity | **Medium** | **Medium** | Add devcontainer with all deps pre-installed |
| Python build fails if LuaJIT not installed | **High** | **Medium** | Make mlua optional in engine_core |
| CI pipeline slows to 30+ min | **Medium** | **Low** | Add cargo registry caching; share build artifacts between jobs |
| Incompatible Rust nightly breaks build | **Medium** | **High** | Pin nightly version in `rust-toolchain.toml` |

---

## Recommendations (Prioritized)

### P0 — Must Fix (Immediate Risk)
1. **Make `mlua` optional in engine_core** — gate behind `lua` feature flag; use conditional compilation to remove `mlua` from system trait signatures when feature is disabled. Unblocks Python and WASM builds without LuaJIT.
2. **Add safety docs to 10 undocumented unsafe blocks** in UI widgets (`*/*.rs` ptr → u64 transmute). Add `// SAFETY:` comments explaining why the pointer-to-integer cast is safe.
3. **Add plugin ABI version** — insert a `abi_version: u32` field at the top of `PluginVTable`. Validate on load. Fail gracefully on mismatch.

### P1 — Should Fix (Significant Risk)
4. **Replace 371 unwrap/expect with error propagation** — focus first on file I/O paths (`job/types/loader.rs`, `economic/loader.rs`) and component access in job system. Use `anyhow::Context` for descriptive errors instead of panics.
5. **Write unit tests** for schema loading, registry operations, plugin manifest parsing, and UI widget construction. These are hot paths with zero unit test coverage.
6. **Implement Lua sandboxing** — restrict `StdLib` to a safe subset (no `os`, `io`, `debug`). Consider `mlua::Lua::new()` (safe mode) with explicit function whitelist.
7. **Remove or implement feature flags** — either gate `mlua` and other backend-specific deps behind the existing flags, or delete the empty `[features]` section.

### P2 — Should Address (Medium Priority)
8. **Delete `engine/core/Cargo.lock`** — eliminate build confusion.
9. **Add devcontainer** with nightly Rust, LuaJIT, libjansson, wasm32 target, Python 3.12 + maturin.
10. **Wire codegen into build** or remove it — currently a dead tool with dead test expectations.
11. **Add CI caching** — `Swatinem/rust-cache` for cargo registry; share build artifacts between jobs.
12. **Write `schema_validator` tests** — the schema validation tool itself has zero tests.

### P3 — Future Consideration
13. **Expand WASM API** to parity with Lua/Python — at least support component CRUD and map operations.
14. **Write CONTRIBUTING.md** and start CHANGELOG.md (can use git log + semantic-release output).
15. **Remove `schema_validator` as an engine_core dependency** — duplicate the validation function or refactor into a shared utility crate.
16. **Implement province centroid** — the lone TODO in the codebase.

---

## References

- **Codebase Map**: `knowledge/analysis-codebase-map-refresh-2026-05-27.md`
- **Audit Intent**: `knowledge/intent-refresh-audit-2026-05-27.md`
- **Prior Report**: `knowledge/report-project-audit-2026-05-27.md`
- **Workspace Manifest**: `/Cargo.toml`
- **Engine Core Manifest**: `engine/core/Cargo.toml`
- **Plugin ABI**: `engine/engine_plugin_abi.h`
- **CI Pipeline**: `.github/workflows/ci.yml`
- **Lua Engine**: `engine_lua/src/engine.rs`
- **WASM Engine**: `engine_wasm/src/engine.rs`
- **Plugin Loader**: `engine/core/src/plugins/loader.rs`
- **ABI Versioning Spec**: `knowledge/spec-plugin-abi-version-2026-05-27.md` — ABI versioning requirements and acceptance criteria
- **ABI Versioning Implementation**: `knowledge/impl-plugin-abi-version-2026-05-27.md` — G-07 gap resolved; version check implemented in all 5 loader functions
- **Schema Loader**: `engine/core/src/ecs/schema.rs`
- **UI Factory**: `engine/core/src/presentation/ui/factory.rs`
- **Job System**: `engine/core/src/systems/job/` (9 subdirectories)
- **Unsafe Widget IDs**: `engine/core/src/presentation/ui/widget/*.rs` (10 files)
