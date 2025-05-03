# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation.
It is designed for simulation, games, and rapid prototyping with robust ECS, plugin, and scripting support.

---

## Project Overview

MGE provides:

- A Rust-based core engine with a macro-driven ECS framework.
- Hot-reloadable plugin support and cross-language scripting (Lua, Python, WASM).
- Mode-specific logic and data (e.g., Colony, Roguelike).
- Schema-driven, versioned component management.
- An architecture designed for tooling, modding, and rapid iteration.

For the full architecture and design blueprint, see [`docs/idea.md`](docs/idea.md).

---

## Repository Structure

```
engine/
  core/          # ECS core, registry, schema, migration (Rust)
  docs/          # Engine-specific documentation and specs
  assets/        # Game assets and data
  scripts/       # Build or tooling scripts
  tools/         # Engine tools and utilities
engine_macros/   # Procedural macros for component ergonomics
docs/            # Project-wide docs and blueprints
```

---

## Getting Started

1. **Explore the ECS Core:**

   - See [`engine/core`](engine/core) and [`engine_macros`](engine_macros).
   - Define components with `#[component(...)]` and auto-generate schemas.

2. **Run tests:**

   ```sh
   cargo test
   ```

3. **Read [`docs/idea.md`](docs/idea.md) for the full blueprint.**

---

## Related Crates

- [`engine/core`](engine/core): ECS core, registry, migration, and schema logic.
- [`engine_macros`](engine_macros): Procedural macros for ergonomic component definition.

---

## Continuous Integration & Releases

- **CI**: All pull requests and pushes to main run automated checks (formatting, linting, tests) via GitHub Actions.

- **Release**: Merges to main automatically trigger a semantic-release pipeline:

  - Versioning and changelog are managed by semantic-release.

  - Rust crate version is updated using `@timada/semantic-release-cargo`.

  - Releases are published to GitHub Releases

## Branch Protection:

The main branch is protected: all PRs require passing status checks before merging.
