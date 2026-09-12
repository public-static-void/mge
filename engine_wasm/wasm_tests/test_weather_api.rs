// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_weather_api() -> i32 {
    #[link(wasm_import_module = "weather")]
    unsafe extern "C" {
        fn get_weather(out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_weather(
            condition_ptr: *const u8,
            condition_len: i32,
            intensity: f64,
            duration: u32,
        ) -> i32;
        fn get_weather_visibility_modifier() -> f64;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
    }

    unsafe {
        // get_weather returns valid JSON with the expected keys
        let mut out_buf = [0u8; 128];
        let written = get_weather(out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if !result.contains("condition")
            || !result.contains("intensity")
            || !result.contains("duration_remaining")
        {
            return 0;
        }

        // set_weather round-trip: rain at 0.8 intensity for 100 ticks
        let condition = b"rain";
        let ret = set_weather(condition.as_ptr(), condition.len() as i32, 0.8, 100);
        if ret != 0 {
            return 0;
        }
        let written = get_weather(out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if !result.contains("\"condition\":\"rain\"")
            || !result.contains("\"intensity\":0.8")
            || !result.contains("\"duration_remaining\":100")
        {
            return 0;
        }

        // get_weather_visibility_modifier: rain 0.8 → 0.7 * 0.8 = 0.56 after a tick
        tick();
        let vis = get_weather_visibility_modifier();
        if (vis - 0.56).abs() > 0.0001 {
            return 0;
        }

        1
    }
}