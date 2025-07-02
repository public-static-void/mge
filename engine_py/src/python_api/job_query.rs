use super::PyWorld;
use pyo3::prelude::*;
use serde_json::json;

pub trait JobQueryApi {
    fn list_jobs(&self, py: Python<'_>) -> PyResult<PyObject>;
    fn get_job(&self, py: Python<'_>, job_id: u32) -> PyResult<PyObject>;
    fn find_jobs(
        &self,
        py: Python<'_>,
        state: Option<String>,
        job_type: Option<String>,
        assigned_to: Option<u32>,
        category: Option<String>,
    ) -> PyResult<PyObject>;
}

impl JobQueryApi for PyWorld {
    fn list_jobs(&self, py: Python<'_>) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let mut jobs = Vec::new();
        if let Some(job_map) = world.components.get("Job") {
            for (eid, comp) in job_map.iter() {
                let mut job = comp.clone();
                job["id"] = json!(eid);
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
                if let Some(ref s) = state {
                    if job.get("state").and_then(|v| v.as_str()) != Some(s) {
                        continue;
                    }
                }
                if let Some(ref jt) = job_type {
                    if job.get("job_type").and_then(|v| v.as_str()) != Some(jt) {
                        continue;
                    }
                }
                if let Some(aid) = assigned_to {
                    if job.get("assigned_to").and_then(|v| v.as_u64()) != Some(aid as u64) {
                        continue;
                    }
                }
                if let Some(ref cat) = category {
                    if job.get("category").and_then(|v| v.as_str()) != Some(cat) {
                        continue;
                    }
                }
                job["id"] = json!(eid);
                jobs.push(job);
            }
        }
        Ok(serde_pyobject::to_pyobject(py, &jobs)?.into())
    }
}
