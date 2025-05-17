# MGE Scripting Examples

This document contains practical code samples for Lua, Python, and Rust usage.

---

## Entity Management

### Lua

```
local id = spawn_entity()
despawn_entity(id)
local ids = get_entities()
```

### Python

```
eid = world.spawn_entity()
world.despawn_entity(eid)
ids = world.get_entities()
```

---

## Component Access

### Lua

```
set_component(id, "Health", { current = 10, max = 10 })
local health = get_component(id, "Health")
```

### Python

```
world.set_component(eid, "Health", {"current": 10, "max": 10})
health = world.get_component(eid, "Health")
```

---

## Entity Queries

### Lua

```
local ids = get_entities_with_component("Health")
```

### Python

```
ids = world.get_entities_with_component("Health")
```

---

## Mode Switching

### Lua

```
set_mode("colony")
print(get_mode())
print(get_available_modes())
```

### Python

```
world.set_mode("colony")
print(world.get_mode())
print(world.get_available_modes())
```

---

## Systems

### Lua

```
local ran = false
register_system("test_lua_system", function(dt)
    ran = true
end)
run_system("test_lua_system")
assert(ran == true, "System did not run!")
```

### Python

```
ran = {"flag": False}
def sys(dt):
    ran["flag"] = True
world.register_system("test_py_system", sys)
world.run_system("test_py_system")
assert ran["flag"] is True
```

---

## Event Bus

### Lua

```
send_event("my_event", {foo = 42})
local events = poll_event("my_event")
for i, evt in ipairs(events) do
    print(evt.foo)
end
update_event_buses()
```

### Python

```
world.send_event("my_event", '{"foo": 42}')
events = world.poll_event("my_event")
for evt in events:
    print(evt["foo"])
world.update_event_buses()
```

---

## User Input

### Lua

```
local name = get_user_input("Enter your name: ")
print("Hello, " .. name)
```

### Python

```
name = world.get_user_input("Enter your name: ")
print("Hello,", name)
```

---

## ECS Usage in Rust

```rust
use std::sync::Arc;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::World;

let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");

let mut registry = ComponentRegistry::new();
for (_name, schema) in schemas {
    registry.lock().unwrap().register_external_schema(schema);
}
let registry = Arc::new(registry);

let mut world = World::new(registry.clone());
world.current_mode = "colony".to_string();

// Spawn an entity and set components
let entity = world.spawn_entity();
world.set_component(entity, "Health", serde_json::json!({"current": 10, "max": 10})).unwrap();
```

---

## C ABI Plugin Example

```c
#include "engine_plugin_abi.h"
#include <stdint.h>
#include <stdio.h>

void hello_system(void *world, float delta_time) {
    printf("[PLUGIN] Hello from system!\n");
}

static SystemPlugin system_plugins[] = {
    { "hello_system", hello_system }
};

int register_systems(struct EngineApi *api, void *world, SystemPlugin **systems, int *count) {
    *systems = system_plugins;
    *count = 1;
    return 0;
}
```

---

- See [docs/idea.md](idea.md) for architecture.
- See [docs/api.md](api.md) for full API reference.
