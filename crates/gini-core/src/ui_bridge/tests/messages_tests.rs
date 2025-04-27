use std::time::SystemTime;

use crate::ui_bridge::{MessageSeverity, UiMessage, UiUpdateType};
use crate::ui_bridge::messages::{MessageResult, UiMessageType};
use crate::ui_bridge::messages::util;

#[test]
fn test_message_result_display() {
    // Test all variants of MessageResult for Display implementation
    let test_cases = vec![
        (MessageResult::None, "<no response>"),
        (MessageResult::Text("Hello".to_string()), "Hello"),
        (MessageResult::Boolean(true), "true"),
        (MessageResult::Boolean(false), "false"),
        (MessageResult::Number(42.5), "42.5"),
        (MessageResult::Error("Test error".to_string()), "Error: Test error"),
    ];
    
    for (result, expected) in test_cases {
        assert_eq!(result.to_string(), expected);
    }
}

#[test]
fn test_ui_message_type() {
    // Test UiMessageType variants
    let command = UiMessageType::Command("test_command".to_string());
    let query = UiMessageType::Query("test_query".to_string());
    let response = UiMessageType::Response("test_response".to_string());
    let event = UiMessageType::Event("test_event".to_string(), MessageSeverity::Info);
    
    match command {
        UiMessageType::Command(cmd) => assert_eq!(cmd, "test_command"),
        _ => panic!("Expected Command variant"),
    }
    
    match query {
        UiMessageType::Query(q) => assert_eq!(q, "test_query"),
        _ => panic!("Expected Query variant"),
    }
    
    match response {
        UiMessageType::Response(r) => assert_eq!(r, "test_response"),
        _ => panic!("Expected Response variant"),
    }
    
    match event {
        UiMessageType::Event(e, severity) => {
            assert_eq!(e, "test_event");
            assert_eq!(severity, MessageSeverity::Info);
        },
        _ => panic!("Expected Event variant"),
    }
}

#[test]
fn test_util_info_message() {
    let source = "test_source";
    let message = "test info message";
    
    let ui_message = util::info(source, message);
    
    assert_eq!(ui_message.source, source);
    
    match ui_message.update_type {
        UiUpdateType::Log(content, severity) => {
            assert_eq!(content, message);
            assert_eq!(severity, MessageSeverity::Info);
        },
        _ => panic!("Expected Log update type"),
    }
}

#[test]
fn test_util_warning_message() {
    let source = "test_source";
    let message = "test warning message";
    
    let ui_message = util::warning(source, message);
    
    assert_eq!(ui_message.source, source);
    
    match ui_message.update_type {
        UiUpdateType::Log(content, severity) => {
            assert_eq!(content, message);
            assert_eq!(severity, MessageSeverity::Warning);
        },
        _ => panic!("Expected Log update type"),
    }
}

#[test]
fn test_util_error_message() {
    let source = "test_source";
    let message = "test error message";
    
    let ui_message = util::error(source, message);
    
    assert_eq!(ui_message.source, source);
    
    match ui_message.update_type {
        UiUpdateType::Log(content, severity) => {
            assert_eq!(content, message);
            assert_eq!(severity, MessageSeverity::Error);
        },
        _ => panic!("Expected Log update type"),
    }
}

#[test]
fn test_util_status_message() {
    let source = "test_source";
    let message = "test status message";
    
    let ui_message = util::status(source, message);
    
    assert_eq!(ui_message.source, source);
    
    match ui_message.update_type {
        UiUpdateType::Status(content) => {
            assert_eq!(content, message);
        },
        _ => panic!("Expected Status update type"),
    }
}

#[test]
fn test_util_progress_message() {
    let source = "test_source";
    let value: f32 = 0.75;
    
    let ui_message = util::progress(source, value);
    
    assert_eq!(ui_message.source, source);
    
    match ui_message.update_type {
        UiUpdateType::Progress(progress_value) => {
            assert_eq!(progress_value, value);
        },
        _ => panic!("Expected Progress update type"),
    }
}

#[test]
fn test_ui_message_timestamp() {
    // Test that timestamps are set correctly
    let before = SystemTime::now();
    let message = util::info("test", "message");
    let after = SystemTime::now();
    
    // Timestamp should be between before and after
    assert!(message.timestamp >= before);
    assert!(message.timestamp <= after);
}