use std::sync::Arc;
use sparsion_types::{MemoryTier, ScoredMemory};

use crate::clock::{Clock, SystemClock};
use crate::policy::RuntimePolicy;
use crate::traits::DecayPolicy;

/// Time-based decay with configurable tier thresholds.
pub struct TimeDecayPolicy {
    policy: RuntimePolicy,
    clock: Arc<dyn Clock>,
}

impl Default for TimeDecayPolicy {
    fn default() -> Self {
        Self {
            policy: RuntimePolicy::default(),
            clock: Arc::new(SystemClock),
        }
    }
}

impl TimeDecayPolicy {
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            policy: RuntimePolicy::default(),
            clock,
        }
    }

    pub fn from_policy(policy: RuntimePolicy, clock: Arc<dyn Clock>) -> Self {
        Self { policy, clock }
    }
}

impl DecayPolicy for TimeDecayPolicy {
    fn decay(&self, memory: &ScoredMemory) -> f64 {
        let age_hours = self
            .clock
            .now()
            .signed_duration_since(memory.last_accessed)
            .num_minutes() as f64
            / 60.0;

        let decay_factor = (0.5_f64).powf(age_hours / self.policy.half_life_hours);

        memory.salience * decay_factor
    }

    fn assign_tier(&self, salience: f64) -> MemoryTier {
        if salience >= self.policy.hot_threshold {
            MemoryTier::Hot
        } else if salience >= self.policy.warm_threshold {
            MemoryTier::Warm
        } else if salience >= self.policy.forget_threshold {
            MemoryTier::Cold
        } else {
            MemoryTier::Forgotten
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::MockClock;
    use chrono::{Duration, Utc};
    use sparsion_types::{Event, EventKind};

    fn make_memory_at(salience: f64, timestamp: chrono::DateTime<Utc>) -> ScoredMemory {
        let event = Event::new("test", EventKind::UserAction, "test content");
        ScoredMemory {
            event,
            salience,
            tier: MemoryTier::Hot,
            occurrence_count: 1,
            last_accessed: timestamp,
            is_overridden: false,
        }
    }

    #[test]
    fn recent_memories_decay_less() {
        let now = Utc::now();
        let clock = Arc::new(MockClock::new(now));
        let policy = TimeDecayPolicy::from_policy(RuntimePolicy::default(), clock);

        let recent = make_memory_at(5.0, now - Duration::hours(1));
        let old = make_memory_at(5.0, now - Duration::hours(168));

        assert!(policy.decay(&recent) > policy.decay(&old));
    }

    #[test]
    fn tier_assignment() {
        let policy = TimeDecayPolicy::default();

        assert_eq!(policy.assign_tier(3.0), MemoryTier::Hot);
        assert_eq!(policy.assign_tier(1.0), MemoryTier::Warm);
        assert_eq!(policy.assign_tier(0.1), MemoryTier::Cold);
        assert_eq!(policy.assign_tier(0.01), MemoryTier::Forgotten);
    }

    #[test]
    fn decay_causes_tier_demotion() {
        let now = Utc::now();
        let clock = Arc::new(MockClock::new(now));
        let policy = TimeDecayPolicy::from_policy(RuntimePolicy::default(), clock.clone());

        let memory = make_memory_at(3.0, now);

        let s0 = policy.decay(&memory);
        assert_eq!(policy.assign_tier(s0), MemoryTier::Hot);

        clock.advance(Duration::hours(168));
        let s1 = policy.decay(&memory);
        assert_eq!(policy.assign_tier(s1), MemoryTier::Warm);

        clock.advance(Duration::hours(336));
        let s3 = policy.decay(&memory);
        assert_eq!(policy.assign_tier(s3), MemoryTier::Cold);

        clock.advance(Duration::hours(504));
        let s6 = policy.decay(&memory);
        assert_eq!(policy.assign_tier(s6), MemoryTier::Forgotten);
    }
}
