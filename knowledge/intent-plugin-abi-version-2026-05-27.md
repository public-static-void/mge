---
title: "INTENT: Add ABI Versioning to Plugin System"
version: 1.0.0
status: approved
type: intent
created: 2026-05-27
author: Overseer
superseded_by: null
---

## Overview

Add a version field to the Plugin VTable ABI to prevent silent undefined behavior when loading incompatible plugins. Currently, mismatched `.so` files (compiled against a different engine version) are loaded without any compatibility check, causing unpredictable crashes.

## Context

The MGE plugin system uses a C ABI defined in `engine/engine_plugin_abi.h`. Plugins export a `PluginVTable` struct with function pointers (init, shutdown, update, worldgen, system registration, hot-reload). There is no version field — any `.so` file that matches the symbol names will be loaded, even if the ABI has changed.

There are 5 plugins:
- `plugins/rust_test_plugin/` (Rust cdylib)
- `plugins/simple_square_plugin/` (C)
- `plugins/simple_hex_plugin/` (C)
- `plugins/simple_province_plugin/` (C)
- `plugins/test_plugin/` (C)

All need to be updated to export a version constant, and the plugin loader in engine_core needs to validate it on load.

## Content / Scope

### Requirements

1. **Add ABI version constant** — Define a `PLUGIN_ABI_VERSION` constant (integer, e.g., 1) that plugins must match.
2. **Add version field to PluginVTable** — Include the version in the exported VTable struct.
3. **Update C header** — `engine/engine_plugin_abi.h`: add version field, document the versioning scheme.
4. **Update plugin loader** — `engine_core` plugin loading code: read the version and reject with a clear error message on mismatch.
5. **Update all 5 plugins** — All plugins export the version and set the field.
6. **Add graceful error handling** — Mismatch produces a clear error message (not a panic/unwrap).
7. **Test** — Verify that version mismatch is caught and reported.

### Out of Scope

- Bumping the ABI version (stays at 1 for now)
- Semantic versioning (single integer is sufficient for now)
- Backward compatibility layer

## Decisions

- Single integer version (not semver) — keeps it simple
- Version checked at load time, not at each function call
- Error is returned (not panicked) — consistent with engine error handling patterns

## References

- `knowledge/report-refresh-audit-2026-05-27.md` — Audit identifying this gap
- `engine/engine_plugin_abi.h` — Plugin ABI header
- `engine_core` plugin loader source files
- `plugins/*` — Plugin implementations
