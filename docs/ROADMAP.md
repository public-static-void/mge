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
- [ ] Unit and equipment designer
- [x] Combat and damage system
- [ ] Body part damage model (per-part health, wounds)
- [ ] Skill and attribute system
- [x] Job system (job board, query, mutation, AI assignment, events, dependency chains)
- [x] Economic engine (stockpile management, production recipes, resource reservations)
- [x] Movement system (entity positioning and translation)
- [x] Region, province, and territory map system
- [x] Map generation, validation, and postprocessing hooks
- [ ] Z-level / multi-layer map support
- [ ] Multi-scale map navigation
- [ ] Procedural dungeon generation
- [ ] Fluid simulation (water, magma)
- [ ] Field-of-view and lighting simulation
- [ ] Fog of war and visibility system
- [ ] AI behaviors (enemy tactics, patrol routes)
- [ ] Noise and detection mechanics
- [x] Item generation and loot tables
- [ ] Material and property system
- [x] Time-of-day and season cycle
- [ ] Weather and climate system
- [ ] Building and construction system
- [ ] Administration and zone management
- [ ] Temperature and environment simulation
- [ ] Ecosystem and wildlife simulation
- [ ] Vehicle support
- [ ] Crafting system (recipes, tools, materials)
- [ ] Manufacturing and production queue
- [ ] Diplomacy AI (relationships, treaties, war)
- [ ] Faction and reputation system
- [ ] Event-driven narrative engine (scenarios, decision events)
- [ ] Procedural history and lore generation
- [ ] Tech tree and research system
- [ ] Resource economy (production, trade, consumption)
- [ ] Supply and logistics network

## Presentation Layer

- [x] Camera viewport (scrollable camera with world-space mapping)
- [x] Terminal UI widget library (button, label, checkbox, dropdown, text input, context menu, panel, event log)
- [x] UI layout system (linear arrangement, z-ordering)
- [x] UI event handling and propagation

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
