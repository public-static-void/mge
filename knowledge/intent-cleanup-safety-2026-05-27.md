---
title: "INTENT: Stale Cargo.lock cleanup + SAFETY comments for unsafe blocks"
version: 1.0.0
status: draft
type: intent
created: 2026-05-27
author: Overseer
superseded_by: null
---

## Overview

Two quick-win cleanup tasks from the project audit:
1. Delete the stale nested `engine/core/Cargo.lock` (root workspace lockfile is authoritative)
2. Add `// SAFETY:` comments to ~10 undocumented unsafe blocks in engine_core UI widget code

## Context

The project audit (knowledge/report-refresh-audit-2026-05-27.md) identified these as high-impact, low-effort improvements. The stale Cargo.lock causes confusion for direct builds from the engine/core directory. The undocumented unsafe blocks are a memory safety risk — without documented invariants, future refactoring could introduce UB.

## Content / Scope

### Task 1: Delete stale Cargo.lock
- Delete `engine/core/Cargo.lock`
- Verify the root workspace `Cargo.lock` is used instead
- Run `cargo build` or relevant commands to confirm no breakage

### Task 2: Safety comments for unsafe blocks
- Find all undocumented unsafe blocks in `engine/core/src/` and `engine_lua/` UI-related code (not the plugin loader — those are already documented)
- For each, analyze the invariants that make it sound
- Add `// SAFETY:` comments explaining the safety invariants
- Do NOT change any behavior, only add documentation comments

### Out of Scope
- Fixing or refactoring the unsafe code itself
- Touching the plugin loader unsafe blocks (those are already documented per the audit)

## Decisions
- Pure documentation change for unsafe blocks — zero behavior change
- Cargo.lock deletion is safe per project docs: "The root workspace Cargo.lock is authoritative. Do not use the nested one."

## References
- knowledge/report-refresh-audit-2026-05-27.md — Audit identifying these gaps
- engine/core/Cargo.lock — The stale file to delete
- engine/core/src/ — UI widget source with unsafe blocks
