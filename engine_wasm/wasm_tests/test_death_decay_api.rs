// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_death_decay_api() -> i32 {
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

    #[link(wasm_import_module = "death_decay")]
    unsafe extern "C" {
        fn process_deaths();
        fn process_decay();
    }

    unsafe {
        // Spawn an entity with Health at 0
        let eid = spawn_entity();
        let comp_name = "Health";
        let json_data = "{\"current\":0.0}";
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            json_data.as_ptr(),
            json_data.len() as i32,
        );

        // Process deaths — entity should become a Corpse with Decay
        process_deaths();

        // Verify Health component is gone
        let mut out_buf = [0u8; 128];
        let written = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written != -1 {
            return 0; // Health should be removed
        }

        // Verify Corpse exists
        let corpse_name = "Corpse";
        let written2 = get_component(
            eid,
            corpse_name.as_ptr(),
            corpse_name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written2 < 0 {
            return 0; // Corpse should exist
        }

        // Process decay — entity should be despawned after enough decays
        process_decay();
        process_decay();
        process_decay();
        process_decay();
        process_decay();

        // Entity should be despawned now
        let written3 = get_component(
            eid,
            corpse_name.as_ptr(),
            corpse_name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written3 != -1 {
            return 0;
        }

        1
    }
}
