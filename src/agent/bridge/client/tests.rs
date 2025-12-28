//! Tests for bridge client.

use super::super::types::BridgeEvent;

#[test]
fn test_parse_bridge_event_text() {
    let json = r#"{"type":"text","sessionId":"abc123","timestamp":1234567890,"content":"Hello","partial":false}"#;
    let event: BridgeEvent = serde_json::from_str(json).unwrap();

    match event {
        BridgeEvent::Text {
            session_id,
            content,
            partial,
            ..
        } => {
            assert_eq!(session_id, "abc123");
            assert_eq!(content, "Hello");
            assert!(!partial);
        }
        _ => panic!("Expected Text event"),
    }
}

#[test]
fn test_parse_bridge_event_session_complete() {
    let json = r#"{"type":"session.complete","sessionId":"xyz789","timestamp":1234567890,"success":true,"durationMs":5000}"#;
    let event: BridgeEvent = serde_json::from_str(json).unwrap();

    match event {
        BridgeEvent::SessionComplete {
            session_id,
            success,
            duration_ms,
            ..
        } => {
            assert_eq!(session_id, "xyz789");
            assert!(success);
            assert_eq!(duration_ms, 5000);
        }
        _ => panic!("Expected SessionComplete event"),
    }
}
