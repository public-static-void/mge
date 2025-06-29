use engine_core::presentation::renderer::TestRenderer;
use engine_core::presentation::ui::UiWidget;
use engine_core::presentation::ui::widget::EventLogWidget;
use engine_core::systems::job::system::events::{init_job_event_logger, job_event_logger};
use serde_json::json;

#[test]
fn test_event_log_widget_renders_events() {
    init_job_event_logger();
    let logger = job_event_logger();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let mut widget = EventLogWidget::new(0);
    widget.update();

    let mut renderer = TestRenderer::new();
    widget.render(&mut renderer);

    assert!(
        !renderer.draws.is_empty(),
        "Event log widget should render something"
    );
}

#[test]
fn test_event_log_widget_filters_events() {
    init_job_event_logger();
    let logger = job_event_logger();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let mut widget = EventLogWidget::new(0);
    widget.set_filter("completed".to_string());
    widget.update();

    let mut renderer = TestRenderer::new();
    widget.render(&mut renderer);

    assert!(
        !renderer.draws.is_empty(),
        "Filtered event log should render something"
    );
}
