# MGE Python Scripting API Reference

This document describes the Python API for interacting with the Modular Game Engine (MGE) ECS.

## PyWorld Class

### Entity Management

- `spawn() -> int`
  Spawn a new entity and return its ID.

- `despawn(entity_id: int)`
  Remove an entity and all its components.

### Component Management

- `set_component(entity_id: int, name: str, value: dict)`
  Set a component on an entity.

- `get_component(entity_id: int, name: str) -> dict or None`
  Get a component's data from an entity.

- `remove_component(entity_id: int, name: str)`
  Remove a component from an entity.

- `list_components() -> List[str]`
  List all registered component names.

- `get_component_schema(name: str) -> dict`
  Get the JSON schema of a component.

### Mode Management

- `set_mode(mode: str)`
  Set the current game mode.

- `get_mode() -> str`
  Get the current game mode.

- `get_available_modes() -> List[str]`
  Get all available modes.

### Systems and Gameplay

- `move_entity(entity_id: int, dx: float, dy: float)`
  Move an entity by (dx, dy).

- `move_all(dx: float, dy: float)`
  Move all entities with Position component.

- `damage_entity(entity_id: int, amount: float)`
  Damage an entity.

- `damage_all(amount: float)`
  Damage all entities with Health component.

- `tick()`
  Advance the game simulation by one tick.

- `get_turn() -> int`
  Get the current turn number.

- `process_deaths()`
  Process entities with zero or less health.

- `process_decay()`
  Process decay timers and remove decayed entities.

- `count_entities_with_type(type_name: str) -> int`
  Count entities with a given Type component kind.

## Example Usage

```python
from mge import PyWorld

world = PyWorld()

eid = world.spawn()
world.set_component(eid, "Health", {"current": 10, "max": 10})

print(world.get_component(eid, "Health"))

modes = world.get_available_modes()
print("Available modes:", modes)

schema = world.get_component_schema("Health")
print("Health schema:", schema)
```
