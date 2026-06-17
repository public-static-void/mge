// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_region_api() -> i32 {
    #[link(wasm_import_module = "region")]
    unsafe extern "C" {
        fn get_entities_in_region(
            region_id_ptr: *const u8,
            region_id_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_entities_in_region_kind(
            kind_ptr: *const u8,
            kind_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_cells_in_region(
            region_id_ptr: *const u8,
            region_id_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_cells_in_region_kind(
            kind_ptr: *const u8,
            kind_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        // All 4 should return empty results on an empty world
        let rid = "zone_1";
        let mut buf1 = [0u8; 128];
        let n = get_entities_in_region(rid.as_ptr(), rid.len() as i32, buf1.as_mut_ptr(), buf1.len() as i32);
        if n < 0 { return 0; }

        let kind = "forest";
        let mut buf2 = [0u8; 128];
        let n2 = get_entities_in_region_kind(kind.as_ptr(), kind.len() as i32, buf2.as_mut_ptr(), buf2.len() as i32);
        if n2 < 0 { return 0; }

        let mut buf3 = [0u8; 128];
        let n3 = get_cells_in_region(rid.as_ptr(), rid.len() as i32, buf3.as_mut_ptr(), buf3.len() as i32);
        if n3 < 0 { return 0; }

        let mut buf4 = [0u8; 128];
        let n4 = get_cells_in_region_kind(kind.as_ptr(), kind.len() as i32, buf4.as_mut_ptr(), buf4.len() as i32);
        if n4 < 0 { return 0; }

        1
    }
}
