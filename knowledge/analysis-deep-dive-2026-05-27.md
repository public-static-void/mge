---
title: "ANALYSIS: Deep-Dive Investigation — Gaps, Issues, Improvements"
version: 1.0.0
status: draft
type: analysis
created: 2026-05-27
author: Analyzer
superseded_by: null
---

# ANALYSIS: Deep-Dive Investigation — Gaps, Issues, Improvements

## 1. Build Health

### Build Status
- Project builds in CI (GitHub Actions) on push/PR to main
- Requires nightly Rust (edition "2024") — blocks stable/beta toolchains
- Requires LuaJIT system library (`libluajit-5.1-dev`)
- Requires C plugins: gcc + libjansson-dev
- Python: maturin + venv setup required
- WASM: requires wasm32-unknown-unknown target

### Build Issues
| Issue | Severity | Details |
|-------|----------|---------|
| Stale nested Cargo.lock | Medium | `engine/core/Cargo.lock` exists and is stale. Building from engine/core/ would use incorrect deps. Root workspace Cargo.lock is authoritative. |
| Nightly-only | High | Blocks all non-nightly users. 2024 edition not yet stable (expected Rust 1.85+). |
| No devcontainer/Docker | Medium | Setup requires 5+ system dependencies, no reproducible dev environment. |
| xtask builds are slow | Low | Builds all plugins in release mode unconditionally — no caching distinction. |

### Warnings
- No build warnings were suppressed — CI enforces clean builds
- Some dead code detected in feature-gated modules (not compiled but present)

## 2. Code Quality

### Clippy / Linting
- CI enforces `cargo clippy --all-targets --all-features -- -D warnings` — clean in CI

### Error Handling
- **Good**: Uses `thiserror` for library errors and `anyhow` for application-level errors. Consistent pattern.
- **Good**: Most Result types are properly propagated.
- **Issue**: ~1465 `.unwrap()` calls across the codebase (found via grep). Many are in test code or safe contexts, but production paths include unwraps on:
  - JSON deserialization (schema loading)
  - Plugin symbol resolution (ffi)
  - Registry lookups
  - File I/O
- **Issue**: ~201 `.expect()` calls — slightly better (messages provided) but still risky in production paths.
- **Recommendation**: Systematic reduction of unwrap/expect in production code, replacing with proper error propagation.

### Unsafe Code
- ~84 `unsafe` blocks across the codebase
- **Justified**: FFI calls (plugin loading, libloading, C ABI), type punning in internals
- **Unjustified (approx 10)**: Raw pointer manipulation in UI code (presentation/layout.rs, presentation/ui/), unchecked indexing
- **Recommendation**: Audit all ~84 blocks; document safety invariants for each; refactor UI unsafe to safe alternatives.

### Dead Code
- Feature flag gates (`#[cfg(feature = "lua")]` etc.) never trigger since features are empty
- Some modules may have dead code paths behind these flags
- Codegen tool exists but is never invoked — may have bitrotted

### Large / Complex Functions
- `engine_lua/src/bin/mge_lua_test_runner.rs` (380 lines) — the most complex binary, could be refactored
- `engine/core/src/systems/job/ai/assigner.rs` — complex AI assignment logic, moderate complexity
- Schema loading functions are well-factored

## 3. Dependencies

### Outdated / Problematic Dependencies

| Dependency | Crate | Version | Issue |
|------------|-------|---------|-------|
| mlua | engine_core | 0.10.5 | **Architectural concern** — makes engine_core depend on LuaJIT unconditionally despite claiming to be "pure Rust core" |
| bson | engine_macros | ~0.17 | Heavy dependency for proc-macro; used for BSON serialization in the macro |
| serde_yaml | engine_core | ~0.9 | Single use (loading game.toml which could be TOML or JSON). Redundant with toml crate |
| wasmtime | engine_wasm | >=36.0.3 | Async feature enabled — check if actually needed |
| gag | engine_lua | (test only) | Captures stdout during Lua tests — unusual but intentional |

### Redundancy
- Both `serde_yaml` AND `toml` in engine_core — game.toml could be read with just `toml` or just JSON
- `once_cell` and `lazy_static` both present — redundant, `OnceCell`/`OnceLock` now in std
- `parking_lot` for mutex — std Mutex has improved but parking_lot is justified for perf

### Size Concerns
- `bson` in `engine_macros` is heavy for a proc-macro crate — adds MongoDB BSON serialization for what could be simpler encoding
- `jsonschema` crate is a major dependency but justified (runtime schema validation)

## 4. Test Coverage

### Test Locations & Counts (Refined)

| Location | Count | What It Tests | Coverage Quality |
|----------|-------|---------------|-----------------|
| engine/core/tests/ | 108 | Integration: jobs (heavy), ECS, map, plugins, movement, combat, save/load | Good for integration, but ZERO unit tests |
| engine/core/src/ | 0 unit tests | Core library code | **Critical gap** — no #[cfg(test)] mod tests anywhere in engine_core/src/ |
| engine_py/tests/ | 44 | Python bindings — full API surface | Good coverage of Python side |
| engine/scripts/lua/tests/ | 47 | Lua scripts — ECS, map, jobs, systems | Good coverage of Lua API |
| engine_lua/tests/ | 2 | Script loading, input | Minimal |
| engine_wasm/tests/ | 3 | WASM entity API | Minimal |
| engine_macros/tests/ | Check needed | #[component] macro tests | Unknown |
| schema_validator/tests/ | Check needed | Schema validation | Unknown |
| tools/codegen/ | 0 | Codegen tool | None |

### Critical Gaps

| Area | Gap | Severity |
|------|-----|----------|
| **Unit tests** | Zero unit tests in engine_core/src/ — all testing is integration-level | **HIGH** |
| **UI / Presentation** | No tests for UI widgets, layout, renderer | **HIGH** |
| **Mod loader** | No tests for mod loading/manifest parsing | MEDIUM |
| **Worldgen** | Minimal testing | LOW |
| **Plugin subsystem** | FFI loader, subprocess protocol untested | MEDIUM |
| **Schema validator** | Tool itself may have untested edge cases | MEDIUM |
| **Engine macros** | Generated code not tested for edge cases | MEDIUM |
| **WASM** | Only basic entity API tested | HIGH (relative to WASM scope) |

### Test Quality Observations
- Lua tests have full state isolation (fresh World each test) — excellent pattern
- Python tests use conftest.py fixtures — good pattern
- Rust integration tests require pre-built C plugins and WASM modules — makes running them complex
- No fuzz testing, property-based testing, or stress testing

## 5. Documentation

### What Exists
- `AGENTS.md` at root — comprehensive project guide
- `.gitmessage` — commit format convention
- README — basic, needs expansion

### What's Missing

| Document | Gap | Severity |
|----------|-----|----------|
| Architecture Decision Records | No ADRs — no record of WHY decisions were made | **HIGH** |
| CONTRIBUTING.md | No contribution guide | **HIGH** |
| API docs | Public API has some doc comments but many undocumented functions | MEDIUM |
| Plugin developer guide | No docs for writing C/Rust plugins | MEDIUM |
| C ABI memory ownership | `engine_plugin_abi.h` doesn't document who owns allocated memory | MEDIUM |
| Mod developer guide | No docs for creating mods | LOW |
| Setup guide | No single "getting started" doc | MEDIUM |
| CHANGELOG | Check if exists (semantic-release should generate) | LOW |

### Inline Documentation Quality
- Engine core modules: moderate doc coverage, some modules well-documented, others sparse
- Lua/Python API: function-level docs present but minimal
- Job system: well-documented internally
- UI widgets: mostly undocumented

## 6. Security

### Unsafe Block Audit
- ~84 unsafe blocks
- Mostly in: plugin FFI (libloading), C ABI boundary, internal type casting
- ~10 blocks in UI/presentation code without clear justification
- No `# Safety` docs on unsafe functions in most cases

### Input Validation
| Surface | Status | Risk |
|---------|--------|------|
| JSON Schema loading | Validated by schema_validator | Low |
| Lua scripts | Full LuaJIT runtime access (os.execute, io.*) | **HIGH** — no sandboxing |
| Python scripts | Full Python runtime access | **HIGH** — no sandboxing |
| Plugin .so loading | Arbitrary native code execution by design | Medium (expected) |
| Subprocess protocol | JSON-line protocol, no auth | MEDIUM |
| Mod manifests | JSON parsed, minimal validation | LOW |
| File paths in config | Path traversal risk in schema dir, mod dir overrides | MEDIUM |
| C ABI strings | Null-terminated C strings from plugins | MEDIUM |

### Key Security Gaps
1. **No Lua sandbox** — Lua scripts have full access to os.execute, io, etc. This is by design for mods but risky for untrusted content
2. **No Python sandbox** — Same issue as Lua
3. **Plugin trust model** — All plugins are implicitly trusted (loaded from configured paths)
4. **Subprocess protocol** — No authentication or integrity checking on the Unix socket

## 7. Developer Experience

### Setup Friction

| Requirement | Friction |
|-------------|----------|
| Nightly Rust | Must install specific toolchain |
| LuaJIT system dep | Requires dev package + pkg-config |
| gcc + libjansson-dev | Required for C plugin compilation |
| maturin + venv | Python build overhead |
| WASM target | rustup target add wasm32-unknown-unknown |
| LD_LIBRARY_PATH | Must set env var before running tests |

### CI/CD
- GitHub Actions with 4 workflows (ci.yml, release.yml, lint-schemas.yml)
- CI runs on push/PR to main
- 8 CI jobs with artifact passing — fragile chain
- Release automated with semantic-release
- Pre-commit: .gitmessage convention, no automated hook scripts

### Tooling
- Makefile provides good entry points
- No devcontainer or Docker setup
- No pre-commit hooks configured
- No `.env` or config templates

## 8. Cross-Language

### API Parity

| Feature | Lua | Python | WASM | C ABI |
|---------|-----|--------|------|-------|
| Entity CRUD | ✅ Full | ✅ Full | ✅ Basic | ✅ Via EngineApi |
| Component CRUD | ✅ Full | ✅ Full | ❌ | ✅ Via EngineApi |
| Map operations | ✅ Full | ✅ Full | ❌ | ❌ |
| Movement | ✅ | ✅ | ❌ | ❌ |
| Combat | ✅ | ✅ | ❌ | ❌ |
| Mode switching | ✅ | ✅ | ❌ | ❌ |
| Full job system | ✅ | ✅ (extra) | ❌ | ❌ |
| Worldgen | ✅ | ✅ | ❌ | ✅ worldgen |
| Event bus | ✅ | ✅ | ❌ | ❌ |
| UI API | ✅ | ✅ | ❌ | ❌ |
| Save/load | ✅ | ✅ | ❌ | ❌ |

### Issues
1. **WASM severely incomplete** — only entity API exposed, 3 test files, 2 host API modules
2. **Python has extra modules** — job_production, job_children, job_dependencies, job_reservation exist in Python but NOT in Lua API (or they use different paths)
3. **mlua configured for LuaJIT** — no PUC-Rio Lua support, limits portability
4. **C ABI ownership undefined** — `free_result_json` exists but protocol for who allocates/frees other strings is undocumented
5. **No cross-language type system** — Components serialized as JSON strings across language boundaries, no shared type definitions

## 9. Performance

### Hot Path Concerns

| Pattern | Concern | Severity |
|---------|---------|----------|
| JSON everywhere | Component data serialized to/from JSON on every cross-language access | **HIGH** |
| String-keyed ECS | Component names, queries use String keys — O(n) lookups | MEDIUM |
| Clone-heavy | Unnecessary clones in hot paths (query results, entity iteration) | MEDIUM |
| No dirty tracking | Renderer appears to redraw everything each frame | MEDIUM |
| BSON in macros | Heavy encoding format in proc-macro | LOW |
| Serialization overhead | serde_json round-trips for every component access from Lua/Python | **HIGH** |

### Architecture Notes
- ECS fits typical game engine pattern — should be cache-friendly
- Job system uses complex state machines which may add overhead
- Map operations (pathfinding) are likely the hottest path in gameplay

## 10. Architecture

### Identified Issues

| Issue | Severity | Details |
|-------|----------|---------|
| **Vestigial feature flags** | HIGH | `[features] lua = [] python = [] wasm = []` in engine_core. Empty, no cfg gates. Misleading. Should either be removed or properly implemented. |
| **mlua in engine_core** | HIGH | engine_core claims "no language binding dependencies" but depends on mlua (LuaJIT). Makes core depend on a specific scripting runtime. Should be optional or behind a feature flag. |
| **Stale nested Cargo.lock** | MEDIUM | engine/core/Cargo.lock exists and will be stale. Confusing for anyone building from that directory. |
| **Codegen unwired** | MEDIUM | engine/tools/codegen/ exists but no build step invokes it. May have bitrotted. |
| **Two plugin systems** | MEDIUM | Native ABI (libloading + PluginVTable) and subprocess (Unix socket) coexist. Need clear guidance on when to use which. |
| **Schema modes extension** | LOW | Non-standard JSON Schema extension (`modes` field) — no standard tooling understands it. |
| **late-bound component types** | MEDIUM | Components are resolved by string name at runtime — no compile-time safety for cross-language access. |

## 11. Top Issues (Ranked)

| Rank | Issue | Severity | Area | Impact |
|------|-------|----------|------|--------|
| 1 | Zero unit tests in engine_core/src/ | CRITICAL | Testing | All testing is integration-level; no isolated verification of individual modules |
| 2 | mlua dependency in engine_core | HIGH | Architecture | Contradicts design principle; forces LuaJIT link on all builds |
| 3 | Vestigial feature flags | HIGH | Architecture | Misleading; no way to disable language bridges |
| 4 | No Lua/Python sandbox | HIGH | Security | Untrusted scripts have full system access |
| 5 | ~1465 unwrap() calls in production | HIGH | Code Quality | Risk of panics on unexpected states |
| 6 | ~10 unjustified unsafe blocks | HIGH | Security | UI code uses unsafe without documentation |
| 7 | WASM backend incomplete | HIGH | Completeness | Only entity API works; far behind Lua/Python |
| 8 | No ADRs or CONTRIBUTING.md | HIGH | Documentation | No design rationale or contribution process |
| 9 | JSON serialization overhead | MEDIUM | Performance | Every cross-language component access serializes/deserializes JSON |
| 10 | Stale nested Cargo.lock | MEDIUM | Build | Confusing, potential stale builds |
| 11 | UI/Presentation not tested | MEDIUM | Testing | Entire terminal UI framework has no tests |
| 12 | No Docker/devcontainer | MEDIUM | DX | High setup friction for new contributors |

## 12. Improvement Opportunities (Ranked by Impact/Effort)

| Rank | Improvement | Impact | Effort | Area |
|------|-------------|--------|--------|------|
| 1 | Add unit tests to engine_core/src/ | High | Medium | Testing |
| 2 | Make mlua optional in engine_core | High | Medium | Architecture |
| 3 | Implement/remove feature flags properly | High | Low | Architecture |
| 4 | Create Docker/devcontainer setup | High | Medium | DX |
| 5 | Add Lua sandbox (luau or mlua sandbox) | High | Medium | Security |
| 6 | Systematically reduce unwrap/expect | High | High | Code Quality |
| 7 | Remove stale nested Cargo.lock | Medium | Low | Build |
| 8 | Add ADR process and initial ADRs | Medium | Low | Documentation |
| 9 | Write CONTRIBUTING.md | Medium | Low | Documentation |
| 10 | Add unit tests for UI subsystem | Medium | Medium | Testing |
| 11 | Wire up codegen tool to build | Medium | Low | Build |
| 12 | Add dirty-region tracking to renderer | Medium | High | Performance |
| 13 | Document C ABI memory ownership | Low | Low | Documentation |
| 14 | Add property-based tests for core ECS | Low | Medium | Testing |
| 15 | Complete WASM API | Low | High | Cross-Language |
