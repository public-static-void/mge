//! Layout strategies for mapping logical map cells to screen coordinates.

use crate::map::cell_key::CellKey;

/// Trait for mapping a logical cell to a screen coordinate.
pub trait CellLayout {
    /// Returns (x, y) screen position for a given cell.
    fn cell_to_screen(&self, cell: &CellKey) -> (i32, i32);
}

/// Square grid layout: direct mapping.
pub struct SquareLayout;

impl CellLayout for SquareLayout {
    fn cell_to_screen(&self, cell: &CellKey) -> (i32, i32) {
        match cell {
            CellKey::Square { x, y, .. } => (*x, *y),
            _ => (0, 0),
        }
    }
}

/// Hex grid layout: offset or axial mapping.
pub struct HexLayout;

impl CellLayout for HexLayout {
    fn cell_to_screen(&self, cell: &CellKey) -> (i32, i32) {
        match cell {
            CellKey::Hex { q, r, .. } => {
                // Axial to pixel (pointy-topped)
                let x = *q * 2 + *r;
                let y = (*r * 3) / 2;
                (x, y)
            }
            _ => (0, 0),
        }
    }
}

/// Province layout: default to (0,0) or implement centroid/bbox as needed.
pub struct ProvinceLayout;

impl CellLayout for ProvinceLayout {
    fn cell_to_screen(&self, cell: &CellKey) -> (i32, i32) {
        match cell {
            CellKey::Province { .. } => (0, 0), // TODO: implement province centroid
            _ => (0, 0),
        }
    }
}
