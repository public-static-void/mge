// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the material API (get_properties, set_entity_material, get_entity_material, get_names).

#[unsafe(no_mangle)]
pub extern "C" fn test_material_api() -> i32 {
    #[link(wasm_import_module = "material")]
    unsafe extern "C" {
        fn get_properties(
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn set_entity_material(entity_id: i32, name_ptr: *const u8, name_len: i32) -> i32;
        fn get_entity_material(
            entity_id: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_names(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> i32;
    }

    unsafe {
        // Step 1: get_properties for known material "wood"
        let name = "wood";
        let mut out_buf = [0u8; 1024];
        let written = get_properties(
            name.as_ptr(),
            name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written <= 0 {
            return 0;
        }
        let json_str = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if !json_str.contains("density") || !json_str.contains("hardness") || !json_str.contains("flammability") {
            return 0;
        }

        // Step 2: get_properties for unknown material
        let name2 = "nonexistent";
        let result = get_properties(
            name2.as_ptr(),
            name2.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if result != -1 {
            return 0;
        }

        // Step 3: set_entity_material for known material
        let eid1 = spawn_entity();
        let mat_name = "iron";
        let set_result = set_entity_material(
            eid1,
            mat_name.as_ptr(),
            mat_name.len() as i32,
        );
        if set_result != 0 {
            return 0;
        }

        // Step 4: set_entity_material for unknown material
        let eid2 = spawn_entity();
        let bad_name = "nonexistent";
        let bad_result = set_entity_material(
            eid2,
            bad_name.as_ptr(),
            bad_name.len() as i32,
        );
        if bad_result != -1 {
            return 0;
        }

        // Step 5: get_entity_material for entity with material
        let mut mat_buf = [0u8; 256];
        let mat_written = get_entity_material(
            eid1,
            mat_buf.as_mut_ptr(),
            mat_buf.len() as i32,
        );
        if mat_written <= 0 {
            return 0;
        }
        let mat_json = core::str::from_utf8(&mat_buf[..mat_written as usize]).unwrap_or("");
        if !mat_json.contains("iron") {
            return 0;
        }

        // Step 6: get_entity_material for entity without material
        let no_mat_result = get_entity_material(
            eid2,
            mat_buf.as_mut_ptr(),
            mat_buf.len() as i32,
        );
        if no_mat_result != -1 {
            return 0;
        }

        // Step 7: get_names returns all material names
        let mut names_buf = [0u8; 1024];
        let names_written = get_names(names_buf.as_mut_ptr(), names_buf.len() as i32);
        if names_written <= 0 {
            return 0;
        }
        let names_str =
            core::str::from_utf8(&names_buf[..names_written as usize]).unwrap_or("");
        if !names_str.contains("wood")
            || !names_str.contains("iron")
            || !names_str.contains("steel")
            || !names_str.contains("stone")
        {
            return 0;
        }

        1
    }
}
