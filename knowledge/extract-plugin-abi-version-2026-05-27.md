---
title: "EXTRACT: Plugin ABI Versioning — Patterns, Context, Stale KD Cleanup"
version: 1.0.0
status: draft
type: extract
created: 2026-05-27
author: Scribe
superseded_by: null
---

# EXTRACT: Plugin ABI Versioning — Knowledge Extraction

## Overview

Knowledge extraction from the Plugin ABI Versioning implementation lifecycle (INTENT → SPEC → PLAN → IMPL → REVIEW). Documents the patterns that emerged, composes context for future agents, cleans up stale references, and updates cross-references across the KD library.

## Patterns Extracted

### Pattern 1: ABI Versioning at Offset 0
The version field is placed as the **first field** (offset 0) of `PluginVTable`, before all function pointers. This is intentional:
- A pre-versioning plugin (old `.so`) has its `init` function pointer at offset 0, which when read as `u32` almost certainly won't equal the expected version — automatically rejecting outdated plugins.
- The engine always reads offset 0 as `abi_version`, making it the single invariant check point.
- **Applies to**: Any C ABI plugin system that needs backward-incompatibility detection.

### Pattern 2: Loader Version Check Sequence
Every plugin load function follows this exact sequence:
1. `Library::new(path)` — open the `.so`
2. `lib.get(b"PLUGIN_VTABLE\0")` — resolve the symbol
3. Null-check on `plugin_vtable` — reject if null
4. **Check `vtable_ref.abi_version` against `PLUGIN_ABI_VERSION`** — reject on mismatch
5. `vtable_ref.init(...)` — call init function
6. ... rest of loading ...

The version check is **always** before the `init` call — an incompatible plugin never executes any code.

### Pattern 3: Dual-Constant Definition
`PLUGIN_ABI_VERSION` is defined twice:
- C: `#define PLUGIN_ABI_VERSION 1` in `engine/engine_plugin_abi.h` (authoritative)
- Rust: `pub const PLUGIN_ABI_VERSION: u32 = 1;` in `engine/core/src/plugins/types.rs`

A third copy exists in `plugins/rust_test_plugin/src/lib.rs` because the standalone cdylib cannot import from `engine_core`. This is a documented exception — tests verify runtime equivalence.

### Pattern 4: Test Plugin Per Directory
The xtask `build_c_plugins()` expects exactly **one `.c` file per directory** under `plugins/`. Each test plugin with a wrong ABI version needs its own directory:
- `plugins/test_abi_mismatch/` — `abi_version = 999`
- `plugins/test_abi_zero/` — `abi_version = 0`

They are automatically picked up by the build, and must be excluded from production builds.

### Pattern 5: 5-in-1 Function Coverage Testing
All 5 loader functions must be independently tested with a mismatch plugin to prove version-check coverage. The test file (`abi_version.rs`) has one test per function:
- `test_load_plugin_rejects_mismatch`
- `test_load_plugin_and_register_worldgen_rejects_mismatch`
- `test_load_plugin_and_register_worldgen_threadsafe_rejects_mismatch`
- `test_load_plugin_and_register_systems_rejects_mismatch`
- `test_load_plugin_with_manifest_rejects_mismatch`

### Pattern 6: Consistent Error Message Format
All version mismatch errors use an identical format string:
```
Plugin '<resolved_path>' ABI version mismatch: expected <PLUGIN_ABI_VERSION>, got <actual_version>
```
This is enforced across all 5 loader functions and tested in `test_mismatched_version_rejected`.

### Pattern 7: Version Check Before Library Lifetime Ends
In loader functions where `lib` (Library handle) is moved into a closure (e.g., `load_plugin_and_register_worldgen_threadsafe`), the version check must occur **before** the move. `vtable_ref` borrows from `lib` via `Symbol` — the check happens while `lib` is still alive. Rust's borrow checker enforces this.

## Composed Context Summary

A COMPOSED KD has been created at:
- `knowledge/composed-plugin-abi-version-2026-05-27.md` — Minimal KD set for future developers

It includes:
- Required reading: INTENT → SPEC → PLAN (in order) for full lifecycle understanding
- Reference documents: C header, Rust types, loader source, tests
- Optional: REVIEW KD for findings, IMPL KD for implementation details
- Excluded: Analysis/report KDs from the audit (background only, not needed for understanding the feature itself)

## Stale KD Cleanup

### Assessment

After auditing all 13 KDs in `knowledge/`, no existing KDs are **superseded** by the ABI versioning implementation.

**Rationale**:
- `report-refresh-audit-2026-05-27.md` — Historical report that identified the ABI versioning gap (item 6 in Quick Wins). It correctly captured the problem. Resolving the gap doesn't supersede the report — the report remains valid as the audit document.
- `analysis-deep-dive-refresh-2026-05-27.md` — Deep analysis that found G-07 (Plugin ABI has no versioning). Historical snapshot of findings. Not superseded.
- `analysis-codebase-map-refresh-2026-05-27.md` — Codebase map documenting the old PluginVTable struct. Snapshot of architecture at time of exploration. Not superseded.

These documents served their purpose: they identified the gap that motivated this implementation. Cross-references have been added so readers can find the fix.

### Status Transitions Applied

The lifecycle KDs for this feature have been advanced:

| KD | Old Status | New Status | Reason |
|----|-----------|------------|--------|
| `intent-plugin-abi-version-2026-05-27.md` | `draft` | `approved` | INTENT was implemented and verified |
| `spec-plugin-abi-version-2026-05-27.md` | `draft` | `approved` | SPEC was fully implemented and accepted |
| `plan-plugin-abi-version-2026-05-27.md` | `draft` | `approved` | PLAN was executed completely |
| `impl-plugin-abi-version-2026-05-27.md` | `verified` | `verified` | No change — correct status |
| `review-plugin-abi-version-2026-05-27.md` | `verified` | `verified` | No change — correct status |

## Cross-Reference Updates

### Added to existing KDs

| KD | Cross-Reference Added |
|----|----------------------|
| `report-refresh-audit-2026-05-27.md` | References to INTENT → SPEC → PLAN → IMPL → REVIEW chain |
| `analysis-deep-dive-refresh-2026-05-27.md` | References to SPEC, IMPL (gap resolved) |
| `analysis-codebase-map-refresh-2026-05-27.md` | References to SPEC, IMPL (struct layout updated) |

### Already present (verified)

| KD | Cross-References Present |
|----|-------------------------|
| `intent-plugin-abi-version-2026-05-27.md` | References to `report-refresh-audit`, ABI header, plugins |
| `spec-plugin-abi-version-2026-05-27.md` | References to INTENT, audit, analysis docs |
| `plan-plugin-abi-version-2026-05-27.md` | References to SPEC |
| `impl-plugin-abi-version-2026-05-27.md` | References to PLAN |
| `review-plugin-abi-version-2026-05-27.md` | References to SPEC, PLAN, IMPL |

## Knowledge Library Impact

| Dimension | Impact |
|-----------|--------|
| **New KDs created** | `composed-plugin-abi-version-2026-05-27.md`, `extract-plugin-abi-version-2026-05-27.md` |
| **KDs modified** | `intent-*` (status), `spec-*` (status), `plan-*` (status), `report-refresh-audit` (xref), `analysis-deep-dive-refresh` (xref), `analysis-codebase-map-refresh` (xref) |
| **KDs superseded** | None |
| **Patterns captured** | 7 patterns documented above |

## References

- `knowledge/intent-plugin-abi-version-2026-05-27.md`
- `knowledge/spec-plugin-abi-version-2026-05-27.md`
- `knowledge/plan-plugin-abi-version-2026-05-27.md`
- `knowledge/impl-plugin-abi-version-2026-05-27.md`
- `knowledge/review-plugin-abi-version-2026-05-27.md`
- `knowledge/composed-plugin-abi-version-2026-05-27.md`
- `knowledge/report-refresh-audit-2026-05-27.md`
- `knowledge/analysis-deep-dive-refresh-2026-05-27.md`
- `knowledge/analysis-codebase-map-refresh-2026-05-27.md`
