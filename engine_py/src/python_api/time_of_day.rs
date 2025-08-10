use super::PyWorld;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Time of day
pub trait TimeOfDayApi {
    /// Get time of day
    fn get_time_of_day(&self, py: Python) -> PyObject;
}

impl TimeOfDayApi for PyWorld {
    fn get_time_of_day(&self, py: Python) -> PyObject {
        let world = self.inner.borrow();
        let tod = world.get_time_of_day();
        let dict = PyDict::new(py);
        dict.set_item("hour", tod.hour).unwrap();
        dict.set_item("minute", tod.minute).unwrap();
        dict.into_pyobject(py).unwrap().unbind().into()
    }
}
