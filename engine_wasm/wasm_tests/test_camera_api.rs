// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_camera_api() -> i32 {
    #[link(wasm_import_module = "camera")]
    unsafe extern "C" {
        fn set_camera(x: i32, y: i32, w: i32, h: i32);
        fn get_camera(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // Set camera at x=10, y=20, width=80, height=24
        set_camera(10, 20, 80, 24);

        // Get camera back and verify JSON
        let mut buf = [0u8; 128];
        let written = get_camera(buf.as_mut_ptr(), buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&buf[..written as usize]).unwrap_or("");
        // Expected: {"x":10,"y":20,"width":80,"height":24}
        if result != "{\"x\":10,\"y\":20,\"width\":80,\"height\":24}" {
            return 0;
        }

        1
    }
}
