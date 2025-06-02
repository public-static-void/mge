# MGE Scripting API Reference

> **Note:**
> All functions below are available in both **Lua** (as global functions) and **Python** (as methods on the `PyWorld` class).
> See [docs/examples.md](examples.md) for usage in both languages.

---

## Entity Management

| Function             | Description                             |
| -------------------- | --------------------------------------- |
| `spawn_entity()`     | Spawn a new entity                      |
| `despawn_entity(id)` | Remove an entity and all its components |
| `get_entities()`     | List all entity IDs                     |

---

## Component Access

| Function                                     | Description                                    |
| -------------------------------------------- | ---------------------------------------------- |
| `set_component(id, name, data)`              | Set a component on an entity                   |
| `get_component(id, name)`                    | Get a component from an entity                 |
| `remove_component(id, name)`                 | Remove a component from an entity              |
| `list_components()`                          | List all registered component names            |
| `get_component_schema(name)`                 | Get the JSON schema for a component            |
| `modify_stockpile_resource(id, kind, delta)` | Add/remove a resource in a Stockpile component |

---

## Entity Queries

| Function                              | Description                                |
| ------------------------------------- | ------------------------------------------ |
| `get_entities_with_component(name)`   | List entity ids with a given component     |
| `get_entities_with_components(names)` | List entity ids with all listed components |
| `is_entity_alive(id)`                 | Returns true if entity's Health > 0        |
| `count_entities_with_type(type_str)`  | Count entities with Type.kind == type_str  |

---

## Game Modes

| Function                | Description               |
| ----------------------- | ------------------------- |
| `set_mode(name)`        | Switch/set game mode      |
| `get_mode()`            | Get the current game mode |
| `get_available_modes()` | List all available modes  |

---

## Systems

| Function                    | Description                                |
| --------------------------- | ------------------------------------------ |
| `register_system(name, fn)` | Register a function as a named system      |
| `run_system(name)`          | Run a previously registered system by name |
| `run_native_system(name)`   | Run a built-in system by name              |
| `poll_ecs_event(type)`      | Poll ECS events of a type                  |

---

## Job System

| Function                                        | Description                                                          |
| ----------------------------------------------- | -------------------------------------------------------------------- |
| `assign_job(entity, job_type, [fields/kwargs])` | Assign a job to an entity. Extra fields/kwargs become job properties |
| `register_job_type(name, fn)`                   | Register a custom job type                                           |

## Event Bus

| Function                 | Description                                  |
| ------------------------ | -------------------------------------------- |
| `send_event(type, data)` | Send an event of a specific type             |
| `poll_event(type)`       | Poll all events of a type since last update  |
| `update_event_buses()`   | Advance all event buses (call once per tick) |

---

## Region and Zone Queries

| Function                            | Description                                           |
| ----------------------------------- | ----------------------------------------------------- |
| `get_entities_in_region(region_id)` | List entity IDs assigned to a given region/zone       |
| `get_entities_in_region_kind(kind)` | List entity IDs assigned to regions of the given kind |
| `get_cells_in_region(region_id)`    | List cells assigned to a given region/zone            |
| `get_cells_in_region_kind(kind)`    | List cells assigned to regions of the given kind      |

---

## Worldgen Plugins

| Function                               | Description                              |
| -------------------------------------- | ---------------------------------------- |
| `register_worldgen_plugin(name, fn)`   | Register a worldgen plugin               |
| `list_worldgen_plugins()`              | List all registered worldgen plugins     |
| `invoke_worldgen_plugin(name, params)` | Invoke a worldgen plugin with parameters |

---

## Persistence

| Function               | Description                            |
| ---------------------- | -------------------------------------- |
| `save_to_file(path)`   | Save the current world state to a file |
| `load_from_file(path)` | Load world state from a file           |

---

## Inventory, Equipment, and Body Helpers

| Function                                    | Description                                 |
| ------------------------------------------- | ------------------------------------------- |
| `get_inventory(entity)`                     | Get the inventory component as a table/dict |
| `set_inventory(entity, data)`               | Set the inventory component                 |
| `add_item_to_inventory(entity, item_id)`    | Add an item ID to the inventory             |
| `remove_item_from_inventory(entity, index)` | Remove item at index from the inventory     |
| `get_equipment(entity)`                     | Get the equipment component as a table/dict |
| `equip_item(entity, item_id, slot)`         | Equip an item to a slot                     |
| `unequip_item(entity, slot)`                | Unequip the item from the given slot        |
| `get_body(entity)`                          | Get the body component as a table/dict      |
| `set_body(entity, data)`                    | Set the body component                      |
| `add_body_part(entity, part)`               | Add a part to the body (recursive)          |
| `remove_body_part(entity, name)`            | Remove a part by name (recursive)           |
| `get_body_part(entity, name)`               | Get a part by name (recursive)              |

---

## Economic System Helpers

| Function                          | Description                                                |
| --------------------------------- | ---------------------------------------------------------- |
| `get_stockpile_resources(entity)` | Get the resources subtable/dict from a Stockpile component |
| `get_production_job(entity)`      | Get the full ProductionJob component as a table/dict       |

---

## Camera & Viewport

| Function           | Description                               |
| ------------------ | ----------------------------------------- |
| `set_camera(x, y)` | Set the camera position (center viewport) |
| `get_camera()`     | Get the camera position as {x, y}         |

---

## UI API

| Function                               | Description                                                      |
| -------------------------------------- | ---------------------------------------------------------------- |
| `ui.create_widget(type, props)`        | Create a new UI widget of given type and properties              |
| `ui.load_json(json_str)`               | Load a UI widget tree from JSON                                  |
| `ui.remove_widget(id)`                 | Remove a widget by ID                                            |
| `ui.set_widget_props(id, props)`       | Set properties on a widget                                       |
| `ui.get_widget_props(id)`              | Get properties of a widget                                       |
| `ui.add_child(parent_id, child_id)`    | Add a widget as a child to another                               |
| `ui.get_children(parent_id)`           | Get the children IDs of a widget                                 |
| `ui.remove_child(parent_id, child_id)` | Remove a child widget from a parent                              |
| `ui.set_callback(id, event, fn)`       | Register a callback for a widget event (e.g., "click")           |
| `ui.remove_callback(id, event)`        | Remove a callback for a widget event                             |
| `ui.focus_widget(id)`                  | Set focus to a widget                                            |
| `ui.trigger_event(id, event, args)`    | Trigger an event on a widget                                     |
| `ui.send_ui_event(...)`                | Alias for `ui.trigger_event`                                     |
| `ui.get_widget_type(id)`               | Get the type name of a widget (including dynamically registered) |
| `ui.get_parent(id)`                    | Get the parent ID of a widget                                    |
| `ui.set_z_order(id, z)`                | Set the z-order of a widget                                      |
| `ui.get_z_order(id)`                   | Get the z-order of a widget                                      |
| `ui.register_widget(type, ctor)`       | Register a new widget type dynamically from scripting            |

### Supported Widget Types

| Widget Type     | Properties (main)                                                   |
| --------------- | ------------------------------------------------------------------- |
| `"Button"`      | `label: string`, `pos: [x, y]`, `color: [r, g, b]`, `group?: int`   |
| `"Label"`       | `text: string`, `pos: [x, y]`, `color: [r, g, b]`                   |
| `"Checkbox"`    | `label: string`, `pos: [x, y]`, `checked: bool`, `color: [r, g, b]` |
| `"Dropdown"`    | `items: [string]`, `pos: [x, y]`, `selected: int`, `color: [r,g,b]` |
| `"TextInput"`   | `text: string`, `pos: [x, y]`, `color: [r, g, b]`                   |
| `"Panel"`       | `pos: [x, y]`, `size: [w, h]`, `color: [r, g, b]`                   |
| `"GridLayout"`  | `pos: [x, y]`, `rows: int`, `cols: int`, `spacing: int`             |
| `"ContextMenu"` | `items: [string]`, `pos: [x, y]`, `color: [r, g, b]`                |

#### Notes:

- All functions are available as `ui.<function>` in Lua, and as methods on `UiApi` in Python.
- Widget property tables/dicts use the scripting language’s native data structures.
- Dynamic widget registration allows to add new types at runtime from scripts.

---

## Other Functions

| Function                  | Description                                 |
| ------------------------- | ------------------------------------------- |
| `move_entity(id, dx, dy)` | Move an entity by (dx, dy)                  |
| `move_all(dx, dy)`        | Move all entities with Position             |
| `damage_entity(id, amt)`  | Damage an entity (reduces Health)           |
| `damage_all(amt)`         | Damage all entities with Health             |
| `tick()`                  | Advance the game by one tick                |
| `get_turn()`              | Get the current turn number                 |
| `process_deaths()`        | Process deaths and start decay              |
| `process_decay()`         | Process decay timers and remove entities    |
| `get_user_input(prompt)`  | Prompt the user for input and return string |

---

## Notes

- All Lua functions are available globally in scripts.
- All Python functions are methods on the `PyWorld` class.
- Component names are case-sensitive and must match the `"title"` in your schema files.
- For practical code, see [docs/examples.md](examples.md).
