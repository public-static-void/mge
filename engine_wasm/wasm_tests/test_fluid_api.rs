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

        1
    }
}
