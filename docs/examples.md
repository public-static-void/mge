# MGE Scripting Examples

This document contains practical code samples for Lua, Python, and Rust usage.

---

## Entity Management

### Lua

```lua
local id = spawn_entity()
despawn_entity(id)
local ids = get_entities()
```

### Python

```python
eid = world.spawn_entity()
world.despawn_entity(eid)
ids = world.get_entities()
```

---

## Component Access

### Lua

```lua
set_component(id, "Health", { current = 10, max = 10 })
local health = get_component(id, "Health")
```

### Python

```python
world.set_component(eid, "Health", {"current": 10, "max": 10})
health = world.get_component(eid, "Health")
```

---

## Entity Queries

### Lua

```lua
local ids = get_entities_with_component("Health")
```

### Python

```python
ids = world.get_entities_with_component("Health")
```

---

## Mode Switching

### Lua

```lua
set_mode("colony")
print(get_mode())
print(get_available_modes())
```

### Python

```python
world.set_mode("colony")
print(world.get_mode())
print(world.get_available_modes())
```

---

## Systems

### Lua

```lua
local ran = false
register_system("test_lua_system", function(dt)
    ran = true
end)
run_system("test_lua_system")
assert(ran == true, "System did not run!")
```

### Python

```python
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

```lua
send_event("my_event", {foo = 42})
local events = poll_event("my_event")
for i, evt in ipairs(events) do
    print(evt.foo)
end
update_event_buses()
```

### Python

```python
world.send_event("my_event", '{"foo": 42}')
events = world.poll_event("my_event")
for evt in events:
    print(evt["foo"])
world.update_event_buses()
```

---

## Region and Zone Queries

### Lua

```lua
local eid1 = spawn_entity()
set_component(eid1, "Region", { id = "room_1", kind = "room" })
local eid2 = spawn_entity()
set_component(eid2, "Region", { id = { "room_1", "biome_A" }, kind = "room" })
local eid3 = spawn_entity()
set_component(eid3, "Region", { id = "biome_A", kind = "biome" })

local e_room = get_entities_in_region("room_1")
print(#e_room) -- 2
local e_kind_room = get_entities_in_region_kind("room")
print(#e_kind_room) -- 2
```

### Python

```python
eid1 = world.spawn_entity()
world.set_component(eid1, "Region", {"id": "room_1", "kind": "room"})
eid2 = world.spawn_entity()
world.set_component(eid2, "Region", {"id": ["room_1", "biome_A"], "kind": "room"})
eid3 = world.spawn_entity()
world.set_component(eid3, "Region", {"id": "biome_A", "kind": "biome"})

e_room = world.get_entities_in_region("room_1")
print(len(e_room)) # 2
e_kind_room = world.get_entities_in_region_kind("room")
print(len(e_kind_room)) # 2
```

---

## Map Generation, Validation, and Postprocessor Hooks

### Lua

```lua
-- Register a validator: called before map is applied, receives the map table.
world:register_map_validator(function(map)
    -- Return false to block the map, true to accept.
    if not map.topology or #map.cells == 0 then
        return false
    end
    return true
end)

-- Register a postprocessor: called after map is applied, receives the world object.
world:register_map_postprocessor(function(w)
    print("Map postprocessor called, cell count:", w:get_map_cell_count())
end)

-- Apply a generated map (runs validators, then postprocessors)
world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })

-- Clear all
world:clear_map_validators()
world:clear_map_postprocessors()
```

### Python

```python
# Register a validator: called before map is applied, receives the map dict.
def validator(map_obj):
    # Return False to block the map, True to accept.
    return bool(map_obj.get("topology")) and len(map_obj.get("cells", [])) > 0

world.register_map_validator(validator)

# Register a postprocessor: called after map is applied, receives the world object.
def postprocessor(world_obj):
    print("Map postprocessor called, cell count:", world_obj.get_map_cell_count())

world.register_map_postprocessor(postprocessor)

# Apply a generated map (runs validators, then postprocessors)
world.apply_generated_map({ "topology": "square", "cells": [ { "x": 0, "y": 0, "z": 0 } ] })

# Clear all
world.clear_map_validators()
world.clear_map_postprocessors()
```

---

## Inventory Example

### Lua

```lua
local e = spawn_entity()
set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
add_item_to_inventory(e, "ring")
local inv = get_inventory(e)
print(inv.slots) -- "ring"
remove_item_from_inventory(e, 0)
```

### Python

```python
eid = world.spawn_entity()
world.set_inventory(eid, { "slots": [], "weight": 0.0, "volume": 0.0 })
world.add_item_to_inventory(eid, "ring")
inv = world.get_inventory(eid)
print(inv["slots"]) # "ring"
world.remove_item_from_inventory(eid, 0)
```

---

## Equipment Example

### Lua

```lua
set_component(e, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
add_item_to_inventory(e, "sword")
equip_item(e, "sword", "right_hand")
local eq = get_equipment(e)
print(eq.slots.right_hand) -- "sword"
unequip_item(e, "right_hand")
```

### Python

```python
world.set_component(eid, "Item", { "id": "sword", "name": "Sword", "slot": "right_hand" })
world.add_item_to_inventory(eid, "sword")
world.equip_item(eid, "sword", "right_hand")
eq = world.get_equipment(eid)
print(eq["slots"]["right_hand"]) # "sword"
world.unequip_item(eid, "right_hand")
```

---

## Body Example

### Lua

```lua
set_body(e, { parts = {} })
add_body_part(e, {
    name = "torso",
    status = "healthy",
    kind = "flesh",
    temperature = 37.0,
    ideal_temperature = 37.0,
    insulation = 1.0,
    heat_loss = 0.1,
    children = {},
    equipped = {},
})
local body = get_body(e)
print(body.parts.name) -- "torso"
remove_body_part(e, "torso")
```

### Python

```python
world.set_body(eid, { "parts": [] })
world.add_body_part(eid, {
    "name": "torso",
    "status": "healthy",
    "kind": "flesh",
    "temperature": 37.0,
    "ideal_temperature": 37.0,
    "insulation": 1.0,
    "heat_loss": 0.1,
    "children": [],
    "equipped": [],
})
body = world.get_body(eid)
print(body["parts"]["name"]) # "torso"
world.remove_body_part(eid, "torso")
```

---

## Economic System Example

### Lua

```lua
local eid = spawn_entity()
set_component(eid, "Stockpile", { resources = { wood = 5 } })
set_component(eid, "ProductionJob", {
    recipe = "wood_plank",
    progress = 0,
    status = "pending"
})

local resources = get_stockpile_resources(eid)
print(resources.wood) -- 5

local job = get_production_job(eid)
print(job.recipe) -- "wood_plank"
print(job.status) -- "pending"

remove_component(eid, "Stockpile")
assert.is_nil(get_stockpile_resources(eid))

remove_component(eid, "ProductionJob")
assert.is_nil(get_production_job(eid))
```

### Python

```python
eid = world.spawn_entity()
world.set_component(eid, "Stockpile", {"resources": {"wood": 5}})
world.set_component(eid, "ProductionJob", {
    "recipe": "wood_plank",
    "progress": 0,
    "status": "pending"
})

resources = world.get_stockpile_resources(eid)
print(resources["wood"]) # 5

job = world.get_production_job(eid)
print(job["recipe"]) # "wood_plank"
print(job["status"]) # "pending"

world.remove_component(eid, "Stockpile")
assert world.get_stockpile_resources(eid) is None

world.remove_component(eid, "ProductionJob")
assert world.get_production_job(eid) is None
```

---

## UI API

### Lua

```lua
-- Create a Button widget
local id = ui.create_widget("Button", { label = "OK", pos = {1, 2}, color = {255, 255, 255} })
assert(id > 0)

-- Set and get widget properties
ui.set_widget_props(id, { label = "Confirm", color = {0, 255, 0} })
local props = ui.get_widget_props(id)
print(props.label) -- "Confirm"

-- Add a child widget (e.g., add a Label to a Panel)
local panel_id = ui.create_widget("Panel", { pos = {0, 0}, size = {100, 50}, color = {200, 200, 200} })
local label_id = ui.create_widget("Label", { text = "Info", pos = {2, 2}, color = {0, 0, 0} })
ui.add_child(panel_id, label_id)

-- Get children
local children = ui.get_children(panel_id)
for i, child_id in ipairs(children) do
    print("Child ID:", child_id)
end

-- Remove a child
ui.remove_child(panel_id, label_id)

-- Register and trigger a callback
ui.set_callback(id, "click", function(widget_id)
    print("Button clicked! Widget ID:", widget_id)
end)
ui.trigger_event(id, "click", { x = 10, y = 5 })

-- Remove a callback
ui.remove_callback(id, "click")

-- Focus a widget
ui.focus_widget(id)

-- Set/get z-order
ui.set_z_order(id, 10)
print(ui.get_z_order(id))

-- Query widget type and parent
print(ui.get_widget_type(id))
print(ui.get_parent(label_id))

-- Dynamic widget registration (advanced)
ui.register_widget("CustomWidget", function(props)
    return ui.create_widget("Button", props)
end)
local custom_id = ui.create_widget("CustomWidget", { label = "Dynamic!" })
print(ui.get_widget_type(custom_id)) -- "CustomWidget"

-- Load UI from JSON
local ids = ui.load_json([[{"type":"Panel","props":{"pos":[0,0],"size":[100,50]}}]])
print(ids[1])
```

### Python

```python
import engine_py

ui = engine_py.UiApi()

# Create a Button widget
widget_id = ui.create_widget("Button", {"label": "OK", "pos": [1, 2], "color": [255, 255, 255]})
assert widget_id > 0

# Set and get widget properties
ui.set_widget_props(widget_id, {"label": "Confirm", "color": [0, 255, 0]})
props = ui.get_widget_props(widget_id)
print(props["label"])  # "Confirm"

# Add a child widget (e.g., add a Label to a Panel)
panel_id = ui.create_widget("Panel", {"pos": [0, 0], "size": [100, 50], "color": [200, 200, 200]})
label_id = ui.create_widget("Label", {"text": "Info", "pos": [2, 2], "color": [0, 0, 0]})
ui.add_child(panel_id, label_id)

# Get children
children = ui.get_children(panel_id)
for child_id in children:
    print("Child ID:", child_id)

# Remove a child
ui.remove_child(panel_id, label_id)

# Register and trigger a callback
def on_click(widget_id):
    print("Button clicked! Widget ID:", widget_id)

ui.set_callback(widget_id, "click", on_click)
ui.trigger_event(widget_id, "click", {"x": 10, "y": 5})

# Remove a callback
ui.remove_callback(widget_id, "click")

# Focus a widget
ui.focus_widget(widget_id)

# Set/get z-order
ui.set_z_order(widget_id, 10)
print(ui.get_z_order(widget_id))

# Query widget type and parent
print(ui.get_widget_type(widget_id))
print(ui.get_parent(label_id))

# Dynamic widget registration (advanced)
def custom_widget_ctor(props):
    return ui.create_widget("Button", props)

ui.register_widget("CustomWidget", custom_widget_ctor)
custom_id = ui.create_widget("CustomWidget", {"label": "Dynamic!"})
print(ui.get_widget_type(custom_id))  # "CustomWidget"

# Load UI from JSON
ids = ui.load_json('{"type":"Panel","props":{"pos":[0,0],"size":[100,50]}}')
print(ids[0])
```

---

## User Input

### Lua

```lua
local name = get_user_input("Enter your name: ")
print("Hello, " .. name)
```

### Python

```python
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
