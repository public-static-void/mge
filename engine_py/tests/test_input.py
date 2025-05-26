def test_get_user_input(monkeypatch, make_world):
    world = make_world()
    # Simulate user entering "foobar"
    monkeypatch.setattr("builtins.input", lambda prompt="": "foobar")
    result = world.get_user_input("Enter something: ")
    assert result == "foobar"
