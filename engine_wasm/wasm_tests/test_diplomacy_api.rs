// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the diplomacy API (relations, treaties, war/peace) through the host bridge.

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
    fn break_treaty(id: i64) -> i32;
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
pub extern "C" fn test_diplomacy_api() -> i32 {
    unsafe {
        // Unknown pairs start neutral at zero standing.
        if !rel_is("athens", "sparta", "neutral") {
            return 0;
        }
        if standing("athens", "sparta") != 0 {
            return 0;
        }

        // Pair A: propose alliance, accept, war, peace.
        let alliance = propose("athens", "sparta", "alliance", -1);
        if alliance < 0 {
            return 0;
        }
        if accept_treaty(alliance) != 0 {
            return 0;
        }
        if !rel_is("athens", "sparta", "allied") {
            return 0;
        }
        if war("athens", "sparta") != 0 {
            return 0;
        }
        if !rel_is("athens", "sparta", "war") {
            return 0;
        }
        if peace("athens", "sparta") != 0 {
            return 0;
        }
        if !rel_is("athens", "sparta", "neutral") {
            return 0;
        }
        if standing("athens", "sparta") != 0 {
            return 0;
        }

        // Pair B: war, then a peace treaty ends it with the same end state.
        if war("athens", "corinth") != 0 {
            return 0;
        }
        if propose("athens", "corinth", "non_aggression", -1) >= 0 {
            return 0;
        }
        let settlement = propose("athens", "corinth", "peace", -1);
        if settlement < 0 {
            return 0;
        }
        if accept_treaty(settlement) != 0 {
            return 0;
        }
        if !rel_is("athens", "corinth", "neutral") {
            return 0;
        }

        // Standing bounds are enforced by core through the bridge.
        if adjust("athens", "sparta", 200) != 0 {
            return 0;
        }
        if standing("athens", "sparta") != 100 {
            return 0;
        }
        if adjust("athens", "sparta", -350) != 0 {
            return 0;
        }
        if standing("athens", "sparta") != -100 {
            return 0;
        }

        // Self-pair proposals are rejected.
        if propose("athens", "athens", "alliance", -1) >= 0 {
            return 0;
        }

        // Breaking a treaty applies the penalty.
        let pact = propose("thebes", "sparta", "trade", -1);
        if pact < 0 {
            return 0;
        }
        if accept_treaty(pact) != 0 {
            return 0;
        }
        if break_treaty(pact) != 0 {
            return 0;
        }
        if standing("thebes", "sparta") != -25 {
            return 0;
        }

        // Treaty listing reports records.
        let mut out = [0u8; 512];
        let empty: [u8; 1] = [0];
        let listed = list_treaties(empty.as_ptr(), 0, out.as_mut_ptr(), out.len() as i32);
        if listed <= 0 {
            return 0;
        }

        1
    }
}
