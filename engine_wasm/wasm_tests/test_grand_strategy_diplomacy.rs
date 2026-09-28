// This file is compiled to WASM and loaded by the Rust host test harness.
// Grand-strategy diplomacy scenario: three rival nations (valoria, drakmor,
// kesh) driven through the diplomacy host bridge only.

#[link(wasm_import_module = "diplomacy")]
unsafe extern "C" {
    fn get_relation(
        fa_ptr: *const u8,
        fa_len: i32,
        fb_ptr: *const u8,
        fb_len: i32,
        out_ptr: *mut u8,
        out_len: i32,
    ) -> i32;
    fn get_standing(
        fa_ptr: *const u8,
        fa_len: i32,
        fb_ptr: *const u8,
        fb_len: i32,
    ) -> i64;
    fn modify_standing(
        fa_ptr: *const u8,
        fa_len: i32,
        fb_ptr: *const u8,
        fb_len: i32,
        delta: i64,
    ) -> i32;
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
    fn list_treaties(
        faction_ptr: *const u8,
        faction_len: i32,
        out_ptr: *mut u8,
        out_len: i32,
    ) -> i32;
}

fn rel_is(fa: &str, fb: &str, expected: &str) -> bool {
    let mut buf = [0u8; 16];
    let n = unsafe {
        get_relation(
            fa.as_ptr(),
            fa.len() as i32,
            fb.as_ptr(),
            fb.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    if n <= 0 || n as usize != expected.len() {
        return false;
    }
    &buf[..n as usize] == expected.as_bytes()
}

fn standing(fa: &str, fb: &str) -> i64 {
    unsafe {
        get_standing(
            fa.as_ptr(),
            fa.len() as i32,
            fb.as_ptr(),
            fb.len() as i32,
        )
    }
}

fn adjust(fa: &str, fb: &str, delta: i64) -> i32 {
    unsafe {
        modify_standing(
            fa.as_ptr(),
            fa.len() as i32,
            fb.as_ptr(),
            fb.len() as i32,
            delta,
        )
    }
}

fn war(fa: &str, fb: &str) -> i32 {
    unsafe {
        declare_war(
            fa.as_ptr(),
            fa.len() as i32,
            fb.as_ptr(),
            fb.len() as i32,
        )
    }
}

fn peace(fa: &str, fb: &str) -> i32 {
    unsafe {
        declare_peace(
            fa.as_ptr(),
            fa.len() as i32,
            fb.as_ptr(),
            fb.len() as i32,
        )
    }
}

fn propose(proposer: &str, other: &str, kind: &str, duration: i64) -> i64 {
    unsafe {
        propose_treaty(
            proposer.as_ptr(),
            proposer.len() as i32,
            other.as_ptr(),
            other.len() as i32,
            kind.as_ptr(),
            kind.len() as i32,
            duration,
        )
    }
}

#[no_mangle]
pub extern "C" fn test_grand_strategy_diplomacy() -> i32 {
    unsafe {
        // Three nations start neutral at zero standing on every pair.
        if !rel_is("valoria", "drakmor", "neutral") {
            return 0;
        }
        if !rel_is("valoria", "kesh", "neutral") {
            return 0;
        }
        if !rel_is("drakmor", "kesh", "neutral") {
            return 0;
        }
        if standing("valoria", "drakmor") != 0 {
            return 0;
        }
        if standing("valoria", "kesh") != 0 {
            return 0;
        }

        // A goodwill gesture lifts one pair; the third nation is untouched.
        if adjust("valoria", "drakmor", 40) != 0 {
            return 0;
        }
        if standing("valoria", "drakmor") != 40 {
            return 0;
        }
        if standing("drakmor", "valoria") != 40 {
            return 0;
        }
        if standing("valoria", "kesh") != 0 {
            return 0;
        }

        // An accepted alliance unites two nations; the third stays neutral.
        let alliance = propose("valoria", "drakmor", "alliance", -1);
        if alliance < 0 {
            return 0;
        }
        if accept_treaty(alliance) != 0 {
            return 0;
        }
        if !rel_is("valoria", "drakmor", "allied") {
            return 0;
        }
        if !rel_is("valoria", "kesh", "neutral") {
            return 0;
        }

        // War on the third nation reads war while the alliance holds.
        if war("valoria", "kesh") != 0 {
            return 0;
        }
        if !rel_is("valoria", "kesh", "war") {
            return 0;
        }
        if !rel_is("valoria", "drakmor", "allied") {
            return 0;
        }

        // Peace ends the war; war and peace alone record no treaties.
        if peace("valoria", "kesh") != 0 {
            return 0;
        }
        if !rel_is("valoria", "kesh", "neutral") {
            return 0;
        }
        if standing("valoria", "kesh") != 0 {
            return 0;
        }

        // Treaty listing reflects the lifecycle: only the alliance recorded.
        let mut out = [0u8; 512];
        let empty: [u8; 1] = [0];
        let listed = list_treaties(empty.as_ptr(), 0, out.as_mut_ptr(), out.len() as i32);
        if listed <= 0 {
            return 0;
        }

        1
    }
}
