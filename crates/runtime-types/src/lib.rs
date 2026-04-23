use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single event observed by the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub kind: EventKind,
    pub content: String,
    pub metadata: serde_json::Value,
    pub importance: Importance,
    /// If set, this event explicitly overrides/supersedes another event.
    pub overrides: Option<Uuid>,
}

impl Event {
    pub fn new(source: impl Into<String>, kind: EventKind, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: source.into(),
            kind,
            content: content.into(),
            metadata: serde_json::Value::Null,
            importance: Importance::Normal,
            overrides: None,
        }
    }

    pub fn with_importance(mut self, importance: Importance) -> Self {
        self.importance = importance;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_overrides(mut self, target_id: Uuid) -> Self {
        self.overrides = Some(target_id);
        self
    }

    /// Override the event timestamp (used for backfills/migrations).
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Classification of event types — affects base salience weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// User-initiated action or input
    UserAction,
    /// System/tool observation
    Observation,
    /// Decision point or outcome
    Decision,
    /// Error or unexpected state
    Error,
    /// Explicit correction or belief update
    Correction,
}

/// User-provided importance hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    Low,
    Normal,
    High,
    Critical,
}

impl Importance {
    pub fn weight(&self) -> f64 {
        match self {
            Importance::Low => 0.5,
            Importance::Normal => 1.0,
            Importance::High => 2.0,
            Importance::Critical => 4.0,
        }
    }
}

/// Which memory tier an event currently lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    /// Active context, full fidelity
    Hot,
    /// Important history, may be summarized
    Warm,
    /// Compressed knowledge, abstracted
    Cold,
    /// Marked for removal by decay engine
    Forgotten,
}

/// A scored memory entry — an event with its current salience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub event: Event,
    pub salience: f64,
    pub tier: MemoryTier,
    pub occurrence_count: u32,
    pub last_accessed: DateTime<Utc>,
    /// True if this memory has been overridden by a newer event.
    pub is_overridden: bool,
}

/// Query for retrieving memories.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub text: Option<String>,
    pub source: Option<String>,
    pub kind: Option<EventKind>,
    pub min_salience: Option<f64>,
    pub tier: Option<MemoryTier>,
    pub limit: Option<usize>,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn min_salience(mut self, min: f64) -> Self {
        self.min_salience = Some(min);
        self
    }

    pub fn tier(mut self, tier: MemoryTier) -> Self {
        self.tier = Some(tier);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}
