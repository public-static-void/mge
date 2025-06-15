#[path = "helpers/effect.rs"]
mod effect_helper;

use engine_core::systems::job::effect_processor_registry::EffectProcessorRegistry;
use serde_json::json;

#[test]
fn test_effect_chaining_triggers_another_effect() {
    let (mut world, effect_registry) = effect_helper::setup_world_and_registry();

    world.effect_processor_registry = Some(effect_registry.clone());

    effect_registry
        .lock()
        .unwrap()
        .register_handler("spawn", |world, eid, _effect| {
            println!("Handler 'spawn' called for eid {}", eid);
            let new_effect = json!({"action": "mark", "value": 42});
            let effect_proc = world.effect_processor_registry.as_ref().unwrap().clone();
            EffectProcessorRegistry::process_effects_arc(&effect_proc, world, eid, &[new_effect]);
        });

    effect_registry
        .lock()
        .unwrap()
        .register_handler("mark", |world, eid, effect| {
            println!("Handler 'mark' called for eid {}", eid);
            world
                .set_component(eid, "Marked", json!({"value": effect["value"]}))
                .unwrap();
        });

    let eid = world.spawn_entity();
    let effects = vec![json!({"action": "spawn"})];

    let effect_proc = world.effect_processor_registry.as_ref().unwrap().clone();
    EffectProcessorRegistry::process_effects_arc(&effect_proc, &mut world, eid, &effects);

    let marked = world.get_component(eid, "Marked").unwrap();
    assert_eq!(marked["value"], 42, "Marked component should have value 42");
}

#[test]
fn test_scripted_effect_handler_invoked() {
    let (mut world, effect_registry) = effect_helper::setup_world_and_registry();

    world.effect_processor_registry = Some(effect_registry.clone());

    effect_registry
        .lock()
        .unwrap()
        .register_handler("scripted", |world, eid, effect| {
            println!("Handler 'scripted' called for eid {}", eid);
            world
                .set_component(
                    eid,
                    "Scripted",
                    json!({"ran": true, "param": effect["param"]}),
                )
                .unwrap();
        });

    let eid = world.spawn_entity();
    let effects = vec![json!({"action": "scripted", "param": "test"})];

    let effect_proc = world.effect_processor_registry.as_ref().unwrap().clone();
    EffectProcessorRegistry::process_effects_arc(&effect_proc, &mut world, eid, &effects);

    let scripted = world.get_component(eid, "Scripted").unwrap();
    assert_eq!(
        scripted["ran"], true,
        "Scripted component should have ran=true"
    );
    assert_eq!(
        scripted["param"], "test",
        "Scripted component should have param='test'"
    );
}
