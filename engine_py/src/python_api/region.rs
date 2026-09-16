use super::PyWorld;
use crate::PyObject;
use engine_core::ecs::world::ZoneShape;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::depythonize;

/// Methods for accessing entities in regions
pub trait RegionApi {
    /// Get all entities in a region
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32>;
    /// Get all entities of a kind in a region
    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32>;
    /// Get all cells in a region
    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject>;
    /// Get all cells of a kind in a region
    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject>;
}

impl RegionApi for PyWorld {
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32> {
        let world = self.inner.borrow();
        world.entities_in_region(&region_id)
    }

    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32> {
        let world = self.inner.borrow();
        world.entities_in_region_kind(&kind)
    }

    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let cells = world.cells_in_region(&region_id);
        Ok(serde_pyobject::to_pyobject(py, &cells)?.into())
    }

    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let cells = world.cells_in_region_kind(&kind);
        Ok(serde_pyobject::to_pyobject(py, &cells)?.into())
    }
}

/// Extracts an integer coordinate from a shape dict, naming the field.
fn rect_int(rect: &serde_json::Value, key: &str) -> PyResult<i64> {
    rect.get(key)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
        })
        .ok_or_else(|| PyValueError::new_err(format!("shape.rect missing integer '{key}'")))
}

/// Parses a [`ZoneShape`] from a Python shape dict.
///
/// Accepts `{"rect": {"x0", "y0", "z", "x1", "y1"}}` (Square-topology
/// inclusive rectangle) or `{"cells": [...]}` (explicit cell list).
fn parse_zone_shape(shape: &Bound<'_, PyAny>) -> PyResult<ZoneShape> {
    let val: serde_json::Value =
        depythonize(shape).map_err(|e| PyValueError::new_err(format!("Invalid shape: {e}")))?;
    let obj = val
        .as_object()
        .ok_or_else(|| PyValueError::new_err("shape must be a dict with 'rect' or 'cells'"))?;
    if let Some(rect) = obj.get("rect") {
        return Ok(ZoneShape::Rect {
            x0: rect_int(rect, "x0")?,
            y0: rect_int(rect, "y0")?,
            z: rect_int(rect, "z")?,
            x1: rect_int(rect, "x1")?,
            y1: rect_int(rect, "y1")?,
        });
    }
    if let Some(cells) = obj.get("cells") {
        let arr = cells
            .as_array()
            .ok_or_else(|| PyValueError::new_err("shape.cells must be an array"))?;
        return Ok(ZoneShape::Cells(arr.clone()));
    }
    Err(PyValueError::new_err(
        "shape must contain 'rect' or 'cells'",
    ))
}

/// Parses an explicit cell list into JSON values.
fn parse_cells(cells: &Bound<'_, PyAny>) -> PyResult<Vec<serde_json::Value>> {
    depythonize::<Vec<serde_json::Value>>(cells)
        .map_err(|e| PyValueError::new_err(format!("Invalid cells: {e}")))
}

/// Zone designate/manage/query surface.
///
/// Argument order and value shapes mirror the Lua/WASM surface:
/// `designate_zone(kind, label, shape)` with `shape` as
/// `{"rect": {...}}` or `{"cells": [...]}`. Core colony-gate and validation
/// errors propagate as `ValueError`.
pub trait ZoneApi {
    /// Designate a zone; returns the unique zone id string.
    fn designate_zone(
        &self,
        kind: String,
        label: Option<String>,
        shape: &Bound<'_, PyAny>,
    ) -> PyResult<String>;
    /// Remove a zone; False for unknown ids.
    fn remove_zone(&self, zone_id: String) -> PyResult<bool>;
    /// Rename a zone; False for unknown ids.
    fn rename_zone(&self, zone_id: String, label: String) -> PyResult<bool>;
    /// Change a zone's kind; False for unknown ids.
    fn set_zone_kind(&self, zone_id: String, kind: String) -> PyResult<bool>;
    /// Assign explicit cells; ignores duplicates idempotently.
    fn assign_cells_to_zone(&self, zone_id: String, cells: &Bound<'_, PyAny>) -> PyResult<bool>;
    /// Unassign explicit cells; ignores unmembered cells idempotently.
    fn unassign_cells_from_zone(&self, zone_id: String, cells: &Bound<'_, PyAny>)
    -> PyResult<bool>;
    /// List zones as `{id, label, kind, cell_count}` dicts.
    fn list_zones(&self, py: Python) -> PyResult<PyObject>;
    /// Get a zone as `{id, label, kind, rects, cells}`, or None.
    fn get_zone(&self, py: Python, zone_id: String) -> PyResult<Option<PyObject>>;
}

impl ZoneApi for PyWorld {
    fn designate_zone(
        &self,
        kind: String,
        label: Option<String>,
        shape: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let zone_shape = parse_zone_shape(shape)?;
        let mut world = self.inner.borrow_mut();
        world
            .designate_zone(&kind, label.as_deref(), zone_shape)
            .map_err(PyValueError::new_err)
    }

    fn remove_zone(&self, zone_id: String) -> PyResult<bool> {
        let mut world = self.inner.borrow_mut();
        world.remove_zone(&zone_id).map_err(PyValueError::new_err)
    }

    fn rename_zone(&self, zone_id: String, label: String) -> PyResult<bool> {
        let mut world = self.inner.borrow_mut();
        world
            .rename_zone(&zone_id, &label)
            .map_err(PyValueError::new_err)
    }

    fn set_zone_kind(&self, zone_id: String, kind: String) -> PyResult<bool> {
        let mut world = self.inner.borrow_mut();
        world
            .set_zone_kind(&zone_id, &kind)
            .map_err(PyValueError::new_err)
    }

    fn assign_cells_to_zone(&self, zone_id: String, cells: &Bound<'_, PyAny>) -> PyResult<bool> {
        let cells = parse_cells(cells)?;
        let mut world = self.inner.borrow_mut();
        world
            .assign_cells_to_zone(&zone_id, cells)
            .map_err(PyValueError::new_err)
    }

    fn unassign_cells_from_zone(
        &self,
        zone_id: String,
        cells: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let cells = parse_cells(cells)?;
        let mut world = self.inner.borrow_mut();
        world
            .unassign_cells_from_zone(&zone_id, cells)
            .map_err(PyValueError::new_err)
    }

    fn list_zones(&self, py: Python) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let zones = world.list_zones();
        Ok(serde_pyobject::to_pyobject(py, &zones)?.into())
    }

    fn get_zone(&self, py: Python, zone_id: String) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        match world.get_zone(&zone_id) {
            Some(zone) => Ok(Some(serde_pyobject::to_pyobject(py, &zone)?.into())),
            None => Ok(None),
        }
    }
}
