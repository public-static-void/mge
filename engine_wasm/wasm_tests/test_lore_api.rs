// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the lore API (generate, list, get, render, len, clear) plus the
// live narrative-to-chronicle feed through the "lore" host bridge. Backfill
// generation stays in Rust core; the guest only generates, queries, ticks,
// and clears.
#[no_mangle]
pub extern "C" fn test_lore_api() -> i32 {
    #[link(wasm_import_module = "lore")]
    unsafe extern "C" {
        fn generate_founding_history(seed: i64, era_count: i32) -> i64;
        fn list_chronicle(
            filter_ptr: *const u8,
            filter_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_chronicle_entry(id: i64, out_ptr: *mut u8, out_len: i32) -> i32;
        fn render_chronicle(
            filter_ptr: *const u8,
            filter_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn chronicle_len() -> i64;
        fn clear_lore_history() -> i32;
    }

    #[link(wasm_import_module = "narrative")]
    unsafe extern "C" {
        fn register_scenario(def_ptr: *const u8, def_len: i32) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
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
        let mut out = [0u8; 8192];

        // Backfill appends exactly era_count entries; negative inputs fail.
        if generate_founding_history(42, 5) != 5 {
            return 0;
        }
        if generate_founding_history(-1, 5) != -1 {
            return 0;
        }
        if generate_founding_history(42, -1) != -1 {
            return 0;
        }
        if chronicle_len() != 5 {
            return 0;
        }

        // Empty-object and empty-input filters both return every entry.
        let all = "{}";
        let w = list_chronicle(
            all.as_ptr(),
            all.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "\"founding\"") {
            return 0;
        }
        let w = list_chronicle(out.as_mut_ptr(), 0, out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"founding\"") {
            return 0;
        }

        // Kind narrowing keeps founding entries and excludes live kinds.
        let founding = "{\"kind\":\"founding\"}";
        let w = list_chronicle(
            founding.as_ptr(),
            founding.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "\"founding\"") {
            return 0;
        }
        let fired = "{\"kind\":\"fired\"}";
        let w = list_chronicle(
            fired.as_ptr(),
            fired.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if w != 2 {
            return 0;
        }

        // Turn-range narrowing excludes the turn-0..5 backfill window miss.
        let future = "{\"turn_from\":10,\"turn_to\":20}";
        let w = list_chronicle(
            future.as_ptr(),
            future.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if w != 2 {
            return 0;
        }

        // Invalid kind strings fail instead of matching nothing.
        let bogus = "{\"kind\":\"bogus\"}";
        let w = list_chronicle(
            bogus.as_ptr(),
            bogus.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if w != -1 {
            return 0;
        }

        // Single lookup reports the entry; unknown ids return null.
        let w = get_chronicle_entry(0, out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"founding\"") {
            return 0;
        }
        let w = get_chronicle_entry(9999, out.as_mut_ptr(), out.len() as i32);
        if w != 4 || !contains(&out, w, "null") {
            return 0;
        }

        // Rendering follows the fixed template, one line per entry.
        let w = render_chronicle(
            all.as_ptr(),
            all.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "Turn ") {
            return 0;
        }
        if !contains(&out, w, "founding") {
            return 0;
        }

        // A fired scenario is visible through the chronicle in the same tick.
        let def = "{\"id\":\"lore_probe\",\"name\":\"Lore Probe\",\"triggers\":[{\"type\":\"turn_gte\",\"turn\":0}],\"choices\":[{\"id\":\"take\",\"label\":\"Take\"}]}";
        if register_scenario(def.as_ptr(), def.len() as i32) != 0 {
            return 0;
        }
        tick();
        let w = list_chronicle(
            fired.as_ptr(),
            fired.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "\"lore_probe\"") {
            return 0;
        }
        let w = render_chronicle(
            fired.as_ptr(),
            fired.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !contains(&out, w, "fired") {
            return 0;
        }
        if chronicle_len() != 6 {
            return 0;
        }

        // Clearing resets the chronicle so seeded backfill reproduces.
        if clear_lore_history() != 0 {
            return 0;
        }
        if chronicle_len() != 0 {
            return 0;
        }
        if generate_founding_history(42, 2) != 2 {
            return 0;
        }

        1
    }
}
