# MGE Python Scripting API Reference

This document describes the Python API for interacting with the Modular Game Engine (MGE) ECS.
Use these methods in your Python scripts to create, query, and manipulate entities and components, control game flow, and implement gameplay logic.

---

## PyWorld Class

### Initialization

```python
from mge import PyWorld

# Initialize with the path to your component schemas (optional)
world = PyWorld(schema_dir="/path/to/schemas")
```

---

### Entity Management

| Method                      | Description                             |
| --------------------------- | --------------------------------------- |
| `spawn_entity()`            | Spawn a new entity, returns entity ID   |
| `despawn_entity(entity_id)` | Remove an entity and all its components |
| `get_entities()`            | List all entity IDs                     |

---

### Component Management

| Method                                 | Description                                   |
| -------------------------------------- | --------------------------------------------- |
| `set_component(entity_id, name, data)` | Set a component on an entity                  |
| `get_component(entity_id, name)`       | Get a component's data from an entity         |
| `remove_component(entity_id, name)`    | Remove a component from an entity             |
| `list_components()`                    | List all registered component names           |
| `get_component_schema(name)`           | Get the JSON schema for a component (as dict) |

---

### Entity Queries

| Method                                | Description                                   |
| ------------------------------------- | --------------------------------------------- |
| `is_entity_alive(entity_id)`          | Returns True if entity's Health > 0           |
| `count_entities_with_type(type_str)`  | Count entities with Type.kind == type_str     |
| `get_entities_with_component(name)`   | List entity IDs with a given component        |
| `get_entities_with_components(names)` | List entity IDs with all specified components |

---

### Mode Management

| Method                  | Description               |
| ----------------------- | ------------------------- |
| `set_mode(mode)`        | Set the current game mode |
| `get_mode()`            | Get the current game mode |
| `get_available_modes()` | List all available modes  |

---

### Systems and Gameplay

| Method                                              | Description                                                                                                                   |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `move_entity(entity_id, dx, dy)`                    | Move an entity by (dx, dy)                                                                                                    |
| `move_all(dx, dy)`                                  | Move all entities with Position component                                                                                     |
| `damage_entity(entity_id, amount)`                  | Damage an entity (reduces Health)                                                                                             |
| `damage_all(amount)`                                | Damage all entities with Health component                                                                                     |
| `tick()`                                            | Advance the game simulation by one tick                                                                                       |
| `get_turn()`                                        | Get the current turn number                                                                                                   |
| `process_deaths()`                                  | Process entities with zero or less health                                                                                     |
| `process_decay()`                                   | Process decay timers and remove decayed entities                                                                              |
| `modify_stockpile_resource(entity_id, kind, delta)` | Add or remove (delta can be negative) a specific resource in a Stockpile component. Raises ValueError if not enough resource. |

---

## Example Usage

```python
from mge import PyWorld

world = PyWorld("/path/to/schemas")

# Spawn a player entity
player = world.spawn_entity()
world.set_component(player, "Type", {"kind": "player"})
world.set_component(player, "Health", {"current": 10, "max": 10})

# Move the player
world.set_component(player, "Position", {"x": 0, "y": 0})
world.move_entity(player, 1, 2)
print(world.get_component(player, "Position"))  # {'x': 1, 'y': 2}

# List all components
print(world.list_components())

# Get schema for 'Health'
print(world.get_component_schema("Health"))

# Change mode
world.set_mode("roguelike")
print(world.get_mode())
print(world.get_available_modes())
```

---

## Notes

- All methods are available on the `PyWorld` class.
- Component names are case-sensitive and must match the `"title"` field in your schema files.
- You can add new components by adding new JSON schema files-no Rust code changes required.
- For more advanced scripting, see the example scripts in `engine/scripts/python/`.
