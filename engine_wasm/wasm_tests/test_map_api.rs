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
        fn entities_in_zlevel(z: i32, out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
        fn move_entity_3d(entity_id: u32, dx: f32, dy: f32, dz: f32);
        fn count_entities_with_type(type_ptr: *const u8, type_len: i32) -> i32;
    }

    #[link(wasm_import_module = "camera")]
    unsafe extern "C" {
        fn set_camera(x: i32, y: i32, z: i32);
        fn get_camera(out_ptr: *mut u8, out_len: i32) -> i32;
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

        // z-level: spawn an entity with a flat Position at z=5
        let eid = spawn_entity();
        let comp_name = "Position";
        let pos_json = "{\"x\":1.0,\"y\":2.0,\"z\":5.0}";
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            pos_json.as_ptr(),
            pos_json.len() as i32,
        );

        // entities_in_zlevel(5) should return exactly this entity
        let mut buf6 = [0u8; 4096];
        let w6 = entities_in_zlevel(5, buf6.as_mut_ptr(), buf6.len() as i32);
        if w6 != 1 { return 0; }
        let ids6 = core::slice::from_raw_parts(buf6.as_ptr() as *const u32, w6 as usize);
        if ids6[0] != eid { return 0; }

        // entities_in_zlevel(0) should not contain it
        let mut buf7 = [0u8; 4096];
        let w7 = entities_in_zlevel(0, buf7.as_mut_ptr(), buf7.len() as i32);
        if w7 != 0 { return 0; }

        // move_entity_3d shifts z from 5 to 3
        move_entity_3d(eid, 0.0, 0.0, -2.0);
        let mut buf8 = [0u8; 4096];
        let w8 = entities_in_zlevel(3, buf8.as_mut_ptr(), buf8.len() as i32);
        if w8 != 1 { return 0; }
        let ids8 = core::slice::from_raw_parts(buf8.as_ptr() as *const u32, w8 as usize);
        if ids8[0] != eid { return 0; }
        let mut buf9 = [0u8; 4096];
        let w9 = entities_in_zlevel(5, buf9.as_mut_ptr(), buf9.len() as i32);
        if w9 != 0 { return 0; }

        // ---- Hex topology branch (R003 add_cell + R010 7-function parity audit) ----
        // apply_generated_map with all four WasmMap fields (WasmMap has no serde defaults)
        let hex_map = "{\"topology_type\":\"hex\",\"cells\":[{\"Hex\":{\"q\":0,\"r\":0,\"z\":0}}],\"neighbors\":{},\"cell_metadata\":{}}";
        apply_generated_map(hex_map.as_ptr(), hex_map.len() as i32);

        // Topology is "hex" after apply_generated_map
        let mut buf_hex_topo = [0u8; 128];
        let w_hex_topo = get_map_topology_type(buf_hex_topo.as_mut_ptr(), buf_hex_topo.len() as i32);
        if w_hex_topo < 0 { return 0; }
        let topo = core::str::from_utf8(&buf_hex_topo[..w_hex_topo as usize]).unwrap_or("");
        if topo != "hex" { return 0; }

        // add_cell on a hex map builds CellKey::Hex, never Square (AC006)
        add_cell(1, 2, 3);
        if get_map_cell_count() != 2 { return 0; }
        let w_hex_topo1 = get_map_topology_type(buf_hex_topo.as_mut_ptr(), buf_hex_topo.len() as i32);
        if w_hex_topo1 < 0 { return 0; }
        let topo1 = core::str::from_utf8(&buf_hex_topo[..w_hex_topo1 as usize]).unwrap_or("");
        if topo1 != "hex" { return 0; }

        let mut buf_hex_cells = [0u8; 4096];
        let w_hex_cells = get_all_cells(buf_hex_cells.as_mut_ptr(), buf_hex_cells.len() as i32);
        if w_hex_cells < 0 { return 0; }
        let cells = core::str::from_utf8(&buf_hex_cells[..w_hex_cells as usize]).unwrap_or("");
        // The cell added via add_cell is present as Hex
        if !cells.contains("{\"Hex\":{\"q\":1,\"r\":2,\"z\":3}}") { return 0; }
        // Zero Square pollution (AC007)
        if cells.contains("Square") { return 0; }

        // add_neighbor with Hex JSON keys yields hex neighbors, topology stays hex (AC020)
        let hex_a = "{\"Hex\":{\"q\":0,\"r\":0,\"z\":0}}";
        let hex_b = "{\"Hex\":{\"q\":1,\"r\":0,\"z\":0}}";
        add_neighbor(hex_a.as_ptr(), hex_a.len() as i32, hex_b.as_ptr(), hex_b.len() as i32);
        let w_hex_topo2 = get_map_topology_type(buf_hex_topo.as_mut_ptr(), buf_hex_topo.len() as i32);
        if w_hex_topo2 < 0 { return 0; }
        let topo2 = core::str::from_utf8(&buf_hex_topo[..w_hex_topo2 as usize]).unwrap_or("");
        if topo2 != "hex" { return 0; }
        let mut buf_hex_neighbors = [0u8; 4096];
        let w_hex_neighbors = get_neighbors(hex_a.as_ptr(), hex_a.len() as i32, buf_hex_neighbors.as_mut_ptr(), buf_hex_neighbors.len() as i32);
        if w_hex_neighbors < 0 { return 0; }
        let neighbors = core::str::from_utf8(&buf_hex_neighbors[..w_hex_neighbors as usize]).unwrap_or("");
        if !neighbors.contains("{\"Hex\":{\"q\":1,\"r\":0,\"z\":0}}") { return 0; }

        // get_all_cells after add_cell + add_neighbor still has zero Square entries (AC020)
        let mut buf_hex_cells2 = [0u8; 4096];
        let w_hex_cells2 = get_all_cells(buf_hex_cells2.as_mut_ptr(), buf_hex_cells2.len() as i32);
        if w_hex_cells2 < 0 { return 0; }
        let cells2 = core::str::from_utf8(&buf_hex_cells2[..w_hex_cells2 as usize]).unwrap_or("");
        if cells2.contains("Square") { return 0; }

        // entities_in_cell matches a flat Hex Position (AC021)
        let hex_123 = "{\"Hex\":{\"q\":1,\"r\":2,\"z\":3}}";
        let hex_eid = spawn_entity();
        let hex_pos = "{\"q\":1.0,\"r\":2.0,\"z\":3.0}";
        set_component(
            hex_eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            hex_pos.as_ptr(),
            hex_pos.len() as i32,
        );
        let mut buf_hex_entities = [0u8; 4096];
        let w_hex_entities = entities_in_cell(hex_123.as_ptr(), hex_123.len() as i32, buf_hex_entities.as_mut_ptr(), buf_hex_entities.len() as i32);
        if w_hex_entities != 1 { return 0; }
        let ids_hex = core::slice::from_raw_parts(buf_hex_entities.as_ptr() as *const u32, w_hex_entities as usize);
        if ids_hex[0] != hex_eid { return 0; }

        // find_path between connected hex cells returns a non-empty path (AC022)
        let mut buf_hex_path = [0u8; 4096];
        let w_hex_path = find_path(hex_a.as_ptr(), hex_a.len() as i32, hex_b.as_ptr(), hex_b.len() as i32, buf_hex_path.as_mut_ptr(), buf_hex_path.len() as i32);
        if w_hex_path < 0 { return 0; }
        let path_json = core::str::from_utf8(&buf_hex_path[..w_hex_path as usize]).unwrap_or("");
        if !path_json.contains("path") { return 0; }

        // move_entity_3d shifts q/r/z on a Hex flat position (AC023): (1,2,3) + (1,-1,-1) = (2,1,2)
        move_entity_3d(hex_eid, 1.0, -1.0, -1.0);
        let mut buf_hex_z = [0u8; 4096];
        let w_hex_z = entities_in_zlevel(2, buf_hex_z.as_mut_ptr(), buf_hex_z.len() as i32);
        if w_hex_z != 1 { return 0; }
        let ids_hex_z = core::slice::from_raw_parts(buf_hex_z.as_ptr() as *const u32, w_hex_z as usize);
        if ids_hex_z[0] != hex_eid { return 0; }
        let hex_211 = "{\"Hex\":{\"q\":2,\"r\":1,\"z\":2}}";
        let mut buf_hex_entities2 = [0u8; 4096];
        let w_hex_entities2 = entities_in_cell(hex_211.as_ptr(), hex_211.len() as i32, buf_hex_entities2.as_mut_ptr(), buf_hex_entities2.len() as i32);
        if w_hex_entities2 != 1 { return 0; }
        let ids_hex_2 = core::slice::from_raw_parts(buf_hex_entities2.as_ptr() as *const u32, w_hex_entities2 as usize);
        if ids_hex_2[0] != hex_eid { return 0; }

        // count_entities_with_type is type-based, topology-agnostic (AC023)
        let warrior_name = "Warrior";
        let empty_json = "{}";
        set_component(
            hex_eid,
            warrior_name.as_ptr(),
            warrior_name.len() as i32,
            empty_json.as_ptr(),
            empty_json.len() as i32,
        );
        let warrior_count = count_entities_with_type(warrior_name.as_ptr(), warrior_name.len() as i32);
        if warrior_count != 1 { return 0; }

        // Camera topology parity: identical {x,y,z} round-trip on a hex map (AC019)
        set_camera(3, 7, 2);
        let mut buf_cam = [0u8; 128];
        let w_cam = get_camera(buf_cam.as_mut_ptr(), buf_cam.len() as i32);
        if w_cam <= 0 { return 0; }
        let cam_json = core::str::from_utf8(&buf_cam[..w_cam as usize]).unwrap_or("");
        if cam_json != "{\"x\":3,\"y\":7,\"z\":2}" { return 0; }

        1
    }
}
