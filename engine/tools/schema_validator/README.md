# Schema Validator

A CLI tool for validating JSON schema files used in the MGE engine.

## Features

- Validates all JSON schema files in a directory or a single file
- Checks for required fields (`title`, `modes`, etc.)
- Ensures `modes` are from a set of allowed game modes
- Checks for property constraints (e.g., `minimum` =< `maximum`)
- Color-coded, human-friendly CLI output
- Options for fail-fast and summary-only modes
- Ready for CI integration

## Usage

From the project root:

```bash
cargo run -p schema_validator -- <path-to-schema-or-directory> [OPTIONS]
```

### Examples

Validate all schemas in a directory:

```bash
cargo run -p schema_validator -- engine/assets/schemas/
```

Validate a single schema file:

```bash
cargo run -p schema_validator -- engine/assets/schemas/health.json
```

Show only the summary (no per-file output):

```bash
cargo run -p schema_validator -- engine/assets/schemas/ --summary-only
```

Stop at the first error:

```bash
cargo run -p schema_validator -- engine/assets/schemas/ --fail-fast
```

## Allowed Modes

The following modes are currently allowed in schemas:

- `colony`
- `roguelike`
- `single`
- `multi`
- `editor`
- `simulation`

To add more modes, update the `allowed_modes` list in [`src/lib.rs`](src/lib.rs).

## Validation Rules

- Every schema must have a `"title"` field.
- Every schema must have a `"modes"` array.
- All modes must be one of the allowed modes.
- For each property, if both `minimum` and `maximum` are present, `minimum` must not be greater than `maximum`.

## Extending

- To add new validation rules, edit [`src/lib.rs`](src/lib.rs).
- To add new CLI options, edit [`src/main.rs`](src/main.rs).
- To add new allowed modes, update the `allowed_modes` list in [`src/lib.rs`](src/lib.rs).

## CI Integration

See the main project’s `.github/workflows/lint-schemas.yml` for how to run this tool automatically on every PR.
