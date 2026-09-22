// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_temperature_api() -> i32 {
    #[link(wasm_import_module = "temperature")]
    unsafe extern "C" {
        fn get_temperature() -> f64;
        fn set_temperature(ambient: f64) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
    }

    unsafe {
        // Default ambient is 15.0
        if (get_temperature() - 15.0).abs() > 0.0001 {
            return 0;
        }

        // Round-trip: hold at 20.0
        if set_temperature(20.0) != 0 {
            return 0;
        }
        if (get_temperature() - 20.0).abs() > 0.0001 {
            return 0;
        }

        // Override holds across a tick
        tick();
        if (get_temperature() - 20.0).abs() > 0.0001 {
            return 0;
        }

        // Clamp high: 100.0 -> 60.0
        set_temperature(100.0);
        if (get_temperature() - 60.0).abs() > 0.0001 {
            return 0;
        }

        // Clamp low: -100.0 -> -60.0
        set_temperature(-100.0);
        if (get_temperature() + 60.0).abs() > 0.0001 {
            return 0;
        }

        1
    }
}
