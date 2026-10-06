// This file is compiled to WASM and loaded by the Rust host test harness.
// Supply bridge parity: the five `supply` host functions with identical
// validation, atomicity, epsilon discipline, and gate order as the Lua/Python
// bridges. Mutations report the `{"ok": bool, ...}` envelope with the core
// variant name in the error text; `create`/`get` additionally carry the link
// id / record, and `list` writes a JSON array of ascending link ids.
#[no_mangle]
pub extern "C" fn test_supply_api() -> i32 {
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

    #[link(wasm_import_module = "supply")]
    unsafe extern "C" {
        fn create_supply_link(
            source: u32,
            target: u32,
            kind_ptr: *const u8,
            kind_len: i32,
            amount: f64,
            capacity: f64,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn remove_supply_link(link: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn list_supply_links(out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_supply_link_active(
            link: u32,
            active: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_supply_link(link: u32, out_ptr: *mut u8, out_len: i32) -> i32;
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

    /// Parses the `"link":<id>` field out of a create response.
    /// Returns `u32::MAX` when the field is absent (error envelope).
    fn parse_link_id(buf: &[u8], written: i32) -> u32 {
        if written <= 0 {
            return u32::MAX;
        }
        let hay = &buf[..written as usize];
        let key = b"\"link\":";
        let mut start = None;
        for i in 0..hay.len().saturating_sub(key.len()) {
            if &hay[i..i + key.len()] == key {
                start = Some(i + key.len());
                break;
            }
        }
        let Some(mut i) = start else {
            return u32::MAX;
        };
        let mut id: u32 = 0;
        let mut digits = 0;
        while i < hay.len() && hay[i].is_ascii_digit() {
            id = id.saturating_mul(10).saturating_add((hay[i] - b'0') as u32);
            digits += 1;
            i += 1;
        }
        if digits == 0 { u32::MAX } else { id }
    }

    fn render_u32(mut v: u32, out: &mut [u8]) -> usize {
        if v == 0 {
            out[0] = b'0';
            return 1;
        }
        let mut tmp = [0u8; 10];
        let mut n = 0;
        while v > 0 {
            tmp[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
        for i in 0..n {
            out[i] = tmp[n - 1 - i];
        }
        n
    }

    /// Builds the exact expected list payload (`[a]`, `[a,b]`, ...) and
    /// byte-compares it against the host response.
    fn list_is(ids: &[u32]) -> bool {
        let mut out = [0u8; 256];
        let w = unsafe { list_supply_links(out.as_mut_ptr(), out.len() as i32) };
        if w <= 0 {
            return false;
        }
        let mut expected = [0u8; 256];
        let mut n = 0;
        expected[n] = b'[';
        n += 1;
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                expected[n] = b',';
                n += 1;
            }
            let mut digits = [0u8; 10];
            let dn = render_u32(*id, &mut digits);
            expected[n..n + dn].copy_from_slice(&digits[..dn]);
            n += dn;
        }
        expected[n] = b']';
        n += 1;
        w as usize == n && out[..n] == expected[..n]
    }

    fn create_ok(source: u32, target: u32, kind: &str, amount: f64, capacity: f64) -> u32 {
        let mut out = [0u8; 1024];
        let w = unsafe {
            create_supply_link(
                source,
                target,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                capacity,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        if !contains(&out, w, "\"ok\":true") {
            return u32::MAX;
        }
        parse_link_id(&out, w)
    }

    fn create_err(
        source: u32,
        target: u32,
        kind: &str,
        amount: f64,
        capacity: f64,
        variant: &str,
    ) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            create_supply_link(
                source,
                target,
                kind.as_ptr(),
                kind.len() as i32,
                amount,
                capacity,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn remove_ok(link: u32) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe { remove_supply_link(link, out.as_mut_ptr(), out.len() as i32) };
        contains(&out, w, "\"ok\":true")
    }

    fn remove_err(link: u32, variant: &str) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe { remove_supply_link(link, out.as_mut_ptr(), out.len() as i32) };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn set_active_ok(link: u32, active: i32) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            set_supply_link_active(link, active, out.as_mut_ptr(), out.len() as i32)
        };
        contains(&out, w, "\"ok\":true")
    }

    fn set_active_err(link: u32, variant: &str) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe {
            set_supply_link_active(link, 1, out.as_mut_ptr(), out.len() as i32)
        };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn get_has(link: u32, needles: &[&str]) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe { get_supply_link(link, out.as_mut_ptr(), out.len() as i32) };
        if !contains(&out, w, "\"ok\":true") {
            return false;
        }
        needles.iter().all(|n| contains(&out, w, n))
    }

    fn get_err(link: u32, variant: &str) -> bool {
        let mut out = [0u8; 1024];
        let w = unsafe { get_supply_link(link, out.as_mut_ptr(), out.len() as i32) };
        contains(&out, w, "\"ok\":false") && contains(&out, w, variant)
    }

    fn delivered_poll(needles: &[&str]) -> bool {
        let mut out = [0u8; 4096];
        let et = "supply_delivered";
        let w = unsafe {
            poll_ecs_event(
                et.as_ptr(),
                et.len() as i32,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        if w <= 0 {
            return false;
        }
        needles.iter().all(|n| contains(&out, w, n))
    }

    fn delivered_poll_empty() -> bool {
        let mut out = [0u8; 4096];
        let et = "supply_delivered";
        let w = unsafe {
            poll_ecs_event(
                et.as_ptr(),
                et.len() as i32,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        w <= 0 || !contains(&out, w, "\"link\"")
    }

    fn blocked_poll(reason: &str) -> bool {
        let mut out = [0u8; 4096];
        let et = "supply_blocked";
        let w = unsafe {
            poll_ecs_event(
                et.as_ptr(),
                et.len() as i32,
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        w > 0 && contains(&out, w, reason)
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
        // Lifecycle: create two links, verify verbatim records and ordering.
        let a = spawn_entity();
        let b = spawn_entity();
        set_stock(a, "{\"resources\":{\"grain\":100.0}}");
        set_stock(b, "{\"resources\":{\"grain\":0.0}}");
        let first = create_ok(a, b, "grain", 10.0, 25.0);
        if first == u32::MAX {
            return 0;
        }
        if !get_has(
            first,
            &[
                "\"kind\":\"grain\"",
                "\"amount_per_tick\":10.0",
                "\"capacity_per_tick\":25.0",
                "\"active\":true",
            ],
        ) {
            return 0;
        }
        if !list_is(&[first]) {
            return 0;
        }

        // Rejection matrix: no link entity may leak on failure.
        let c = spawn_entity();
        set_stock(c, "{\"resources\":{\"grain\":0.0}}");
        let bare = spawn_entity();
        if !create_err(a, a, "grain", 10.0, 10.0, "SameEndpoint") {
            return 0;
        }
        if !create_err(a, b, "", 10.0, 10.0, "UnknownKind") {
            return 0;
        }
        if !create_err(a, b, "grain", 0.0, 10.0, "NonPositiveAmount") {
            return 0;
        }
        if !create_err(a, b, "grain", -1.0, 10.0, "NonPositiveAmount") {
            return 0;
        }
        let nan = 0.0f64 / 0.0f64;
        if !create_err(a, b, "grain", nan, 10.0, "NonPositiveAmount") {
            return 0;
        }
        if !create_err(a, b, "grain", f64::INFINITY, 10.0, "NonPositiveAmount") {
            return 0;
        }
        if !create_err(bare, b, "grain", 1.0, 1.0, "NoStockpile") {
            return 0;
        }
        if !create_err(a, bare, "grain", 1.0, 1.0, "NoStockpile") {
            return 0;
        }
        if !list_is(&[first]) {
            return 0;
        }

        // Unknown-link errors on every mutating/query path.
        if !get_err(9999, "UnknownLink") {
            return 0;
        }
        if !remove_err(9999, "UnknownLink") {
            return 0;
        }
        if !set_active_err(9999, "UnknownLink") {
            return 0;
        }

        // A second link lists after the first (ascending order).
        let second = create_ok(a, c, "grain", 30.0, 12.0);
        if second == u32::MAX || second <= first {
            return 0;
        }
        if !list_is(&[first, second]) {
            return 0;
        }

        // Per-tick delivery honors the capacity cap on each leg.
        tick();
        if !balance_has(a, "\"grain\":78.0") {
            return 0;
        }
        if !balance_has(b, "\"grain\":10.0") || !balance_has(c, "\"grain\":12.0") {
            return 0;
        }
        if !delivered_poll(&["\"amount\":10.0", "\"amount\":12.0"]) {
            return 0;
        }

        // Inactive links skip silently, then resume on reactivation.
        if !set_active_ok(first, 0) || !set_active_ok(second, 0) {
            return 0;
        }
        if !get_has(first, &["\"active\":false"]) {
            return 0;
        }
        tick();
        if !balance_has(a, "\"grain\":78.0") {
            return 0;
        }
        if !delivered_poll_empty() {
            return 0;
        }
        if !set_active_ok(first, 1) || !set_active_ok(second, 1) {
            return 0;
        }
        tick();
        if !balance_has(a, "\"grain\":56.0") {
            return 0;
        }
        if !balance_has(b, "\"grain\":20.0") || !balance_has(c, "\"grain\":24.0") {
            return 0;
        }

        // Wartime blocks the cross-faction leg; peace resumes it.
        join_faction(a, "f1");
        join_faction(b, "f2");
        join_faction(c, "f1");
        if declare_war("f1".as_ptr(), 2, "f2".as_ptr(), 2) != 0 {
            return 0;
        }
        tick();
        if !balance_has(a, "\"grain\":44.0") {
            return 0;
        }
        if !balance_has(b, "\"grain\":20.0") || !balance_has(c, "\"grain\":36.0") {
            return 0;
        }
        if !blocked_poll("\"reason\":\"war\"") {
            return 0;
        }
        if declare_peace("f1".as_ptr(), 2, "f2".as_ptr(), 2) != 0 {
            return 0;
        }
        tick();
        if !balance_has(a, "\"grain\":22.0") {
            return 0;
        }
        if !balance_has(b, "\"grain\":30.0") || !balance_has(c, "\"grain\":48.0") {
            return 0;
        }

        // Removal drops the link from list and query alike.
        if !remove_ok(first) {
            return 0;
        }
        if !get_err(first, "UnknownLink") {
            return 0;
        }
        if !list_is(&[second]) {
            return 0;
        }

        1
    }
}
