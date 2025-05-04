# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation.
It is designed for simulation, games, and rapid prototyping with robust ECS, plugin, and scripting support.

---

## Project Overview

MGE provides:

- A Rust-based core engine with a macro-driven ECS framework.
- Hot-reloadable plugin support and cross-language scripting (Lua, Python, WASM).
- Out-of-the-box Lua scripting bridge for entity/component manipulation and rapid prototyping.
- Mode-specific logic and data (e.g., Colony, Roguelike).
- Schema-driven, versioned component management.
- An architecture designed for tooling, modding, and rapid iteration.

For the full architecture and design blueprint, see [`docs/idea.md`](docs/idea.md).

---

## Repository Structure

```
engine/
  core/          # ECS core, registry, schema, migration (Rust)
  docs/          # Engine-specific documentation and specs
  assets/        # Game assets and data
  scripts/       # Scripts for gameplay, modding, and tests
  tools/         # Engine tools and utilities
engine_macros/   # Procedural macros for component ergonomics
docs/            # Project-wide docs and blueprints
```

---

## Scripting (Lua)

MGE supports Lua scripting for rapid prototyping, modding, and gameplay logic.

- Scripts can spawn entities, set/get components (e.g., Position, Health), and interact with the ECS.
- Lua scripts are loaded from `engine/scripts/lua/` and can be tested and run as part of the engine.

**Example Lua script (`engine/scripts/lua/demo.lua`):**

```
local id = spawn_entity()
set_component(id, "Position", { x = 1.1, y = 2.2 })
local pos = get_component(id, "Position")
print("Entity " .. id .. " position: x=" .. pos.x .. " y=" .. pos.y)
```

**For engine developers:**
Lua scripts are run as part of the Rust integration tests to ensure scripting API stability and correctness:

```
cargo test -p engine_core
```

**For users/modders:**
Direct execution of Lua scripts (outside of tests) is planned for a future CLI tool or in-game scripting console.

**Adding new Lua-exposed ECS features:**

- Just define a new Rust component and register it with the ECS.
- No manual scripting bridge changes are needed.
- All components are accessible from Lua via `set_component(entity, "ComponentName", { ... })` and `get_component(entity, "ComponentName")`.
- Add Lua and Rust tests as needed.

> **Note**:
> The Lua scripting bridge is now fully generic. Any registered ECS component can be set or queried from Lua using set_component and get_component. No Rust-side scripting boilerplate is required for new components.

See [`engine/core/src/scripting/mod.rs`](engine/core/src/scripting/mod.rs) for details and documentation.

---

## Getting Started

1. **Explore the ECS Core:**

   - See [`engine/core`](engine/core) and [`engine_macros`](engine_macros).
   - Define components with `#[component(...)]` and auto-generate schemas.

2. **Run tests:**

   ```sh
   cargo test
   ```

3. **Read [`docs/idea.md`](docs/idea.md) for the full blueprint.**

---

## Related Crates

- [`engine/core`](engine/core): ECS core, registry, migration, and schema logic.
- [`engine_macros`](engine_macros): Procedural macros for ergonomic component definition.

---

## Continuous Integration & Releases

- **CI**: All pull requests and pushes to main run automated checks (formatting, linting, tests) via GitHub Actions.

- **Release**: Merges to main automatically trigger a semantic-release pipeline:

  - Versioning and changelog are managed by semantic-release.

  - Rust crate version is updated using `@timada/semantic-release-cargo`.

  - Releases are published to GitHub Releases

## Branch Protection:

The main branch is protected: all PRs require passing status checks before merging.
