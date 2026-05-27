---
title: "INTENT: Refreshed Project Audit — State, Gaps, Issues, Improvements"
version: 1.0.0
status: draft
type: intent
created: 2026-05-27
author: Overseer
superseded_by: null
---

## Overview

Refresh and verify the MGE project audit. Re-examine the codebase for current state, functional/test/documentation gaps, latent issues, and improvement opportunities. Supersedes the initial audit in `knowledge/report-project-audit-2026-05-27.md`.

## Context

MGE is a Rust workspace monorepo (10 crates) — a cross-language game engine supporting Rust, Lua, Python, C, and WASM. Uses ECS architecture with JSON schema-driven components. Build system: Cargo + Makefile + xtask.

Prior audit (today) identified critical issues including: zero unit tests in engine_core, mlua dependency contradiction in engine_core, ~1465 unwrap() calls, no sandboxing, vestigial feature flags, incomplete WASM backend, stale Cargo.lock, and missing documentation. This refresh verifies current state and re-assesses.

## Content / Scope

### Investigation Areas (refreshed)

1. **Architecture & Structure** — Workspace layout, crate boundaries, dependency graph, entrypoints. Verify prior findings.
2. **Health & Build** — Does it build cleanly? CI health, test status, warnings. Check if stale Cargo.lock still present.
3. **Code Quality** — Dead code, unwrap/expect counts, unsafe blocks, clippy hygiene, error handling patterns.
4. **Test Coverage** — Unit vs integration test split, Lua/Python test health, coverage gaps.
5. **Documentation** — README, inline docs, API docs, ADRs, contribution guide completeness.
6. **Dependencies** — Outdated deps, security vulnerabilities, unnecessary deps, vestigial features.
7. **Configuration & DX** — Nightly Rust setup, system deps, devcontainer, CI/CD maturity.
8. **Security** — Unsafe code audit, input validation, plugin sandboxing, script runtime sandboxing.
9. **Performance** — Potential bottlenecks, cross-language serialization overhead, memory management.
10. **Cross-Language** — Lua/Python/WASM API parity, C ABI stability, Plugin protocol completeness.

### Deliverable

A refreshed REPORT KD (`knowledge/report-refresh-audit-2026-05-27.md`) summarizing:
- Current project state vs prior audit
- Top gaps (still open, new, or resolved)
- Top improvement opportunities refreshed
- Issues requiring immediate attention
- Recommended next steps

## Decisions

- This audit refreshes and supersedes the initial audit from 2026-05-27
- All prior findings should be re-validated

## References

- `knowledge/intent-project-audit-2026-05-27.md` — Prior INTENT (superseded)
- `knowledge/report-project-audit-2026-05-27.md` — Prior REPORT (to be superseded)
- `AGENTS.md` — Project-level agent instructions
- `Makefile` — Build orchestration
- `Cargo.toml` — Workspace manifest
