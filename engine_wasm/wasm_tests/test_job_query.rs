// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_query() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "component")]
    unsafe extern "C" {
        fn set_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            json_ptr: *const u8,
            json_len: i32,
        );
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "job_query")]
    unsafe extern "C" {
        fn list_jobs(include_terminal: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_job(job_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn find_jobs(
            filter_ptr: *const u8,
            filter_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn advance_job_state(job_id: i32) -> i32;
        fn get_job_children(job_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_job_children(job_id: i32, data_ptr: *const u8, data_len: i32) -> i32;
        fn get_job_dependencies(job_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_job_dependencies(job_id: i32, data_ptr: *const u8, data_len: i32) -> i32;
    }

    unsafe {
        let eid = spawn_entity();

        // Set a Job component with state "pending"
        let comp_name = "Job";
        let job_json = r#"{"job_type":"mining","state":"pending","progress":0.0}"#;
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            job_json.as_ptr(),
            job_json.len() as i32,
        );

        // list_jobs(0) excluding terminal — should find the pending job
        let mut list_buf = [0u8; 512];
        let list_written = list_jobs(0, list_buf.as_mut_ptr(), list_buf.len() as i32);
        if list_written <= 0 {
            return 0;
        }
        let list_str = core::str::from_utf8(&list_buf[..list_written as usize]).unwrap_or("");
        if !list_str.contains("\"state\":\"pending\"") {
            return 0;
        }

        // list_jobs(1) including terminal — should also find the pending job
        let mut list_buf2 = [0u8; 512];
        let list_written2 = list_jobs(1, list_buf2.as_mut_ptr(), list_buf2.len() as i32);
        if list_written2 <= 0 {
            return 0;
        }

        // get_job returns the job with an "id" field
        let mut job_buf = [0u8; 512];
        let job_written = get_job(eid as i32, job_buf.as_mut_ptr(), job_buf.len() as i32);
        if job_written < 0 {
            return 0;
        }
        let job_str = core::str::from_utf8(&job_buf[..job_written as usize]).unwrap_or("");
        if !job_str.contains("\"id\":") || !job_str.contains("\"mining\"") {
            return 0;
        }

        // find_jobs with filter by state
        let filter = r#"{"state":"pending"}"#;
        let mut find_buf = [0u8; 512];
        let find_written = find_jobs(
            filter.as_ptr(),
            filter.len() as i32,
            find_buf.as_mut_ptr(),
            find_buf.len() as i32,
        );
        if find_written <= 0 {
            return 0;
        }
        let find_str = core::str::from_utf8(&find_buf[..find_written as usize]).unwrap_or("");
        if !find_str.contains("mining") {
            return 0;
        }

        // advance_job_state: "pending" → "going_to_site"
        let adv_result = advance_job_state(eid as i32);
        if adv_result != 0 {
            return 0;
        }

        // Verify state advanced
        let mut adv_buf = [0u8; 512];
        let adv_written = get_job(eid as i32, adv_buf.as_mut_ptr(), adv_buf.len() as i32);
        if adv_written < 0 {
            return 0;
        }
        let adv_str = core::str::from_utf8(&adv_buf[..adv_written as usize]).unwrap_or("");
        if !adv_str.contains("\"going_to_site\"") {
            return 0;
        }

        // set_job_children / get_job_children roundtrip
        let children = "[3,4]";
        let set_child = set_job_children(eid as i32, children.as_ptr(), children.len() as i32);
        if set_child != 0 {
            return 0;
        }
        let mut child_buf = [0u8; 128];
        let child_written = get_job_children(eid as i32, child_buf.as_mut_ptr(), child_buf.len() as i32);
        if child_written < 0 {
            return 0;
        }
        let child_str = core::str::from_utf8(&child_buf[..child_written as usize]).unwrap_or("");
        if !child_str.contains("3") || !child_str.contains("4") {
            return 0;
        }

        // set_job_dependencies / get_job_dependencies roundtrip
        let deps = "[5]";
        let set_dep = set_job_dependencies(eid as i32, deps.as_ptr(), deps.len() as i32);
        if set_dep != 0 {
            return 0;
        }
        let mut dep_buf = [0u8; 128];
        let dep_written = get_job_dependencies(eid as i32, dep_buf.as_mut_ptr(), dep_buf.len() as i32);
        if dep_written < 0 {
            return 0;
        }
        let dep_str = core::str::from_utf8(&dep_buf[..dep_written as usize]).unwrap_or("");
        if !dep_str.contains("5") {
            return 0;
        }

        // get_job for nonexistent id — should return -1
        let mut miss_buf = [0u8; 128];
        let miss_result = get_job(99999, miss_buf.as_mut_ptr(), miss_buf.len() as i32);
        if miss_result != -1 {
            return 0;
        }

        // advance_job_state for nonexistent id — should return -1
        let bad_adv = advance_job_state(99999);
        if bad_adv != -1 {
            return 0;
        }

        1
    }
}
