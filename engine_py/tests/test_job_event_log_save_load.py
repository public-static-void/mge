def test_job_event_log_save_and_load(make_world, tmp_path):
    world = make_world()
    e1 = world.spawn_entity()
    world.assign_job(e1, "TestJob", state="pending", category="test")
    world.advance_job_state(e1)
    events_before = world.get_job_event_log()
    assert isinstance(events_before, list)
    assert len(events_before) > 0

    # Save the event log to a file
    log_path = tmp_path / "test_job_event_log.json"
    world.save_job_event_log(str(log_path))

    # Clear the event log (simulate fresh session)
    world.clear_job_event_log()
    events_cleared = world.get_job_event_log()

    assert isinstance(events_cleared, list)
    assert len(events_cleared) == 0

    # Load the event log from file
    world.load_job_event_log(str(log_path))
    events_loaded = world.get_job_event_log()
    assert len(events_loaded) == len(events_before)

    # Replay the event log (should not error)
    world.replay_job_event_log()
