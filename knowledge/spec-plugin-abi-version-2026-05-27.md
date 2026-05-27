---
title: "SPEC: Plugin ABI Versioning"
version: 1.0.0
status: approved
type: spec
created: 2026-05-27
author: Spec Weaver
superseded_by: null
---

# SPEC: Plugin ABI Versioning

## Overview

Add an ABI version field to the PluginVTable struct and validate it at plugin load time. This prevents silent undefined behavior when loading plugins compiled against a different version of the C ABI. The version is a single integer (starting at 1), placed at offset 0 of PluginVTable for forward-compatibility detection. The check runs once at load time and returns an error on mismatch — never a panic.

## Functional Requirements

- **R001 — ABI version constant**: Define `PLUGIN_ABI_VERSION` as the integer `1` in the authoritative C header. Mirror it as a Rust constant in the plugin types module. This is the single source of truth for the current ABI version.
- **R002 — Version field in PluginVTable**: Add an `abi_version` field (`unsigned int` in C, `u32` in Rust) as the **first field** (offset 0) of the `PluginVTable` struct, before all function pointers.
- **R003 — Load-time version validation**: Every plugin loader function reads `abi_version` from the loaded vtable and compares it to `PLUGIN_ABI_VERSION`. If they differ, loading is aborted with an error.
- **R004 — Graceful error on mismatch**: Version mismatch produces a returned `Err(String)` — never a panic, unwrap, or abort. The error message must identify the plugin path, the expected version, and the actual version.
- **R005 — All 5 plugins export the version**: Every plugin (4 C, 1 Rust) sets `vtable.abi_version = PLUGIN_ABI_VERSION` in its vtable initialization.
- **R006 — Version check before init**: The abi_version check occurs **before** calling `vtable_ref.init()`, so an incompatible plugin never executes any code.
- **R007 — Outdated plugin detection**: A plugin compiled without the version field (pre-versioning) must be rejected. Because the field is at offset 0, what was previously the `init` function pointer will be read as a `u32` and almost certainly not equal `1`, causing rejection.
- **R008 — Documentation update**: `docs/plugin_abi.md` must document the version field, the constant, and the versioning scheme.

## Non-Functional Requirements

- **NFR001 — Zero overhead for compatible plugins**: The version check is a single integer comparison. No measurable impact on load performance.
- **NFR002 — Backward-incompatible detection only**: The version is a single integer that increments on any breaking ABI change. No semver, no compatibility matrix.
- **NFR003 — Consistent error format**: All version mismatch errors across all 5 loader functions use the same error message format (see Interface Specification).
- **NFR004 — Version constant is compile-time**: `PLUGIN_ABI_VERSION` is a preprocessor constant in C and a `const`/`constexpr` in Rust — zero runtime overhead.

## Interface Specification

### 3.1 `PLUGIN_ABI_VERSION` Constant

**C header** (`engine/engine_plugin_abi.h`):

```c
#define PLUGIN_ABI_VERSION 1
```

Insert after the `#include` guard / before struct definitions. This macro is used by both C plugins and the engine's C-side validation logic.

**Rust mirror** (`engine/core/src/plugins/types.rs`):

```rust
/// The current ABI version that the engine expects plugins to match.
pub const PLUGIN_ABI_VERSION: u32 = 1;
```

### 3.2 Modified `PluginVTable` Struct (C)

**Current** (`engine/engine_plugin_abi.h`, lines 24-35):

```c
typedef struct PluginVTable {
  int (*init)(struct EngineApi *api, void *world);
  void (*shutdown)();
  void (*update)(float delta_time);
  const char *(*worldgen_name)();
  int (*generate_world)(const char *params_json, char **out_result_json);
  void (*free_result_json)(char *result_json);
  int (*register_systems)(struct EngineApi *api, void *world,
                          SystemPlugin **systems, int *count);
  void (*free_systems)(SystemPlugin *systems, int count);
  void *(*hot_reload)(void *old_state);
} PluginVTable;
```

**Required change**: Add `abi_version` as the **first field**:

```c
typedef struct PluginVTable {
  unsigned int abi_version;  // MUST equal PLUGIN_ABI_VERSION
  int (*init)(struct EngineApi *api, void *world);
  void (*shutdown)();
  void (*update)(float delta_time);
  const char *(*worldgen_name)();
  int (*generate_world)(const char *params_json, char **out_result_json);
  void (*free_result_json)(char *result_json);
  int (*register_systems)(struct EngineApi *api, void *world,
                          SystemPlugin **systems, int *count);
  void (*free_systems)(SystemPlugin *systems, int count);
  void *(*hot_reload)(void *old_state);
} PluginVTable;
```

**Requirements for the field**:
- Type: `unsigned int` (C) / `u32` (Rust) — same width on all platforms
- Position: **first field, offset 0** — before all function pointers
- Documentation comment: `// MUST equal PLUGIN_ABI_VERSION` or equivalent

### 3.3 Modified `PluginVTable` Struct (Rust)

**Current** (`engine/core/src/plugins/types.rs`, lines 51-78):

```rust
#[repr(C)]
pub struct PluginVTable {
    pub init: unsafe extern "C" fn(*mut EngineApi, *mut c_void) -> c_int,
    pub shutdown: unsafe extern "C" fn(),
    pub update: unsafe extern "C" fn(c_float),
    pub worldgen_name: Option<unsafe extern "C" fn() -> *const c_char>,
    pub generate_world: Option<unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int>,
    pub free_result_json: Option<unsafe extern "C" fn(*mut c_char)>,
    pub register_systems: Option<...>,
    pub free_systems: Option<...>,
    pub hot_reload: Option<unsafe extern "C" fn(old_state: *mut c_void) -> *mut c_void>,
}
```

**Required change**: Insert `abi_version` as the first field:

```rust
#[repr(C)]
pub struct PluginVTable {
    /// MUST equal PLUGIN_ABI_VERSION
    pub abi_version: u32,
    pub init: unsafe extern "C" fn(*mut EngineApi, *mut c_void) -> c_int,
    // ... all existing fields unchanged ...
}
```

### 3.4 Loader Version Check Logic

**Pattern** (applied in all 5 loader functions in `engine/core/src/plugins/loader.rs`):

After obtaining `vtable_ref` and before calling any function through it:

```
let plugin_version = vtable_ref.abi_version;
if plugin_version != PLUGIN_ABI_VERSION {
    return Err(format!(
        "Plugin '{{path}}' ABI version mismatch: expected {}, got {}",
        PLUGIN_ABI_VERSION,
        plugin_version,
    ));
}
```

**Applicable functions** (all 5 in `loader.rs`):

| Function | Line (approx) | Notes |
|---|---|---|
| `load_plugin` | 24 | Basic plugin load |
| `load_plugin_and_register_worldgen_threadsafe` | 51 | Thread-safe worldgen load |
| `load_plugin_and_register_worldgen` | 120 | Worldgen load |
| `load_plugin_and_register_systems` | 189 | System registration load |
| `load_plugin_with_manifest` | 268 | Manifest-based load |

**Check placement**: Immediately after the null-pointer check on `plugin_vtable` (e.g., after line 36 in `load_plugin`). The check sequence in every function must be:

1. `Library::new(path)` — open the .so
2. `lib.get(b"PLUGIN_VTABLE\0")` — get the symbol
3. Null-check on `plugin_vtable` — reject if null
4. **✦ NEW: Check `vtable_ref.abi_version` against `PLUGIN_ABI_VERSION`** — reject on mismatch
5. `vtable_ref.init(...)` — call init function
6. ... rest of loading ...

### 3.5 Error Message Format

All mismatch errors across all 5 loader functions must use an identical format:

```
Plugin '<resolved_path>' ABI version mismatch: expected <PLUGIN_ABI_VERSION>, got <actual_version>
```

Where `<resolved_path>` is `path.as_ref().display()` or the resolved absolute path, and `<actual_version>` is the integer value read from the vtable.

### 3.6 Plugin Changes (4 C plugins)

**Each C plugin constructor** must add:

```c
__attribute__((constructor)) void init_vtable() {
  vtable.abi_version = PLUGIN_ABI_VERSION;
  // ... existing field assignments ...
}
```

**Affected files**:
- `plugins/simple_square_plugin/simple_square_plugin.c` (line 141)
- `plugins/simple_hex_plugin/simple_hex_plugin.c` (line 113)
- `plugins/simple_province_plugin/simple_province_plugin.c` (line 54)
- `plugins/test_plugin/test_plugin.c` (line 32)

### 3.7 Plugin Change (1 Rust plugin)

**Rust vtable initialization** must add the field:

```rust
static mut VTABLE: PluginVTable = PluginVTable {
    abi_version: PLUGIN_ABI_VERSION,
    init,
    shutdown,
    update,
    // ... existing fields unchanged ...
};
```

**Affected file**: `plugins/rust_test_plugin/src/lib.rs` (lines 115-125)

### 3.8 PluginVTable Declaration in Rust Plugin

The `plugins/rust_test_plugin/src/lib.rs` file has its own `PluginVTable` struct definition (lines 36-62). This duplicate definition must also include the `abi_version: u32` field at position 0 to match the ABI layout.

## Acceptance Criteria

### AC001 — Matching version loads successfully
- **Given**: A plugin `.so` compiled with `abi_version = PLUGIN_ABI_VERSION` (i.e., 1)
- **When**: The engine loads the plugin via any of the 5 load functions
- **Then**: Loading succeeds with `Ok(())`

### AC002 — Mismatched version is rejected
- **Given**: A plugin `.so` with `abi_version = 999` (or any value ≠ 1)
- **When**: The engine attempts to load the plugin
- **Then**: Loading returns `Err(...)` with message matching the format in §3.5

### AC003 — Version check precedes init
- **Given**: A plugin whose `abi_version` does not match
- **When**: The engine attempts to load the plugin
- **Then**: The plugin's `init` function is **never called** (the error is returned before the `init` call)

### AC004 — Error is returned, not panicked
- **Given**: Any version mismatch scenario
- **When**: The engine loads the plugin
- **Then**: The result is a returned `Err(String)` — no panic, no abort, no unwrap

### AC005 — Pre-versioning plugin is rejected
- **Given**: A plugin `.so` compiled against the old ABI (no `abi_version` field)
- **When**: The engine loads the plugin
- **Then**: Loading returns `Err(...)` because the data at offset 0 (previously the `init` function pointer) does not equal `PLUGIN_ABI_VERSION`

### AC006 — All 5 plugins set the version correctly
- **Given**: Each of the 5 plugin source files
- **When**: Inspected at source level
- **Then**: Each plugin's vtable initialization sets `abi_version = PLUGIN_ABI_VERSION`
- **Plugins to verify**: `simple_square_plugin.c`, `simple_hex_plugin.c`, `simple_province_plugin.c`, `test_plugin.c`, `rust_test_plugin/src/lib.rs`

### AC007 — Error message contains path, expected, and actual
- **Given**: A version mismatch
- **When**: The error is returned
- **Then**: The error string contains the plugin path, `PLUGIN_ABI_VERSION` (1), and the actual version read from the vtable

### AC008 — All 5 loader functions perform the check
- **Given**: Each of the 5 plugin loader functions
- **When**: The function loads a plugin with mismatched version
- **Then**: The function returns an ABI version error
- **Functions to verify**: `load_plugin`, `load_plugin_and_register_worldgen_threadsafe`, `load_plugin_and_register_worldgen`, `load_plugin_and_register_systems`, `load_plugin_with_manifest`

### AC009 — Constant is defined once in C, once in Rust
- **Given**: The codebase
- **When**: Searched for `PLUGIN_ABI_VERSION`
- **Then**: Exactly two definitions exist — one `#define` in `engine/engine_plugin_abi.h` and one `pub const` in `engine/core/src/plugins/types.rs`

### AC010 — Version 1 remains 1
- **Given**: The initial implementation
- **When**: The value of `PLUGIN_ABI_VERSION`
- **Then**: It is `1` in both the C header and the Rust constant

## Edge Cases and Failure Modes

| Edge case | Behavior | Requirement |
|---|---|---|
| Plugin `.so` does not exist | Returns `Err("...not found...")` — existing behavior, unchanged | — |
| `PLUGIN_VTABLE` symbol missing | Returns `Err("...symbol not found...")` — existing behavior, unchanged | — |
| VTable pointer is null | Returns `Err("PLUGIN_VTABLE symbol is null")` — existing behavior, unchanged | — |
| VTable `abi_version` is 0 | Rejected with version mismatch error | R003, R004 |
| VTable `abi_version` is garbage (pre-versioning plugin) | Almost certainly rejected — the old `init` pointer address as `u32` won't equal 1 | R007 |
| VTable `abi_version` matches (success case) | Loading proceeds normally to `init` call | R001, R002 |
| Engine constant is bumped but plugin is old | Plugin rejected with error `"...expected 2, got 1..."` | R003, R004 |
| Plugin compiled with version > engine expects (future ABI) | Plugin rejected with error `"...expected 1, got 2..."` | R003 |
| Same plugin loaded twice | Version check passes both times (loading itself may fail on duplicate registration — existing behavior) | — |

## File Change List

### Modified files

| # | File | Change |
|---|---|---|
| 1 | `engine/engine_plugin_abi.h` | Add `#define PLUGIN_ABI_VERSION 1`; add `unsigned int abi_version` as first field of `PluginVTable`; add doc comment |
| 2 | `engine/core/src/plugins/types.rs` | Add `pub const PLUGIN_ABI_VERSION: u32 = 1;`; add `pub abi_version: u32` as first field of Rust `PluginVTable` |
| 3 | `engine/core/src/plugins/loader.rs` | Add version check in all 5 load functions after null check, before `init` call |
| 4 | `plugins/simple_square_plugin/simple_square_plugin.c` | Set `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 5 | `plugins/simple_hex_plugin/simple_hex_plugin.c` | Set `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 6 | `plugins/simple_province_plugin/simple_province_plugin.c` | Set `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 7 | `plugins/test_plugin/test_plugin.c` | Set `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 8 | `plugins/rust_test_plugin/src/lib.rs` | Add `pub abi_version: u32` to its local `PluginVTable` definition; set `abi_version: PLUGIN_ABI_VERSION` in static VTABLE |
| 9 | `docs/plugin_abi.md` | Document the version field, the `PLUGIN_ABI_VERSION` constant, and the versioning scheme; update the VTable structure table |

### Unchanged files (explicit)

| File | Reason |
|---|---|
| `engine/core/src/plugins/mod.rs` | No changes needed — public API re-exports remain the same |
| `engine_core` public API | `EngineApi`, `SystemPlugin`, plugin manifest — unchanged |
| `game.toml` | Plugin config format unchanged |
| Lua/Python bindings | These use `engine_core` APIs, not the C ABI directly — no changes needed |
| xtask | Plugin build/deploy logic unchanged |

## Open Questions

1. **Should the version constant be exported as a dynamic symbol from the engine for runtime comparison?** Currently, the engine checks the plugin's version against a compile-time constant on the engine side. This means the engine and plugin must be compiled with the same header. An alternative would be for the engine to also export `PLUGIN_ABI_VERSION` as a dynamic symbol so a plugin could verify the engine's version against its own. **Decision from INTENT**: Not needed — single integer, compile-time constant, engine is authoritative.
2. **Should the version check extend to the Lua/Python test runners?** These load plugins indirectly via `engine_core` — the check happens inside the rust loader, so it's covered automatically.
3. **Should there be a dedicated unit test for version mismatch?** Yes — create a minimal `.so` with a wrong version and verify it's rejected. This is not a spec concern (Pathfinder/Artisan domain), but is noted here for completeness.

## References

- **Intent**: `knowledge/intent-plugin-abi-version-2026-05-27.md`
- **Audit**: `knowledge/report-refresh-audit-2026-05-27.md` (gap identification)
- **Analysis**: `knowledge/analysis-deep-dive-refresh-2026-05-27.md` (risk analysis, §3 recommendation)
- **Code map**: `knowledge/analysis-codebase-map-refresh-2026-05-27.md`
- **C ABI header**: `engine/engine_plugin_abi.h`
- **Rust types**: `engine/core/src/plugins/types.rs`
- **Plugin loader**: `engine/core/src/plugins/loader.rs`
- **Plugin sources**: `plugins/simple_square_plugin/`, `plugins/simple_hex_plugin/`, `plugins/simple_province_plugin/`, `plugins/test_plugin/`, `plugins/rust_test_plugin/`
- **ABI documentation**: `docs/plugin_abi.md`
