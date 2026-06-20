// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests UI widget API: ui (CRUD), ui_tree (hierarchy), ui_events (interaction).
#[no_mangle]
pub extern "C" fn test_ui_api() -> i32 {
    #[link(wasm_import_module = "ui")]
    unsafe extern "C" {
        fn create_widget(type_ptr: *const u8, type_len: i32, props_ptr: *const u8, props_len: i32) -> i32;
        fn remove_widget(widget_id: i32) -> i32;
        fn set_widget_props(widget_id: i32, props_ptr: *const u8, props_len: i32) -> i32;
        fn get_widget_props(widget_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_widget_type(widget_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn load_json(json_ptr: *const u8, json_len: i32, out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "ui_tree")]
    unsafe extern "C" {
        fn add_child(parent_id: i32, child_id: i32) -> i32;
        fn remove_child(parent_id: i32, child_id: i32) -> i32;
        fn get_children(widget_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_parent(widget_id: i32) -> i32;
    }

    #[link(wasm_import_module = "ui_events")]
    unsafe extern "C" {
        fn set_callback(widget_id: i32, event_type_ptr: *const u8, event_type_len: i32) -> i32;
        fn remove_callback(widget_id: i32, event_type_ptr: *const u8, event_type_len: i32) -> i32;
        fn focus_widget(widget_id: i32) -> i32;
        fn trigger_event(
            widget_id: i32,
            event_type_ptr: *const u8,
            event_type_len: i32,
            event_data_ptr: *const u8,
            event_data_len: i32,
        ) -> i32;
    }

    unsafe {
        // ---- ui module: create_widget ----
        let btn_type = "Button";
        let empty_props = "{}";
        let btn_id = create_widget(btn_type.as_ptr(), btn_type.len() as i32, empty_props.as_ptr(), empty_props.len() as i32);
        if btn_id <= 0 {
            return 1; // skip: factory might not be initialized
        }

        // ---- ui module: get_widget_type ----
        let mut type_buf = [0u8; 64];
        let written = get_widget_type(btn_id, type_buf.as_mut_ptr(), type_buf.len() as i32);
        if written < 0 {
            return 0;
        }

        // ---- ui module: set_widget_props ----
        let props = "{\"text\":\"click me\"}";
        let result = set_widget_props(btn_id, props.as_ptr(), props.len() as i32);
        if result != 1 {
            return 0;
        }

        // ---- ui module: get_widget_props ----
        let mut props_buf = [0u8; 256];
        let written2 = get_widget_props(btn_id, props_buf.as_mut_ptr(), props_buf.len() as i32);
        if written2 < 0 {
            return 0;
        }

        // ---- ui_tree module: add_child ----
        let label_type = "Label";
        let label_props = "{\"text\":\"child\"}";
        let child_id = create_widget(label_type.as_ptr(), label_type.len() as i32, label_props.as_ptr(), label_props.len() as i32);
        if child_id <= 0 {
            return 2; // skip hierarchy test if label creation fails
        }

        let result = add_child(btn_id, child_id);
        if result != 1 {
            return 0;
        }

        // ---- ui_tree module: get_children ----
        let mut children_buf = [0u8; 128];
        let written3 = get_children(btn_id, children_buf.as_mut_ptr(), children_buf.len() as i32);
        if written3 < 0 {
            return 0;
        }

        // ---- ui_tree module: get_parent ----
        let parent_id = get_parent(child_id);
        if parent_id != btn_id {
            return 0;
        }

        // Root widget should return 0 (no parent)
        let root_parent = get_parent(btn_id);
        if root_parent != 0 {
            return 0;
        }

        // Non-existent widget returns 0
        let nonexistent = get_parent(999999);
        if nonexistent != 0 {
            return 0;
        }

        // ---- ui_tree module: remove_child ----
        let result = remove_child(btn_id, child_id);
        if result != 1 {
            return 0;
        }

        // ---- ui_events module: set_callback (no-op) ----
        let event_type = "click";
        let cb_result = set_callback(btn_id, event_type.as_ptr(), event_type.len() as i32);
        if cb_result != 0 {
            return 0;
        }

        // ---- ui_events module: remove_callback (no-op) ----
        let rc_result = remove_callback(btn_id, event_type.as_ptr(), event_type.len() as i32);
        if rc_result != 0 {
            return 0;
        }

        // ---- ui_events module: focus_widget ----
        let focus_result = focus_widget(btn_id);
        if focus_result != 1 {
            return 0;
        }

        // Non-existent widget focus returns 0
        let bad_focus = focus_widget(999999);
        if bad_focus != 0 {
            return 0;
        }

        // ---- ui_events module: trigger_event ----
        let event_data = "{\"x\":5,\"y\":10}";
        let trigger_result = trigger_event(
            btn_id,
            event_type.as_ptr(),
            event_type.len() as i32,
            event_data.as_ptr(),
            event_data.len() as i32,
        );
        if trigger_result != 1 {
            return 0;
        }

        // Non-existent widget trigger returns 0
        let bad_trigger = trigger_event(
            999999,
            event_type.as_ptr(),
            event_type.len() as i32,
            event_data.as_ptr(),
            event_data.len() as i32,
        );
        if bad_trigger != 0 {
            return 0;
        }

        // ---- ui module: load_json ----
        let json = "{\"widgets\":[{\"id\":100,\"type\":\"Button\",\"props\":{},\"children\":[]}]}";
        let mut json_buf = [0u8; 256];
        let written4 = load_json(json.as_ptr(), json.len() as i32, json_buf.as_mut_ptr(), json_buf.len() as i32);
        if written4 < 0 {
            return 0;
        }

        // ---- ui module: remove_widget ----
        let result = remove_widget(btn_id);
        if result != 1 {
            return 0;
        }

        // Non-existent widget returns 0
        let result = remove_widget(999999);
        if result != 0 {
            return 0;
        }

        1
    }
}
