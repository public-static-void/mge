//! Fluid simulation system.
//!
//! Simulates water and magma flow across map cells using level-based flooding.
//! All fluid state is stored in cell metadata under the `"fluid"` key and
//! written exclusively through `merge_cell_metadata` to preserve existing
//! metadata (`walkable`, `transparent`, `terrain`, etc.).

/// Fluid level at which a cell becomes impassable and opaque.
///
/// When a cell's fluid level reaches this threshold, the fluid system writes
/// `walkable: false` and `transparent: false` into the cell's metadata. Values
/// below this threshold restore the pre-fluid values.
pub const FLUID_BLOCK_LEVEL: i64 = 3;

/// Maximum fluid level per cell (saturation).
///
/// Fluid cannot exceed this level. Flow calculations cap transfers so that the
/// receiving cell never exceeds this value.
pub const FLUID_MAX_LEVEL: i64 = 8;
