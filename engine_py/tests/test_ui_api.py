import pytest

import engine_py


def test_create_button_widget():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "OK", "pos": [1, 2], "color": [255, 255, 255]}
    )
    assert widget_id is not None
    assert widget_id > 0


def test_load_json_button():
    ui = engine_py.UiApi()
    ids = ui.load_json(
        '{"type": "Button", "props": {"label": "Root", "pos": [0,0], "color":'
        " [255,255,255]}}"
    )
    assert isinstance(ids, list)
    assert len(ids) >= 1
    assert ids[0] > 0


def test_remove_widget():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "RemoveMe", "pos": [0, 0], "color": [10, 10, 10]}
    )
    assert widget_id > 0
    removed = ui.remove_widget(widget_id)
    assert removed is True
    removed_again = ui.remove_widget(widget_id)
    assert removed_again is False
    assert ui.remove_widget(999999999) is False


def test_set_widget_props():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "Old", "pos": [0, 0], "color": [1, 2, 3]}
    )
    assert widget_id > 0
    updated = ui.set_widget_props(
        widget_id, {"label": "New", "pos": [5, 6], "color": [7, 8, 9]}
    )
    assert updated is True
    assert ui.set_widget_props(999999999, {"label": "X"}) is False


def test_get_widget_props():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "QueryMe", "pos": [3, 4], "color": [10, 20, 30]}
    )
    props = ui.get_widget_props(widget_id)
    print("props:", props)
    print("type(props):", type(props))
    if props is not None:
        print("props keys:", list(props.keys()))
    assert props is not None
    assert props["label"] == "QueryMe"
    assert isinstance(props["pos"], list)
    assert props["pos"][0] == 3
    assert props["pos"][1] == 4


def test_add_remove_child_panel():
    ui = engine_py.UiApi()
    panel_id = ui.create_widget("Panel", {"pos": [0, 0]})
    btn_id = ui.create_widget(
        "Button", {"label": "Child", "pos": [1, 1], "color": [1, 2, 3]}
    )
    assert ui.add_child(panel_id, btn_id)
    # Now remove it
    assert ui.remove_child(panel_id, btn_id)
    # Remove again should fail
    assert not ui.remove_child(panel_id, btn_id)


def test_set_callback_and_trigger(monkeypatch):
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "CB", "pos": [1, 2], "color": [1, 2, 3]}
    )
    called = {"flag": False}

    def cb(wid):
        print("PYTHON: callback called with wid", wid)
        called["flag"] = wid == widget_id

    ui.set_callback(widget_id, "click", cb)
    ui.trigger_event(widget_id, "click", {"x": 1, "y": 2})
    assert called["flag"]


def test_focus_widget():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "Focus", "pos": [1, 2], "color": [1, 2, 3]}
    )
    assert ui.focus_widget(widget_id)


def test_send_ui_event_alias():
    ui = engine_py.UiApi()
    widget_id = ui.create_widget(
        "Button", {"label": "SendEvent", "pos": [1, 2], "color": [1, 2, 3]}
    )
    called = {"flag": False}

    def cb(wid):
        print("PYTHON: callback called with wid", wid)
        called["flag"] = wid == widget_id

    ui.set_callback(widget_id, "click", cb)
    ui.send_ui_event(widget_id, "click", {"x": 1, "y": 2})
    assert called["flag"]


def test_get_widget_type():
    ui = engine_py.UiApi()
    btn_id = ui.create_widget(
        "Button", {"label": "Test", "pos": [0, 0], "color": [255, 255, 255]}
    )
    wtype = ui.get_widget_type(btn_id)
    assert wtype == "Button"

    panel_id = ui.create_widget("Panel", {"pos": [0, 0]})
    wtype = ui.get_widget_type(panel_id)
    assert wtype == "Panel"

    # Non-existent widget returns None or empty string
    assert ui.get_widget_type(999999) in (None, "")


def test_get_parent_and_child_relationship():
    ui = engine_py.UiApi()
    panel_id = ui.create_widget("Panel", {"pos": [0, 0]})
    btn_id = ui.create_widget(
        "Button", {"label": "Child", "pos": [1, 1], "color": [1, 2, 3]}
    )

    # Initially no parent
    assert ui.get_parent(btn_id) is None

    # Add child
    assert ui.add_child(panel_id, btn_id)

    # Parent should be panel_id now
    assert ui.get_parent(btn_id) == panel_id

    # Remove child
    assert ui.remove_child(panel_id, btn_id)

    # Parent should be None again
    assert ui.get_parent(btn_id) is None


def test_set_and_get_z_order():
    ui = engine_py.UiApi()
    btn_id = ui.create_widget(
        "Button", {"label": "ZOrder", "pos": [0, 0], "color": [255, 255, 255]}
    )

    # Default z-order
    assert ui.get_z_order(btn_id) == 0

    # Set z-order
    assert ui.set_z_order(btn_id, 5)

    # Get updated z-order
    assert ui.get_z_order(btn_id) == 5

    # Non-existent widget returns default or error
    assert ui.get_z_order(999999) == 0
    assert not ui.set_z_order(999999, 10)


def test_dynamic_widget_registration():
    ui = engine_py.UiApi()

    # Register a new widget type "CustomWidget" dynamically
    def custom_widget_ctor(props):
        # For test, create a Button with label "Custom"
        return ui.create_widget("Button", props)

    assert ui.register_widget("CustomWidget", custom_widget_ctor)

    # Create widget of new type
    custom_id = ui.create_widget(
        "CustomWidget", {"label": "Dynamic", "pos": [0, 0], "color": [1, 1, 1]}
    )
    assert custom_id > 0

    # Check type is "CustomWidget"
    assert ui.get_widget_type(custom_id) == "CustomWidget"


def test_remove_callback():
    ui = engine_py.UiApi()
    btn_id = ui.create_widget(
        "Button", {"label": "CB", "pos": [1, 2], "color": [1, 2, 3]}
    )
    called = {"flag": False}

    def cb(wid):
        called["flag"] = True

    ui.set_callback(btn_id, "click", cb)
    ui.trigger_event(btn_id, "click", {"x": 1, "y": 2})
    assert called["flag"]

    # Remove callback
    ui.remove_callback(btn_id, "click")

    called["flag"] = False
    ui.trigger_event(btn_id, "click", {"x": 1, "y": 2})
    assert not called["flag"]


def test_get_children_explicit():
    ui = engine_py.UiApi()
    panel_id = ui.create_widget("Panel", {"pos": [0, 0]})
    btn1_id = ui.create_widget(
        "Button", {"label": "Child1", "pos": [1, 1], "color": [1, 2, 3]}
    )
    btn2_id = ui.create_widget(
        "Button", {"label": "Child2", "pos": [2, 2], "color": [4, 5, 6]}
    )

    assert ui.add_child(panel_id, btn1_id)
    assert ui.add_child(panel_id, btn2_id)

    children = ui.get_children(panel_id)
    assert btn1_id in children
    assert btn2_id in children

    # Remove one child
    assert ui.remove_child(panel_id, btn1_id)
    children = ui.get_children(panel_id)
    assert btn1_id not in children
    assert btn2_id in children
