// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the faction/reputation API (set_faction, get_faction, modify_reputation, get_reputation).

#[no_mangle]
pub extern "C" fn test_faction_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "faction")]
    unsafe extern "C" {
        fn set_faction(
            entity: u32,
            faction_id_ptr: *const u8,
            faction_id_len: i32,
            role_ptr: *const u8,
            role_len: i32,
        );
        fn get_faction(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn modify_reputation(
            entity: u32,
            faction_id_ptr: *const u8,
            faction_id_len: i32,
            delta: i64,
        );
        fn get_reputation(
            entity: u32,
            faction_id_ptr: *const u8,
            faction_id_len: i32,
        ) -> i64;
    }

    unsafe {
        // Test 1: Basic set_faction/get_faction round-trip
        let eid = spawn_entity();
        let faction_id = "goblins";
        let role = "member";
        set_faction(
            eid,
            faction_id.as_ptr(),
            faction_id.len() as i32,
            role.as_ptr(),
            role.len() as i32,
        );
        let mut out_buf = [0u8; 64];
        let written = get_faction(eid, out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if result != "goblins" {
            return 0;
        }

        // Test 2: get_faction returns -1 when entity has no Faction component
        let eid2 = spawn_entity();
        let mut out_buf2 = [0u8; 64];
        let written2 = get_faction(eid2, out_buf2.as_mut_ptr(), out_buf2.len() as i32);
        if written2 != -1 {
            return 0;
        }

        // Test 3: modify_reputation/get_reputation round-trip
        let eid3 = spawn_entity();
        let faction_goblins = "goblins";
        modify_reputation(
            eid3,
            faction_goblins.as_ptr(),
            faction_goblins.len() as i32,
            25,
        );
        let rep = get_reputation(
            eid3,
            faction_goblins.as_ptr(),
            faction_goblins.len() as i32,
        );
        if rep != 25 {
            return 0;
        }

        // Test 4: get_reputation returns 0 when entity has no Reputation component
        let eid4 = spawn_entity();
        let faction_humans = "humans";
        let rep2 = get_reputation(
            eid4,
            faction_humans.as_ptr(),
            faction_humans.len() as i32,
        );
        if rep2 != 0 {
            return 0;
        }

        // Test 5: Reputation clamping at upper bound (100)
        let eid5 = spawn_entity();
        let faction_clamp = "clamp";
        modify_reputation(
            eid5,
            faction_clamp.as_ptr(),
            faction_clamp.len() as i32,
            200,
        );
        let rep3 = get_reputation(
            eid5,
            faction_clamp.as_ptr(),
            faction_clamp.len() as i32,
        );
        if rep3 != 100 {
            return 0;
        }

        // Test 6: Reputation clamping at lower bound (-100)
        let eid6 = spawn_entity();
        let faction_neg = "negative";
        modify_reputation(
            eid6,
            faction_neg.as_ptr(),
            faction_neg.len() as i32,
            -200,
        );
        let rep4 = get_reputation(
            eid6,
            faction_neg.as_ptr(),
            faction_neg.len() as i32,
        );
        if rep4 != -100 {
            return 0;
        }

        1
    }
}
