---
title: "COMPOSED: Context for Future Developers — Plugin ABI Versioning"
version: 1.0.0
status: draft
type: composed
created: 2026-05-27
author: Scribe
superseded_by: null
---

# COMPOSED: Context for Future Developers — Plugin ABI Versioning

## Target

- **Agent**: Future Artisan / Pathfinder (or human developer)
- **Task**: Understanding or modifying the Plugin ABI Versioning system
- **Overseer Reference**: `knowledge/intent-plugin-abi-version-2026-05-27.md`

## Required Reading

These KDs must be loaded in order for full lifecycle understanding:

1. `knowledge/intent-plugin-abi-version-2026-05-27.md` — Motivation and scope
2. `knowledge/spec-plugin-abi-version-2026-05-27.md` — Requirements, acceptance criteria, interface spec
3. `knowledge/plan-plugin-abi-version-2026-05-27.md` — Task breakdown, dependency graph, risk analysis

## Reference Documents

- `engine/engine_plugin_abi.h` — C header with `PLUGIN_ABI_VERSION` define and modified `PluginVTable` struct
- `engine/core/src/plugins/types.rs` — Rust mirror of `PluginVTable` + `PLUGIN_ABI_VERSION` constant
- `engine/core/src/plugins/loader.rs` — All 5 loader functions with version check (search for `abi_version`)
- `docs/plugin_abi.md` — Updated documentation with versioning scheme

## Optional Reading

- `knowledge/impl-plugin-abi-version-2026-05-27.md` — Implementation summary (12 files, deviations, test results)
- `knowledge/review-plugin-abi-version-2026-05-27.md` — Review findings, traceability, minor findings
- `engine/core/tests/abi_version.rs` — 12 integration tests covering all ACs
- `plugins/test_abi_mismatch/test_abi_mismatch.c` — Test plugin with version 999
- `plugins/test_abi_zero/test_abi_zero.c` — Test plugin with version 0

## Excluded

- `knowledge/report-refresh-audit-2026-05-27.md` — Background only; the gap it identified has been resolved
- `knowledge/analysis-deep-dive-refresh-2026-05-27.md` — Analysis that found the gap; not needed to understand the fix
- `knowledge/analysis-codebase-map-refresh-2026-05-27.md` — Codebase map showing pre-fix state; struct layout is now updated
- `plugins/simple_square_plugin/simple_square_plugin.c` — Individual C plugin updates are trivial (single line each)
- `plugins/rust_test_plugin/src/lib.rs` — Standalone Rust plugin with local constant copy; follows same pattern

## Key Architecture Points

- **Version at offset 0**: `abi_version` is the first field of `PluginVTable` — enables automatic rejection of pre-versioning plugins (old `init` pointer at offset 0 won't equal expected version)
- **Check before init**: Version check occurs before `vtable_ref.init()` — incompatible plugins never execute
- **5 loader functions**: All must be updated together when changing the ABI version
- **Dual constants**: `PLUGIN_ABI_VERSION` defined in C header (`#define`) and Rust types.rs (`pub const`). The Rust test plugin defines a third copy locally (standalone cdylib constraint)
- **Single integer version**: Not semver. Bump on any breaking ABI change. Currently `1`.

## Common Tasks

### Bumping the ABI Version
1. Update `#define PLUGIN_ABI_VERSION` in `engine/engine_plugin_abi.h`
2. Update `pub const PLUGIN_ABI_VERSION` in `engine/core/src/plugins/types.rs`
3. Update local const in `plugins/rust_test_plugin/src/lib.rs`
4. Rebuild all plugins (`cargo run -p xtask -- build-plugins && cargo run -p xtask -- build-c-plugins`)
5. Update test plugins (`test_abi_mismatch`, `test_abi_zero`) if they should test against the new version
6. Run `cargo test --all` to verify

### Adding a New Loader Function
1. Add the standard version check after null pointer check, before `init` call
2. Use the standard error format: `"Plugin '{}' ABI version mismatch: expected {}, got {}"`
3. Add a test to `abi_version.rs` covering the new function

### Adding a New Native Plugin
1. Include `engine/engine_plugin_abi.h` (C) or define local `PluginVTable` (Rust)
2. Set `abi_version = PLUGIN_ABI_VERSION` in the vtable initialization
3. Ensure the field is at offset 0

## Cross-References

- `knowledge/intent-plugin-abi-version-2026-05-27.md` — Origin: gap was identified in project audit
- `knowledge/report-refresh-audit-2026-05-27.md` — Audit report recommending this as a quick win
- `knowledge/analysis-deep-dive-refresh-2026-05-27.md` — Deep-dive analysis section on the unversioned ABI risk
- `engine/engine_plugin_abi.h` — Canonical C ABI header with version constant and struct
