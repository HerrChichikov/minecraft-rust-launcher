use serde::Serialize;
use uuid::Uuid;

use crate::profiles::ProfileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct OperationId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStage {
    Initializing,
    Checking,
    Downloading,
    Repairing,
    Extracting,
    Ready,
    Launching,
    Running,
    Cancelling,
    Cancelled,
    Error,
}

/// Public progress only; credentials and process arguments must never enter this type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub operation_id: OperationId,
    pub profile_id: ProfileId,
    pub sequence: u64,
    pub stage: OperationStage,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
}

/// Implementations must bound queues and must not block a worker on frontend rendering.
pub trait EventSink: Send + Sync {
    fn publish(&self, event: OperationEvent);
}
