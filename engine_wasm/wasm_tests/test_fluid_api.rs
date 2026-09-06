// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the fluid API (get_fluid) and fluid simulation across ticks.

#[no_mangle]
pub extern "C" fn test_fluid_api() -> i32 {
    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn add_cell(x: i32, y: i32, z: i32);
        fn add_neighbor(
            from_ptr: *const u8,
            from_len: i32,
            to_ptr: *const u8,
            to_len: i32,
        );
        fn set_cell_metadata(
            cell_ptr: *const u8,
            cell_len: i32,
            meta_ptr: *const u8,
            meta_len: i32,
        );
        fn get_fluid(
            cell_ptr: *const u8,
            cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
    }

    unsafe {
        // Build a 3x3 plane with all neighbors connected.
        for x in 0..3 {
            for y in 0..3 {
                add_cell(x, y, 0);
            }
        }
        for x in 0..3 {
            for y in 0..3 {
                for dx in -1..2 {
                    for dy in -1..2 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx <= 2 && ny >= 0 && ny <= 2 {
                            let from = format!(
                                "{{\"Square\":{{\"x\":{},\"y\":{},\"z\":0}}}}",
                                x, y
                            );
                            let to = format!(
                                "{{\"Square\":{{\"x\":{},\"y\":{},\"z\":0}}}}",
                                nx, ny
                            );
                            add_neighbor(
                                from.as_ptr(),
                                from.len() as i32,
                                to.as_ptr(),
                                to.len() as i32,
                            );
                        }
                    }
                }
            }
        }

        // Test 1: get_fluid returns -1 for a cell with no fluid metadata.
        let cell_a = "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}";
        let mut buf0 = [0u8; 128];
        let w0 = get_fluid(
            cell_a.as_ptr(),
            cell_a.len() as i32,
            buf0.as_mut_ptr(),
            buf0.len() as i32,
        );
        if w0 != -1 {
            return 0;
        }

        // Test 2: set_cell_metadata with fluid, then get_fluid round-trip.
        let meta = "{\"fluid\":{\"type\":\"water\",\"level\":5}}";
        set_cell_metadata(
            cell_a.as_ptr(),
            cell_a.len() as i32,
            meta.as_ptr(),
            meta.len() as i32,
        );
        let mut buf1 = [0u8; 128];
        let w1 = get_fluid(
            cell_a.as_ptr(),
            cell_a.len() as i32,
            buf1.as_mut_ptr(),
            buf1.len() as i32,
        );
        if w1 <= 0 {
            return 0;
        }
        let fluid_json = core::str::from_utf8(&buf1[..w1 as usize]).unwrap_or("");
        if !fluid_json.contains("\"type\":\"water\"") || !fluid_json.contains("\"level\":5") {
            return 0;
        }

        // Test 3: fluid spreads to a neighbor after tick.
        // Source cell (0,0) has level 8; neighbor (1,0) should gain fluid.
        let cell_b = "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}";
        let mut buf2 = [0u8; 128];
        let w2 = get_fluid(
            cell_b.as_ptr(),
            cell_b.len() as i32,
            buf2.as_mut_ptr(),
            buf2.len() as i32,
        );
        if w2 != -1 {
            return 0;
        }
        tick();
        let mut buf3 = [0u8; 128];
        let w3 = get_fluid(
            cell_b.as_ptr(),
            cell_b.len() as i32,
            buf3.as_mut_ptr(),
            buf3.len() as i32,
        );
        if w3 <= 0 {
            return 0;
        }
        let neighbor_json = core::str::from_utf8(&buf3[..w3 as usize]).unwrap_or("");
        if !neighbor_json.contains("\"level\":") {
            return 0;
        }

        // Test 4: explicit taxonomy fields round-trip through get_fluid.
        let cell_c = "{\"Square\":{\"x\":2,\"y\":2,\"z\":0}}";
        let meta_tax = "{\"fluid\":{\"type\":\"water\",\"level\":4,\"water_type\":\"salt\",\"depth\":\"deep\",\"flow_state\":\"stale\"}}";
        set_cell_metadata(
            cell_c.as_ptr(),
            cell_c.len() as i32,
            meta_tax.as_ptr(),
            meta_tax.len() as i32,
        );
        let mut buf4 = [0u8; 256];
        let w4 = get_fluid(
            cell_c.as_ptr(),
            cell_c.len() as i32,
            buf4.as_mut_ptr(),
            buf4.len() as i32,
        );
        if w4 <= 0 {
            return 0;
        }
        let tax_json = core::str::from_utf8(&buf4[..w4 as usize]).unwrap_or("");
        if !tax_json.contains("\"water_type\":\"salt\"")
            || !tax_json.contains("\"depth\":\"deep\"")
            || !tax_json.contains("\"flow_state\":\"stale\"")
        {
            return 0;
        }

        // Test 5: a fresh water cell gains default taxonomy after a tick.
        let cell_d = "{\"Square\":{\"x\":0,\"y\":1,\"z\":0}}";
        let meta_fresh = "{\"fluid\":{\"type\":\"water\",\"level\":4}}";
        set_cell_metadata(
            cell_d.as_ptr(),
            cell_d.len() as i32,
            meta_fresh.as_ptr(),
            meta_fresh.len() as i32,
        );
        tick();
        let mut buf5 = [0u8; 256];
        let w5 = get_fluid(
            cell_d.as_ptr(),
            cell_d.len() as i32,
            buf5.as_mut_ptr(),
            buf5.len() as i32,
        );
        if w5 <= 0 {
            return 0;
        }
        let fresh_json = core::str::from_utf8(&buf5[..w5 as usize]).unwrap_or("");
        if !fresh_json.contains("\"water_type\":\"fresh\"")
            || !fresh_json.contains("\"depth\":\"shallow\"")
            || !fresh_json.contains("\"flow_state\":\"flowing\"")
        {
            return 0;
        }

        1
    }
}
