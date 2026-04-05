use std::sync::Arc;
use sparsion_types::{Event, EventKind};

use crate::clock::{Clock, SystemClock};
use crate::traits::SalienceScorer;

/// Heuristic salience scorer for v0.1.
///
/// salience = recency_decay × log(occurrence_count + 1) × importance_weight × event_type_weight
pub struct HeuristicScorer {
    /// Half-life in hours — after this many hours, recency contribution halves.
    pub half_life_hours: f64,
    clock: Arc<dyn Clock>,
}

impl Default for HeuristicScorer {
    fn default() -> Self {
        Self {
            half_life_hours: 168.0, // 1 week
            clock: Arc::new(SystemClock),
        }
    }
}

impl HeuristicScorer {
    pub fn new(half_life_hours: f64) -> Self {
        Self {
            half_life_hours,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(half_life_hours: f64, clock: Arc<dyn Clock>) -> Self {
        Self {
            half_life_hours,
            clock,
        }
    }

    fn recency_weight(&self, event: &Event) -> f64 {
        let age_hours = self
            .clock
            .now()
            .signed_duration_since(event.timestamp)
            .num_minutes() as f64
            / 60.0;

        // Exponential decay: 0.5^(age / half_life)
        (0.5_f64).powf(age_hours / self.half_life_hours)
    }

    fn event_type_weight(kind: EventKind) -> f64 {
        match kind {
            EventKind::Correction => 3.0,  // belief updates are high signal
            EventKind::Decision => 2.0,    // decisions shape future behavior
            EventKind::Error => 1.5,       // errors are notable
            EventKind::UserAction => 1.0,  // baseline
            EventKind::Observation => 0.7, // lowest default weight
        }
    }
}

impl SalienceScorer for HeuristicScorer {
    fn score(&self, event: &Event, occurrence_count: u32) -> f64 {
        let recency = self.recency_weight(event);
        let frequency = (occurrence_count as f64 + 1.0).ln_1p();
        let importance = event.importance.weight();
        let event_type = Self::event_type_weight(event.kind);

        recency * frequency * importance * event_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsion_types::{Event, EventKind, Importance};

    #[test]
    fn critical_scores_higher_than_normal() {
        let scorer = HeuristicScorer::default();

        let normal = Event::new("test", EventKind::UserAction, "did something");
        let critical = Event::new("test", EventKind::UserAction, "did something important")
            .with_importance(Importance::Critical);

        let normal_score = scorer.score(&normal, 1);
        let critical_score = scorer.score(&critical, 1);

        assert!(critical_score > normal_score);
    }

    #[test]
    fn corrections_score_higher_than_observations() {
        let scorer = HeuristicScorer::default();

        let obs = Event::new("test", EventKind::Observation, "saw something");
        let correction = Event::new("test", EventKind::Correction, "actually, this is wrong");

        let obs_score = scorer.score(&obs, 1);
        let correction_score = scorer.score(&correction, 1);

        assert!(correction_score > obs_score);
    }

    #[test]
    fn frequency_increases_salience() {
        let scorer = HeuristicScorer::default();
        let event = Event::new("test", EventKind::UserAction, "repeated thing");

        let once = scorer.score(&event, 1);
        let many = scorer.score(&event, 10);

        assert!(many > once);
    }

    #[test]
    fn recency_decays_with_mock_clock() {
        use crate::clock::MockClock;
        use chrono::Duration;

        let clock = Arc::new(MockClock::now_fixed());
        let scorer = HeuristicScorer::with_clock(168.0, clock.clone());

        let event = Event::new("test", EventKind::Decision, "chose Rust");
        let fresh_score = scorer.score(&event, 1);

        // Advance 1 week (one half-life)
        clock.advance(Duration::hours(168));
        let week_old_score = scorer.score(&event, 1);

        // After one half-life, score should be ~half
        assert!(week_old_score < fresh_score * 0.6);
        assert!(week_old_score > fresh_score * 0.4);

        // Advance another week
        clock.advance(Duration::hours(168));
        let two_week_score = scorer.score(&event, 1);

        // Should be ~quarter of original
        assert!(two_week_score < fresh_score * 0.3);
    }
}
