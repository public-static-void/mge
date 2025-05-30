# PositionComponent

**Kind:** Component
**Source schema:** `position.json`

## Fields

| Name | Type     |
| ---- | -------- |
| pos  | Position |

### Position

A tagged union (enum) with the following variants:

- **Square**:

  - `x` (integer)
  - `y` (integer)
  - `z` (integer)

- **Hex**:

  - `q` (integer)
  - `r` (integer)
  - `z` (integer)

- **Region**:

  - `id` (string)
