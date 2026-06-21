# Project Roadmap

## Core Engine & ECS

- [x] Schema-driven ECS (component registry, schemas, entity lifecycle)
- [x] Event bus (publish, subscribe, poll)
- [x] Save/load persistence (full state round-trip serialization)
- [x] Simulation tick (deterministic turn-based loop)
- [x] Death/decay processing cycle
- [x] Mode switching (query and change between game modes)
- [x] Component macros for automated schema generation, versioning, and migration

## Scripting & Language Bridges

- [x] Lua scripting backend with complete ECS and world API
- [x] Python scripting backend with complete ECS and world API
- [x] WASM scripting backend with complete ECS and world API
- [x] Identical API surface across all three scripting backends (~139 functions each)
- [x] C ABI plugin system with versioned PluginVTable (init, shutdown, update, worldgen)
- [x] Modular world generation plugin system supporting multiple backends (Rust, Lua, Python, C ABI)
- [x] Python sandbox support (restricted execution environment)
- [x] Lua StdLib restricted to safe subset (no os, io, package, require)

## Game Systems

- [x] Grid map with pathfinding (square, hex, province topologies)
- [x] Inventory management (pickup, use, drop)
- [x] Equipment and gear system (wield, wear, inventory slots)
- [x] Body management and equipment synchronization
- [x] Combat and damage system
- [x] Job system (job board, query, mutation, AI assignment, events, dependency chains)
- [x] Economic engine (stockpile management, production recipes, resource reservations)
- [x] Movement system (entity positioning and translation)
- [x] Region, province, and territory map system
- [x] Map generation, validation, and postprocessing hooks
- [ ] Procedural dungeon generation
- [ ] Field-of-view and lighting simulation
- [ ] AI behaviors (enemy tactics, patrol routes)
- [ ] Item generation and loot tables
- [ ] Time-of-day and season cycle
- [ ] Building and construction system
- [ ] Temperature and environment simulation
- [ ] Vehicle support
- [ ] Crafting system (recipes, tools, materials)
- [ ] Diplomacy AI (relationships, treaties, war)
- [ ] Event-driven narrative engine (scenarios, decision events)
- [ ] Tech tree and research system
- [ ] Resource economy (production, trade, consumption)

## Presentation Layer

- [x] Camera viewport (scrollable camera with world-space mapping)
- [x] Terminal UI widget library (button, label, checkbox, dropdown, text input, context menu, panel, event log)
- [x] UI layout system (linear arrangement, z-ordering)
- [x] UI event handling and propagation
- [x] UI test suite (widget rendering, layout, events)

## Tooling & CI

- [x] Makefile orchestration (validate-schema, build, test, lint targets)
- [x] Custom xtask build orchestrator (plugin deploy, C plugin compilation, WASM test builds)
- [x] CI pipeline with caching (Swatinem/rust-cache, toolchain pinning, ~3-5 min on cache hit)
- [x] Toolchain pinned to nightly (edition 2024)
- [x] Schema validation tooling
- [x] Release automation (semantic-release, changelog generation)
- [x] Rust unit and integration test suite
- [x] Lua test suite (47 test files, source-parsing discovery, isolated world per test)
- [x] Python test suite (44 test files, maturin-based build)
- [ ] Python wheel packaging
- [ ] Code coverage reporting

## Documentation

- [ ] README.md (project overview, quick start, build/test instructions)
- [ ] docs/dev.md (developer documentation, toolchain setup)
- [ ] docs/ROADMAP.md (capability roadmap)
- [ ] docs/api.md (scripting API reference)
- [ ] docs/idea.md (architecture vision)
- [ ] docs/plugin_abi.md (C plugin ABI reference)
- [ ] docs/examples.md (demo walkthrough)
- [ ] docs/worldgen.md (worldgen pipeline)
- [ ] CONTRIBUTING.md
