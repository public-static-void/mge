// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the skill/stat API: BaseStats, Stats, DerivedStats, SkillLevels component roundtrips.
// Note: WASM environment does NOT run simulation systems, so we test component CRUD only.
#[no_mangle]
pub extern "C" fn test_skill_stat_api() -> i32 {
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

    unsafe {
        // Test 1: Set BaseStats and verify via get_component
        let eid = spawn_entity();
        let comp_name = "BaseStats";
        let json_data = r#"{"strength":10.0,"dexterity":8.0,"intelligence":6.0}"#;
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            json_data.as_ptr(),
            json_data.len() as i32,
        );

        let mut out_buf = [0u8; 256];
        let written = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written < 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if !result.contains("strength") {
            return 0;
        }

        // Test 2: Set Stats component directly and verify
        let eid2 = spawn_entity();
        let stats_name = "Stats";
        let stats_data = r#"{"strength":8.0,"dexterity":4.0,"intelligence":3.0,"constitution":5.0}"#;
        set_component(
            eid2,
            stats_name.as_ptr(),
            stats_name.len() as i32,
            stats_data.as_ptr(),
            stats_data.len() as i32,
        );

        let mut stats_buf = [0u8; 256];
        let stats_written = get_component(
            eid2,
            stats_name.as_ptr(),
            stats_name.len() as i32,
            stats_buf.as_mut_ptr(),
            stats_buf.len() as i32,
        );
        if stats_written < 0 {
            return 0;
        }
        let stats_result =
            core::str::from_utf8(&stats_buf[..stats_written as usize]).unwrap_or("");
        if !stats_result.contains("\"strength\":8.0") && !stats_result.contains("\"strength\":8") {
            return 0;
        }

        // Test 3: Set DerivedStats component directly and verify
        let derived_name = "DerivedStats";
        let derived_data = r#"{"MaxHP":150.0,"MeleeDamage":5.0,"CritChance":0.07}"#;
        set_component(
            eid2,
            derived_name.as_ptr(),
            derived_name.len() as i32,
            derived_data.as_ptr(),
            derived_data.len() as i32,
        );

        let mut derived_buf = [0u8; 256];
        let derived_written = get_component(
            eid2,
            derived_name.as_ptr(),
            derived_name.len() as i32,
            derived_buf.as_mut_ptr(),
            derived_buf.len() as i32,
        );
        if derived_written < 0 {
            return 0;
        }
        let derived_result =
            core::str::from_utf8(&derived_buf[..derived_written as usize]).unwrap_or("");
        if !derived_result.contains("\"MaxHP\":150.0") && !derived_result.contains("\"MaxHP\":150")
        {
            return 0;
        }

        // Test 4: SkillLevels component set/get
        let eid3 = spawn_entity();
        let skill_data = r#"{"skills":{"mining":3.0},"skill_levels":{"mining":3.0},"total_xp":40.0,"skill_xp":{"mining":40.0}}"#;
        set_component(
            eid3,
            "SkillLevels".as_ptr(),
            "SkillLevels".len() as i32,
            skill_data.as_ptr(),
            skill_data.len() as i32,
        );

        let mut skill_buf = [0u8; 256];
        let skill_written = get_component(
            eid3,
            "SkillLevels".as_ptr(),
            "SkillLevels".len() as i32,
            skill_buf.as_mut_ptr(),
            skill_buf.len() as i32,
        );
        if skill_written < 0 {
            return 0;
        }
        let skill_result =
            core::str::from_utf8(&skill_buf[..skill_written as usize]).unwrap_or("");
        if !skill_result.contains("mining") {
            return 0;
        }
        if !skill_result.contains("\"total_xp\":40.0") && !skill_result.contains("\"total_xp\":40")
        {
            return 0;
        }

        // Test 5: Remove BaseStats component
        let remove_name = "BaseStats";
        #[link(wasm_import_module = "component")]
        unsafe extern "C" {
            fn remove_component(entity: u32, name_ptr: *const u8, name_len: i32);
        }
        remove_component(eid, remove_name.as_ptr(), remove_name.len() as i32);

        let mut removed_buf = [0u8; 64];
        let removed_written = get_component(
            eid,
            remove_name.as_ptr(),
            remove_name.len() as i32,
            removed_buf.as_mut_ptr(),
            removed_buf.len() as i32,
        );
        if removed_written != -1 {
            return 0;
        }

        1
    }
}
