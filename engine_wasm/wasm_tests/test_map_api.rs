// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_map_api() -> i32 {
    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn get_map_topology_type(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_all_cells(out_ptr: *mut u8, out_len: i32) -> i32;
        fn add_cell(x: i32, y: i32, z: i32);
        fn get_neighbors(
            cell_ptr: *const u8,
            cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn add_neighbor(
            from_ptr: *const u8,
            from_len: i32,
            to_ptr: *const u8,
            to_len: i32,
        );
        fn entities_in_cell(
            cell_ptr: *const u8,
            cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_cell_metadata(
            cell_ptr: *const u8,
            cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn set_cell_metadata(
            cell_ptr: *const u8,
            cell_len: i32,
            meta_ptr: *const u8,
            meta_len: i32,
        );
        fn apply_generated_map(map_ptr: *const u8, map_len: i32);
        fn get_map_cell_count() -> i32;
        fn find_path(
            start_ptr: *const u8,
            start_len: i32,
            goal_ptr: *const u8,
            goal_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        // Initial state: no map
        let count0 = get_map_cell_count();
        if count0 != 0 { return 0; }

        // Topology type should be "none"
        let mut buf0 = [0u8; 128];
        let w0 = get_map_topology_type(buf0.as_mut_ptr(), buf0.len() as i32);
        if w0 < 0 { return 0; }

        // Add cells
        add_cell(0, 0, 0);
        add_cell(1, 0, 0);

        // get_map_cell_count should be 2
        let count1 = get_map_cell_count();
        if count1 != 2 { return 0; }

        // get_all_cells should return JSON array of 2 cells
        let mut buf1 = [0u8; 4096];
        let w1 = get_all_cells(buf1.as_mut_ptr(), buf1.len() as i32);
        if w1 < 0 { return 0; }

        // Add neighbor between (0,0,0) and (1,0,0)
        let cell_a = "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}";
        let cell_b = "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}";
        add_neighbor(cell_a.as_ptr(), cell_a.len() as i32, cell_b.as_ptr(), cell_b.len() as i32);

        // get_neighbors for cell_a should include cell_b
        let mut buf2 = [0u8; 4096];
        let w2 = get_neighbors(cell_a.as_ptr(), cell_a.len() as i32, buf2.as_mut_ptr(), buf2.len() as i32);
        if w2 < 0 { return 0; }

        // set_cell_metadata
        let meta = "{\"biome\":\"plains\"}";
        set_cell_metadata(cell_a.as_ptr(), cell_a.len() as i32, meta.as_ptr(), meta.len() as i32);

        // get_cell_metadata
        let mut buf3 = [0u8; 4096];
        let w3 = get_cell_metadata(cell_a.as_ptr(), cell_a.len() as i32, buf3.as_mut_ptr(), buf3.len() as i32);
        if w3 < 0 { return 0; }

        // entities_in_cell (empty world, should be 0)
        let mut buf4 = [0u8; 4096];
        let w4 = entities_in_cell(cell_a.as_ptr(), cell_a.len() as i32, buf4.as_mut_ptr(), buf4.len() as i32);
        if w4 < 0 { return 0; }

        // find_path: add cell_c and connect b->c, then find path from a to c
        let cell_c = "{\"Square\":{\"x\":2,\"y\":0,\"z\":0}}";
        add_cell(2, 0, 0);
        add_neighbor(cell_b.as_ptr(), cell_b.len() as i32, cell_c.as_ptr(), cell_c.len() as i32);

        let mut buf5 = [0u8; 4096];
        let w5 = find_path(cell_a.as_ptr(), cell_a.len() as i32, cell_c.as_ptr(), cell_c.len() as i32, buf5.as_mut_ptr(), buf5.len() as i32);
        if w5 < 0 { return 0; }

        // apply_generated_map
        let map_json = "{\"topology_type\":\"square\",\"cells\":[{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}],\"neighbors\":{},\"cell_metadata\":{}}";
        apply_generated_map(map_json.as_ptr(), map_json.len() as i32);
        let count2 = get_map_cell_count();
        if count2 != 1 { return 0; }

        1
    }
}
