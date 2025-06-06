// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_entity_api() -> i32 {
    unsafe extern "C" {
        fn spawn_entity() -> u32;
        fn move_entity(entity: u32, dx: f32, dy: f32);
        fn damage_entity(entity: u32, amount: f32);
        fn is_entity_alive(entity: u32) -> bool;
        fn despawn_entity(entity: u32);
    }

    unsafe {
        let eid = spawn_entity();
        move_entity(eid, 1.0, 2.0);
        damage_entity(eid, 10.0);
        let alive = is_entity_alive(eid);
        if !alive {
            return 0;
        }
        despawn_entity(eid);
        1
    }
}
