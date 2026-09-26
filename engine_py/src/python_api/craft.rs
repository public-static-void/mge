use crate::PyObject;
use crate::python_api::world::PyWorld;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Register a craft recipe under `name` from its JSON definition.
///
/// Argument order mirrors the Lua/WASM surface: `(name, recipe_json)`.
/// Raises `ValueError` carrying the loader reason on invalid JSON.
///
/// Example:
/// ```python
/// world.register_craft_recipe("iron_sword", '{"name":"iron_sword","duration":3,...}')
/// ```
pub fn register_craft_recipe(pyworld: &PyWorld, name: String, recipe_json: String) -> PyResult<()> {
    let recipe: serde_json::Value = serde_json::from_str(&recipe_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid recipe JSON: {e}")))?;
    let mut world = pyworld.inner.borrow_mut();
    world
        .register_craft_recipe(name, recipe)
        .map_err(PyValueError::new_err)
}

/// Names of registered craft-path recipes (those with `output_item`),
/// sorted ascending.
///
/// Example:
/// ```python
/// names = world.list_craft_recipes()
/// ```
pub fn list_craft_recipes(pyworld: &PyWorld) -> Vec<String> {
    let world = pyworld.inner.borrow();
    world.list_craft_recipes()
}

/// Pure gate check: `(True, None)` when `crafter` may start `recipe`, else
/// `(False, reason)` with one of `unknown_recipe`/`already_crafting`/
/// `missing_input:<kind>`/`missing_material:<material>`/`missing_tool:<item>`/
/// `insufficient_skill` — byte-identical to the Lua/WASM surfaces.
///
/// Example:
/// ```python
/// ok, err = world.can_craft(crafter, "iron_sword")
/// ```
pub fn can_craft(pyworld: &PyWorld, crafter: u32, recipe: String) -> (bool, Option<String>) {
    let world = pyworld.inner.borrow();
    match world.can_craft(crafter, &recipe) {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err)),
    }
}

/// Start crafting: runs the shared gate, creates an in-progress `CraftOrder`,
/// deducts inputs/materials and removes consumed tools once.
///
/// Returns `(True, None)` on success, `(False, reason)` with the same error
/// strings as `can_craft`.
///
/// Example:
/// ```python
/// ok, err = world.start_craft(crafter, "iron_sword")
/// ```
pub fn start_craft(pyworld: &PyWorld, crafter: u32, recipe: String) -> (bool, Option<String>) {
    let mut world = pyworld.inner.borrow_mut();
    match world.start_craft(crafter, &recipe) {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err)),
    }
}

/// Clone of the crafter's `CraftOrder` as a dict, or `None` when absent.
///
/// Returns a dict with `{recipe, progress, state, output_entity}`.
///
/// Example:
/// ```python
/// state = world.get_craft_state(crafter)
/// ```
pub fn get_craft_state(pyworld: &PyWorld, py: Python, crafter: u32) -> PyResult<Option<PyObject>> {
    let world = pyworld.inner.borrow();
    if let Some(order) = world.get_craft_state(crafter) {
        Ok(Some(serde_pyobject::to_pyobject(py, &order)?.into()))
    } else {
        Ok(None)
    }
}

/// Cancel an in-progress order: refunds stockpile inputs/materials
/// (consumable tools are not refunded), removes the order, and emits
/// `craft_cancelled`.
///
/// Returns `(True, None)` on success, `(False, "no_craft_order")` when no
/// in-progress order exists.
///
/// Example:
/// ```python
/// ok, err = world.cancel_craft(crafter)
/// ```
pub fn cancel_craft(pyworld: &PyWorld, crafter: u32) -> (bool, Option<String>) {
    let mut world = pyworld.inner.borrow_mut();
    match world.cancel_craft(crafter) {
        Ok(done) => (done, None),
        Err(err) => (false, Some(err)),
    }
}
