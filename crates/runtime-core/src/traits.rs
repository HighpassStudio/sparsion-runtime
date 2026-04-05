use sparsion_types::{Event, MemoryQuery, MemoryTier, ScoredMemory};
use uuid::Uuid;

use crate::error::RuntimeError;

/// Append-only event storage.
pub trait EventStore {
    /// Store a new event.
    fn append(&self, event: &Event) -> Result<(), RuntimeError>;

    /// Retrieve an event by ID.
    fn get(&self, id: Uuid) -> Result<Event, RuntimeError>;

    /// Count total events in store.
    fn count(&self) -> Result<u64, RuntimeError>;

    /// Increment the occurrence count for a content fingerprint.
    /// Returns the new count.
    fn record_occurrence(&self, event: &Event) -> Result<u32, RuntimeError>;
}

/// Computes salience score for an event given its context.
pub trait SalienceScorer {
    /// Score an event. Higher = more salient.
    fn score(&self, event: &Event, occurrence_count: u32) -> f64;
}

/// Determines how memories decay over time.
pub trait DecayPolicy {
    /// Apply decay to a scored memory. Returns updated salience.
    fn decay(&self, memory: &ScoredMemory) -> f64;

    /// Determine which tier a memory belongs in given its current salience.
    fn assign_tier(&self, salience: f64) -> MemoryTier;
}

/// Retrieves memories ranked by temporal salience.
pub trait MemoryRetriever {
    /// Query memories, returned in descending salience order.
    fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>, RuntimeError>;

    /// Run a full decay sweep — recompute salience and reassign tiers for all memories.
    fn sweep(&self) -> Result<SweepResult, RuntimeError>;
}

/// Result of a decay sweep.
#[derive(Debug, Clone)]
pub struct SweepResult {
    pub total_processed: u64,
    pub promoted: u64,
    pub demoted: u64,
    pub forgotten: u64,
}
