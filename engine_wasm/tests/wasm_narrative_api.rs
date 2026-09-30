/// Standing suite for the event-driven narrative engine (incident director).
///
/// M1 placeholder: the WASM narrative bridge (register_scenario,
/// list_scenarios, poll_pending_decisions, resolve_decision,
/// get_narrative_history) lands in M5 with its guest test artifact.
/// This shell keeps the harness green until then, exercising the
/// scenario content-file loader path with no guest artifact.

#[test]
fn narrative_content_file_loads_without_breaking_startup() {
    let defs = engine_core::narrative::load_scenario_definitions();
    assert!(
        defs.iter().all(|d| !d.id.is_empty()),
        "loaded scenario definitions must carry non-empty ids"
    );
}
