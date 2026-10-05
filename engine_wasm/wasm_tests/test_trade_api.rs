// This file is compiled to WASM and loaded by the Rust host test harness.
// Trade bridge parity: the three `trade` host functions with identical
// validation, atomicity, epsilon discipline, and gate order as the Lua/Python
// bridges. Transfers report the `{"ok": bool, "err": string|null}` envelope
// with the core variant name in the error text.
#[no_mangle]
pub extern "C" fn test_trade_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
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
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
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

    #[link(wasm_import_module = "economic")]
    unsafe extern "C" {
        fn get_stockpile_resources(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
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
    }

    #[link(wasm_import_module = "diplomacy")]
    unsafe extern "C" {
        fn propose_treaty(
            proposer_ptr: *const u8,
            proposer_len: i32,
            other_ptr: *const u8,
            other_len: i32,
            kind_ptr: *const u8,
            kind_len: i32,
            duration: i64,
        ) -> i64;
        fn accept_treaty(id: i64) -> i32;
        fn declare_war(
            fa_ptr: *const u8,
            fa_len: i32,
            fb_ptr: *const u8,
            fb_len: i32,
        ) -> i32;
        fn declare_peace(
            fa_ptr: *const u8,
            fa_len: i32,
            fb_ptr: *const u8,
            fb_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "trade")]
    unsafe extern "C" {
        fn transfer_stockpile_resource(
            from: u32,
            to: u32,
            kind_ptr: *const u8,
            kind_len: i32,
            amount: f64,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn has_active_trade_treaty(
            fa_ptr: *const u8,
            fa_len: i32,
            fb_ptr: *const u8,
            fb_len: i32,
        ) -> i32;
        fn execute_treaty_trade(
            from: u32,
            to: u32,
            kind_ptr: *const u8,
            kind_len: i32,
            amount: f64,
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

    fn set_stock(entity: u32, json: &str) {
        let name = "Stockpile";
        unsafe {
            set_component(
                entity,
                name.as_ptr(),
                name.len() as i32,
                json.as_ptr(),
                json.len() as i32,
            );
        }
    }

    fn balance_has(entity: u32, needle: &str) -> bool {
        let mut buf = [0u8; 512];
        let w = unsafe { get_stockpile_resources(entity, buf.as_mut_ptr(), buf.len() as i32) };
        contains(&buf, w, needle)
    }

    fn transfer_ok(from: u32, to: u32, kind: &str, amount: f64) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            transfer_stockpile_resource(
                from,
                to,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        contains(&out, w, "\"ok\":true")
    }

    fn transfer_err(from: u32, to: u32, kind: &str, amount: f64, variant: &str) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            transfer_stockpile_resource(
                from,
                to,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn gated_err(from: u32, to: u32, kind: &str, amount: f64, variant: &str) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            execute_treaty_trade(
                from,
                to,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn gated_ok(from: u32, to: u32, kind: &str, amount: f64) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            execute_treaty_trade(
                from,
                to,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        contains(&out, w, "\"ok\":true")
    }

    fn treaty_active(fa: &str, fb: &str) -> bool {
        unsafe {
            has_active_trade_treaty(
                fa.as_ptr(),
                fa.len() as i32,
                fb.as_ptr(),
                fb.len() as i32,
            ) == 1
        }
    }

    fn join_faction(entity: u32, faction: &str) {
        let role = "member";
        unsafe {
            set_faction(
                entity,
                faction.as_ptr(),
                faction.len() as i32,
                role.as_ptr(),
                role.len() as i32,
            );
        }
    }

    unsafe {
        // Direct move plus an overdraft that leaves both sides untouched.
        let a = spawn_entity();
        let b = spawn_entity();
        set_stock(a, "{\"resources\":{\"grain\":10.0}}");
        set_stock(b, "{\"resources\":{\"grain\":0.0}}");
        if !transfer_ok(a, b, "grain", 4.0) {
            return 0;
        }
        if !balance_has(a, "\"grain\":6.0") || !balance_has(b, "\"grain\":4.0") {
            return 0;
        }
        if !transfer_err(a, b, "grain", 99.0, "InsufficientFunds") {
            return 0;
        }
        if !balance_has(a, "\"grain\":6.0") || !balance_has(b, "\"grain\":4.0") {
            return 0;
        }

        // Kind and amount validation rejects before touching state.
        let c = spawn_entity();
        let d = spawn_entity();
        set_stock(c, "{\"resources\":{\"grain\":10.0}}");
        set_stock(d, "{\"resources\":{\"grain\":0.0}}");
        if !transfer_err(c, d, "", 1.0, "UnknownKind") {
            return 0;
        }
        if !transfer_err(c, d, "grain", 0.0, "NonPositiveAmount") {
            return 0;
        }
        if !transfer_err(c, d, "grain", -1.0, "NonPositiveAmount") {
            return 0;
        }
        let nan = 0.0f64 / 0.0f64;
        if !transfer_err(c, d, "grain", nan, "NonPositiveAmount") {
            return 0;
        }
        if !transfer_err(c, d, "grain", f64::INFINITY, "NonPositiveAmount") {
            return 0;
        }
        if !balance_has(c, "\"grain\":10.0") || !balance_has(d, "\"grain\":0.0") {
            return 0;
        }

        // Missing stockpiles fail on either endpoint; self-transfer is a no-op.
        let e = spawn_entity();
        let f = spawn_entity();
        let bare = spawn_entity();
        set_stock(e, "{\"resources\":{\"grain\":10.0}}");
        set_stock(f, "{\"resources\":{\"grain\":0.0}}");
        if !transfer_err(bare, f, "grain", 1.0, "NoStockpile") {
            return 0;
        }
        if !transfer_err(e, bare, "grain", 1.0, "NoStockpile") {
            return 0;
        }
        if !balance_has(e, "\"grain\":10.0") || !balance_has(f, "\"grain\":0.0") {
            return 0;
        }
        if !transfer_ok(e, e, "grain", 2.0) {
            return 0;
        }
        if !balance_has(e, "\"grain\":10.0") {
            return 0;
        }

        // Treaty lifecycle gates execution: propose, accept, trade, war, peace.
        let h = spawn_entity();
        let i = spawn_entity();
        set_stock(h, "{\"resources\":{\"grain\":10.0}}");
        set_stock(i, "{\"resources\":{\"grain\":0.0}}");
        join_faction(h, "f1");
        join_faction(i, "f2");
        if treaty_active("f1", "f2") {
            return 0;
        }
        let kind = "trade";
        let tid = propose_treaty(
            "f1".as_ptr(),
            2,
            "f2".as_ptr(),
            2,
            kind.as_ptr(),
            kind.len() as i32,
            -1,
        );
        if tid < 0 {
            return 0;
        }
        if accept_treaty(tid) != 0 {
            return 0;
        }
        if !treaty_active("f1", "f2") || !treaty_active("f2", "f1") {
            return 0;
        }
        if !gated_ok(h, i, "grain", 3.0) {
            return 0;
        }
        if !balance_has(h, "\"grain\":7.0") || !balance_has(i, "\"grain\":3.0") {
            return 0;
        }
        if declare_war("f1".as_ptr(), 2, "f2".as_ptr(), 2) != 0 {
            return 0;
        }
        if !treaty_active("f1", "f2") {
            return 0;
        }
        if !gated_err(h, i, "grain", 1.0, "RelationIsWar") {
            return 0;
        }
        if !balance_has(h, "\"grain\":7.0") || !balance_has(i, "\"grain\":3.0") {
            return 0;
        }
        if declare_peace("f1".as_ptr(), 2, "f2".as_ptr(), 2) != 0 {
            return 0;
        }
        if !gated_ok(h, i, "grain", 1.0) {
            return 0;
        }
        if !balance_has(h, "\"grain\":6.0") || !balance_has(i, "\"grain\":4.0") {
            return 0;
        }

        // No treaty, or no faction membership, blocks gated execution.
        let j = spawn_entity();
        let k = spawn_entity();
        set_stock(j, "{\"resources\":{\"grain\":10.0}}");
        set_stock(k, "{\"resources\":{\"grain\":0.0}}");
        join_faction(j, "f3");
        join_faction(k, "f4");
        if !gated_err(j, k, "grain", 1.0, "NoLiveTreaty") {
            return 0;
        }
        if !balance_has(j, "\"grain\":10.0") || !balance_has(k, "\"grain\":0.0") {
            return 0;
        }
        let m = spawn_entity();
        let n = spawn_entity();
        set_stock(m, "{\"resources\":{\"grain\":10.0}}");
        set_stock(n, "{\"resources\":{\"grain\":0.0}}");
        if !gated_err(m, n, "grain", 1.0, "NoLiveTreaty") {
            return 0;
        }
        if !balance_has(m, "\"grain\":10.0") || !balance_has(n, "\"grain\":0.0") {
            return 0;
        }

        // Upkeep parity via the generic component path: round-trip, drain,
        // shortage event, atomic multi-drain, and no-upkeep sparing.
        // (Invalid-drain rejection traps on set_component in WASM, so it is
        // covered by the Lua/Python mirrors and the core Rust suite instead.)
        let upkeep_name = "Upkeep";
        let set_upkeep = |entity: u32, json: &str| unsafe {
            set_component(
                entity,
                upkeep_name.as_ptr(),
                upkeep_name.len() as i32,
                json.as_ptr(),
                json.len() as i32,
            );
        };
        let upkeep_has = |entity: u32, needle: &str| unsafe {
            let mut buf = [0u8; 512];
            let w = get_component(
                entity,
                upkeep_name.as_ptr(),
                upkeep_name.len() as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
            );
            contains(&buf, w, needle)
        };
        let shortage_poll = |out: &mut [u8]| unsafe {
            let et = "consumption_shortage";
            poll_ecs_event(
                et.as_ptr(),
                et.len() as i32,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };

        let u = spawn_entity();
        set_stock(u, "{\"resources\":{\"grain\":10.0}}");
        set_upkeep(u, "{\"drains\":[{\"kind\":\"grain\",\"amount_per_tick\":2.0}]}");
        if !upkeep_has(u, "\"amount_per_tick\":2.0") {
            return 0;
        }
        tick();
        if !balance_has(u, "\"grain\":8.0") {
            return 0;
        }
        tick();
        tick();
        if !balance_has(u, "\"grain\":4.0") {
            return 0;
        }

        let s = spawn_entity();
        set_stock(s, "{\"resources\":{\"grain\":1.0}}");
        set_upkeep(s, "{\"drains\":[{\"kind\":\"grain\",\"amount_per_tick\":2.0}]}");
        tick();
        if !balance_has(s, "\"grain\":1.0") {
            return 0;
        }
        let mut sev = [0u8; 2048];
        let sw = shortage_poll(&mut sev);
        if sw <= 0
            || !contains(&sev, sw, "\"kind\":\"grain\"")
            || !contains(&sev, sw, "\"required\":2.0")
            || !contains(&sev, sw, "\"available\":1.0")
        {
            return 0;
        }

        let v = spawn_entity();
        set_stock(v, "{\"resources\":{\"grain\":10.0,\"wood\":1.0}}");
        set_upkeep(
            v,
            "{\"drains\":[{\"kind\":\"grain\",\"amount_per_tick\":2.0},{\"kind\":\"wood\",\"amount_per_tick\":2.0}]}",
        );
        tick();
        if !balance_has(v, "\"grain\":10.0") || !balance_has(v, "\"wood\":1.0") {
            return 0;
        }
        let mut vev = [0u8; 2048];
        let vw = shortage_poll(&mut vev);
        if vw <= 0 || !contains(&vev, vw, "\"kind\":\"wood\"") {
            return 0;
        }

        let p = spawn_entity();
        set_stock(p, "{\"resources\":{\"grain\":10.0}}");
        tick();
        if !balance_has(p, "\"grain\":10.0") {
            return 0;
        }

        1
    }
}
