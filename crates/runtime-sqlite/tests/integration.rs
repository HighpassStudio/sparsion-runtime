use std::sync::Arc;

use chrono::{Duration, Utc};
use sparsion_core::MockClock;
use sparsion_sqlite::SqliteRuntime;
use sparsion_types::{Event, EventKind, Importance, MemoryQuery, MemoryTier};

/// The core claim: a stored memory weakens over time, moves tiers,
/// and becomes less retrievable unless reinforced.
#[test]
fn forgetting_loop_through_storage() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    // --- Record events at t0 ---

    // A critical correction — should start Hot and stay longer
    let correction = Event::new("user", EventKind::Correction, "Switch to Svelte")
        .with_importance(Importance::Critical);

    // A normal observation — should decay faster
    let observation = Event::new("user", EventKind::Observation, "Build takes 12s");

    let correction_salience = rt.record(&correction).unwrap();
    let observation_salience = rt.record(&observation).unwrap();

    // Verify initial scores
    assert!(correction_salience > 10.0, "correction should be high: {}", correction_salience);
    assert!(observation_salience < 1.0, "observation should be low: {}", observation_salience);

    // Verify initial tiers
    let cm = rt.get_memory(correction.id).unwrap();
    let om = rt.get_memory(observation.id).unwrap();
    assert_eq!(cm.tier, MemoryTier::Hot, "correction should start Hot");
    assert_eq!(om.tier, MemoryTier::Warm, "observation should start Warm");

    // --- Advance 2 weeks, sweep ---
    clock.advance(Duration::hours(336));
    let sweep1 = rt.sweep().unwrap();

    assert_eq!(sweep1.total_processed, 2);

    let cm = rt.get_memory(correction.id).unwrap();
    let om = rt.get_memory(observation.id).unwrap();

    // After 2 weeks: correction decayed but still significant, observation much weaker
    assert!(
        cm.salience < correction_salience,
        "correction salience should decay: {} -> {}",
        correction_salience, cm.salience
    );
    assert!(
        om.salience < observation_salience,
        "observation salience should decay: {} -> {}",
        observation_salience, om.salience
    );

    // Correction should still be Warm or Hot; observation should be Cold or Forgotten
    assert!(
        cm.tier == MemoryTier::Hot || cm.tier == MemoryTier::Warm,
        "correction should still be Hot/Warm after 2 weeks: {:?} (salience: {})",
        cm.tier, cm.salience
    );

    // --- Advance to 6 weeks total, sweep ---
    clock.advance(Duration::hours(672)); // 4 more weeks
    let sweep2 = rt.sweep().unwrap();

    let cm = rt.get_memory(correction.id).unwrap();
    let om = rt.get_memory(observation.id).unwrap();

    // After 6 weeks: observation should be Forgotten
    assert_eq!(
        om.tier,
        MemoryTier::Forgotten,
        "observation should be Forgotten after 6 weeks: {:?} (salience: {})",
        om.tier, om.salience
    );

    // Correction may be Cold but should NOT be Forgotten yet (high initial salience)
    assert_ne!(
        cm.tier,
        MemoryTier::Forgotten,
        "critical correction should survive 6 weeks: {:?} (salience: {})",
        cm.tier, cm.salience
    );

    assert!(sweep2.forgotten > 0, "sweep should have forgotten at least one memory");
}

/// Forgotten memories are excluded from query results.
#[test]
fn forgotten_memories_excluded_from_queries() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    let obs = Event::new("user", EventKind::Observation, "trivial note");
    rt.record(&obs).unwrap();

    // Should appear in query
    let results = rt.query(&MemoryQuery::new().limit(10)).unwrap();
    assert_eq!(results.len(), 1);

    // Advance 8 weeks — well past forget threshold for a low-salience observation
    clock.advance(Duration::hours(1344));
    rt.sweep().unwrap();

    // Should no longer appear
    let results = rt.query(&MemoryQuery::new().limit(10)).unwrap();
    assert_eq!(results.len(), 0, "forgotten memories should not appear in queries");
}

/// Reinforcement through repetition increases effective salience.
#[test]
fn repetition_reinforces_memory() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    // Record same content multiple times (different event IDs, same content+source)
    let e1 = Event::new("user", EventKind::Observation, "recurring pattern");
    let s1 = rt.record(&e1).unwrap();

    let e2 = Event::new("user", EventKind::Observation, "recurring pattern");
    let s2 = rt.record(&e2).unwrap();

    let e3 = Event::new("user", EventKind::Observation, "recurring pattern");
    let s3 = rt.record(&e3).unwrap();

    // Each repetition should score higher due to increasing occurrence count
    assert!(s2 > s1, "second occurrence should score higher: {} vs {}", s2, s1);
    assert!(s3 > s2, "third occurrence should score higher: {} vs {}", s3, s2);
}

/// High-importance events survive longer than low-importance ones.
#[test]
fn importance_affects_survival() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    let low = Event::new("user", EventKind::Observation, "low importance thing")
        .with_importance(Importance::Low);
    let high = Event::new("user", EventKind::Decision, "high importance decision")
        .with_importance(Importance::High);

    rt.record(&low).unwrap();
    rt.record(&high).unwrap();

    // Advance 4 weeks
    clock.advance(Duration::hours(672));
    rt.sweep().unwrap();

    let lm = rt.get_memory(low.id).unwrap();
    let hm = rt.get_memory(high.id).unwrap();

    // Low importance should be forgotten or cold; high should survive
    assert!(
        lm.tier == MemoryTier::Forgotten || lm.tier == MemoryTier::Cold,
        "low importance should be Cold/Forgotten: {:?}",
        lm.tier
    );
    assert!(
        hm.tier != MemoryTier::Forgotten,
        "high importance decision should survive 4 weeks: {:?} (salience: {})",
        hm.tier, hm.salience
    );
}

/// Sweep reports correct counts for demotions and forgetting.
#[test]
fn sweep_reports_accurate_counts() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    // Mix of event types
    rt.record(&Event::new("u", EventKind::Correction, "critical fix").with_importance(Importance::Critical)).unwrap();
    rt.record(&Event::new("u", EventKind::Observation, "trivial 1")).unwrap();
    rt.record(&Event::new("u", EventKind::Observation, "trivial 2")).unwrap();

    // First sweep at t0 — no changes expected
    let s0 = rt.sweep().unwrap();
    assert_eq!(s0.total_processed, 3);
    assert_eq!(s0.demoted, 0);
    assert_eq!(s0.forgotten, 0);

    // Advance 8 weeks — trivial observations should be forgotten
    clock.advance(Duration::hours(1344));
    let s1 = rt.sweep().unwrap();

    assert_eq!(s1.total_processed, 3);
    assert!(s1.forgotten >= 2, "at least 2 trivial observations should be forgotten: {}", s1.forgotten);
}
