use std::sync::Arc;
use std::time::Instant;

use chrono::{Duration, Utc};
use sparsion_core::MockClock;
use sparsion_sqlite::SqliteRuntime;
use sparsion_types::{Event, EventKind, Importance, MemoryQuery, MemoryTier};

fn event_mix(i: u32) -> (EventKind, Importance) {
    match i % 20 {
        0 => (EventKind::Correction, Importance::Critical),  // 5% corrections
        1..=3 => (EventKind::Decision, Importance::High),     // 15% decisions
        4..=6 => (EventKind::Error, Importance::Normal),      // 15% errors
        7..=10 => (EventKind::UserAction, Importance::Normal), // 20% actions
        _ => (EventKind::Observation, Importance::Normal),    // 45% observations
    }
}

#[test]
fn scale_10k_events() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    let n = 10_000u32;

    // Record events spread over 8 weeks
    let start = Instant::now();
    for i in 0..n {
        // Advance time by ~5 minutes per event (8 weeks / 10k ≈ 4.8 min)
        if i > 0 {
            clock.advance(Duration::minutes(5));
        }

        let (kind, imp) = event_mix(i);
        let content = if i % 50 == 0 {
            // 2% are repeated content (reinforcement test)
            format!("recurring-pattern-{}", i % 5)
        } else {
            format!("event-{}", i)
        };

        let event = Event::new("agent", kind, content).with_importance(imp);
        rt.record(&event).unwrap();
    }
    let record_time = start.elapsed();

    let count = rt.count().unwrap();
    assert_eq!(count, n as u64);

    // Sweep
    let start = Instant::now();
    let sweep = rt.sweep().unwrap();
    let sweep_time = start.elapsed();

    // Query hot memories
    let start = Instant::now();
    let hot = rt.query(&MemoryQuery::new().tier(MemoryTier::Hot).limit(20)).unwrap();
    let query_hot_time = start.elapsed();

    // Query with text filter
    let start = Instant::now();
    let text_results = rt.query(&MemoryQuery::new().text("recurring").limit(20)).unwrap();
    let query_text_time = start.elapsed();

    // Inspect
    let info = rt.inspect().unwrap();

    println!("\n=== SCALE BENCHMARK: {} events ===", n);
    println!("Record all:   {:>8.1}ms ({:.1} events/ms)", record_time.as_millis(), n as f64 / record_time.as_millis() as f64);
    println!("Sweep:        {:>8.1}ms", sweep_time.as_millis());
    println!("Query (hot):  {:>8.1}ms ({} results)", query_hot_time.as_millis(), hot.len());
    println!("Query (text): {:>8.1}ms ({} results)", query_text_time.as_millis(), text_results.len());
    println!();
    println!("Tier distribution:");
    println!("  Hot:       {:>6}", info.hot);
    println!("  Warm:      {:>6}", info.warm);
    println!("  Cold:      {:>6}", info.cold);
    println!("  Forgotten: {:>6}", info.forgotten);
    println!();
    println!("Sweep results:");
    println!("  Processed: {:>6}", sweep.total_processed);
    println!("  Demoted:   {:>6}", sweep.demoted);
    println!("  Forgotten: {:>6}", sweep.forgotten);
    println!("  Promoted:  {:>6}", sweep.promoted);

    // Assertions: sweep should have forgotten many old observations
    assert!(sweep.forgotten > 0, "should forget some old events over 8 weeks");
    assert!(info.hot < n as u64 / 2, "less than half should be hot after 8 weeks");
    assert!(info.forgotten > 0, "some should be forgotten");

    // Performance assertions (generous — just making sure nothing is catastrophically slow)
    assert!(sweep_time.as_millis() < 10_000, "sweep should complete in <10s for 10k events");
    assert!(query_hot_time.as_millis() < 1_000, "hot query should complete in <1s");
}

#[test]
fn scale_50k_events() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    let n = 50_000u32;

    // Record events spread over 12 weeks (~1.5 min per event)
    let start = Instant::now();
    for i in 0..n {
        if i > 0 {
            clock.advance(Duration::seconds(90));
        }

        let (kind, imp) = event_mix(i);
        let content = if i % 50 == 0 {
            format!("recurring-pattern-{}", i % 10)
        } else {
            format!("event-{}", i)
        };

        let event = Event::new("agent", kind, content).with_importance(imp);
        rt.record(&event).unwrap();
    }
    let record_time = start.elapsed();

    // Sweep
    let start = Instant::now();
    let sweep = rt.sweep().unwrap();
    let sweep_time = start.elapsed();

    // Queries
    let start = Instant::now();
    let hot = rt.query(&MemoryQuery::new().tier(MemoryTier::Hot).limit(20)).unwrap();
    let query_time = start.elapsed();

    let info = rt.inspect().unwrap();

    println!("\n=== SCALE BENCHMARK: {} events ===", n);
    println!("Record all:   {:>8}ms ({:.1} events/ms)", record_time.as_millis(), n as f64 / record_time.as_millis() as f64);
    println!("Sweep:        {:>8}ms", sweep_time.as_millis());
    println!("Query (hot):  {:>8}ms ({} results)", query_time.as_millis(), hot.len());
    println!();
    println!("Tier distribution:");
    println!("  Hot:       {:>6}", info.hot);
    println!("  Warm:      {:>6}", info.warm);
    println!("  Cold:      {:>6}", info.cold);
    println!("  Forgotten: {:>6}", info.forgotten);
    println!();
    println!("Sweep: {} processed, {} forgotten, {} demoted",
        sweep.total_processed, sweep.forgotten, sweep.demoted);

    // Performance assertions
    assert!(sweep_time.as_millis() < 60_000, "sweep should complete in <60s for 50k events");
    assert!(query_time.as_millis() < 2_000, "query should complete in <2s for 50k events");
    assert!(info.forgotten > 0, "some events should be forgotten over 12 weeks");
}

/// Verify that rare high-importance corrections survive while
/// frequent low-importance noise gets forgotten.
#[test]
fn reinforcement_beats_noise_at_scale() {
    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    // Record 1 critical correction at the start
    let correction = Event::new("user", EventKind::Correction, "IMPORTANT: switch to Svelte")
        .with_importance(Importance::Critical);
    rt.record(&correction).unwrap();

    // Then 5000 low-importance observations over 6 weeks (~7 min each)
    for i in 0..5000 {
        if i > 0 {
            clock.advance(Duration::minutes(7));
        }
        let noise = Event::new("agent", EventKind::Observation, format!("noise-{}", i))
            .with_importance(Importance::Low);
        rt.record(&noise).unwrap();
    }

    // Sweep at the end (6 weeks later)
    let sweep = rt.sweep().unwrap();
    let info = rt.inspect().unwrap();

    // The critical correction should NOT be forgotten
    let correction_mem = rt.get_memory(correction.id).unwrap();
    assert_ne!(
        correction_mem.tier,
        MemoryTier::Forgotten,
        "critical correction should survive 6 weeks of noise (salience: {}, tier: {:?})",
        correction_mem.salience,
        correction_mem.tier
    );

    // Many noise observations should be forgotten
    assert!(
        info.forgotten > 1000,
        "most old low-importance noise should be forgotten: {} forgotten out of 5001",
        info.forgotten
    );

    println!("\n=== REINFORCEMENT vs NOISE ===");
    println!("Correction survived: {:?} (salience: {:.4})", correction_mem.tier, correction_mem.salience);
    println!("Noise forgotten: {} / 5000", info.forgotten);
    println!("Sweep: {} demoted, {} forgotten", sweep.demoted, sweep.forgotten);
}
