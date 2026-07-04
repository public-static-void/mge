-- test_skill_stat_integration.lua: Tests for skill and stat integration.
-- Each test gets a fresh world via the test runner.
-- Global functions: spawn_entity, set_component, get_component, tick, run_native_system

local assert = require("assert")

-- 1. Set and get BaseStats component
local function test_set_and_get_base_stats()
    local id = spawn_entity()
    set_component(id, "BaseStats", { strength = 10.0, dexterity = 8.0, intelligence = 6.0 })
    local stats = get_component(id, "BaseStats")
    assert.equals(stats.strength, 10.0, "BaseStats.strength should be 10.0")
    assert.equals(stats.dexterity, 8.0, "BaseStats.dexterity should be 8.0")
    assert.equals(stats.intelligence, 6.0, "BaseStats.intelligence should be 6.0")
end

-- 2. Stats pipeline: BaseStats + EquipmentEffects -> Stats via tick()
local function test_stats_pipeline_via_tick()
    local id = spawn_entity()
    set_component(id, "BaseStats", { strength = 5.0, constitution = 3.0 })
    set_component(id, "EquipmentEffects", { strength = 3.0 })
    tick()
    local stats = get_component(id, "Stats")
    assert.not_nil(stats, "Stats component should exist after tick")
    assert.equals(stats.strength, 8.0, "Stats.strength should be 5 + 3 = 8.0")
end

-- 3. Stats pipeline with no EquipmentEffects
local function test_stats_pipeline_no_effects()
    local id = spawn_entity()
    set_component(id, "BaseStats", { strength = 7.0, constitution = 2.0 })
    tick()
    local stats = get_component(id, "Stats")
    assert.not_nil(stats, "Stats component should exist after tick")
    assert.equals(stats.strength, 7.0, "Stats.strength should equal BaseStats.strength")
end

-- 4. DerivedStats computed via tick()
local function test_derived_stats_via_tick()
    local id = spawn_entity()
    set_component(id, "BaseStats", { strength = 10.0, constitution = 5.0, intelligence = 4.0 })
    tick()
    local derived = get_component(id, "DerivedStats")
    assert.not_nil(derived, "DerivedStats component should exist after tick")
    assert.equals(derived.MaxHP, 150.0, "MaxHP = 100 + 5*10 = 150")
    assert.equals(derived.MeleeDamage, 6.0, "MeleeDamage = 1.0 + 10*0.5 = 6.0")
    assert.equals(derived.CritChance, 0.07, "CritChance = 0.05 + 4*0.005 = 0.07")
end

-- 5. DerivedStats with minimal stats
local function test_derived_stats_minimal()
    local id = spawn_entity()
    set_component(id, "BaseStats", {})
    tick()
    local derived = get_component(id, "DerivedStats")
    assert.not_nil(derived, "DerivedStats should exist even with empty BaseStats")
    -- All stats default to 0, so derived values are baseline
    assert.equals(derived.MaxHP, 100.0, "MaxHP should be 100.0 with no constitution")
    assert.equals(derived.MeleeDamage, 1.0, "MeleeDamage should be 1.0 with no strength")
    assert.equals(derived.CritChance, 0.05, "CritChance should be 0.05 with no intelligence")
end

-- 6. SkillLevels component set/get
local function test_skill_levels_component()
    local id = spawn_entity()
    set_component(id, "SkillLevels", {
        skills = { mining = 3.0, crafting = 2.0 },
        skill_levels = { mining = 3.0, crafting = 2.0 },
        total_xp = 60.0,
        skill_xp = { mining = 40.0, crafting = 20.0 }
    })
    local levels = get_component(id, "SkillLevels")
    assert.not_nil(levels, "SkillLevels component should exist")
    assert.equals(levels.skills.mining, 3.0, "Mining skill should be 3.0")
    assert.equals(levels.skills.crafting, 2.0, "Crafting skill should be 2.0")
    assert.equals(levels.total_xp, 60.0, "Total XP should be 60.0")
end

-- 7. Entities with BaseStats are found by query
local function test_query_entities_with_base_stats()
    local id1 = spawn_entity()
    local id2 = spawn_entity()
    local id3 = spawn_entity()
    set_component(id1, "BaseStats", { strength = 5.0 })
    set_component(id3, "BaseStats", { strength = 8.0 })
    local entities = get_entities_with_component("BaseStats")
    -- Should find 2 of 3 entities
    assert.equals(#entities, 2, "Should find 2 entities with BaseStats")
end

-- 8. Entities with Stats are found by query
local function test_query_entities_with_stats()
    local id1 = spawn_entity()
    local id2 = spawn_entity()
    set_component(id1, "BaseStats", { strength = 5.0 })
    tick()
    local entities = get_entities_with_component("Stats")
    -- Should find that entity has Stats after tick
    assert.is_true(#entities >= 1, "Should find at least 1 entity with Stats")
end

-- 9. Equipment effect aggregation via tick (end-to-end)
local function test_equipment_effects_aggregation_via_tick()
    local eid = spawn_entity()
    set_component(eid, "BaseStats", { strength = 5.0, dexterity = 3.0 })
    set_component(eid, "EquipmentEffects", { strength = 2.0, dexterity = 1.0 })
    tick()
    local stats = get_component(eid, "Stats")
    assert.not_nil(stats, "Stats component should exist")
    assert.equals(stats.strength, 7.0, "Strength should be 5 + 2 = 7")
    assert.equals(stats.dexterity, 4.0, "Dexterity should be 3 + 1 = 4")
end

return {
    test_set_and_get_base_stats = test_set_and_get_base_stats,
    test_stats_pipeline_via_tick = test_stats_pipeline_via_tick,
    test_stats_pipeline_no_effects = test_stats_pipeline_no_effects,
    test_derived_stats_via_tick = test_derived_stats_via_tick,
    test_derived_stats_minimal = test_derived_stats_minimal,
    test_skill_levels_component = test_skill_levels_component,
    test_query_entities_with_base_stats = test_query_entities_with_base_stats,
    test_query_entities_with_stats = test_query_entities_with_stats,
    test_equipment_effects_aggregation_via_tick = test_equipment_effects_aggregation_via_tick,
}
