# MGE v1.0.0 Milestone Checklist

## Core Engine & ECS

- [x] ECS framework (entity, component, system, event)
- [x] Component registry (schema-driven, hot-reloadable)
- [x] Entity manager (unique IDs, lifecycle)
- [x] System scheduler (dependency-aware, deterministic)
- [x] Serialization & deserialization (save/load, versioning)
- [x] Mode controller (mode switching, mode-specific logic)

## Scripting & Language Bridges

- [x] Lua bridge (full ECS/world API, modding support)
- [x] Python bridge (full ECS/world API, data pipeline)
- [x] C ABI bridge (plugin system, FFI)
- [ ] WASM runtime integration (for web/hosted scripting)

## Component Model & Schema

- [x] JSON/YAML schema-driven components
- [x] Mode-specific component availability & enforcement
- [x] Schema validation (in CI)
- [x] Component versioning & migration (data evolution)
- [x] Data-driven field constraints (min/max, enums, etc.)
- [x] Component macro system (Rust derive macros, migration, schema parsing)
- [ ] Code generation tools (for components, schemas, bindings)

## Map, World, and Simulation

- [x] Map/cell/region management (add/remove/query)
- [x] Entity queries by cell, region, type, and multi-cell
- [x] Pathfinding API (exposed to scripting)
- [x] Cell/region metadata (terrain, tags, properties)
- [x] Region/zone support (multi-region, region kinds)
- [x] Deterministic tick & event loop
- [x] Modular world generation system (plugin/scriptable, documented)

## Systems & Simulation Layer

- [x] Core systems: movement, health, inventory, jobs, events
- [x] Stockpile/resource/inventory management systems
- [x] Dynamic system registration (from scripting/plugins)
- [x] Event bus (inter-system comms)
- [x] World generator (data-driven, pluggable)
- [x] Economic engine
- [x] Temporal system (turns, ticks, time-of-day)

## Plugin/Extensibility

- [x] Plugin/mod loader (Rust and C ABI, hot-reloadable, manifest support)
- [ ] C ABI plugin loader (with example) (WIP)
- [ ] Rust dynamic plugin loader (WIP)
- [ ] Hot-reloadable plugins, systems, and components (WIP)

## Tooling & Testing

- [x] Rust unit/integration tests (ECS, systems, registry)
- [x] Scripting (Lua/Python) test suites
- [x] Schema validator (already in CI)
- [x] Example mods/plugins/scripts

## Documentation

- [x] API reference (Rust, Lua, Python, C ABI)
- [ ] Quickstart and usage guides (WIP)
- [ ] Component schema authoring guide
- [ ] Plugin authoring guide (WIP)
- [x] World generation documentation

## Packaging & Distribution

- [ ] Python wheel packaging (portable, pip-installable)
- [x] Release automation (semantic-release, changelog)
- [ ] Code coverage reporting (Rust, Lua, Python)

## Presentation Layer (Planned)

- [ ] Render adapter (graphics API abstraction)
- [ ] UI framework (for in-game and editor UI)
