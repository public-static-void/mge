use engine_core::presentation::ui::factory::{UiFactory, WidgetProps, WidgetConstructor};
use engine_core::presentation::ui::widget::Button;
use engine_core::presentation::renderer::RenderColor;

#[test]
fn test_factory_register_and_create_widget() {
    let mut factory = UiFactory::new();
    let ctor: WidgetConstructor = Box::new(|_props| {
        Box::new(Button::new(
            "Test",
            (0, 0),
            RenderColor(255, 255, 255),
            None,
            None,
        )) as Box<dyn engine_core::presentation::ui::widget::UiWidget + Send>
    });
    factory.register_widget("Button", ctor);
    let widget = factory.create_widget("Button", WidgetProps::new());
    assert!(widget.is_some());
    assert_eq!(widget.unwrap().widget_type(), "Button");
}

#[test]
fn test_factory_create_unregistered_returns_none() {
    let factory = UiFactory::new();
    let widget = factory.create_widget("NonExistent", WidgetProps::new());
    assert!(widget.is_none());
}

#[test]
fn test_factory_has_widget_type() {
    let mut factory = UiFactory::new();
    let ctor: WidgetConstructor = Box::new(|_props| {
        Box::new(Button::new(
            "Test",
            (0, 0),
            RenderColor(255, 255, 255),
            None,
            None,
        )) as Box<dyn engine_core::presentation::ui::widget::UiWidget + Send>
    });
    factory.register_widget("MyWidget", ctor);
    assert!(factory.has_widget_type("MyWidget"));
    assert!(!factory.has_widget_type("NonExistent"));
}

#[test]
fn test_factory_create_with_props() {
    let mut factory = UiFactory::new();
    let ctor: WidgetConstructor = Box::new(|props| {
        let label = props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Default")
            .to_string();
        Box::new(Button::new(
            label,
            (0, 0),
            RenderColor(255, 255, 255),
            None,
            None,
        )) as Box<dyn engine_core::presentation::ui::widget::UiWidget + Send>
    });
    factory.register_widget("DynamicButton", ctor);
    let mut props = WidgetProps::new();
    props.insert(
        "label".to_string(),
        serde_json::Value::String("Custom".to_string()),
    );
    let widget = factory.create_widget("DynamicButton", props).unwrap();
    let btn = widget.as_any().downcast_ref::<Button>().unwrap();
    assert_eq!(btn.label, "Custom");
}

#[test]
fn test_factory_empty_registry_has_no_types() {
    let factory = UiFactory::new();
    assert!(!factory.has_widget_type("Anything"));
}
