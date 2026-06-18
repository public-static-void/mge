// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_system() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "component")]
    unsafe extern "C" {
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "job_system")]
    unsafe extern "C" {
        fn assign_job(
            entity_id: i32,
            job_type_ptr: *const u8,
            job_type_len: i32,
            fields_ptr: *const u8,
            fields_len: i32,
        ) -> i32;
        fn get_job_types(out_ptr: *mut u8, out_len: i32) -> i32;
        fn register_job_type(
            name_ptr: *const u8,
            name_len: i32,
            metadata_ptr: *const u8,
            metadata_len: i32,
        ) -> i32;
        fn get_job_type_metadata(
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        let eid = spawn_entity();

        // Assign a mining job
        let job_type = "mining";
        let fields = "{}";
        let result = assign_job(
            eid as i32,
            job_type.as_ptr(),
            job_type.len() as i32,
            fields.as_ptr(),
            fields.len() as i32,
        );
        if result != 0 {
            return 0;
        }

        // Verify Job component has state="pending" and progress=0.0
        let comp_name = "Job";
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
        if !job_str.contains("\"state\":\"pending\"") || !job_str.contains("\"progress\":0.0") {
            return 0;
        }

        // Register a job type — requires handler export to succeed
        let mining_name = "mining";
        let metadata = r#"{"category":"gathering"}"#;
        let reg_result = register_job_type(
            mining_name.as_ptr(),
            mining_name.len() as i32,
            metadata.as_ptr(),
            metadata.len() as i32,
        );
        if reg_result != 0 {
            return 0;
        }

        // get_job_types should include "mining"
        let mut type_buf = [0u8; 128];
        let type_written = get_job_types(type_buf.as_mut_ptr(), type_buf.len() as i32);
        if type_written <= 0 {
            return 0;
        }
        let types_str = core::str::from_utf8(&type_buf[..type_written as usize]).unwrap_or("");
        if !types_str.contains("mining") {
            return 0;
        }

        // get_job_type_metadata should return stored metadata
        let mut meta_buf = [0u8; 256];
        let meta_written = get_job_type_metadata(
            mining_name.as_ptr(),
            mining_name.len() as i32,
            meta_buf.as_mut_ptr(),
            meta_buf.len() as i32,
        );
        if meta_written < 0 {
            return 0;
        }
        let meta_str = core::str::from_utf8(&meta_buf[..meta_written as usize]).unwrap_or("");
        if !meta_str.contains("gathering") {
            return 0;
        }

        // get_job_type_metadata for unknown type should return -1
        let unknown = "unknown";
        let unknown_result = get_job_type_metadata(
            unknown.as_ptr(),
            unknown.len() as i32,
            meta_buf.as_mut_ptr(),
            meta_buf.len() as i32,
        );
        if unknown_result >= 0 {
            return 0;
        }

        1
    }
}

// Handler export required by register_job_type for successful registration
#[no_mangle]
pub extern "C" fn mge_job_handler_mining(
    _job_id: i32,
    _assigned_to: i32,
    _job_json_ptr: i32,
    _job_json_len: i32,
    _out_ptr: i32,
    _out_len: i32,
) -> i32 {
    -1
}
