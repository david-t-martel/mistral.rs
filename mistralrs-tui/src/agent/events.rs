//! Event system for agent tool execution
//!
//! Provides async event notifications for tool execution lifecycle:
//! - Execution started
//! - Execution progress (for long-running tools)
//! - Execution completed (success)
//! - Execution failed (error)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::toolkit::ToolCallResult;

/// Event emitted during tool execution lifecycle
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// Tool execution has started
    Started {
        call_id: Uuid,
        tool_name: String,
        timestamp: DateTime<Utc>,
    },
    /// Tool execution progress update (for long-running operations)
    Progress {
        call_id: Uuid,
        message: String,
        percentage: Option<f32>,
        timestamp: DateTime<Utc>,
    },
    /// Tool execution completed successfully
    Completed {
        call_id: Uuid,
        tool_name: String,
        result: ToolCallResult,
        timestamp: DateTime<Utc>,
    },
    /// Tool execution failed with error
    Failed {
        call_id: Uuid,
        tool_name: String,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<ToolCallResult>,
        timestamp: DateTime<Utc>,
    },
}

impl ExecutionEvent {
    /// Get the call ID for this event
    pub fn call_id(&self) -> Uuid {
        match self {
            Self::Started { call_id, .. }
            | Self::Progress { call_id, .. }
            | Self::Completed { call_id, .. }
            | Self::Failed { call_id, .. } => *call_id,
        }
    }

    /// Get the timestamp for this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Started { timestamp, .. }
            | Self::Progress { timestamp, .. }
            | Self::Completed { timestamp, .. }
            | Self::Failed { timestamp, .. } => *timestamp,
        }
    }

    /// Create a started event
    pub fn started(call_id: Uuid, tool_name: impl Into<String>) -> Self {
        Self::Started {
            call_id,
            tool_name: tool_name.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a progress event
    pub fn progress(call_id: Uuid, message: impl Into<String>, percentage: Option<f32>) -> Self {
        Self::Progress {
            call_id,
            message: message.into(),
            percentage,
            timestamp: Utc::now(),
        }
    }

    /// Create a completed event
    pub fn completed(call_id: Uuid, tool_name: impl Into<String>, result: ToolCallResult) -> Self {
        Self::Completed {
            call_id,
            tool_name: tool_name.into(),
            result,
            timestamp: Utc::now(),
        }
    }

    /// Create a failed event
    pub fn failed(
        call_id: Uuid,
        tool_name: impl Into<String>,
        error: impl Into<String>,
        result: Option<ToolCallResult>,
    ) -> Self {
        Self::Failed {
            call_id,
            tool_name: tool_name.into(),
            error: error.into(),
            result,
            timestamp: Utc::now(),
        }
    }
}

/// Event bus for broadcasting execution events
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<ExecutionEvent>,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("receiver_count", &self.sender.receiver_count())
            .finish()
    }
}

impl EventBus {
    /// Create a new event bus with the specified channel capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: ExecutionEvent) {
        // Ignore send errors (no active receivers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events from this bus
    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(100) // Default capacity of 100 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_event_bus_broadcast() {
        let bus = EventBus::new(10);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let call_id = Uuid::new_v4();
        let event = ExecutionEvent::started(call_id, "test_tool");

        bus.emit(event.clone());

        // Both receivers should get the event
        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        assert_eq!(received1.call_id(), call_id);
        assert_eq!(received2.call_id(), call_id);
    }

    #[tokio::test]
    async fn test_event_lifecycle() {
        let bus = EventBus::new(10);
        let mut rx = bus.subscribe();

        let call_id = Uuid::new_v4();

        // Started
        bus.emit(ExecutionEvent::started(call_id, "test_tool"));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ExecutionEvent::Started { .. }));

        // Progress
        bus.emit(ExecutionEvent::progress(
            call_id,
            "Processing...",
            Some(50.0),
        ));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ExecutionEvent::Progress { .. }));

        // Completed
        let result = ToolCallResult {
            success: true,
            output: serde_json::json!({"status": "ok"}),
            error: None,
            duration: Duration::from_millis(100),
        };
        bus.emit(ExecutionEvent::completed(call_id, "test_tool", result));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ExecutionEvent::Completed { .. }));

        // Failed
        let failure = ToolCallResult {
            success: false,
            output: serde_json::Value::Null,
            error: Some("boom".to_string()),
            duration: Duration::from_millis(25),
        };
        bus.emit(ExecutionEvent::failed(
            call_id,
            "test_tool",
            "boom",
            Some(failure.clone()),
        ));
        let event = rx.recv().await.unwrap();
        match event {
            ExecutionEvent::Failed { result, .. } => {
                assert!(result.is_some());
                assert_eq!(result.unwrap().error.as_deref(), Some("boom"));
            }
            _ => panic!("expected failed event"),
        }
    }

    #[test]
    fn test_receiver_count() {
        let bus = EventBus::new(10);
        assert_eq!(bus.receiver_count(), 0);

        let _rx1 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);
    }
}
