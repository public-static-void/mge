// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the Fog API (is_explored, get_explored_cells, reset_fog, get_visibility_state).

#[no_mangle]
pub extern "C" fn test_fog_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "wasm_fov")]
    unsafe extern "C" {
        fn is_explored(entity: u32, x: i32, y: i32, z: i32) -> i32;
        fn get_explored_cells(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn reset_fog(entity: u32);
        fn get_visibility_state(entity: u32, x: i32, y: i32, z: i32) -> i32;
        fn set_sight(entity: u32, range: i32);
    }

    unsafe {
        // Test 1: is_explored returns 0 for entity with no explored cells
        let eid = spawn_entity();
        let e = is_explored(eid, 0, 0, 0);
        if e != 0 {
            return 0;
        }

        // Test 2: get_explored_cells returns -1 for entity with no explored cells
        let eid2 = spawn_entity();
        let mut out_buf2 = [0u8; 128];
        let written2 = get_explored_cells(eid2, out_buf2.as_mut_ptr(), out_buf2.len() as i32);
        if written2 != -1 {
            return 0;
        }

        // Test 3: reset_fog does not crash on entity with no fog data
        let eid3 = spawn_entity();
        reset_fog(eid3);

        // Test 4: get_visibility_state returns 0 for entity with no data
        let eid4 = spawn_entity();
        let state = get_visibility_state(eid4, 0, 0, 0);
        if state != 0 {
            return 0;
        }

        // Test 5: set_sight works (bridge to wasm_fov namespace still works)
        let eid5 = spawn_entity();
        set_sight(eid5, 5);

        // Test 6: is_explored returns 0 even after setting sight (no tick, so no explored)
        let eid6 = spawn_entity();
        set_sight(eid6, 5);
        let e6 = is_explored(eid6, 0, 0, 0);
        if e6 != 0 {
            return 0;
        }

        // Test 7: reset_fog on entity with sight does not crash
        let eid7 = spawn_entity();
        set_sight(eid7, 3);
        reset_fog(eid7);

        1
    }
}
