# engine_macros

Procedural macros for the Modular Game Engine.

This crate provides the `#[component]` macro for ECS component code generation.
It is not intended for standalone use-see the main project for context and examples.

## Features

- Versioned ECS component definition
- Migration support between versions
- Mode restriction (e.g., colony, roguelike)
- Serde (de)serialization
- Optional JSON schema generation

## Example

```rust
use engine_macros::component;

#[component(modes(Single), version = "1.0.0", schema)]
#[derive(Debug, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}
```
