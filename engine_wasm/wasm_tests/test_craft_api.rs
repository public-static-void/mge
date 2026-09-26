// This file is compiled to WASM and loaded by the Rust host test harness.
// Crafting bridge parity (ROADMAP L57): the six `craft` host functions with
// identical names, argument order, and error strings as the Lua/Python
// bridges, plus host-driven tick progress (`turn.tick` advances CraftOrders
// through the WASM `tick_craft` mirror) and save/load round-trip.
#[no_mangle]
pub extern "C" fn test_craft_api() -> i32 {
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
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "craft")]
    unsafe extern "C" {
        fn register_craft_recipe(
            name_ptr: *const u8,
            name_len: i32,
            recipe_ptr: *const u8,
            recipe_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn list_craft_recipes(out_ptr: *mut u8, out_len: i32) -> i32;
        fn can_craft(
            crafter: u32,
            recipe_ptr: *const u8,
            recipe_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn start_craft(
            crafter: u32,
            recipe_ptr: *const u8,
            recipe_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_craft_state(crafter: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn cancel_craft(crafter: u32, out_ptr: *mut u8, out_len: i32) -> i32;
    }

    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
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

    fn contains(buf: &[u8], written: i32, needle: &str) -> bool {
        if written <= 0 {
            return false;
        }
        let hay = &buf[..written as usize];
        let ndl = needle.as_bytes();
        if ndl.is_empty() || ndl.len() > hay.len() {
            return false;
        }
        hay.windows(ndl.len()).any(|w| w == ndl)
    }

    unsafe {
        let recipe_name = "iron_sword";
        let recipe_json = "{\"name\":\"iron_sword\",\"inputs\":[{\"kind\":\"iron\",\"amount\":2}],\"outputs\":[],\"duration\":3,\"tools\":[{\"item\":\"hammer\",\"consumed\":false}],\"materials\":[{\"material\":\"iron\",\"amount\":2}],\"required_skill\":{\"skill\":\"crafting\",\"level\":2},\"output_item\":{\"id\":\"iron_sword\",\"name\":\"Iron Sword\",\"slot\":\"hand\"},\"station\":null,\"xp\":12}";

        let mut out = [0u8; 4096];

        // Registration reports the ok envelope and lists the recipe.
        let w = register_craft_recipe(
            recipe_name.as_ptr(),
            recipe_name.len() as i32,
            recipe_json.as_ptr(),
            recipe_json.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }
        let w = list_craft_recipes(out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[\"iron_sword\"]") {
            return 0;
        }

        // Fresh world: the crafter is entity 1.
        let crafter = spawn_entity();
        if crafter != 1 {
            return 0;
        }
        let stock_name = "Stockpile";
        let stock_json = "{\"resources\":{\"iron\":200}}";
        set_component(
            crafter,
            stock_name.as_ptr(),
            stock_name.len() as i32,
            stock_json.as_ptr(),
            stock_json.len() as i32,
        );
        let inv_name = "Inventory";
        let inv_json =
            "{\"slots\":[\"hammer\"],\"max_slots\":10,\"weight\":0.0,\"volume\":0.0}";
        set_component(
            crafter,
            inv_name.as_ptr(),
            inv_name.len() as i32,
            inv_json.as_ptr(),
            inv_json.len() as i32,
        );
        let skill_name = "SkillLevels";
        let skill_json =
            "{\"skills\":{\"crafting\":2},\"total_xp\":0.0,\"skill_xp\":{},\"skill_levels\":{}}";
        set_component(
            crafter,
            skill_name.as_ptr(),
            skill_name.len() as i32,
            skill_json.as_ptr(),
            skill_json.len() as i32,
        );

        // Gate parity: unknown recipe, then the passing gate.
        let ghost = "ghost_recipe";
        let w = can_craft(
            crafter,
            ghost.as_ptr(),
            ghost.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"unknown_recipe\"}") {
            return 0;
        }
        let w = can_craft(
            crafter,
            recipe_name.as_ptr(),
            recipe_name.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }

        // Start consumes inputs+materials and reports the ok envelope.
        let w = start_craft(
            crafter,
            recipe_name.as_ptr(),
            recipe_name.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !bytes_eq(&out, w, "{\"ok\":true,\"err\":null}") {
            return 0;
        }
        let w = start_craft(
            crafter,
            recipe_name.as_ptr(),
            recipe_name.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"already_crafting\"}") {
            return 0;
        }
        let mut stock_buf = [0u8; 512];
        let w = get_component(
            crafter,
            stock_name.as_ptr(),
            stock_name.len() as i32,
            stock_buf.as_mut_ptr(),
            stock_buf.len() as i32,
        );
        if !contains(&stock_buf, w, "\"iron\":196") {
            return 0;
        }

        // Ticks drive progress to completion without a delivery sim.
        tick();
        let w = get_craft_state(crafter, out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"in_progress\"") {
            return 0;
        }
        tick();
        tick();
        let w = get_craft_state(crafter, out.as_mut_ptr(), out.len() as i32);
        if !contains(&out, w, "\"state\":\"complete\"") {
            return 0;
        }
        // The output entity is the second spawn in this fresh world.
        if !contains(&out, w, "\"output_entity\":2") {
            return 0;
        }
        if !contains(&out, w, "\"progress\":3") {
            return 0;
        }

        // Output entity carries Item + Material with the iron key.
        let item_name = "Item";
        let mut item_buf = [0u8; 1024];
        let w = get_component(
            2,
            item_name.as_ptr(),
            item_name.len() as i32,
            item_buf.as_mut_ptr(),
            item_buf.len() as i32,
        );
        if !contains(&item_buf, w, "\"id\":\"iron_sword\"") {
            return 0;
        }
        if !contains(&item_buf, w, "\"material\":\"iron\"") {
            return 0;
        }
        let material_name = "Material";
        let mut mat_buf = [0u8; 512];
        let w = get_component(
            2,
            material_name.as_ptr(),
            material_name.len() as i32,
            mat_buf.as_mut_ptr(),
            mat_buf.len() as i32,
        );
        if !contains(&mat_buf, w, "\"material\":\"iron\"") {
            return 0;
        }

        // Completion emits craft_completed; XP was granted off zero.
        let completed_type = "craft_completed";
        let w = poll_ecs_event(
            completed_type.as_ptr(),
            completed_type.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
        );
        if w <= 0 {
            return 0;
        }
        if !contains(&out, w, "\"xp_gained\"") {
            return 0;
        }
        if !contains(&out, w, "\"output_entity\":2") {
            return 0;
        }
        let mut levels_buf = [0u8; 1024];
        let w = get_component(
            crafter,
            skill_name.as_ptr(),
            skill_name.len() as i32,
            levels_buf.as_mut_ptr(),
            levels_buf.len() as i32,
        );
        if !contains(&levels_buf, w, "\"total_xp\"") {
            return 0;
        }
        if contains(&levels_buf, w, "\"total_xp\":0") {
            return 0;
        }

        // Completed orders are terminal: cancel reports no order.
        let w = cancel_craft(crafter, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"no_craft_order\"}") {
            return 0;
        }
        let stranger = spawn_entity();
        let w = cancel_craft(stranger, out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "{\"ok\":false,\"err\":\"no_craft_order\"}") {
            return 0;
        }

        // Save/load round-trip preserves the completed order byte-identically.
        let w = get_craft_state(crafter, out.as_mut_ptr(), out.len() as i32);
        if w <= 0 {
            return 0;
        }
        let mut before = [0u8; 4096];
        let before_len = w as usize;
        before[..before_len].copy_from_slice(&out[..before_len]);
        let save_path = "/tmp/wasm_craft_test.json";
        save_to_file(save_path.as_ptr(), save_path.len() as i32);
        let _extra = spawn_entity();
        load_from_file(save_path.as_ptr(), save_path.len() as i32);
        let w = get_craft_state(crafter, out.as_mut_ptr(), out.len() as i32);
        if w as usize != before_len || &out[..before_len] != &before[..before_len] {
            return 0;
        }
        // The output entity survives alongside the order.
        let w = get_component(
            2,
            item_name.as_ptr(),
            item_name.len() as i32,
            item_buf.as_mut_ptr(),
            item_buf.len() as i32,
        );
        if !contains(&item_buf, w, "\"id\":\"iron_sword\"") {
            return 0;
        }
        // Recipes survive the trip too.
        let w = list_craft_recipes(out.as_mut_ptr(), out.len() as i32);
        if !bytes_eq(&out, w, "[\"iron_sword\"]") {
            return 0;
        }

        1
    }
}
