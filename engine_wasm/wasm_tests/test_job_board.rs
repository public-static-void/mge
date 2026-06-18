// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_board() -> i32 {
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
    }

    #[link(wasm_import_module = "job_board")]
    unsafe extern "C" {
        fn get_job_board(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_job_board_policy(out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_job_board_policy(policy_ptr: *const u8, policy_len: i32) -> i32;
        fn get_job_priority(job_id: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_job_priority(job_id: i32, value: i64) -> i32;
        fn add_job_to_job_board(job_id: i32) -> i32;
    }

    unsafe {
        let eid = spawn_entity();

        // Set a Job component so the board has data to show
        let comp_name = "Job";
        let job_json = r#"{"job_type":"mining","state":"pending","progress":0.0,"priority":0}"#;
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            job_json.as_ptr(),
            job_json.len() as i32,
        );

        // Add job to the board
        let add_result = add_job_to_job_board(eid as i32);
        if add_result != 0 {
            return 0;
        }

        // Get the board — should contain the job entry
        let mut board_buf = [0u8; 512];
        let board_written = get_job_board(board_buf.as_mut_ptr(), board_buf.len() as i32);
        if board_written <= 0 {
            return 0;
        }
        let board_str = core::str::from_utf8(&board_buf[..board_written as usize]).unwrap_or("");
        if !board_str.contains("\"eid\":") || !board_str.contains("\"state\":\"pending\"") {
            return 0;
        }

        // Set priority to 5
        let set_prio = set_job_priority(eid as i32, 5);
        if set_prio != 0 {
            return 0;
        }

        // Get priority — should return 5
        let mut prio_buf = [0u8; 32];
        let prio_written = get_job_priority(eid as i32, prio_buf.as_mut_ptr(), prio_buf.len() as i32);
        if prio_written < 0 {
            return 0;
        }
        let prio_str = core::str::from_utf8(&prio_buf[..prio_written as usize]).unwrap_or("");
        if !prio_str.contains("5") {
            return 0;
        }

        // Default policy should be "priority"
        let mut policy_buf = [0u8; 32];
        let policy_written = get_job_board_policy(policy_buf.as_mut_ptr(), policy_buf.len() as i32);
        if policy_written <= 0 {
            return 0;
        }

        // Set policy to "fifo"
        let fifo = "fifo";
        let set_policy = set_job_board_policy(fifo.as_ptr(), fifo.len() as i32);
        if set_policy != 0 {
            return 0;
        }

        // Verify policy changed
        let mut policy_buf2 = [0u8; 32];
        let policy_written2 = get_job_board_policy(policy_buf2.as_mut_ptr(), policy_buf2.len() as i32);
        if policy_written2 <= 0 {
            return 0;
        }
        let policy_str2 = core::str::from_utf8(&policy_buf2[..policy_written2 as usize]).unwrap_or("");
        if policy_str2 != "fifo" {
            return 0;
        }

        // Set policy to "lifo" as well
        let lifo = "lifo";
        let set_lifo = set_job_board_policy(lifo.as_ptr(), lifo.len() as i32);
        if set_lifo != 0 {
            return 0;
        }

        // Set invalid policy — should return -1
        let invalid = "invalid";
        let invalid_result = set_job_board_policy(invalid.as_ptr(), invalid.len() as i32);
        if invalid_result != -1 {
            return 0;
        }

        // Get priority for nonexistent job — should return -1
        let mut dummy_buf = [0u8; 32];
        let missing_prio = get_job_priority(99999, dummy_buf.as_mut_ptr(), dummy_buf.len() as i32);
        if missing_prio != -1 {
            return 0;
        }

        1
    }
}
