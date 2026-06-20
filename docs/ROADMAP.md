# Project Roadmap

## Core Engine & ECS
- [x] ECS framework (entity, component, system, event)
- [x] Component registry (schema-driven, hot-reloadable)
- [x] Entity manager with lifecycle
- [x] System scheduler (dependency-aware, deterministic)
- [x] Serialization & deserialization (save/load, versioning)
- [x] Mode controller (mode switching, mode-specific logic)
- [x] Component macro system (derive macros, migration, schema parsing)
- [x] Deterministic tick loop and event bus
- [x] mlua decoupled from engine_core

## Scripting & Language Bridges
- [x] Lua bridge — full ECS/world API
- [x] Python bridge — full ECS/world API
- [x] WASM bridge — 100% parity with Lua
- [x] All 3 backends at identical API surface (full scripting parity)
- [x] C ABI plugin system with versioned vtable
- [x] Python sandboxing
- [x] Lua StdLib restricted (safe subset, no os/io/package/require)

## Game Systems
- [x] Grid map with pathfinding
- [x] Cell/region management and metadata
- [x] Inventory, equipment, body management
- [x] Job system (board, query, AI assignment, events, dependencies)
- [x] Economic engine (stockpile, production, resource reservations)
- [x] Combat and death/decay systems
- [x] Terminal UI framework (7+ widget types, events, z-order)
- [x] UI test suite (widget rendering, layout, events)
- [x] Movement system
- [x] Camera and viewport

## World Generation
- [x] Modular world generation plugin system
- [x] Multi-language support (Rust, Lua, Python, C ABI)
- [x] Map generation, validation, and postprocessing hooks

## Tooling & CI
- [x] Custom xtask build orchestrator
- [x] CI pipeline with caching (~3-5 min on cache hit)
- [x] Toolchain pinned to nightly (edition 2024)
- [x] Schema validation in CI
- [x] Release automation (semantic-release, changelog)
- [x] Rust unit/integration tests (ECS, systems, registry)
- [x] Lua test suite
- [x] Python test suite

## Documentation
- [x] API reference (Rust, Lua, Python, C ABI)
- [x] Plugin authoring guide
- [x] World generation documentation
- [ ] CONTRIBUTING.md
