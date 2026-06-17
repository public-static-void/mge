#[no_mangle]
pub extern "C" fn test_reserve_job_resources_only() -> i32 {
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
    }

    #[link(wasm_import_module = "economic")]
    unsafe extern "C" {
        fn reserve_job_resources();
    }

    unsafe {
        let stockpile_eid = spawn_entity();
        let stock_json = "{\"resources\":{\"iron_ore\":100.0}}";
        set_component(stockpile_eid, "Stockpile".as_ptr(), 9, stock_json.as_ptr(), stock_json.len() as i32);

        let job_eid = spawn_entity();
        let job_json = "{\"state\":\"pending\",\"resource_requirements\":[{\"kind\":\"iron_ore\",\"amount\":10}]}";
        set_component(job_eid, "Job".as_ptr(), 3, job_json.as_ptr(), job_json.len() as i32);

        reserve_job_resources();
        1
    }
}

#[no_mangle]
pub extern "C" fn test_get_job_resource_reservations_before() -> i32 {
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
    }

    #[link(wasm_import_module = "economic")]
    unsafe extern "C" {
        fn get_job_resource_reservations(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        let job_eid = spawn_entity();
        let job_json = "{\"state\":\"pending\",\"resource_requirements\":[{\"kind\":\"iron_ore\",\"amount\":10}]}";
        set_component(job_eid, "Job".as_ptr(), 3, job_json.as_ptr(), job_json.len() as i32);

        let mut buf = [0u8; 128];
        let w = get_job_resource_reservations(job_eid, buf.as_mut_ptr(), buf.len() as i32);
        if w == -1 { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn test_reserve_then_query() -> i32 {
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

    #[link(wasm_import_module = "economic")]
    unsafe extern "C" {
        fn get_job_resource_reservations(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn reserve_job_resources();
    }

    unsafe {
        let stockpile_eid = spawn_entity();
        let stock_json = "{\"resources\":{\"iron_ore\":100.0}}";
        set_component(stockpile_eid, "Stockpile".as_ptr(), 9, stock_json.as_ptr(), stock_json.len() as i32);

        let job_eid = spawn_entity();
        let job_json = "{\"state\":\"pending\",\"resource_requirements\":[{\"kind\":\"iron_ore\",\"amount\":10}]}";
        set_component(job_eid, "Job".as_ptr(), 3, job_json.as_ptr(), job_json.len() as i32);

        // Read back Job component to verify it was stored correctly
        let mut buf_verify = [0u8; 4096];
        let w_verify = get_component(job_eid, "Job".as_ptr(), 3, buf_verify.as_mut_ptr(), buf_verify.len() as i32);
        if w_verify < 0 { return 0; }

        // Run reservation
        reserve_job_resources();

        // Check job resource reservations
        let mut buf1 = [0u8; 4096];
        let w1 = get_job_resource_reservations(job_eid, buf1.as_mut_ptr(), buf1.len() as i32);
        if w1 < 0 { return 0; }

        1
    }
}

#[no_mangle]
pub extern "C" fn test_reserve_and_release() -> i32 {
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
    }

    #[link(wasm_import_module = "economic")]
    unsafe extern "C" {
        fn get_job_resource_reservations(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn reserve_job_resources();
        fn release_job_resource_reservations(entity: u32);
    }

    unsafe {
        let stockpile_eid = spawn_entity();
        let stock_json = "{\"resources\":{\"iron_ore\":100.0}}";
        set_component(stockpile_eid, "Stockpile".as_ptr(), 9, stock_json.as_ptr(), stock_json.len() as i32);

        let job_eid = spawn_entity();
        let job_json = "{\"state\":\"pending\",\"resource_requirements\":[{\"kind\":\"iron_ore\",\"amount\":10}]}";
        set_component(job_eid, "Job".as_ptr(), 3, job_json.as_ptr(), job_json.len() as i32);

        // Reserve
        reserve_job_resources();

        // Verify reservation exists
        let mut buf1 = [0u8; 4096];
        let w1 = get_job_resource_reservations(job_eid, buf1.as_mut_ptr(), buf1.len() as i32);
        if w1 < 0 { return 0; }

        // Release
        release_job_resource_reservations(job_eid);

        // Verify reservation cleared
        let mut buf2 = [0u8; 128];
        let w2 = get_job_resource_reservations(job_eid, buf2.as_mut_ptr(), buf2.len() as i32);
        if w2 != -1 { return 0; }

        1
    }
}
