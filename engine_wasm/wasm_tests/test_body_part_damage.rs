// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_body_part_damage() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
        fn damage_entity(entity: u32, amount: f32);
        fn damage_entity_part(
            entity: u32,
            part_name_ptr: *const u8,
            part_name_len: i32,
            amount: f32,
        );
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
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "body_part_damage")]
    unsafe extern "C" {
        fn process_body_part_damage();
    }

    unsafe {
        let comp_body = "Body";
        let comp_health = "Health";

        // Test AC017: damage_entity distributes across body parts
        let eid = spawn_entity();
        let body_json = "{\"parts\":[{\"name\":\"torso\",\"kind\":\"torso\",\"status\":\"healthy\",\"hp\":50.0,\"max_hp\":50.0,\"temperature\":null,\"ideal_temperature\":null,\"insulation\":null,\"heat_loss\":null,\"children\":[{\"name\":\"left arm\",\"kind\":\"arm\",\"status\":\"healthy\",\"hp\":25.0,\"max_hp\":25.0,\"temperature\":null,\"ideal_temperature\":null,\"insulation\":null,\"heat_loss\":null,\"children\":[{\"name\":\"left hand\",\"kind\":\"hand\",\"status\":\"healthy\",\"hp\":10.0,\"max_hp\":10.0,\"temperature\":null,\"ideal_temperature\":null,\"insulation\":null,\"heat_loss\":null,\"children\":[],\"equipped\":[]}],\"equipped\":[]}],\"equipped\":[]}]}";
        let health_json = "{\"current\":85.0,\"max\":85.0}";

        set_component(
            eid,
            comp_body.as_ptr(),
            comp_body.len() as i32,
            body_json.as_ptr(),
            body_json.len() as i32,
        );
        set_component(
            eid,
            comp_health.as_ptr(),
            comp_health.len() as i32,
            health_json.as_ptr(),
            health_json.len() as i32,
        );

        damage_entity(eid, 85.0);
        process_body_part_damage();

        let mut buf = [0u8; 4096];
        let written = get_component(
            eid,
            comp_body.as_ptr(),
            comp_body.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written <= 0 {
            return 0;
        }

        // Test AC018: damage_entity_part targets specific part
        let eid2 = spawn_entity();
        set_component(
            eid2,
            comp_body.as_ptr(),
            comp_body.len() as i32,
            body_json.as_ptr(),
            body_json.len() as i32,
        );
        set_component(
            eid2,
            comp_health.as_ptr(),
            comp_health.len() as i32,
            health_json.as_ptr(),
            health_json.len() as i32,
        );

        let part_name = "left hand";
        damage_entity_part(eid2, part_name.as_ptr(), part_name.len() as i32, 5.0);
        process_body_part_damage();

        let mut buf2 = [0u8; 4096];
        let written2 = get_component(
            eid2,
            comp_body.as_ptr(),
            comp_body.len() as i32,
            buf2.as_mut_ptr(),
            buf2.len() as i32,
        );
        if written2 <= 0 {
            return 0;
        }

        1
    }
}
