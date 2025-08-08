use super::PyWorld;
use pyo3::prelude::*;
use serde_json::json;

/// JobQueryApi provides job querying and mutation for scripting.
pub trait JobQueryApi {
    /// List jobs. If `include_terminal` is true, include jobs in terminal states ("complete", "failed", "cancelled").
    fn list_jobs(&self, py: Python<'_>, include_terminal: Option<bool>) -> PyResult<PyObject>;
    fn get_job(&self, py: Python<'_>, job_id: u32) -> PyResult<PyObject>;
    fn find_jobs(
        &self,
        py: Python<'_>,
        state: Option<String>,
        job_type: Option<String>,
        assigned_to: Option<u32>,
        category: Option<String>,
    ) -> PyResult<PyObject>;
    fn set_job_field(&self, job_id: u32, field: &str, value: &Bound<'_, PyAny>) -> PyResult<()>;
    fn update_job(
        &self,
        job_id: u32,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<()>;
    fn cancel_job(&self, job_id: u32) -> PyResult<()>;
}

impl JobQueryApi for PyWorld {
    fn list_jobs(&self, py: Python<'_>, include_terminal: Option<bool>) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let mut jobs = Vec::new();
        if let Some(job_map) = world.components.get("Job") {
            for (eid, comp) in job_map.iter() {
                let mut job = comp.clone();
                job["id"] = json!(eid);
                let state = job.get("state").and_then(|v| v.as_str());
                let is_terminal =
                    matches!(state, Some("complete") | Some("failed") | Some("cancelled"));
                if !include_terminal.unwrap_or(false) && is_terminal {
                    continue;
                }
                jobs.push(job);
            }
        }
        Ok(serde_pyobject::to_pyobject(py, &jobs)?.into())
    }

    fn get_job(&self, py: Python<'_>, job_id: u32) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        if let Some(job) = world.get_component(job_id, "Job") {
            let mut job = job.clone();
            job["id"] = json!(job_id);
            Ok(serde_pyobject::to_pyobject(py, &job)?.into())
        } else {
            Ok(py.None())
        }
    }

    fn find_jobs(
        &self,
        py: Python<'_>,
        state: Option<String>,
        job_type: Option<String>,
        assigned_to: Option<u32>,
        category: Option<String>,
    ) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let mut jobs = Vec::new();
        if let Some(job_map) = world.components.get("Job") {
            for (eid, comp) in job_map.iter() {
                let mut job = comp.clone();
                if let Some(ref s) = state
                    && job.get("state").and_then(|v| v.as_str()) != Some(s)
                {
                    continue;
                }
                if let Some(ref jt) = job_type
                    && job.get("job_type").and_then(|v| v.as_str()) != Some(jt)
                {
                    continue;
                }
                if let Some(aid) = assigned_to
                    && job.get("assigned_to").and_then(|v| v.as_u64()) != Some(aid as u64)
                {
                    continue;
                }
                if let Some(ref cat) = category
                    && job.get("category").and_then(|v| v.as_str()) != Some(cat)
                {
                    continue;
                }
                job["id"] = json!(eid);
                jobs.push(job);
            }
        }
        Ok(serde_pyobject::to_pyobject(py, &jobs)?.into())
    }

    fn set_job_field(&self, job_id: u32, field: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            let val: serde_json::Value = pythonize::depythonize(value)?;
            job[field] = val;
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        } else {
            Err(pyo3::exceptions::PyKeyError::new_err("Job not found"))
        }
    }

    fn update_job(
        &self,
        job_id: u32,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            if let Some(kwargs) = kwargs {
                let extra: serde_json::Value = pythonize::depythonize(kwargs)?;
                if let Some(obj) = extra.as_object() {
                    for (k, v) in obj {
                        job[k] = v.clone();
                    }
                }
            }
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        } else {
            Err(pyo3::exceptions::PyKeyError::new_err("Job not found"))
        }
    }

    fn cancel_job(&self, job_id: u32) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            job["state"] = serde_json::json!("cancelled");
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        } else {
            Err(pyo3::exceptions::PyKeyError::new_err("Job not found"))
        }
    }
}
