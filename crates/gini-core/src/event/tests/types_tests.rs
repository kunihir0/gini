
use crate::event::{Event, EventPriority};
use crate::event::types::{SystemEvent, PluginEvent, StageEvent, IssueSeverity};

#[test]
fn test_system_event_properties() {
    let event = SystemEvent::ApplicationStart;

    // Test basic event properties
    assert_eq!(event.name(), "application.start");
    assert_eq!(event.priority(), EventPriority::Critical); // Corrected priority
    assert!(!event.is_cancelable());

    // Test clone_event
    let cloned = event.clone_event();
    assert_eq!(cloned.name(), event.name());

    // Test downcasting
    let any = event.as_any();
    assert!(any.downcast_ref::<SystemEvent>().is_some());
}

#[test]
fn test_system_event_variants() {
    // Test available system event variants
    let events = vec![
        (SystemEvent::ApplicationStart, "application.start"),
        (SystemEvent::ApplicationShutdown, "application.shutdown"),
        (SystemEvent::PluginLoad { plugin_id: "test".to_string() }, "plugin.load"),
        (SystemEvent::PluginLoaded { plugin_id: "test".to_string() }, "plugin.loaded"),
        (SystemEvent::PluginUnload { plugin_id: "test".to_string() }, "plugin.unload"),
        (SystemEvent::StageBegin { stage_id: "test".to_string() }, "stage.begin"),
        (SystemEvent::StageComplete { stage_id: "test".to_string(), success: true }, "stage.complete"),
        (SystemEvent::PipelineBegin { pipeline_id: "test".to_string() }, "pipeline.begin"),
        (SystemEvent::PipelineComplete { pipeline_id: "test".to_string(), success: true }, "pipeline.complete"),
        (SystemEvent::ConfigChange { key: "test_key".to_string(), value: "test_value".to_string() }, "config.change"),
    ];

    for (event, name) in events {
        assert_eq!(event.name(), name);
    }
}

#[test]
fn test_plugin_event_properties() {
    // Create a plugin event struct
    let event = PluginEvent {
        name: "custom.plugin.event".to_string(),
        source: "test-plugin".to_string(),
        data: "some data".to_string(),
        priority: EventPriority::High,
        cancelable: true,
    };

    // Test basic event properties
    // Note: The `name()` method for PluginEvent currently returns a static string "plugin.custom"
    // This might need adjustment in the main code if dynamic names are desired.
    assert_eq!(event.name(), "plugin.custom");
    assert_eq!(event.priority(), EventPriority::High);
    assert!(event.is_cancelable());

    // Test clone_event
    let cloned = event.clone_event();
    assert_eq!(cloned.name(), event.name());
    assert_eq!(cloned.priority(), event.priority());

    // Test downcasting
    let any = event.as_any();
    let downcasted = any.downcast_ref::<PluginEvent>();
    assert!(downcasted.is_some());
    if let Some(e) = downcasted {
        assert_eq!(e.name, "custom.plugin.event"); // Check the actual struct field
        assert_eq!(e.source, "test-plugin");
        assert_eq!(e.data, "some data");
    }
}

#[test]
fn test_stage_event_properties() {
    let event = StageEvent::Progress {
        stage_id: "test-stage".to_string(),
        progress: 0.5,
        message: "Halfway done".to_string(),
    };

    // Test basic event properties
    assert_eq!(event.name(), "stage.progress");
    assert_eq!(event.priority(), EventPriority::Low); // Corrected priority
    assert!(!event.is_cancelable());

    // Test clone_event
    let cloned = event.clone_event();
    assert_eq!(cloned.name(), event.name());

    // Test downcasting
    let any = event.as_any();
    assert!(any.downcast_ref::<StageEvent>().is_some());

    // Check stage_id for specific variant
    if let StageEvent::Progress { stage_id, progress, message } = event {
        assert_eq!(stage_id, "test-stage");
        assert_eq!(progress, 0.5);
        assert_eq!(message, "Halfway done");
    } else {
        panic!("Expected StageEvent::Progress variant");
    }
}

#[test]
fn test_stage_event_variants() {
    // Test stage event variants with corrected variant names and fields
    let events = vec![
        (StageEvent::Progress { stage_id: "test".to_string(), progress: 0.1, message: "Starting".to_string() }, "stage.progress"),
        (StageEvent::UserInput { stage_id: "test".to_string(), prompt: "Choose option".to_string(), options: vec!["A".to_string(), "B".to_string()] }, "stage.input"),
        (StageEvent::Error { stage_id: "test".to_string(), error: "Failed".to_string() }, "stage.error"),
        (StageEvent::Warning { stage_id: "test".to_string(), message: "Be careful".to_string() }, "stage.warning"),
        (StageEvent::CompatibilityIssue { stage_id: "test".to_string(), message: "Incompatible".to_string(), severity: IssueSeverity::Warning }, "stage.compatibility"),
    ];

    for (event, name) in events {
        assert_eq!(event.name(), name);
    }
}