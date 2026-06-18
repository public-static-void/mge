// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_cancel() -> i32 {
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

    #[link(wasm_import_module = "job_cancel")]
    unsafe extern "C" {
        fn cancel_job(job_id: i32) -> i32;
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

        // Cancel the job
        let cancel_result = cancel_job(eid as i32);
        if cancel_result != 0 {
            return 0;
        }

        // Verify state is now "cancelled"
        let mut buf = [0u8; 256];
        let written = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written < 0 {
            return 0;
        }
        let job_str = core::str::from_utf8(&buf[..written as usize]).unwrap_or("");
        if !job_str.contains("\"cancelled\"") {
            return 0;
        }

        // Cancel nonexistent job — should return -1
        let bad_cancel = cancel_job(99999);
        if bad_cancel != -1 {
            return 0;
        }

        1
    }
}
