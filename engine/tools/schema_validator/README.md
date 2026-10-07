# Schema Validator

A CLI tool for validating JSON schema files used in the MGE engine.

## Features

- Validates all JSON schema files in a directory or a single file
- Checks for required fields (`title`, `modes`, etc.)
- Ensures `modes` are from a set of allowed game modes
- Checks for property constraints (e.g., `minimum` <= `maximum`)
- Color-coded, human-friendly CLI output
- Options for fail-fast and summary-only modes
- Ready for CI integration

## Usage

From the project root, always via the canonical make target:

```bash
make validate-schema
```

Validate a single schema file or pass extra flags by forwarding them with `ARGS`:

```bash
make validate-schema ARGS="engine/assets/schemas/health.json"
```

Show only the summary (no per-file output):

```bash
make validate-schema ARGS="engine/assets/schemas/ --summary-only"
```

Stop at the first error:

```bash
make validate-schema ARGS="engine/assets/schemas/ --fail-fast"
```

Validate against a different game config:

```bash
make validate-schema ARGS="engine/assets/schemas/ --config game.toml"
```

## Allowed Modes

The following modes are currently allowed in schemas (mirrors `allowed_modes` in `game.toml` at the repo root):

- `colony`
- `roguelike`
- `editor`
- `simulation`
- `single`
- `multi`
- `grand-strategy`
- `4x`

The allowed list is loaded at runtime from `allowed_modes` in `game.toml` (see `load_allowed_modes` in `src/main.rs`; override with `--config <path>`), so this list follows that file — to allow a new mode, update `game.toml`, not the validator.

## Validation Rules

- Every schema must have a `"title"` field.
- Every schema must have a `"modes"` array.
- All modes must be one of the allowed modes.
- For each property, if both `minimum` and `maximum` are present, `minimum` must not be greater than `maximum`.

## Extending

- To add new validation rules, edit [`src/lib.rs`](src/lib.rs).
- To add new CLI options, edit [`src/main.rs`](src/main.rs).
- To allow a new mode, add it to `allowed_modes` in `game.toml` at the repo root.

## CI Integration

Schemas are validated automatically on every PR by the `validate-schema` job in `.github/workflows/ci.yml`, which runs the same `make validate-schema` target documented above.
