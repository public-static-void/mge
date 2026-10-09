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
- [x] Identical API surface across all three scripting backends
- [x] C ABI plugin system with versioned PluginVTable
- [x] Modular world generation plugin system supporting multiple backends (Rust, Lua, Python, C ABI)
- [x] Python sandbox support
- [x] Lua StdLib restricted to safe subset (no os, io, package, require)

## Game Systems

- [x] Grid map with pathfinding (square, hex, province topologies)
- [x] Inventory management (pickup, use, drop)
- [x] Equipment and gear system (wield, wear, inventory slots)
- [x] Body management and equipment synchronization
- [x] Unit and equipment designer
- [x] Combat and damage system
- [x] Body part damage model (per-part health, wounds)
- [x] Skill and attribute system
- [x] Job system (job board, query, mutation, AI assignment, events, dependency chains)
- [x] Economic engine (stockpile management, production recipes, resource reservations)
- [x] Movement system (entity positioning and translation)
- [x] Region, province, and territory map system
- [x] Map generation, validation, and postprocessing hooks
- [x] Z-level / multi-layer map support
- [x] Multi-scale map navigation
- [x] Procedural dungeon generation
- [x] Swappable mapgen algorithm registry (shared invoke core, MapgenAlgorithm trait, config-driven selection)
- [x] Fluid simulation (water, magma)
- [x] Field-of-view and lighting simulation
- [x] Fog of war and visibility system
- [x] AI behaviors (enemy tactics, patrol routes)
- [x] Noise and detection mechanics
- [x] WASM noise transport parity (emit → tick → get round-trip + hearing at Lua/Python parity)
- [x] Item generation and loot tables
- [x] Material and property system
- [x] Time-of-day and season cycle
- [x] Weather and climate system
- [x] Building and construction system
- [x] Administration and zone management
- [x] Temperature and environment simulation
- [x] Temperature v2 (per-cell diffusion, humidity/pressure modifiers, insulation aggregation)
- [x] Ecosystem and wildlife simulation
- [x] Vehicle support
- [x] Crafting system (recipes, tools, materials)
- [x] Manufacturing and production queue
- [x] Diplomacy AI (relationships, treaties, war)
- [x] Faction and reputation system
- [x] Event-driven narrative engine (scenarios, decision events)
- [x] Procedural history and lore generation
- [x] Tech tree and research system
- [x] Resource economy (production, trade, consumption)
- [x] Supply and logistics network

## Presentation Layer

- [x] Camera viewport (scrollable camera with world-space mapping)
- [x] Z-stacked camera viewport (camera z, per-z FOV, renderer z-filtering)
- [x] Terminal UI widget library (button, label, checkbox, dropdown, text input, context menu, panel, event log)
- [x] UI layout system (linear arrangement, z-ordering)
- [x] UI event handling and propagation
- [x] ANSI 24-bit terminal color output (colored glyph rendering via escape sequences)

## Tooling & Documentation

- [x] Makefile orchestration
- [x] Custom xtask build orchestrator
- [x] Schema validation tooling
- [x] Release automation
- [x] Rust unit + integration test suite
- [x] Lua test suite
- [x] Python test suite
- [x] Python wheel packaging
- [x] API reference documentation
- [x] Plugin authoring guide
- [x] World generation documentation
- [x] Development guide
- [x] README

## Engine Modularity (DRY/SOLID building blocks)

- [x] Balanced-brace Lua test-runner discovery (no silently skipped test files)
- [x] Job event log test-reset hook plus global test hooks (order-independent suites)
- [x] C-plugin artifact guard shared by all native test legs
- [x] Per-domain named defaults with no central god-object
- [x] CLI default-system registration helper
- [x] Shared noise propagation kernel across the core/WASM transport boundary
- [x] Topology construction/dispatch registry (one-registration topologies)
- [x] Dependency-aware system execution-order tie-break
- [x] Asset-path resolver honoring environment overrides
- [x] Bridge registry parity (WASM/Python module inventories pinned vs shared domains)
- [x] Task decoupling (required_progress-routed handlers, component-read enemy params)

## Planetary World Generation

- [ ] Planet-scale biome and climate assignment (temperature/humidity/pressure-driven biomes on a planet grid)
- [ ] Planet-to-region-to-hex hierarchical link generation (auto-link child maps through the link-maps surface)
- [ ] Native planet-gen worldgen plugin backend (Rust/C plugin emitting planet maps through the worldgen hook)
- [ ] Cross-bridge planet-gen scenario coverage (Lua/Python/WASM dungeon-gen-parity tests for planet output)

## Strategy-Layer Presentation

- [ ] Strategic overview renderer (province-colored terminal overview of the named strategic map, wired into the viewport)
- [ ] Minimap widget in the terminal UI library (viewport-linked, fog-aware)
- [ ] Strategic-to-tactical zoom switching in the camera viewport (per-scale rendering through the existing camera surface)
- [ ] Faction-relation province tinting (diplomacy-driven coloring of the strategic overview)

## Native Plugin Hot-Reload

- [ ] Host-side shared-object reload flow (unload/reload, state migration through the ABI hot-reload pointer, system re-registration)
- [ ] CLI reload trigger surfacing (reload a native plugin without restarting the simulation)
- [ ] Save-safe reload round-trip tests (state preserved across a reload cycle in core plus Lua suites, reusing the save/load harness)
