use engine_core::systems::job::JobSystem;

#[test]
fn test_lua_dynamic_job_registration_and_completion() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::ComponentSchema;
    use engine_core::scripting::ScriptEngine;
    use engine_core::scripting::world::World;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    // --- Load the Job schema into the registry ---
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("Failed to find project root")
            .join("engine")
            .join("assets")
            .join("schemas")
            .join("job.json");
        let schema_file = std::fs::read_to_string(&schema_path)
            .unwrap_or_else(|_| panic!("Could not find schema at {:?}", schema_path));
        let schema_json: serde_json::Value = serde_json::from_str(&schema_file).unwrap();
        let wrapped = serde_json::json!({
            "name": schema_json["name"].clone(),
            "modes": schema_json["modes"].clone(),
            "schema": schema_json.clone()
        });
        let component_schema: ComponentSchema = serde_json::from_value(wrapped).unwrap();
        reg.register_external_schema(component_schema);
    }
    let world = Rc::new(RefCell::new(World::new(registry)));
    world.borrow_mut().register_system(JobSystem::default());
    let mut engine = ScriptEngine::new();
    engine.register_world(world.clone()).unwrap();

    let code = r#"
        set_mode("colony")
        eid = spawn_entity()
        register_job_type("LuaJob", function(job, progress)
            if job.status == "pending" then
                job.status = "in_progress"
            elseif job.status == "in_progress" then
                job.progress = (job.progress or 0) + 1
                if job.progress >= 2 then
                    job.status = "complete"
                end
            end
            return job
        end)
        assign_job(eid, "LuaJob")
        for i=1,4 do
            run_native_system("JobSystem")
        end
        local job = get_component(eid, "Job")
        print("DEBUG: job.status =", job.status, "job.progress =", job.progress)
        assert(job.status == "complete", "Job should be complete")
    "#;
    engine.run_script(code).unwrap();
}

#[test]
fn test_lua_dynamic_system_registration_and_run() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::scripting::ScriptEngine;
    use engine_core::scripting::world::World;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = Rc::new(RefCell::new(World::new(registry)));
    world.borrow_mut().register_system(JobSystem::default());
    let mut engine = ScriptEngine::new();
    engine.register_world(world.clone()).unwrap();

    let code = r#"
        called = { value = false }
        register_system("LuaSystem", function(dt)
            called.value = true
        end)
        run_system("LuaSystem")
        assert(called.value, "Lua system should have been called")
    "#;
    engine.run_script(code).unwrap();
}
