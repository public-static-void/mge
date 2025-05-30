# Codegen Tool

Generate Rust, Lua, Python, C, and Markdown from JSON schemas.

---

## Overview

The **codegen** tool generates type-safe data structures and documentation from JSON schemas for multiple languages:

- **Rust**
- **Lua**
- **Python**
- **C**
- **Markdown** (human-readable docs)

It ensures all code and documentation are always up-to-date with the schema definitions.

---

## Installation

If working inside this repository, no installation is needed - just use Cargo:

```
cargo run -p codegen -- <schema.json> <output_dir> [--lang <langs>]
```

Or build a standalone binary:

```
cargo build -p codegen --release
./target/release/codegen <schema.json> <output_dir> [--lang <langs>]
```

---

## Usage

```
codegen <schema.json> <output_dir> [--lang <langs>]
```

- `<schema.json>`: Path to JSON schema file.
- `<output_dir>`: Directory where generated files will be written.
- `--lang <langs>`: Comma-separated list of targets (default: `rust`).
  Supported: `rust`, `lua`, `python`, `c`, `md`

**Examples:**

Generate Rust and Markdown docs:

```
codegen my_schema.json ./generated --lang rust,md
```

Generate all targets:

```
codegen my_schema.json ./generated --lang rust,lua,python,c,md
```

---

## Features

- **Schema-driven:** Always in sync with your JSON schema.
- **Multiple targets:** Rust, Lua, Python, C, Markdown.
- **Robust Markdown doc tests:** Whitespace/formatting-insensitive.
- **Easy to extend:** Add new targets or schema features as needed.

---

## Example

Given `position.json`:

```
{
  "title": "Position",
  "properties": {
    "pos": {
      "oneOf": [
        { "properties": { "Square": { "properties": { "x": {"type": "integer"}, "y": {"type": "integer"}, "z": {"type": "integer"} } } } },
        { "properties": { "Hex":    { "properties": { "q": {"type": "integer"}, "r": {"type": "integer"}, "z": {"type": "integer"} } } } },
        { "properties": { "Region": { "properties": { "id": {"type": "string"} } } } }
      ]
    }
  }
}
```

This will generate:

- `position.rs` (Rust struct/enum)
- `position.lua` (Lua type stub)
- `position.py` (Python TypedDict)
- `position.h` (C header)
- `position.md` (Markdown documentation)
