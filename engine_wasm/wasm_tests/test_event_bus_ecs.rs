// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_poll_ecs_event() -> i32 {
    #[link(wasm_import_module = "event_bus")]
    unsafe extern "C" {
        fn send_event(
            type_ptr: *const u8,
            type_len: i32,
            data_ptr: *const u8,
            data_len: i32,
        );
        fn poll_ecs_event(
            type_ptr: *const u8,
            type_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        // Send a test event
        let event_type = "test_event";
        let event_data = r#"{"value":42}"#;
        send_event(
            event_type.as_ptr(),
            event_type.len() as i32,
            event_data.as_ptr(),
            event_data.len() as i32,
        );

        // Poll the event — should return JSON array with the event (bytes written > 0)
        let mut buf = [0u8; 4096];
        let written = poll_ecs_event(
            event_type.as_ptr(),
            event_type.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written <= 0 {
            return 0;
        }

        // Poll again — should return -1 because bus was consumed/removed
        let written2 = poll_ecs_event(
            event_type.as_ptr(),
            event_type.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written2 >= 0 {
            return 0;
        }

        // Poll unknown event type — should return -1 (event type never existed)
        let unknown = "unknown_event";
        let written3 = poll_ecs_event(
            unknown.as_ptr(),
            unknown.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written3 >= 0 {
            return 0;
        }

        1
    }
}
