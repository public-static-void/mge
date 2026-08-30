// This file is compiled to WASM and loaded by the Rust host test harness.
// Exercises the 9 multi-scale map navigation functions (AC001-AC012, AC019)
// plus the pre-existing wasm_map/camera surfaces used to verify transitions.

fn eq(bytes: &[u8], expected: &str) -> bool {
    bytes.len() == expected.len() && bytes == expected.as_bytes()
}

fn contains(bytes: &[u8], needle: &str) -> bool {
    if needle.len() > bytes.len() {
        return false;
    }
    bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
}

fn count_quotes(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'"').count()
}

#[no_mangle]
pub extern "C" fn test_multiscale_map() -> i32 {
    #[link(wasm_import_module = "multiscale_map")]
    unsafe extern "C" {
        fn register_map(
            name_ptr: *const u8,
            name_len: i32,
            map_ptr: *const u8,
            map_len: i32,
        ) -> i32;
        fn set_active_map(name_ptr: *const u8, name_len: i32) -> i32;
        fn get_map_names(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_active_map_name(out_ptr: *mut u8, out_len: i32) -> i32;
        fn link_maps(
            source_map_ptr: *const u8,
            source_map_len: i32,
            source_cell_ptr: *const u8,
            source_cell_len: i32,
            target_map_ptr: *const u8,
            target_map_len: i32,
            target_cell_ptr: *const u8,
            target_cell_len: i32,
        ) -> i32;
        fn enter_map(
            name_ptr: *const u8,
            name_len: i32,
            entry_cell_ptr: *const u8,
            entry_cell_len: i32,
        ) -> i32;
        fn exit_map() -> i32;
        fn map_cell(
            source_map_ptr: *const u8,
            source_map_len: i32,
            source_cell_ptr: *const u8,
            source_cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn unmap_cell(
            target_map_ptr: *const u8,
            target_map_len: i32,
            target_cell_ptr: *const u8,
            target_cell_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn get_map_topology_type(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "camera")]
    unsafe extern "C" {
        fn get_camera(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // AC005: register a square "overmap" map; topology preserved as square.
        let overmap = "{\"topology\":\"square\",\"cells\":[{\"x\":0,\"y\":0,\"z\":0},{\"x\":1,\"y\":0,\"z\":0}]}";
        if register_map(b"overmap".as_ptr(), 7, overmap.as_ptr(), overmap.len() as i32) != 0 {
            return 0;
        }
        if set_active_map(b"overmap".as_ptr(), 7) != 0 {
            return 0;
        }
        let mut buf = [0u8; 128];
        let w = get_map_topology_type(buf.as_mut_ptr(), buf.len() as i32);
        if w <= 0 || !eq(&buf[..w as usize], "square") {
            return 0;
        }

        // AC006: register a province "strategic" map; topology preserved as province.
        let strategic = "{\"topology\":\"province\",\"cells\":[{\"id\":\"prov_a\"},{\"id\":\"prov_b\"}]}";
        if register_map(b"strategic".as_ptr(), 9, strategic.as_ptr(), strategic.len() as i32) != 0 {
            return 0;
        }
        if set_active_map(b"strategic".as_ptr(), 9) != 0 {
            return 0;
        }
        let w = get_map_topology_type(buf.as_mut_ptr(), buf.len() as i32);
        if w <= 0 || !eq(&buf[..w as usize], "province") {
            return 0;
        }

        // AC001: register a third map; get_map_names contains all three.
        let field = "{\"topology\":\"square\",\"cells\":[{\"x\":0,\"y\":0,\"z\":0},{\"x\":1,\"y\":0,\"z\":0},{\"x\":2,\"y\":1,\"z\":0}]}";
        if register_map(b"field".as_ptr(), 5, field.as_ptr(), field.len() as i32) != 0 {
            return 0;
        }
        let mut names_buf = [0u8; 512];
        let wn = get_map_names(names_buf.as_mut_ptr(), names_buf.len() as i32);
        if wn <= 0 {
            return 0;
        }
        let names = &names_buf[..wn as usize];
        if !contains(names, "\"overmap\"") || !contains(names, "\"strategic\"") || !contains(names, "\"field\"") {
            return 0;
        }
        if count_quotes(names) != 6 {
            return 0;
        }

        // AC002: set_active_map both ways; topology reflects the active map.
        if set_active_map(b"field".as_ptr(), 5) != 0 {
            return 0;
        }
        let w = get_map_topology_type(buf.as_mut_ptr(), buf.len() as i32);
        if w <= 0 || !eq(&buf[..w as usize], "square") {
            return 0;
        }
        if set_active_map(b"overmap".as_ptr(), 7) != 0 {
            return 0;
        }
        let w = get_map_topology_type(buf.as_mut_ptr(), buf.len() as i32);
        if w <= 0 || !eq(&buf[..w as usize], "square") {
            return 0;
        }

        // AC003: set_active_map on an unknown name returns -1.
        if set_active_map(b"nonexistent".as_ptr(), 11) != -1 {
            return 0;
        }

        // AC004: get_active_map_name is the last successfully selected map.
        let mut active_buf = [0u8; 128];
        let wa = get_active_map_name(active_buf.as_mut_ptr(), active_buf.len() as i32);
        if wa <= 0 || !eq(&active_buf[..wa as usize], "overmap") {
            return 0;
        }
        let wn = get_map_names(names_buf.as_mut_ptr(), names_buf.len() as i32);
        if wn <= 0 || count_quotes(&names_buf[..wn as usize]) != 6 {
            return 0;
        }

        // AC010: link strategic prov_a -> field (2,1,0); round-trip both ways.
        let prov_a = "{\"Province\":{\"id\":\"prov_a\"}}";
        let field_210 = "{\"Square\":{\"x\":2,\"y\":1,\"z\":0}}";
        if link_maps(
            b"strategic".as_ptr(), 9,
            prov_a.as_ptr(), prov_a.len() as i32,
            b"field".as_ptr(), 5,
            field_210.as_ptr(), field_210.len() as i32,
        ) != 0 {
            return 0;
        }
        let mut cell_buf = [0u8; 256];
        let wc = map_cell(
            b"strategic".as_ptr(), 9,
            prov_a.as_ptr(), prov_a.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        );
        if wc <= 0 || !eq(&cell_buf[..wc as usize], field_210) {
            return 0;
        }
        let wc = unmap_cell(
            b"field".as_ptr(), 5,
            field_210.as_ptr(), field_210.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        );
        if wc <= 0 || !eq(&cell_buf[..wc as usize], prov_a) {
            return 0;
        }

        // AC011: unlinked cells return -1 (WASM equivalent of nil/None).
        let field_999 = "{\"Square\":{\"x\":9,\"y\":9,\"z\":9}}";
        if map_cell(
            b"field".as_ptr(), 5,
            field_999.as_ptr(), field_999.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        ) != -1 {
            return 0;
        }
        let overmap_000 = "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}";
        if unmap_cell(
            b"overmap".as_ptr(), 7,
            overmap_000.as_ptr(), overmap_000.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        ) != -1 {
            return 0;
        }

        // AC012: province -> square link across topologies (region prov_b -> tactical (3,4,0)).
        let region = "{\"topology\":\"province\",\"cells\":[{\"id\":\"prov_b\"}]}";
        if register_map(b"region".as_ptr(), 6, region.as_ptr(), region.len() as i32) != 0 {
            return 0;
        }
        let tactical = "{\"topology\":\"square\",\"cells\":[{\"x\":3,\"y\":4,\"z\":0}]}";
        if register_map(b"tactical".as_ptr(), 8, tactical.as_ptr(), tactical.len() as i32) != 0 {
            return 0;
        }
        let prov_b = "{\"Province\":{\"id\":\"prov_b\"}}";
        let tactical_340 = "{\"Square\":{\"x\":3,\"y\":4,\"z\":0}}";
        if link_maps(
            b"region".as_ptr(), 6,
            prov_b.as_ptr(), prov_b.len() as i32,
            b"tactical".as_ptr(), 8,
            tactical_340.as_ptr(), tactical_340.len() as i32,
        ) != 0 {
            return 0;
        }
        let wc = map_cell(
            b"region".as_ptr(), 6,
            prov_b.as_ptr(), prov_b.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        );
        if wc <= 0 || !eq(&cell_buf[..wc as usize], tactical_340) {
            return 0;
        }
        let wc = unmap_cell(
            b"tactical".as_ptr(), 8,
            tactical_340.as_ptr(), tactical_340.len() as i32,
            cell_buf.as_mut_ptr(), cell_buf.len() as i32,
        );
        if wc <= 0 || !eq(&cell_buf[..wc as usize], prov_b) {
            return 0;
        }
        let wn = get_map_names(names_buf.as_mut_ptr(), names_buf.len() as i32);
        if wn <= 0 || count_quotes(&names_buf[..wn as usize]) != 10 {
            return 0;
        }

        // AC007: enter_map positions the camera at the entry cell.
        let field_100 = "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}";
        if enter_map(b"field".as_ptr(), 5, field_100.as_ptr(), field_100.len() as i32) != 0 {
            return 0;
        }
        let wa = get_active_map_name(active_buf.as_mut_ptr(), active_buf.len() as i32);
        if wa <= 0 || !eq(&active_buf[..wa as usize], "field") {
            return 0;
        }
        let mut cam_buf = [0u8; 128];
        let wcam = get_camera(cam_buf.as_mut_ptr(), cam_buf.len() as i32);
        if wcam <= 0 || !eq(&cam_buf[..wcam as usize], "{\"x\":1,\"y\":0,\"z\":0}") {
            return 0;
        }

        // AC008: exit_map returns to the previously active map (overmap).
        if exit_map() != 0 {
            return 0;
        }
        let wa = get_active_map_name(active_buf.as_mut_ptr(), active_buf.len() as i32);
        if wa <= 0 || !eq(&active_buf[..wa as usize], "overmap") {
            return 0;
        }

        // AC009: enter_map on unknown name returns -1; exit_map on empty stack returns -1.
        if enter_map(b"nonexistent".as_ptr(), 11, field_100.as_ptr(), field_100.len() as i32) != -1 {
            return 0;
        }
        if exit_map() != -1 {
            return 0;
        }

        // AC019: duplicate register_map returns -1; link_maps with unknown map returns -1.
        if register_map(b"field".as_ptr(), 5, field.as_ptr(), field.len() as i32) != -1 {
            return 0;
        }
        if link_maps(
            b"nonexistent".as_ptr(), 11,
            prov_a.as_ptr(), prov_a.len() as i32,
            b"field".as_ptr(), 5,
            field_210.as_ptr(), field_210.len() as i32,
        ) != -1 {
            return 0;
        }
        if link_maps(
            b"field".as_ptr(), 5,
            field_210.as_ptr(), field_210.len() as i32,
            b"nonexistent".as_ptr(), 11,
            prov_a.as_ptr(), prov_a.len() as i32,
        ) != -1 {
            return 0;
        }

        1
    }
}