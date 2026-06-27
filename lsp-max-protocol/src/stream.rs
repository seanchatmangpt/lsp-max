use serde::{Deserialize, Serialize};

// max/stream.subscribe
pub const STREAM_SUBSCRIBE: &str = "max/stream.subscribe";
// max/stream.unsubscribe
pub const STREAM_UNSUBSCRIBE: &str = "max/stream.unsubscribe";
// max/stream.event — pushed from server to client
pub const STREAM_EVENT: &str = "max/stream.event";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSubscribeParams {
    /// Subscription ID chosen by the client
    pub subscription_id: String,
    /// Which event kinds to subscribe to
    pub event_kinds: Vec<StreamEventKind>,
    /// Optional: only events for this document URI (serialised URI string)
    pub uri_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSubscribeResult {
    pub subscription_id: String,
    /// "CANDIDATE" — streaming is active but not ADMITTED
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUnsubscribeParams {
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUnsubscribeResult {
    pub subscription_id: String,
    pub events_received: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamEventKind {
    Diagnostic,
    ConformanceChange,
    GateChange,
    ReceiptAdmission,
    LawViolation,
    /// Admission gate outcome for an agent operation request.
    /// Payload: BrokerDecisionPayload
    BrokerDecision,
    /// Agent's position in the admission queue changed.
    /// Payload: QueuePositionPayload
    QueuePositionChange,
}

/// Admission decision variants pushed via BrokerDecision events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionDecision {
    Admitted,
    Refused,
    Queued { position: u32 },
}

/// Payload for StreamEventKind::BrokerDecision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerDecisionPayload {
    /// Operation ID, e.g. "ggen.sync", "ggen.build.workspace"
    pub op: String,
    pub decision: AdmissionDecision,
    /// BLAKE3 receipt hash if admitted
    pub receipt_hash: Option<String>,
    /// DefectClass name if refused
    pub refusal_reason: Option<String>,
}

impl BrokerDecisionPayload {
    pub fn admitted(op: impl Into<String>, receipt_hash: impl Into<String>) -> Self {
        Self {
            op: op.into(),
            decision: AdmissionDecision::Admitted,
            receipt_hash: Some(receipt_hash.into()),
            refusal_reason: None,
        }
    }

    pub fn refused(op: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            op: op.into(),
            decision: AdmissionDecision::Refused,
            receipt_hash: None,
            refusal_reason: Some(reason.into()),
        }
    }

    pub fn queued(op: impl Into<String>, position: u32) -> Self {
        Self {
            op: op.into(),
            decision: AdmissionDecision::Queued { position },
            receipt_hash: None,
            refusal_reason: None,
        }
    }
}

/// Payload for StreamEventKind::QueuePositionChange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePositionPayload {
    pub op: String,
    pub position: u32,
    pub queue_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub subscription_id: String,
    pub sequence: u64,
    pub kind: StreamEventKind,
    pub payload: serde_json::Value,
    /// Law-axis status of this event's source.
    pub status: String,
    pub timestamp_secs: u64,
}

/// In-process event bus for max/stream subscriptions.
/// Backed by tokio::sync::broadcast.
pub struct StreamBus {
    sender: tokio::sync::broadcast::Sender<StreamEvent>,
}

impl StreamBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<StreamEvent> {
        self.sender.subscribe()
    }

    /// Publish an event to all current subscribers. Returns the receiver count.
    pub fn publish(&self, event: StreamEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}
