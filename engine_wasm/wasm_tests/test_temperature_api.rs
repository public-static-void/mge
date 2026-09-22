// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_temperature_api() -> i32 {
    #[link(wasm_import_module = "temperature")]
    unsafe extern "C" {
        fn get_temperature() -> f64;
        fn set_temperature(ambient: f64) -> i32;
        fn get_humidity() -> f64;
        fn set_humidity(humidity: f64) -> i32;
        fn get_pressure() -> f64;
        fn set_pressure(pressure: f64) -> i32;
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

        // Humidity/pressure parity: defaults, round-trip, and clamps.
        // Per-cell diffusion and per-part drift stay host-side in v2, so the
        // guest probes scalar ambient/humidity/pressure parity only.
        if (get_humidity() - 0.5).abs() > 0.0001 {
            return 0;
        }
        if (get_pressure() - 1013.0).abs() > 0.0001 {
            return 0;
        }
        if set_humidity(0.8) != 0 || (get_humidity() - 0.8).abs() > 0.0001 {
            return 0;
        }
        if set_pressure(1000.0) != 0 || (get_pressure() - 1000.0).abs() > 0.0001 {
            return 0;
        }
        set_humidity(2.0);
        if (get_humidity() - 1.0).abs() > 0.0001 {
            return 0;
        }
        set_pressure(2000.0);
        if (get_pressure() - 1100.0).abs() > 0.0001 {
            return 0;
        }

        1
    }
}
