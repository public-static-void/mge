// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the FOV API (set_sight, get_sight, get_visible_cells, is_visible).

#[no_mangle]
pub extern "C" fn test_fov_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "wasm_fov")]
    unsafe extern "C" {
        fn set_sight(entity: u32, range: i32);
        fn get_sight(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_visible_cells(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn is_visible(entity: u32, x: i32, y: i32, z: i32) -> i32;
        fn set_fov_algorithm(name_ptr: *const u8, name_len: i32);
    }

    unsafe {
        // Test 1: set_sight/get_sight round-trip
        let eid = spawn_entity();
        set_sight(eid, 5);
        let mut out_buf = [0u8; 128];
        let written = get_sight(eid, out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let json_str = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if !json_str.contains("\"range\":5") {
            return 0;
        }

        // Test 2: get_sight returns -1 when entity has no Sight component
        let eid2 = spawn_entity();
        let mut out_buf2 = [0u8; 128];
        let written2 = get_sight(eid2, out_buf2.as_mut_ptr(), out_buf2.len() as i32);
        if written2 != -1 {
            return 0;
        }

        // Test 3: get_visible_cells returns -1 when no visible cells computed
        let eid3 = spawn_entity();
        set_sight(eid3, 3);
        let mut out_buf3 = [0u8; 128];
        let written3 = get_visible_cells(eid3, out_buf3.as_mut_ptr(), out_buf3.len() as i32);
        if written3 != -1 {
            return 0;
        }

        // Test 4: is_visible returns 0 when no visible cells computed
        let eid4 = spawn_entity();
        set_sight(eid4, 3);
        let vis = is_visible(eid4, 0, 0, 0);
        if vis != 0 {
            return 0;
        }

        // Test 5: is_visible on entity without Sight returns 0
        let eid5 = spawn_entity();
        let vis2 = is_visible(eid5, 0, 0, 0);
        if vis2 != 0 {
            return 0;
        }

        // Test 6: set_sight with different ranges
        let eid6 = spawn_entity();
        set_sight(eid6, 10);
        let mut out_buf6 = [0u8; 128];
        let written6 = get_sight(eid6, out_buf6.as_mut_ptr(), out_buf6.len() as i32);
        if written6 <= 0 {
            return 0;
        }
        let json_str6 = core::str::from_utf8(&out_buf6[..written6 as usize]).unwrap_or("");
        if !json_str6.contains("\"range\":10") {
            return 0;
        }

        // Test 7: set_fov_algorithm with known name succeeds
        set_fov_algorithm(
            "recursive_shadowcasting\0".as_ptr(),
            "recursive_shadowcasting\0".len() as i32 - 1,
        );

        1
    }
}
