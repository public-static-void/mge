// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_mutation() -> i32 {
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

    #[link(wasm_import_module = "job_mutation")]
    unsafe extern "C" {
        fn set_job_field(
            job_id: i32,
            field_ptr: *const u8,
            field_len: i32,
            value_ptr: *const u8,
            value_len: i32,
        ) -> i32;
        fn update_job(job_id: i32, fields_ptr: *const u8, fields_len: i32) -> i32;
    }

    unsafe {
        let eid = spawn_entity();

        // Set initial Job component
        let comp_name = "Job";
        let job_json = r#"{"job_type":"mining","state":"pending","progress":0.0}"#;
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            job_json.as_ptr(),
            job_json.len() as i32,
        );

        // set_job_field: change progress to 0.5
        let field = "progress";
        let value = "0.5";
        let field_result = set_job_field(
            eid as i32,
            field.as_ptr(),
            field.len() as i32,
            value.as_ptr(),
            value.len() as i32,
        );
        if field_result != 0 {
            return 0;
        }

        // Verify progress was updated
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
        if !job_str.contains("\"progress\":0.5") {
            return 0;
        }

        // update_job: merge new fields
        let updates = r#"{"priority":3,"state":"going_to_site"}"#;
        let update_result = update_job(eid as i32, updates.as_ptr(), updates.len() as i32);
        if update_result != 0 {
            return 0;
        }

        // Verify fields were merged
        let mut buf2 = [0u8; 256];
        let written2 = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            buf2.as_mut_ptr(),
            buf2.len() as i32,
        );
        if written2 < 0 {
            return 0;
        }
        let job_str2 = core::str::from_utf8(&buf2[..written2 as usize]).unwrap_or("");
        if !job_str2.contains("\"priority\":3") || !job_str2.contains("\"going_to_site\"") {
            return 0;
        }

        // set_job_field for nonexistent job — should return -1
        let missing_result = set_job_field(
            99999,
            field.as_ptr(),
            field.len() as i32,
            value.as_ptr(),
            value.len() as i32,
        );
        if missing_result != -1 {
            return 0;
        }

        // update_job for nonexistent job — should return -1
        let bad_update = update_job(99999, updates.as_ptr(), updates.len() as i32);
        if bad_update != -1 {
            return 0;
        }

        1
    }
}
