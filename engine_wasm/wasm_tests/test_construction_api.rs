// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_construction_api() -> i32 {
    #[link(wasm_import_module = "mode")]
    unsafe extern "C" {
        fn set_mode(mode_ptr: *const u8, mode_len: i32);
    }

    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn add_cell(x: i32, y: i32, z: i32);
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

    #[link(wasm_import_module = "construction")]
    unsafe extern "C" {
        fn place_blueprint(
            building_type_ptr: *const u8,
            building_type_len: i32,
            cell_ptr: *const u8,
            cell_len: i32,
            materials_ptr: *const u8,
            materials_len: i32,
            required_work: i64,
        ) -> i32;
        fn get_construction_state(site_id: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn cancel_construction(site_id: u32) -> i32;
        fn demolish_building(building_id: u32) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
    }

    unsafe fn place(
        building_type: &str,
        cell: &str,
        materials: &str,
        required_work: i64,
    ) -> i32 {
        place_blueprint(
            building_type.as_ptr(),
            building_type.len() as i32,
            cell.as_ptr(),
            cell.len() as i32,
            materials.as_ptr(),
            materials.len() as i32,
            required_work,
        )
    }

    unsafe fn read_state(site_id: u32, buf: &mut [u8]) -> Option<&str> {
        let written = get_construction_state(site_id, buf.as_mut_ptr(), buf.len() as i32);
        if written < 0 {
            return None;
        }
        core::str::from_utf8(&buf[..written as usize]).ok()
    }

    unsafe {
        let colony = "colony";
        set_mode(colony.as_ptr(), colony.len() as i32);
        add_cell(0, 0, 0);
        add_cell(1, 0, 0);
        add_cell(2, 0, 0);

        let hut = "hut";
        let cell = r#"{"Square":{"x":2,"y":0,"z":0}}"#;
        let materials = r#"[{"kind":"wood","amount":2}]"#;
        let site = place(hut, cell, materials, 2);
        if site < 0 {
            return 0;
        }

        // State query reports the pending site.
        let mut buf = [0u8; 512];
        match read_state(site as u32, &mut buf) {
            Some(state) => {
                if !state.contains("\"pending\"") || !state.contains("\"hut\"") {
                    return 0;
                }
            }
            None => return 0,
        }

        // Validation rejects bad blueprints.
        let bad_cell = r#"{"Square":{"x":9,"y":9,"z":0}}"#;
        if place(hut, bad_cell, materials, 1) != -1 {
            return 0;
        }
        if place(hut, cell, materials, 1) != -1 {
            return 0;
        }
        let empty = "[]";
        if place(hut, cell, empty, 1) != -1 {
            return 0;
        }
        if place(hut, cell, materials, 0) != -1 {
            return 0;
        }
        let hex_cell = r#"{"Hex":{"q":0,"r":0,"z":0}}"#;
        if place(hut, hex_cell, materials, 1) != -1 {
            return 0;
        }

        // Ticks drive progress to completion without a delivery sim.
        for _ in 0..5 {
            tick();
        }
        let mut done_buf = [0u8; 512];
        match read_state(site as u32, &mut done_buf) {
            Some(state) => {
                if !state.contains("\"complete\"") {
                    return 0;
                }
            }
            None => return 0,
        }

        // Completed site carries a Building component.
        let building = "Building";
        let mut comp_buf = [0u8; 1024];
        let written = get_component(
            site as u32,
            building.as_ptr(),
            building.len() as i32,
            comp_buf.as_mut_ptr(),
            comp_buf.len() as i32,
        );
        if written < 0 {
            return 0;
        }
        let building_str = core::str::from_utf8(&comp_buf[..written as usize]).unwrap_or("");
        if !building_str.contains("\"hut\"") {
            return 0;
        }

        // Cancel-then-query on a second site; cancel reports success.
        let shed = "shed";
        let shed_cell = r#"{"Square":{"x":1,"y":0,"z":0}}"#;
        let shed_site = place(shed, shed_cell, materials, 5);
        if shed_site < 0 {
            return 0;
        }
        if cancel_construction(shed_site as u32) != 0 {
            return 0;
        }
        let mut gone_buf = [0u8; 64];
        if read_state(shed_site as u32, &mut gone_buf).is_some() {
            return 0;
        }

        // Cancel after completion is rejected; demolish removes with no refund.
        if cancel_construction(site as u32) != -1 {
            return 0;
        }
        if demolish_building(site as u32) != 0 {
            return 0;
        }
        if demolish_building(site as u32) != -1 {
            return 0;
        }
        if demolish_building(99999) != -1 {
            return 0;
        }

        // Mode gate: non-colony placement fails.
        let roguelike = "roguelike";
        set_mode(roguelike.as_ptr(), roguelike.len() as i32);
        if place(hut, cell, materials, 1) != -1 {
            return 0;
        }

        1
    }
}
