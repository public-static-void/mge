# MGE Examples and Usage

This document contains practical examples demonstrating how to use the Modular Game Engine (MGE) features, including Lua scripting, ECS interactions, and CLI usage.

## Lua Scripting Examples

### 1. Spawning Entities and Setting Components

```lua
local id = spawn_entity()
set_component(id, "Position", { x = 10.0, y = 20.0 })
set_component(id, "Health", { current = 10, max = 10 })

local pos = get_component(id, "Position")
print("Entity " .. id .. " position: x=" .. pos.x .. " y=" .. pos.y)
```

### 2. Mode Switching and Component Access Enforcement

```lua
set_mode("colony")
-- Allowed to set Happiness component in colony mode
set_component(entity_id, "Happiness", { base_value = 0.75 })

-- Attempting to set a component not allowed in current mode results in error
local success, err = pcall(function()
    set_component(entity_id, "Inventory", { slots = {}, weight = 0.0 })
end)
if not success then
    print("Error: " .. err)
end
```

### 3. Entity Death and Decay Lifecycle

```lua
local id = spawn_entity()
set_component(id, "Health", { current = 2, max = 10 })

-- Simulate damage and death
set_component(id, "Health", { current = 0, max = 10 })

process_deaths()
print("Corpse component:", get_component(id, "Corpse"))
print("Decay component:", get_component(id, "Decay"))

for i = 1, 5 do
    process_decay()
    print("Decay after tick " .. i .. ":", get_component(id, "Decay"))
end
```

### 4. Turn System Example

```lua
local id = spawn_entity()
set_component(id, "Position", { x = 0, y = 0 })
set_component(id, "Health", { current = 10, max = 10 })

print_positions()
print_healths()
print("Turn: " .. get_turn())

tick()

print_positions()
print_healths()
print("Turn: " .. get_turn())
```

## Python Scripting Examples

```python
from mge import PyWorld

world = PyWorld()  # Optionally: PyWorld("engine/assets/schemas")
eid = world.spawn_entity()
world.set_component(eid, "Health", {"current": 10, "max": 10})
print(world.get_component(eid, "Health"))
world.set_mode("roguelike")
print("Available modes:", world.get_available_modes())
```

- See [`engine_py/tests/`](../engine_py/tests/) for more tested usage patterns and system examples.

## ECS Usage in Rust

### Creating a World with a Component Registry and Loading Schemas

```rust
use std::sync::Arc;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::World;

let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");

let mut registry = ComponentRegistry::new();
for (_name, schema) in schemas {
    registry.register_external_schema(schema);
}
let registry = Arc::new(registry);

let mut world = World::new(registry.clone());
world.current_mode = "colony".to_string();

// Spawn an entity and set components
let entity = world.spawn_entity();
world.set_component(entity, "Health", serde_json::json!({"current": 10, "max": 10})).unwrap();
```

## CLI Usage

### Run any ECS-enabled Lua script with:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
```

### Examples:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/roguelike_mvp.lua
cargo run --bin mge-cli -- engine/scripts/lua/turn_demo.lua
cargo run --bin mge-cli -- engine/scripts/lua/death_removal_demo.lua
```

## C Plugin Example

You can extend MGE at runtime with C ABI plugins. Here’s a minimal example:

```c
#include <stdint.h>
#include "engine_plugin_abi.h"

static int init(EngineApi* api, void* world) {
    uint32_t entity = api->spawn_entity(world);
    api->set_component(world, entity, "Position", "{"x":1,"y":2}");
    return 0;
}
static void shutdown(void) {}
static void update(float dt) {}

attribute((visibility("default")))
PluginVTable PLUGIN_VTABLE = {
    .init = init,
    .shutdown = shutdown,
    .update = update,
};
```

**Build:**

```bash
gcc -Iengine -shared -fPIC plugins/test_plugin.c -o plugins/libtest_plugin.so
```

- Place the resulting `.so` (or `.dll`/`.dylib`) in the project root `plugins/` directory.
- The engine and tests will discover and load it automatically.

---

## Additional Resources

- See [docs/idea.md](idea.md) for the full architecture and design blueprint.
- Lua scripts are located in `engine/scripts/lua/`.
- Component schemas are in `engine/assets/schemas/`.
- [docs/lua_api.md](lua_api.md): Lua API reference
