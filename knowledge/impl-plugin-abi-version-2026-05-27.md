---
title: "IMPL: Plugin ABI Versioning"
version: 1.0.0
status: verified
type: implementation-summary
created: 2026-05-27
author: Artisan
superseded_by: null
---

# IMPL: Plugin ABI Versioning

## Summary

Implemented ABI versioning for the MGE plugin system. Added an `abi_version` field (at offset 0) to `PluginVTable`, a `PLUGIN_ABI_VERSION` constant (value `1`), and load-time version validation in all 5 plugin loader functions. All 5 existing plugins updated to export the version. Two test plugins created for mismatch detection. 12 integration tests cover all acceptance criteria.

## Files Changed / Created

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `engine/engine_plugin_abi.h` | Modified | Added `#define PLUGIN_ABI_VERSION 1` and `unsigned int abi_version` as first struct field |
| 2 | `engine/core/src/plugins/types.rs` | Modified | Added `pub const PLUGIN_ABI_VERSION: u32 = 1;` and `pub abi_version: u32` as first struct field |
| 3 | `engine/core/src/plugins/loader.rs` | Modified | Added version check to all 5 loader functions (after null check, before init) |
| 4 | `plugins/rust_test_plugin/src/lib.rs` | Modified | Added local `PLUGIN_ABI_VERSION` constant, `abi_version` field to struct and VTABLE init |
| 5 | `plugins/simple_square_plugin/simple_square_plugin.c` | Modified | Added `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 6 | `plugins/simple_hex_plugin/simple_hex_plugin.c` | Modified | Added `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 7 | `plugins/simple_province_plugin/simple_province_plugin.c` | Modified | Added `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 8 | `plugins/test_plugin/test_plugin.c` | Modified | Added `vtable.abi_version = PLUGIN_ABI_VERSION;` in `init_vtable()` |
| 9 | `plugins/test_abi_mismatch/test_abi_mismatch.c` | Created | Test plugin with `abi_version = 999` for mismatch testing |
| 10 | `plugins/test_abi_zero/test_abi_zero.c` | Created | Test plugin with `abi_version = 0` for pre-versioning detection testing |
| 11 | `engine/core/tests/abi_version.rs` | Created | 12 integration tests covering AC001-AC010 |
| 12 | `docs/plugin_abi.md` | Modified | Added ABI versioning section, updated VTable structure documentation |

## Deviations from Plan

None significant. Minor adjustments:
- Used `use crate::plugins::PLUGIN_ABI_VERSION;` import in `loader.rs` instead of adding to the existing `types` import
- Adjusted `abi_version.rs` test for clippy compliance (removed `useless_format` lint)
- The test plugin directories follow the plan's naming: `test_abi_mismatch` and `test_abi_zero` (not `test_abi_mismatch_plugin`)

## Test Results

### ABI Version Integration Tests (12 tests) — ALL PASSED

| Test | AC Covered | Result |
|------|-----------|--------|
| `test_matching_version_loads_test_plugin` | AC001 | PASS |
| `test_matching_version_loads_simple_square` | AC001 | PASS |
| `test_matching_version_loads_simple_hex` | AC001 | PASS |
| `test_matching_version_loads_simple_province` | AC001 | PASS |
| `test_matching_version_loads_rust_test_plugin` | AC001 | PASS |
| `test_mismatched_version_rejected` | AC002, AC007 | PASS — error contains "expected 1, got 999" + plugin path |
| `test_zero_version_rejected` | AC005 | PASS — error contains "expected 1, got 0" |
| `test_load_plugin_rejects_mismatch` | AC008 | PASS — `load_plugin` returns error |
| `test_load_plugin_and_register_worldgen_rejects_mismatch` | AC008 | PASS — `load_plugin_and_register_worldgen` returns error |
| `test_load_plugin_and_register_worldgen_threadsafe_rejects_mismatch` | AC008 | PASS — `load_plugin_and_register_worldgen_threadsafe` returns error |
| `test_load_plugin_and_register_systems_rejects_mismatch` | AC008 | PASS — `load_plugin_and_register_systems` returns error |
| `test_load_plugin_with_manifest_rejects_mismatch` | AC008 | PASS — `load_plugin_with_manifest` returns error |

### Full Test Suite — ALL 120+ TESTS PASSED

All existing integration tests (108+ tests across all test files) continue to pass. No regressions.

### Lint & Format

- `cargo clippy -p engine_core --all-targets --all-features -- -D warnings` — PASS
- `cargo fmt --all --check` — PASS

## Acceptance Criteria Verification

| AC | Status | Verification |
|----|--------|-------------|
| AC001 — Matching version loads | ✅ | 5 tests load each real plugin successfully |
| AC002 — Mismatched version rejected | ✅ | `test_abi_mismatch` (version=999) returns `Err` |
| AC003 — Version check precedes init | ✅ | Check is structurally before init in all 5 functions (verified by source + test flow) |
| AC004 — Error returned, not panicked | ✅ | All mismatch tests assert `Err(String)` — no panic |
| AC005 — Pre-versioning rejected | ✅ | `test_abi_zero` (version=0) returns `Err` |
| AC006 — All 5 plugins set version | ✅ | Source inspection + successful loading of all 5 plugins |
| AC007 — Error message format | ✅ | Error contains path, "expected 1", "got <actual>" |
| AC008 — All 5 functions check | ✅ | Each function tested with mismatch plugin |
| AC009 — Constant defined once each | ✅ | `#define` in C header, `pub const` in Rust types.rs |
| AC010 — Version 1 remains 1 | ✅ | Both constants are `1` |

## Notes

- Test plugins `test_abi_mismatch` and `test_abi_zero` are for testing only and **must not** be included in production builds. They are automatically built by `cargo run -p xtask -- build-c-plugins` (they appear as directories with a single `.c` file under `plugins/`).
- The WASM test build failure (`wasm32-unknown-unknown` target not installed) is pre-existing and unrelated to this implementation.
