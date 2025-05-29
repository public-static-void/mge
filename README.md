# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint for rapid prototyping, extensible games, and simulation.

---

## Capabilities

## Capabilities

- **Schema-driven ECS:** JSON-based, hot-reloadable components and systems.
- **Unified scripting API:** Identical ECS/world APIs in Lua and Python.
- **Hot-reloadable plugins:** Extend at runtime with Rust, Lua, Python, or C.
- **Flexible world generation:** Register/invoke worldgen plugins in any supported language.
- **Dependency-aware system scheduler:** Automatic ordering and cycle detection.
- **Runtime mode switching:** Switch between game modes on the fly.
- **Deterministic simulation:** Modular, event-driven tick loop.
- **Region/zone queries:** Query entities/cells by region or kind.

---

## Usage

### Lua Scripting

Execution of ECS-enabled Lua scripts is supported through the CLI:

```sh
cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
```

Example:

```sh
cargo run --bin mge-cli -- engine/scripts/lua/roguelike_mvp.lua
```

Controls: `w/a/s/d` = move, `e` = attack, `q` = quit

---

### Python Scripting

The ECS and scripting API are exposed as a native Python extension.

**Installation and usage:**

1. (Optional) Creation and activation of a Python virtual environment:

   ```sh
   python3 -m venv .venv
   source .venv/bin/activate
   ```

2. Installation of maturin:

   ```sh
   pip install maturin
   ```

3. Building and installation of the Python module:

   ```sh
   cd engine_py
   maturin develop
   cd ..
   ```

4. Importing and usage in Python:

   ```python
   from mge import PyWorld

   world = PyWorld()
   eid = world.spawn_entity()
   world.set_component(eid, "Health", {"current": 10, "max": 10})
   print(world.get_component(eid, "Health"))
   ```

The full scripting API is documented in [docs/api.md](docs/api.md), and additional code samples are available in [docs/examples.md](docs/examples.md).

---

### Rust ECS

MGE may be used as a pure Rust ECS with schema-driven components:

```rust
use std::sync::Arc;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::World;

let schema_dir = "engine/assets/schemas";
let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");

let mut registry = ComponentRegistry::new();
for (_name, schema) in schemas {
    registry.register_external_schema(schema);
}
let registry = Arc::new(std::sync::Mutex::new(registry));

let mut world = World::new(registry.clone());
let entity = world.spawn_entity();
world.set_component(entity, "Health", serde_json::json!({"current": 10, "max": 10})).unwrap();
```

---

## Component Model

Component definitions are provided as JSON schemas in `engine/assets/schemas/`.

- Components for scripting and systems can be added or modified without Rust code changes.
- Schemas specify properties, required fields, and allowed modes.

Example:

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

---

## Documentation

- [docs/dev.md](docs/dev.md): Quick reference how to set up this project for development and run tests and such
- [docs/api.md](docs/api.md): Unified scripting API (Lua & Python)
- [docs/examples.md](docs/examples.md): Usage examples (Lua, Python, Rust, C)
- [docs/idea.md](docs/idea.md): Architecture and design
- [docs/plugin_abi.md](docs/plugin_abi.md): C ABI plugin authoring
- [docs/worldgen.md](docs/worldgen.md): Worldgen plugin system
- [docs/ROADMAP.md](docs/ROADMAP.md): Project Roadmap
