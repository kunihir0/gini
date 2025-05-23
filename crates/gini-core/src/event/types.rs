use std::any::Any;
use crate::event::{Event, EventPriority};

/// System events triggered by the core application
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Application is starting
    ApplicationStart,
    /// Application is shutting down
    ApplicationShutdown,
    /// Plugin is being loaded
    PluginLoad { plugin_id: String },
    /// Plugin has been loaded
    PluginLoaded { plugin_id: String },
    /// Plugin is being unloaded
    PluginUnload { plugin_id: String },
    /// Stage is beginning execution
    StageBegin { stage_id: String },
    /// Stage has completed execution
    StageComplete { stage_id: String, success: bool },
    /// Pipeline execution is beginning
    PipelineBegin { pipeline_id: String },
    /// Pipeline execution has completed
    PipelineComplete { pipeline_id: String, success: bool },
    /// Configuration has changed
    ConfigChange { key: String, value: String },
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct TestEvent {
    name: String,
}

#[cfg(test)]
impl TestEvent {
    pub fn new(name: &str) -> Self {
        TestEvent { name: name.to_string() }
    }
}

#[cfg(test)]
impl crate::event::Event for TestEvent {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
    
    fn clone_event(&self) -> Box<dyn crate::event::Event> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Event for SystemEvent {
    fn name(&self) -> &'static str {
        match self {
            SystemEvent::ApplicationStart => "application.start",
            SystemEvent::ApplicationShutdown => "application.shutdown",
            SystemEvent::PluginLoad { .. } => "plugin.load",
            SystemEvent::PluginLoaded { .. } => "plugin.loaded",
            SystemEvent::PluginUnload { .. } => "plugin.unload",
            SystemEvent::StageBegin { .. } => "stage.begin",
            SystemEvent::StageComplete { .. } => "stage.complete",
            SystemEvent::PipelineBegin { .. } => "pipeline.begin",
            SystemEvent::PipelineComplete { .. } => "pipeline.complete",
            SystemEvent::ConfigChange { .. } => "config.change",
        }
    }
    
    fn priority(&self) -> EventPriority {
        match self {
            SystemEvent::ApplicationStart |
            SystemEvent::ApplicationShutdown => EventPriority::Critical,
            _ => EventPriority::Normal,
        }
    }
    
    fn is_cancelable(&self) -> bool {
        match self {
            SystemEvent::ApplicationShutdown |
            SystemEvent::PluginUnload { .. } => true,
            _ => false,
        }
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Plugin-specific events
#[derive(Debug, Clone)]
pub struct PluginEvent {
    /// Name of the event
    pub name: String,
    /// Source plugin identifier
    pub source: String,
    /// Event payload (any serializable data)
    pub data: String,
    /// Event priority
    pub priority: EventPriority,
    /// Whether this event can be cancelled
    pub cancelable: bool,
}

impl Event for PluginEvent {
    fn name(&self) -> &'static str {
        // This is not ideal, but we can't really return a reference to the owned String
        // In practice, plugin events would need a different approach
        "plugin.custom"
    }
    
    fn priority(&self) -> EventPriority {
        self.priority
    }
    
    fn is_cancelable(&self) -> bool {
        self.cancelable
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Stage-specific events
#[derive(Debug, Clone)]
pub enum StageEvent {
    /// Stage has a progress update
    Progress { stage_id: String, progress: f32, message: String },
    /// Stage requires user input
    UserInput { stage_id: String, prompt: String, options: Vec<String> },
    /// Stage has encountered an error
    Error { stage_id: String, error: String },
    /// Stage is emitting a warning
    Warning { stage_id: String, message: String },
    /// Stage has found a compatibility issue
    CompatibilityIssue { stage_id: String, message: String, severity: IssueSeverity },
}

/// Severity of an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational issue
    Info,
    /// Warning issue
    Warning,
    /// Error issue
    Error,
    /// Critical issue
    Critical,
}

impl Event for StageEvent {
    fn name(&self) -> &'static str {
        match self {
            StageEvent::Progress { .. } => "stage.progress",
            StageEvent::UserInput { .. } => "stage.input",
            StageEvent::Error { .. } => "stage.error",
            StageEvent::Warning { .. } => "stage.warning",
            StageEvent::CompatibilityIssue { .. } => "stage.compatibility",
        }
    }
    
    fn priority(&self) -> EventPriority {
        match self {
            StageEvent::Error { .. } => EventPriority::High,
            StageEvent::UserInput { .. } => EventPriority::High,
            StageEvent::CompatibilityIssue { severity, .. } => match severity {
                IssueSeverity::Critical | IssueSeverity::Error => EventPriority::High,
                _ => EventPriority::Normal,
            },
            StageEvent::Warning { .. } => EventPriority::Normal,
            StageEvent::Progress { .. } => EventPriority::Low,
        }
    }
    
    fn is_cancelable(&self) -> bool {
        match self {
            StageEvent::UserInput { .. } => true,
            _ => false,
        }
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
/// Trait for events that have a specific type identifier and description.
/// This is a new trait defined here as per the requirements.
pub trait EventType {
    /// Returns a unique string identifier for this event type.
    fn event_type_id(&self) -> &'static str;
    /// Returns a brief description of what this event signifies.
    fn description(&self) -> &'static str;
}

/// Event fired when a pipeline has completed execution.
/// This event is intended for use by plugins like core-rpc for dynamic updates.
#[derive(Debug, Clone)]
pub struct PipelineExecutionCompletedEvent {
    pub pipeline_name: String,
    pub success: bool,
    pub timestamp: std::time::SystemTime,
}

impl EventType for PipelineExecutionCompletedEvent {
    fn event_type_id(&self) -> &'static str {
        "gini.core.pipeline.execution_completed"
    }

    fn description(&self) -> &'static str {
        "Fired when a stage pipeline has completed execution."
    }
}

impl Event for PipelineExecutionCompletedEvent {
    fn name(&self) -> &'static str {
        // It's common for the Event::name() to align with an event_type_id if one exists.
        "gini.core.pipeline.execution_completed"
    }

    fn priority(&self) -> EventPriority {
        EventPriority::Normal // Default priority for informational events.
    }

    fn is_cancelable(&self) -> bool {
        false // Completion events are typically not cancelable.
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    // Note: The `Event` trait (from crate::event::Event) does not currently include `timestamp()`.
    // The `timestamp` is available as a public field on `PipelineExecutionCompletedEvent`.
}

/// Event fired when a "ping" command is received from user input.
#[derive(Debug, Clone)]
pub struct PingCommandEvent {
    pub source_id: Option<String>, // Optional: Carry the source_id from UserInput
}

impl EventType for PingCommandEvent {
    fn event_type_id(&self) -> &'static str {
        "gini.core.ui.ping_command"
    }

    fn description(&self) -> &'static str {
        "Fired when a 'ping' command is received via user input."
    }
}

impl Event for PingCommandEvent {
    fn name(&self) -> &'static str {
        "gini.core.ui.ping_command"
    }

    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }

    fn is_cancelable(&self) -> bool {
        false
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}