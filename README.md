# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation for simulation, games, and rapid prototyping.

## Features

- Macro-driven ECS in Rust
- Hot-reloadable plugins & cross-language scripting (Lua, Python, WASM)
- Schema-driven, versioned, and mode-restricted components
- Runtime mode switching with enforcement in both Rust and scripting
- Rapid prototyping via Lua scripting bridge
- Extensible architecture for tooling and modding

See [`docs/idea.md`](docs/idea.md) for the full architecture.

---

## Repository Structure

```text
engine/
  core/        # ECS core, registry, schema, migration (Rust)
  assets/      # Game assets and data
  scripts/     # Scripts for gameplay, modding, and tests
  tools/       # Engine tools and utilities
engine_macros/ # Procedural macros for component ergonomics
docs/          # Project-wide docs and blueprints
```

---

## Getting Started

1. **Create a World:**

```rust
let registry = Arc::new(ComponentRegistry::new());
let mut world = World::new(registry.clone());
```

The registry contains all component schemas (macro-defined and external JSON/YAML).
New schemas can be registered at runtime for dynamic/extensible components.

2. **Run tests:**

```bash
cargo test
```

3. **See [`docs/idea.md`](docs/idea.md) and [`docs/examples.md`](docs/examples.md) for more.**

---

## Schema-Driven Mode Enforcement

- Component schemas (Rust or external JSON/YAML) specify which game modes they support.
- The registry loads all schemas at startup.
- When setting or getting a component, the engine checks the current mode against the schema.
- Errors are raised if a component is not allowed in the current mode.
- Add new components at runtime by dropping schema files in `engine/assets/schemas/`.

**Example schema:**

```json
{
  "title": "Health",
  "type": "object",
  "properties": {
    "current": { "type": "number", "minimum": 0, "maximum": 100 },
    "max": { "type": "number", "minimum": 1, "maximum": 100 }
  },
  "required": ["current", "max"],
  "modes": ["colony", "roguelike"]
}
```

> **Note:** All new code and tests should use `World::new(registry)` and pass an `Arc<ComponentRegistry>`.

---

## Lua Scripting

- Spawn entities, set/get components, and interact with the ECS from Lua.
- Game systems like movement, health, turns, death, and decay are scriptable.
- Switch game modes at runtime; only access components valid for the current mode.
- See [`docs/examples.md`](docs/examples.md) for full Lua scripts and demos.

---

## CLI Usage

Run any ECS-enabled Lua script with:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
```
