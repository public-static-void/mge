// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_ai() -> i32 {
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

    #[link(wasm_import_module = "job_ai")]
    unsafe extern "C" {
        fn ai_assign_jobs(agent_id: i32) -> i32;
        fn ai_query_jobs(agent_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn ai_modify_job_assignment(
            entity_id: i32,
            job_id: i32,
            changes_ptr: *const u8,
            changes_len: i32,
        ) -> i32;
    }

    unsafe {
        let agent_id = spawn_entity();
        let job_id = spawn_entity();

        // Set Agent component on the agent (current_job=0 means unassigned)
        let agent_comp = "Agent";
        let agent_json = r#"{"name":"miner","current_job":0}"#;
        set_component(
            agent_id,
            agent_comp.as_ptr(),
            agent_comp.len() as i32,
            agent_json.as_ptr(),
            agent_json.len() as i32,
        );

        // Set Job component on the job entity with high priority
        let job_comp = "Job";
        let job_json =
            r#"{"job_type":"mining","state":"pending","progress":0.0,"priority":10,"assigned_to":0}"#;
        set_component(
            job_id,
            job_comp.as_ptr(),
            job_comp.len() as i32,
            job_json.as_ptr(),
            job_json.len() as i32,
        );

        // Assign jobs to the agent — should pick the pending high-priority job
        let assign_result = ai_assign_jobs(agent_id as i32);
        if assign_result != 0 {
            return 0;
        }

        // Query jobs assigned to agent — should include our mining job
        let mut query_buf = [0u8; 512];
        let query_written =
            ai_query_jobs(agent_id as i32, query_buf.as_mut_ptr(), query_buf.len() as i32);
        if query_written <= 0 {
            return 0;
        }
        let query_str = core::str::from_utf8(&query_buf[..query_written as usize]).unwrap_or("");
        if !query_str.contains("mining") || !query_str.contains("\"assigned_to\":") {
            return 0;
        }

        // Modify assignment: unassign the job via null
        let changes = r#"{"assigned_to":null}"#;
        let modify_result = ai_modify_job_assignment(
            agent_id as i32,
            job_id as i32,
            changes.as_ptr(),
            changes.len() as i32,
        );
        if modify_result != 0 {
            return 0;
        }

        // Query again — the job should no longer appear for this agent
        let mut query2_buf = [0u8; 512];
        let query2_written =
            ai_query_jobs(agent_id as i32, query2_buf.as_mut_ptr(), query2_buf.len() as i32);
        if query2_written < 0 {
            return 0;
        }
        let query2_str =
            core::str::from_utf8(&query2_buf[..query2_written as usize]).unwrap_or("");
        if query2_str.contains("mining") {
            return 0;
        }

        // AI assign on entity without Agent component — should return -1
        let no_agent = spawn_entity();
        let bad_assign = ai_assign_jobs(no_agent as i32);
        if bad_assign != -1 {
            return 0;
        }

        1
    }
}
