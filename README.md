# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation for simulation, games, and rapid prototyping.

## Features

- Macro-driven ECS in Rust
- Hot-reloadable plugins & cross-language scripting (Lua, Python, WASM)
- Schema-driven, versioned, and mode-restricted components
- Runtime mode switching with enforcement in both Rust and scripting
- Lua scripting bridge for rapid prototyping
- Extensible architecture for tooling and modding

See [docs/idea.md](docs/idea.md) for a deeper dive.

---

## Quickstart

**Try the interactive roguelike demo:**

```bash
cargo run --bin mge-cli -- engine/scripts/lua/roguelike_mvp.lua
```

**Controls:**
`w/a/s/d` = move, `e` = attack, `q` = quit

The demo script (`engine/scripts/lua/roguelike_mvp.lua`) showcases MGE’s scripting, ECS, and runtime mode enforcement.

---

## Usage

- **Run any Lua script:**

```bash
cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
```

- **Create a World in Rust:**

```rust
use std::sync::Arc;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::scripting::World;

// Load schemas (see below)
let registry = Arc::new(ComponentRegistry::new());
let mut world = World::new(registry.clone());
```

- **Add or modify components:**
  Place JSON schemas in `engine/assets/schemas/`.
  All tools, CLI, and tests will pick them up at runtime.

---

## Schema-Driven Components

MGE components are defined by JSON schemas in `engine/assets/schemas/`.

- **No Rust code changes are required** to add, remove, or modify a component for scripting or data-driven systems.
- Schemas specify component properties, required fields, and which game modes the component is available in.
- Rust struct components are only needed for native, type-safe systems.

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

- **Mode enforcement:**
  Only components allowed for the current mode (as defined in their schema) can be accessed or set in Rust or Lua.

- **Dynamic components:**
  You can define components for use in scripting or modding only, without any Rust struct or code.

---

## Schema Validation

All JSON component schemas in `engine/assets/schemas/` are validated in two ways:

- **Static validation:**
  The built-in schema linter checks for required fields, allowed modes, and property constraints.
  Run the linter with:

  ```bash
  cargo run -p schema_validator -- engine/assets/schemas/
  ```

  This runs automatically in CI on every pull request.

- **Runtime validation:**
  Whenever you set component data from Rust, Lua, Python, or plugins, the data is validated against its schema at runtime.
  - Data that does not match the schema (missing required fields, wrong types, out-of-range values) is **rejected with a clear error**.
  - This ensures data consistency and prevents silent bugs in scripting and modding.

**Note:**
When using Lua or Python scripting, empty arrays (such as `slots = {}` for an empty inventory) are fully supported and validated as arrays.

See [`engine/tools/schema_validator/README.md`](engine/tools/schema_validator/README.md) for full details.

## Example: Adding a New Component

1. Create a schema file in `engine/assets/schemas/`, e.g. `mana.json`:
   ```json
   {
     "title": "Mana",
     "type": "object",
     "properties": { "value": { "type": "number", "minimum": 0 } },
     "required": ["value"],
     "modes": ["roguelike"]
   }
   ```
2. Use it immediately in Lua or Rust:
   ```lua
   set_component(id, "Mana", { value = 42 })
   ```
   ```rust
   world.set_component(entity, "Mana", serde_json::json!({ "value": 42 })).unwrap();
   ```

---

## Lua Scripting

- Spawn entities, set/get components, and interact with the ECS from Lua.
- Game systems like movement, health, turns, death, and decay are scriptable.
- Switch game modes at runtime; only access components valid for the current mode.
- See [docs/examples.md](docs/examples.md) for more.

---

## Python Scripting

- Script game logic, systems, and content in Python using the same ECS, schema, and mode system as Rust and Lua.
- The `mge` Python module exposes a `PyWorld` class for entity, component, and mode management.
- Supports rapid prototyping, content pipeline scripting, and simulation logic in Python.
- All component schemas and mode enforcement work identically to Lua and Rust.

**See [docs/examples.md](docs/examples.md#python-scripting-examples) for usage examples.**
**See [`docs/python_api.md`](docs/python_api.md) for the full API reference.**

**To use Python scripting:**

```bash
maturin develop  # or: pip install -e ./engine_py
```

Then import and use the `mge` module in your Python scripts.

---

**Note:**
Python scripting is ideal for data analysis, content tools, and simulation scripts. For gameplay logic and modding, Lua may be more ergonomic, but both are fully supported and interchangeable.

## C ABI Plugin System

MGE supports hot-reloadable plugins via a stable C ABI, enabling dynamic extension of the engine in C, Rust (with `extern "C"`), or other languages.

- Plugins export a vtable (`PLUGIN_VTABLE`) with `init`, `update`, and `shutdown` functions as defined in [`engine/engine_plugin_abi.h`](engine/engine_plugin_abi.h).
- Place compiled plugins (e.g. `.so`, `.dll`) in the project root’s `plugins/` directory.
- The engine and integration tests will automatically load plugins from this directory at runtime.

See [docs/examples.md](docs/examples.md#c-plugin-example) for a minimal C plugin and build instructions.

## Resources

- [docs/idea.md](docs/idea.md): Architecture/design
- [docs/examples.md](docs/examples.md): Usage examples
- [docs/lua_api.md](docs/lua_api.md): Lua API reference
- [docs/python_api.md](docs/python_api.md) Python API reference
