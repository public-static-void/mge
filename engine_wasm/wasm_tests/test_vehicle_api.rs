// This file is compiled to WASM and loaded by the Rust host test harness.
// Vehicle bridge parity (ROADMAP L56): the five `vehicle` host functions with
// identical names, argument order, and JSON shapes as the Lua/Python bridges.
#[no_mangle]
pub extern "C" fn test_vehicle_api() -> i32 {
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
        fn get_component_schema(
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn add_cell(x: i32, y: i32, z: i32);
        fn add_neighbor(
            from_ptr: *const u8,
            from_len: i32,
            to_ptr: *const u8,
            to_len: i32,
        );
        fn set_cell_metadata(
            cell_ptr: *const u8,
            cell_len: i32,
            meta_ptr: *const u8,
            meta_len: i32,
        );
    }

    #[link(wasm_import_module = "vehicle")]
    unsafe extern "C" {
        fn embark_vehicle(
            vehicle: u32,
            rider: u32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn disembark_vehicle(rider: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn assign_vehicle_path(vehicle: u32, goal_ptr: *const u8, goal_len: i32) -> i32;
        fn get_vehicle_occupants(vehicle: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn is_mounted(rider: u32) -> i32;
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

    #[link(wasm_import_module = "save_load")]
    unsafe extern "C" {
        fn save_to_file(path_ptr: *const u8, path_len: i32);
        fn load_from_file(path_ptr: *const u8, path_len: i32);
    }

    fn bytes_eq(buf: &[u8], written: i32, expected: &str) -> bool {
        written >= 0
            && (written as usize) == expected.len()
            && &buf[..written as usize] == expected.as_bytes()
    }

    unsafe {
        // Vehicle schema must be registered (AC001 bridge part).
        let schema_name = "Vehicle";
        let mut schema_buf = [0u8; 4096];
        let schema_written = get_component_schema(
            schema_name.as_ptr(),
            schema_name.len() as i32,
            schema_buf.as_mut_ptr(),
            schema_buf.len() as i32,
        );
        if schema_written <= 0 {
            return 0;
        }

        // Straight-line corridor along y=0 from x=0 to x=4.
        add_cell(0, 0, 0);
        add_cell(1, 0, 0);
        add_cell(2, 0, 0);
        add_cell(3, 0, 0);
        add_cell(4, 0, 0);
        let cell0 = "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}";
        let cell1 = "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}";
        let cell2 = "{\"Square\":{\"x\":2,\"y\":0,\"z\":0}}";
        let cell3 = "{\"Square\":{\"x\":3,\"y\":0,\"z\":0}}";
        let cell4 = "{\"Square\":{\"x\":4,\"y\":0,\"z\":0}}";
        for (a, b) in [(cell0, cell1), (cell1, cell2), (cell2, cell3), (cell3, cell4)] {
            add_neighbor(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32);
            add_neighbor(b.as_ptr(), b.len() as i32, a.as_ptr(), a.len() as i32);
        }

        // Fresh world: entity ids are deterministic (1, 2, 3, 4).
        let vehicle = spawn_entity();
        let rider_a = spawn_entity();
        let rider_b = spawn_entity();
        let rider_c = spawn_entity();
        if vehicle != 1 || rider_a != 2 || rider_b != 3 || rider_c != 4 {
            return 0;
        }

        let pos_name = "Position";
        let pos_json = "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}";
        for eid in [vehicle, rider_a, rider_b, rider_c] {
            set_component(
                eid,
                pos_name.as_ptr(),
                pos_name.len() as i32,
                pos_json.as_ptr(),
                pos_json.len() as i32,
            );
        }
        let vehicle_name = "Vehicle";
        let vehicle_json = "{\"capacity\":2,\"speed\":1,\"blocked_terrains\":[]}";
        set_component(
            vehicle,
            vehicle_name.as_ptr(),
            vehicle_name.len() as i32,
            vehicle_json.as_ptr(),
            vehicle_json.len() as i32,
        );

        let mut out = [0u8; 4096];

        // Embark rider_a: ok envelope, mounted, occupants [2].
        let w = embark_vehicle(vehicle, rider_a, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }
        if is_mounted(rider_a) != 1 {
            return 0;
        }
        let w = get_vehicle_occupants(vehicle, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[2]") {
            return 0;
        }

        // Embark rider_b: occupants [2,3].
        let w = embark_vehicle(vehicle, rider_b, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }
        let w = get_vehicle_occupants(vehicle, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[2,3]") {
            return 0;
        }

        // Embarked event was emitted (consumed by the poll).
        let embarked_type = "vehicle_embarked";
        let w = poll_ecs_event(
            embarked_type.as_ptr(),
            embarked_type.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if w <= 0 {
            return 0;
        }

        // Capacity is 2: rider_c is rejected with the "full" envelope.
        let w = embark_vehicle(vehicle, rider_c, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"full\"}") {
            return 0;
        }
        if is_mounted(rider_c) != 0 {
            return 0;
        }
        let w = get_vehicle_occupants(vehicle, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[2,3]") {
            return 0;
        }

        // Unknown ids: no_vehicle / no_rider envelopes.
        let w = embark_vehicle(99, rider_a, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"no_vehicle\"}") {
            return 0;
        }
        let w = embark_vehicle(vehicle, 99, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"no_rider\"}") {
            return 0;
        }

        // Disembark rider_a: ok envelope, unmounted, occupants [3].
        let w = disembark_vehicle(rider_a, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }
        if is_mounted(rider_a) != 0 {
            return 0;
        }
        let w = get_vehicle_occupants(vehicle, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[3]") {
            return 0;
        }
        // Disembarking again is a not_mounted rejection.
        let w = disembark_vehicle(rider_a, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"not_mounted\"}") {
            return 0;
        }
        // Re-embark so the save/load round-trip below carries two riders.
        let w = embark_vehicle(vehicle, rider_a, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }

        // Assign across the open corridor: path 0->1->2->3 stores 3 steps.
        let steps = assign_vehicle_path(vehicle, cell3.as_ptr(), cell3.len() as i32);
        if steps != 3 {
            return 0;
        }
        // Unknown vehicle / unreachable goal report -1.
        if assign_vehicle_path(99, cell3.as_ptr(), cell3.len() as i32) != -1 {
            return 0;
        }
        let far = "{\"Square\":{\"x\":9,\"y\":9,\"z\":0}}";
        if assign_vehicle_path(vehicle, far.as_ptr(), far.len() as i32) != -1 {
            return 0;
        }

        // Terrain guard: water at (2,0) truncates the path to the [1] prefix.
        let water_vehicle = "{\"capacity\":2,\"speed\":1,\"blocked_terrains\":[\"water\"],\"occupants\":[2,3]}";
        set_component(
            vehicle,
            vehicle_name.as_ptr(),
            vehicle_name.len() as i32,
            water_vehicle.as_ptr(),
            water_vehicle.len() as i32,
        );
        let water_meta = "{\"terrain\":\"water\"}";
        set_cell_metadata(
            cell2.as_ptr(),
            cell2.len() as i32,
            water_meta.as_ptr(),
            water_meta.len() as i32,
        );
        let steps = assign_vehicle_path(vehicle, cell4.as_ptr(), cell4.len() as i32);
        if steps != 1 {
            return 0;
        }
        // Fully blocked (water at (1,0) too) stores an empty path: 0 steps.
        set_cell_metadata(
            cell1.as_ptr(),
            cell1.len() as i32,
            water_meta.as_ptr(),
            water_meta.len() as i32,
        );
        let steps = assign_vehicle_path(vehicle, cell4.as_ptr(), cell4.len() as i32);
        if steps != 0 {
            return 0;
        }

        // Save/load round-trip preserves the mounted 2-rider vehicle.
        let save_path = "/tmp/wasm_vehicle_test.json";
        save_to_file(save_path.as_ptr(), save_path.len() as i32);
        let _extra = spawn_entity();
        load_from_file(save_path.as_ptr(), save_path.len() as i32);
        let w = get_vehicle_occupants(vehicle, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[2,3]") {
            return 0;
        }
        if is_mounted(rider_a) != 1 || is_mounted(rider_b) != 1 {
            return 0;
        }

        1
    }
}
