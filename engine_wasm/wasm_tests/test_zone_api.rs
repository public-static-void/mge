// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_zone_api() -> i32 {
    #[link(wasm_import_module = "mode")]
    unsafe extern "C" {
        fn set_mode(mode_ptr: *const u8, mode_len: i32);
    }

    #[link(wasm_import_module = "region")]
    unsafe extern "C" {
        fn designate_zone(
            kind_ptr: *const u8,
            kind_len: i32,
            label_ptr: *const u8,
            label_len: i32,
            shape_ptr: *const u8,
            shape_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn remove_zone(zone_id_ptr: *const u8, zone_id_len: i32) -> i32;
        fn rename_zone(zone_id_ptr: *const u8, zone_id_len: i32, label_ptr: *const u8, label_len: i32)
        -> i32;
        fn set_zone_kind(zone_id_ptr: *const u8, zone_id_len: i32, kind_ptr: *const u8, kind_len: i32)
        -> i32;
        fn assign_cells_to_zone(
            zone_id_ptr: *const u8,
            zone_id_len: i32,
            cells_ptr: *const u8,
            cells_len: i32,
        ) -> i32;
        fn unassign_cells_from_zone(
            zone_id_ptr: *const u8,
            zone_id_len: i32,
            cells_ptr: *const u8,
            cells_len: i32,
        ) -> i32;
        fn list_zones(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_zone(
            zone_id_ptr: *const u8,
            zone_id_len: i32,
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

    unsafe fn read_str(buf: &[u8], written: i32) -> Option<&str> {
        if written < 0 {
            return None;
        }
        core::str::from_utf8(&buf[..written as usize]).ok()
    }

    unsafe fn designate(
        kind: &str,
        label_json: &str,
        shape: &str,
        out: &mut [u8],
    ) -> Option<usize> {
        let written = designate_zone(
            kind.as_ptr(),
            kind.len() as i32,
            label_json.as_ptr(),
            label_json.len() as i32,
            shape.as_ptr(),
            shape.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if written < 0 {
            return None;
        }
        Some(written as usize)
    }

    unsafe fn read_cells<'a>(id_or_kind: &str, by_kind: bool, buf: &'a mut [u8]) -> Option<&'a str> {
        let written = if by_kind {
            get_cells_in_region_kind(
                id_or_kind.as_ptr(),
                id_or_kind.len() as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
            )
        } else {
            get_cells_in_region(
                id_or_kind.as_ptr(),
                id_or_kind.len() as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
            )
        };
        read_str(buf, written)
    }

    unsafe {
        let colony = "colony";
        set_mode(colony.as_ptr(), colony.len() as i32);

        // designate rect zone: rect 2x2 plus one assigned cell below.
        let mut id_buf = [0u8; 64];
        let shape = r#"{"rect":{"x0":0,"y0":0,"z":0,"x1":1,"y1":1}}"#;
        let label = "\"depot\"";
        let id_len = match designate("stockpile", label, shape, &mut id_buf) {
            Some(n) => n,
            None => return 0,
        };
        let zid = core::str::from_utf8(&id_buf[..id_len]).unwrap_or("");
        if !zid.starts_with("zone-") {
            return 0;
        }

        let cell = r#"[{"Square":{"x":5,"y":5,"z":0}}]"#;
        if assign_cells_to_zone(
            zid.as_ptr(),
            zid.len() as i32,
            cell.as_ptr(),
            cell.len() as i32,
        ) != 1
        {
            return 0;
        }
        // Duplicate assign is idempotent success.
        if assign_cells_to_zone(
            zid.as_ptr(),
            zid.len() as i32,
            cell.as_ptr(),
            cell.len() as i32,
        ) != 1
        {
            return 0;
        }

        // list/get reflect the designation.
        let mut list_buf = [0u8; 1024];
        let list_written = list_zones(list_buf.as_mut_ptr(), list_buf.len() as i32);
        match read_str(&list_buf, list_written) {
            Some(list) => {
                if !list.contains(zid) || !list.contains("depot") || !list.contains("\"cell_count\":5")
                {
                    return 0;
                }
            }
            None => return 0,
        }
        let mut zone_buf = [0u8; 1024];
        let zone_written = get_zone(
            zid.as_ptr(),
            zid.len() as i32,
            zone_buf.as_mut_ptr(),
            zone_buf.len() as i32,
        );
        match read_str(&zone_buf, zone_written) {
            Some(zone) => {
                if !zone.contains("depot") || !zone.contains("stockpile") {
                    return 0;
                }
            }
            None => return 0,
        }

        // rename/rekind moves kind-query membership.
        let store = "store";
        if rename_zone(
            zid.as_ptr(),
            zid.len() as i32,
            store.as_ptr(),
            store.len() as i32,
        ) != 1
        {
            return 0;
        }
        let farm = "farm";
        if set_zone_kind(
            zid.as_ptr(),
            zid.len() as i32,
            farm.as_ptr(),
            farm.len() as i32,
        ) != 1
        {
            return 0;
        }
        let mut kind_buf = [0u8; 1024];
        match read_cells(farm, true, &mut kind_buf) {
            Some(cells) => {
                if cells.matches("\"Square\"").count() != 5 {
                    return 0;
                }
            }
            None => return 0,
        }
        let stockpile = "stockpile";
        let mut stale_buf = [0u8; 64];
        match read_cells(stockpile, true, &mut stale_buf) {
            Some(cells) => {
                if cells != "[]" {
                    return 0;
                }
            }
            None => return 0,
        }

        // unassign drops the explicit cell; unmembered unassign is idempotent.
        if unassign_cells_from_zone(
            zid.as_ptr(),
            zid.len() as i32,
            cell.as_ptr(),
            cell.len() as i32,
        ) != 1
        {
            return 0;
        }
        let stray = r#"[{"Square":{"x":9,"y":9,"z":0}}]"#;
        if unassign_cells_from_zone(
            zid.as_ptr(),
            zid.len() as i32,
            stray.as_ptr(),
            stray.len() as i32,
        ) != 1
        {
            return 0;
        }
        let mut region_buf = [0u8; 1024];
        match read_cells(zid, false, &mut region_buf) {
            Some(cells) => {
                if cells.matches("\"Square\"").count() != 4 {
                    return 0;
                }
            }
            None => return 0,
        }

        // Validation rejects bad designations.
        let mut bad_buf = [0u8; 64];
        if designate("", label, shape, &mut bad_buf).is_some() {
            return 0;
        }
        let bad_rect = r#"{"rect":{"x0":2,"y0":0,"z":0,"x1":1,"y1":1}}"#;
        if designate(farm, label, bad_rect, &mut bad_buf).is_some() {
            return 0;
        }
        if designate(farm, label, "{\"blob\":[]}", &mut bad_buf).is_some() {
            return 0;
        }
        // Empty-kind rekind is rejected; unknown ids return 0.
        let empty = "";
        if set_zone_kind(
            zid.as_ptr(),
            zid.len() as i32,
            empty.as_ptr(),
            empty.len() as i32,
        ) != -1
        {
            return 0;
        }
        let ghost = "zone-99999";
        if rename_zone(
            ghost.as_ptr(),
            ghost.len() as i32,
            store.as_ptr(),
            store.len() as i32,
        ) != 0
        {
            return 0;
        }

        // Nested Region references resolve with a cycle guard, physical only.
        let mut a_buf = [0u8; 64];
        let cell_p = r#"{"cells":[{"Square":{"x":0,"y":0,"z":0}}]}"#;
        let a_len = match designate("room", "null", cell_p, &mut a_buf) {
            Some(n) => n,
            None => return 0,
        };
        let aid = core::str::from_utf8(&a_buf[..a_len]).unwrap_or("");
        let mut b_buf = [0u8; 64];
        let b_len = match designate("room", "null", cell_p, &mut b_buf) {
            Some(n) => n,
            None => return 0,
        };
        // Link A <-> B through Region references (cycle by construction).
        // B references A; A references B; both resolve to the shared cell.
        let mut ref_tmp = [0u8; 128];
        let mut i = 0usize;
        let prefix: &[u8] = b"[{\"Region\":{\"id\":\"";
        let suffix: &[u8] = b"\"}}]";
        for chunk in [prefix, aid.as_bytes(), suffix] {
            for b in chunk {
                if i >= ref_tmp.len() {
                    return 0;
                }
                ref_tmp[i] = *b;
                i += 1;
            }
        }
        let ref_a = core::str::from_utf8(&ref_tmp[..i]).unwrap_or("");
        let bid = core::str::from_utf8(&b_buf[..b_len]).unwrap_or("");
        if assign_cells_to_zone(
            bid.as_ptr(),
            bid.len() as i32,
            ref_a.as_ptr(),
            ref_a.len() as i32,
        ) != 1
        {
            return 0;
        }
        let mut ref_tmp_b = [0u8; 128];
        let mut j = 0usize;
        for chunk in [prefix, bid.as_bytes(), suffix] {
            for b in chunk {
                if j >= ref_tmp_b.len() {
                    return 0;
                }
                ref_tmp_b[j] = *b;
                j += 1;
            }
        }
        let ref_b = core::str::from_utf8(&ref_tmp_b[..j]).unwrap_or("");
        if assign_cells_to_zone(
            aid.as_ptr(),
            aid.len() as i32,
            ref_b.as_ptr(),
            ref_b.len() as i32,
        ) != 1
        {
            return 0;
        }
        let mut cells_a = [0u8; 512];
        let mut cells_b = [0u8; 512];
        let a_cells = match read_cells(aid, false, &mut cells_a) {
            Some(c) => c,
            None => return 0,
        };
        if a_cells.contains("Region") || a_cells.matches("\"Square\"").count() != 2 {
            return 0;
        }
        let b_cells = match read_cells(bid, false, &mut cells_b) {
            Some(c) => c,
            None => return 0,
        };
        if a_cells != b_cells {
            return 0;
        }

        // remove drops the zone; second remove reports false.
        if remove_zone(zid.as_ptr(), zid.len() as i32) != 1 {
            return 0;
        }
        let mut gone_buf = [0u8; 64];
        if get_zone(
            zid.as_ptr(),
            zid.len() as i32,
            gone_buf.as_mut_ptr(),
            gone_buf.len() as i32,
        ) != -1
        {
            return 0;
        }
        if remove_zone(zid.as_ptr(), zid.len() as i32) != 0 {
            return 0;
        }

        // Mode gate: mutators fail outside colony mode, reads stay ungated.
        let roguelike = "roguelike";
        set_mode(roguelike.as_ptr(), roguelike.len() as i32);
        if designate("stockpile", label, shape, &mut bad_buf).is_some() {
            return 0;
        }
        if remove_zone(aid.as_ptr(), aid.len() as i32) != -1 {
            return 0;
        }
        if rename_zone(aid.as_ptr(), aid.len() as i32, store.as_ptr(), store.len() as i32) != -1 {
            return 0;
        }
        if set_zone_kind(aid.as_ptr(), aid.len() as i32, farm.as_ptr(), farm.len() as i32) != -1 {
            return 0;
        }
        if assign_cells_to_zone(aid.as_ptr(), aid.len() as i32, cell.as_ptr(), cell.len() as i32)
            != -1
        {
            return 0;
        }
        if unassign_cells_from_zone(
            aid.as_ptr(),
            aid.len() as i32,
            cell.as_ptr(),
            cell.len() as i32,
        ) != -1
        {
            return 0;
        }
        let mut gate_list = [0u8; 1024];
        if list_zones(gate_list.as_mut_ptr(), gate_list.len() as i32) < 0 {
            return 0;
        }
        let mut gate_cells = [0u8; 512];
        if read_cells(aid, false, &mut gate_cells).is_none() {
            return 0;
        }

        1
    }
}
