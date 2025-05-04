use engine_core::{ColonyHappiness, EcsWorld, Error, Mode, RoguelikeInventory};

#[test]
fn test_mode_switching_and_component_access() {
    let mut world = EcsWorld::new();

    // Register components with mode bindings
    world.register_component::<ColonyHappiness>();
    world.register_component::<RoguelikeInventory>();

    // Start in "colony" mode
    world.set_mode(Mode::Colony);

    let entity = world.spawn();

    // Should succeed: ColonyHappiness is valid in colony mode
    assert!(
        world
            .set_component(entity, ColonyHappiness { base_value: 0.5 })
            .is_ok()
    );

    // Should fail: RoguelikeInventory is not valid in colony mode
    let res = world.set_component(
        entity,
        RoguelikeInventory {
            slots: 5,
            weight: 1.0,
        },
    );
    assert!(matches!(res, Err(Error::ComponentUnavailableInMode)));

    // Switch to "roguelike" mode
    world.set_mode(Mode::Roguelike);

    // Should now succeed for RoguelikeInventory
    assert!(
        world
            .set_component(
                entity,
                RoguelikeInventory {
                    slots: 5,
                    weight: 1.0
                }
            )
            .is_ok()
    );

    // Should now fail for ColonyHappiness
    let res = world.set_component(entity, ColonyHappiness { base_value: 0.5 });
    assert!(matches!(res, Err(Error::ComponentUnavailableInMode)));
}
