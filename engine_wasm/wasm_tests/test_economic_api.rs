// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_economic_api() -> i32 {
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
        fn get_stockpile_resources(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_production_job(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_production_job_progress(entity: u32) -> f64;
        fn set_production_job_progress(entity: u32, value: f64);
        fn get_production_job_state(
            entity: u32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn set_production_job_state(
            entity: u32,
            value_ptr: *const u8,
            value_len: i32,
        );
        fn modify_stockpile_resource(
            entity: u32,
            kind_ptr: *const u8,
            kind_len: i32,
            delta: f64,
        );
        fn get_job_resource_reservations(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn reserve_job_resources();
        fn release_job_resource_reservations(entity: u32);
    }

    unsafe {
        let eid = spawn_entity();

        // Set ProductionJob component
        let job_json = "{\"job_type\":\"woodcutting\",\"progress\":0.0,\"state\":\"queued\"}";
        let comp_name = "ProductionJob";
        set_component(eid, comp_name.as_ptr(), comp_name.len() as i32, job_json.as_ptr(), job_json.len() as i32);

        // get_production_job should succeed
        let mut buf1 = [0u8; 4096];
        let w1 = get_production_job(eid, buf1.as_mut_ptr(), buf1.len() as i32);
        if w1 < 0 { return 0; }

        // get_production_job_progress should return 0.0
        let progress = get_production_job_progress(eid);
        // Simple f64 comparison
        if progress < -0.001 || progress > 0.001 { return 0; }

        // set_production_job_progress to 0.5
        set_production_job_progress(eid, 0.5);

        // get_production_job_progress should now return 0.5
        let progress2 = get_production_job_progress(eid);
        if progress2 < 0.499 || progress2 > 0.501 { return 0; }

        // get_production_job_state should return "queued"
        let mut buf2 = [0u8; 128];
        let w2 = get_production_job_state(eid, buf2.as_mut_ptr(), buf2.len() as i32);
        if w2 < 0 { return 0; }

        // set_production_job_state to "running"
        let state = "running";
        set_production_job_state(eid, state.as_ptr(), state.len() as i32);

        // Set Stockpile component
        let stock_json = "{\"resources\":{\"wood\":10.0,\"stone\":5.0}}";
        set_component(eid, "Stockpile".as_ptr(), 9, stock_json.as_ptr(), stock_json.len() as i32);

        // get_stockpile_resources should succeed
        let mut buf3 = [0u8; 4096];
        let w3 = get_stockpile_resources(eid, buf3.as_mut_ptr(), buf3.len() as i32);
        if w3 < 0 { return 0; }

        // modify_stockpile_resource
        let kind = "wood";
        modify_stockpile_resource(eid, kind.as_ptr(), kind.len() as i32, -3.0);

        // get_job_resource_reservations on entity without Job component should return -1
        let mut buf4 = [0u8; 128];
        let w4 = get_job_resource_reservations(eid, buf4.as_mut_ptr(), buf4.len() as i32);
        if w4 != -1 { return 0; }

        // ---- Test reserve/release job resources ----
        // Create a stockpile entity with resources
        let stockpile_eid = spawn_entity();
        let stock_json = "{\"resources\":{\"iron_ore\":100.0}}";
        set_component(stockpile_eid, "Stockpile".as_ptr(), "Stockpile".len() as i32, stock_json.as_ptr(), stock_json.len() as i32);

        // Create a job entity with resource_requirements
        let job_eid = spawn_entity();
        let job_json = "{\"state\":\"pending\",\"resource_requirements\":[{\"kind\":\"iron_ore\",\"amount\":10}]}";
        set_component(job_eid, "Job".as_ptr(), "Job".len() as i32, job_json.as_ptr(), job_json.len() as i32);

        // Run reservation
        reserve_job_resources();

        // Job should now have reserved_resources
        let mut buf5 = [0u8; 4096];
        let w5 = get_job_resource_reservations(job_eid, buf5.as_mut_ptr(), buf5.len() as i32);
        if w5 < 0 { return 0; }

        // Release reservation
        release_job_resource_reservations(job_eid);

        // Reservation should be cleared
        let mut buf6 = [0u8; 128];
        let w6 = get_job_resource_reservations(job_eid, buf6.as_mut_ptr(), buf6.len() as i32);
        if w6 != -1 { return 0; }

        1
    }
}
