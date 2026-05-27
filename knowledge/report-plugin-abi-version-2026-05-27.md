---
title: "REPORT: Plugin ABI Versioning — Implementation Complete"
version: 1.0.0
status: draft
type: report
created: 2026-05-27
author: Overseer
superseded_by: null
---

# REPORT: Plugin ABI Versioning

## Summary

Implemented ABI versioning for the MGE plugin system, preventing silent undefined behavior when incompatible plugins are loaded. **Commit `872ff8e`** on `main`.

## What Was Done

| Area | Change |
|------|--------|
| **C ABI Header** | Added `#define PLUGIN_ABI_VERSION 1` + `unsigned int abi_version` as first field of `PluginVTable` |
| **Rust Types** | Added `pub const PLUGIN_ABI_VERSION: u32 = 1` + `pub abi_version: u32` to Rust `PluginVTable` |
| **Plugin Loader** | Version check inserted in all 5 load functions (after null check, before `init()`) |
| **5 Plugins** | All set `abi_version = PLUGIN_ABI_VERSION` in their vtables |
| **2 Test Plugins** | Created `test_abi_mismatch` (v999) and `test_abi_zero` (v0) for testing |
| **Integration Tests** | 12 tests covering all acceptance criteria |
| **Documentation** | `docs/plugin_abi.md` updated with versioning scheme |

## Verification

| Check | Result |
|-------|--------|
| New ABI tests (12) | ✅ All pass |
| All existing tests (~200+) | ✅ No regressions |
| Clippy (`-D warnings`) | ✅ Clean |
| Formatting (`cargo fmt --check`) | ✅ Clean |
| Inspector review | ✅ **PASS** — all 10 acceptance criteria met |

## Impact

- **Before**: Loading a plugin compiled against a different engine version → silent UB and crashes
- **After**: Mismatched version → clear error message: `"plugin <path>: ABI version mismatch (expected 1, got N)"`

## Files Changed

**26 files** across 9 directories (4053 insertions, 8 deletions):
- Core: `engine_plugin_abi.h`, `types.rs`, `loader.rs`
- Plugins: 5 plugin source files
- Tests: `abi_version.rs` (291 lines), 2 test plugins
- Docs: `plugin_abi.md`
- Knowledge: 13 KD files

## References

- `knowledge/intent-plugin-abi-version-2026-05-27.md` — Original intent
- `knowledge/spec-plugin-abi-version-2026-05-27.md` — Specification
- `knowledge/plan-plugin-abi-version-2026-05-27.md` — Task plan
- `knowledge/impl-plugin-abi-version-2026-05-27.md` — Implementation summary
- `knowledge/review-plugin-abi-version-2026-05-27.md` — Review with PASS verdict
- `knowledge/composed-plugin-abi-version-2026-05-27.md` — Composed context for future work
- `knowledge/extract-plugin-abi-version-2026-05-27.md` — Knowledge extraction
