// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the narrative API (register, list, get, poll, resolve, history)
// through the host bridge. Trigger evaluation stays in Rust core; the guest
// only registers, ticks, polls, and resolves.
#[no_mangle]
pub extern "C" fn test_narrative_api() -> i32 {
    #[link(wasm_import_module = "narrative")]
    unsafe extern "C" {
        fn register_scenario(def_ptr: *const u8, def_len: i32) -> i32;
        fn list_scenarios(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_scenario(
            id_ptr: *const u8,
            id_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn poll_pending_decisions(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_pending_decision(id: i64, out_ptr: *mut u8, out_len: i32) -> i32;
        fn resolve_decision(id: i64, choice_ptr: *const u8, choice_len: i32) -> i32;
        fn get_narrative_history(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
    }

    #[link(wasm_import_module = "event_bus")]
    unsafe extern "C" {
        fn poll_ecs_event(
            type_ptr: *const u8,
            type_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    fn contains(buf: &[u8], written: i32, needle: &str) -> bool {
        if written <= 0 {
            return false;
        }
        let hay = &buf[..written as usize];
        let ndl = needle.as_bytes();
        if ndl.is_empty() || ndl.len() > hay.len() {
            return false;
        }
        hay.windows(ndl.len()).any(|w| w == ndl)
    }

    unsafe {
        let def = "{\"id\":\"wasm_probe\",\"name\":\"WASM Probe\",\"triggers\":[{\"type\":\"turn_gte\",\"turn\":1}],\"choices\":[{\"id\":\"take\",\"label\":\"Take\",\"effects\":[{\"action\":\"emit_event\",\"data\":{\"bus\":\"narr_wasm_bus\",\"payload\":{\"note\":\"picked\"}}}]}]}";
        let mut out = [0u8; 4096];

        // Registration succeeds; malformed definitions are rejected.
        if register_scenario(def.as_ptr(), def.len() as i32) != 0 {
            return 0;
        }
        let bad = "{\"id\":\"\",\"name\":\"bad\",\"choices\":[]}";
        if register_scenario(bad.as_ptr(), bad.len() as i32) == 0 {
            return 0;
        }

        // Duplicate registration replaces without duplicating evaluation.
        if register_scenario(def.as_ptr(), def.len() as i32) != 0 {
            return 0;
        }
        let w = list_scenarios(out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"wasm_probe\"") {
            return 0;
        }

        // Definition lookup reports the record; missing ids fail.
        let probe = "wasm_probe";
        let w = get_scenario(
            probe.as_ptr(),
            probe.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "\"WASM Probe\"") {
            return 0;
        }
        let ghost = "ghost_scenario";
        if get_scenario(
            ghost.as_ptr(),
            ghost.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        ) != -1
        {
            return 0;
        }

        // Nothing pending before the tick; one decision fires on tick.
        let w = poll_pending_decisions(out.as_mut_ptr(), out.len() as i32);
        if w != 2 {
            return 0;
        }
        tick();
        let w = poll_pending_decisions(out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"wasm_probe\"") {
            return 0;
        }

        // Pending lookup reports the decision; missing ids fail.
        let w = get_pending_decision(0, out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"scenario_id\":\"wasm_probe\"") {
            return 0;
        }
        if get_pending_decision(9999, out.as_mut_ptr(), out.len() as i32) != -1 {
            return 0;
        }

        // Unknown choices fail with no state change.
        let wrong = "ghost";
        if resolve_decision(0, wrong.as_ptr(), wrong.len() as i32) == 0 {
            return 0;
        }

        // Resolve applies the choice effect and clears pending.
        let take = "take";
        if resolve_decision(0, take.as_ptr(), take.len() as i32) != 0 {
            return 0;
        }
        let w = poll_pending_decisions(out.as_mut_ptr(), out.len() as i32);
        if w != 2 {
            return 0;
        }
        // Settled decisions fail a second resolve.
        if resolve_decision(0, take.as_ptr(), take.len() as i32) == 0 {
            return 0;
        }

        // The choice effect lands on its bus.
        let bus = "narr_wasm_bus";
        let w = poll_ecs_event(
            bus.as_ptr(),
            bus.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "\"picked\"") {
            return 0;
        }

        // History records the firing and the resolution in order.
        let w = get_narrative_history(out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"fired\"") {
            return 0;
        }
        if !contains(&out, w, "\"resolved\"") {
            return 0;
        }
        if !contains(&out, w, "\"take\"") {
            return 0;
        }

        1
    }
}
