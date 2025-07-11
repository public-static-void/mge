use engine_core::ecs::world::World;
use engine_core::systems::job::system::events::job_event_logger;
use mlua::{Function, Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Type alias for job event subscriptions: event_type -> Vec<(id, Lua callback)>
type JobEventSubscriptions = HashMap<String, Vec<(usize, mlua::Function)>>;

// Global registry of job event bus subscriptions.
thread_local! {
    static JOB_EVENT_SUBSCRIPTIONS: RefCell<JobEventSubscriptions> = RefCell::new(HashMap::new());
    static SUBSCRIPTION_ID_COUNTER: RefCell<usize> = const { RefCell::new(0) };
}

/// Register the job_events API table in the provided Lua context.
/// All job event bus functions are exposed as methods of the `job_events` table.
pub fn register_job_event_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    let job_events = lua.create_table()?;

    // get_log: returns all job events as a Lua array of tables
    job_events.set(
        "get_log",
        lua.create_function(|lua, ()| {
            let events = job_event_logger().all();
            let lua_events = lua.create_table()?;
            for (i, event) in events.iter().enumerate() {
                let e = lua.create_table()?;
                e.set("timestamp", event.timestamp)?;
                e.set("event_type", event.event_type.clone())?;
                e.set("payload", serde_json::to_string(&event.payload).unwrap())?;
                lua_events.set(i + 1, e)?;
            }
            Ok(lua_events)
        })?,
    )?;

    // get_by_type: returns job events of a specific type
    job_events.set(
        "get_by_type",
        lua.create_function(|lua, event_type: String| {
            let events = job_event_logger().get_events_by_type(&event_type);
            let lua_events = lua.create_table()?;
            for (i, event) in events.iter().enumerate() {
                let e = lua.create_table()?;
                e.set("timestamp", event.timestamp)?;
                e.set("event_type", event.event_type.clone())?;
                e.set("payload", serde_json::to_string(&event.payload).unwrap())?;
                lua_events.set(i + 1, e)?;
            }
            Ok(lua_events)
        })?,
    )?;

    // get_since: returns job events since a given timestamp
    job_events.set(
        "get_since",
        lua.create_function(|lua, timestamp: u128| {
            let events = job_event_logger().get_events_since(timestamp);
            let lua_events = lua.create_table()?;
            for (i, event) in events.iter().enumerate() {
                let e = lua.create_table()?;
                e.set("timestamp", event.timestamp)?;
                e.set("event_type", event.event_type.clone())?;
                e.set("payload", serde_json::to_string(&event.payload).unwrap())?;
                lua_events.set(i + 1, e)?;
            }
            Ok(lua_events)
        })?,
    )?;

    // poll_bus: poll job event bus for a specific event type
    let world_ref = world.clone();
    job_events.set(
        "poll_bus",
        lua.create_function(move |lua, event_type: String| {
            let mut world = world_ref.borrow_mut();
            let events = world.take_events(&event_type);
            let lua_events = lua.create_table()?;
            for (i, event) in events.iter().enumerate() {
                let e = lua.create_table()?;
                e.set("event_type", event_type.clone())?;
                e.set("payload", serde_json::to_string(event).unwrap())?;
                lua_events.set(i + 1, e)?;
            }
            Ok(lua_events)
        })?,
    )?;

    // subscribe_bus: subscribe to job event bus for a specific event type
    job_events.set(
        "subscribe_bus",
        lua.create_function(|_, (event_type, callback): (String, Function)| {
            let id = SUBSCRIPTION_ID_COUNTER.with(|counter| {
                let mut c = counter.borrow_mut();
                *c += 1;
                *c
            });
            JOB_EVENT_SUBSCRIPTIONS.with(|subs| {
                let mut subs = subs.borrow_mut();
                subs.entry(event_type).or_default().push((id, callback));
            });
            Ok(id)
        })?,
    )?;

    // unsubscribe_bus: unsubscribe from job event bus by id
    job_events.set(
        "unsubscribe_bus",
        lua.create_function(|_, (event_type, sub_id): (String, usize)| {
            JOB_EVENT_SUBSCRIPTIONS.with(|subs| {
                let mut subs = subs.borrow_mut();
                if let Some(vec) = subs.get_mut(&event_type) {
                    vec.retain(|(id, _)| *id != sub_id);
                }
            });
            Ok(())
        })?,
    )?;

    // deliver_callbacks: deliver all pending job events to Lua subscribers (should be called after each tick)
    let world_ref = world.clone();
    job_events.set(
        "deliver_callbacks",
        lua.create_function(move |lua, ()| {
            let mut events_by_type: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            JOB_EVENT_SUBSCRIPTIONS.with(|subs| {
                let subs = subs.borrow();
                let mut world = world_ref.borrow_mut();
                for event_type in subs.keys() {
                    let events = world.take_events(event_type);
                    if !events.is_empty() {
                        events_by_type.insert(event_type.to_string(), events);
                    }
                }
                for (event_type, callbacks) in subs.iter() {
                    if let Some(events) = events_by_type.get(event_type) {
                        for event in events {
                            for (_id, cb) in callbacks {
                                let e = lua.create_table().unwrap();
                                e.set("event_type", event_type.clone()).unwrap();
                                e.set("payload", serde_json::to_string(event).unwrap())
                                    .unwrap();
                                cb.call::<()>(e).unwrap();
                            }
                        }
                    }
                }
            });
            Ok(())
        })?,
    )?;

    // save: save the job event log to a file
    job_events.set(
        "save",
        lua.create_function(|_, path: String| {
            engine_core::systems::job::system::events::save_job_event_log(&path)
                .map_err(|e| mlua::Error::external(format!("Failed to save event log: {e}")))?;
            Ok(())
        })?,
    )?;

    // load: load the job event log from a file
    job_events.set(
        "load",
        lua.create_function(|_, path: String| {
            engine_core::systems::job::system::events::load_job_event_log(&path)
                .map_err(|e| mlua::Error::external(format!("Failed to load event log: {e}")))?;
            Ok(())
        })?,
    )?;

    // replay: replay the job event log into the world
    let world_ref = world.clone();
    job_events.set(
        "replay",
        lua.create_function(move |_, ()| {
            let mut world = world_ref.borrow_mut();
            engine_core::systems::job::system::events::replay_job_event_log(&mut world);
            Ok(())
        })?,
    )?;

    // clear: clear the job event log
    job_events.set(
        "clear",
        lua.create_function(|_, ()| {
            job_event_logger().clear();
            Ok(())
        })?,
    )?;

    // Register the job_events table as a global
    globals.set("job_events", job_events)?;
    Ok(())
}
