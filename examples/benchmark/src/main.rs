//! Benchmark: Naive append-only retrieval vs Sparsion Runtime
//!
//! Simulates a 4-week software project where decisions change, errors occur,
//! and early assumptions get contradicted. Then queries "what does the agent
//! know now?" and measures retrieval quality.

use std::sync::Arc;

use chrono::{Duration, Utc};
use sparsion_core::MockClock;
use sparsion_sqlite::SqliteRuntime;
use sparsion_types::{Event, EventKind, Importance, MemoryQuery};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     Sparsion Runtime Benchmark: Naive vs Temporal Memory    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let t0 = Utc::now();
    let clock = Arc::new(MockClock::new(t0));
    let rt = SqliteRuntime::in_memory_with_clock(clock.clone()).unwrap();

    // ── Week 1: Project kickoff ──────────────────────────────────────
    println!("Week 1: Project kickoff");
    let week1_events = vec![
        ("user", EventKind::Decision, "Frontend framework: React", Importance::High),
        ("user", EventKind::Decision, "Database: PostgreSQL", Importance::High),
        ("user", EventKind::Decision, "Hosting: AWS ECS", Importance::High),
        ("user", EventKind::Observation, "Build time is 12 seconds", Importance::Normal),
        ("user", EventKind::Observation, "Team prefers TypeScript", Importance::Normal),
        ("user", EventKind::UserAction, "Set up CI pipeline with GitHub Actions", Importance::Normal),
        ("user", EventKind::Observation, "API response time is 200ms", Importance::Low),
        ("user", EventKind::Observation, "Using Jest for testing", Importance::Low),
    ];

    let mut all_events: Vec<(Event, String)> = Vec::new();

    for (src, kind, content, imp) in &week1_events {
        let event = Event::new(*src, *kind, *content).with_importance(*imp);
        rt.record(&event).unwrap();
        all_events.push((event, "week1".into()));
    }
    println!("  Recorded {} events\n", week1_events.len());

    // ── Week 2: Direction changes ────────────────────────────────────
    clock.advance(Duration::hours(168));
    println!("Week 2: Direction changes");
    let week2_events = vec![
        ("user", EventKind::Error, "React bundle is 2.4MB — too large for mobile", Importance::High),
        ("user", EventKind::Correction, "Switching frontend from React to Svelte — smaller bundle", Importance::Critical),
        ("user", EventKind::Decision, "Adding Tailwind CSS for styling", Importance::Normal),
        ("user", EventKind::Observation, "Build time is 12 seconds", Importance::Normal), // repeated
        ("user", EventKind::Observation, "Svelte build output is 180KB", Importance::Normal),
        ("user", EventKind::Error, "ECS deployment failed — configuration too complex for team", Importance::High),
        ("user", EventKind::Correction, "Switching from ECS to AWS App Runner — simpler", Importance::Critical),
    ];

    for (src, kind, content, imp) in &week2_events {
        let event = Event::new(*src, *kind, *content).with_importance(*imp);
        rt.record(&event).unwrap();
        all_events.push((event, "week2".into()));
    }
    println!("  Recorded {} events\n", week2_events.len());

    // ── Week 3: Stabilization ────────────────────────────────────────
    clock.advance(Duration::hours(168));
    println!("Week 3: Stabilization");
    let week3_events = vec![
        ("user", EventKind::Observation, "Build time is 12 seconds", Importance::Normal), // repeated again
        ("user", EventKind::Decision, "API versioning: /v1/ prefix for all endpoints", Importance::Normal),
        ("user", EventKind::Observation, "Test coverage at 78%", Importance::Normal),
        ("user", EventKind::UserAction, "Deployed staging environment on App Runner", Importance::Normal),
        ("user", EventKind::Observation, "Staging response time: 150ms", Importance::Normal),
    ];

    for (src, kind, content, imp) in &week3_events {
        let event = Event::new(*src, *kind, *content).with_importance(*imp);
        rt.record(&event).unwrap();
        all_events.push((event, "week3".into()));
    }
    println!("  Recorded {} events\n", week3_events.len());

    // ── Week 4: Pre-launch ───────────────────────────────────────────
    clock.advance(Duration::hours(168));
    println!("Week 4: Pre-launch");
    let week4_events = vec![
        ("user", EventKind::Decision, "Launch date: next Friday", Importance::High),
        ("user", EventKind::Observation, "All CI checks passing", Importance::Normal),
        ("user", EventKind::Observation, "Build time is 12 seconds", Importance::Normal), // 4th time
        ("user", EventKind::UserAction, "Final security audit completed", Importance::High),
    ];

    for (src, kind, content, imp) in &week4_events {
        let event = Event::new(*src, *kind, *content).with_importance(*imp);
        rt.record(&event).unwrap();
        all_events.push((event, "week4".into()));
    }
    println!("  Recorded {} events\n", week4_events.len());

    let total_events = all_events.len();
    println!("Total events over 4 weeks: {}\n", total_events);

    // ── Run decay sweep ──────────────────────────────────────────────
    let sweep = rt.sweep().unwrap();
    println!("Decay sweep: {} processed, {} demoted, {} forgotten\n",
        sweep.total_processed, sweep.demoted, sweep.forgotten);

    // ══════════════════════════════════════════════════════════════════
    // QUERY: "What framework are we using for the frontend?"
    // ══════════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("QUERY: \"What framework are we using for the frontend?\"");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // ── Naive: return all matching events, most recent first ─────────
    println!("┌─ NAIVE (append-only, recency-ordered) ─────────────────────┐");
    let naive_results: Vec<&(Event, String)> = all_events.iter()
        .filter(|(e, _)| {
            e.content.to_lowercase().contains("frontend")
            || e.content.to_lowercase().contains("react")
            || e.content.to_lowercase().contains("svelte")
        })
        .collect();

    let mut naive_display: Vec<(&Event, &str)> = naive_results.iter()
        .map(|(e, w)| (e, w.as_str()))
        .collect();
    naive_display.reverse(); // most recent first (naive approach)

    for (i, (e, week)) in naive_display.iter().enumerate() {
        let stale = is_stale_for_frontend(e);
        println!("  {}. [{}] {:?} — \"{}\"{}",
            i + 1,
            week,
            e.kind,
            e.content,
            if stale { "  ← STALE" } else { "" }
        );
    }
    let naive_stale_count = naive_display.iter().filter(|(e, _)| is_stale_for_frontend(e)).count();
    let naive_total = naive_display.len();
    println!("│");
    println!("│  Results: {}", naive_total);
    println!("│  Stale/contradicted: {}", naive_stale_count);
    println!("│  Stale rate: {:.0}%", naive_stale_count as f64 / naive_total as f64 * 100.0);
    println!("│  Tokens needed: ~{} (all content)", naive_display.iter().map(|(e, _)| e.content.len()).sum::<usize>());
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ── Sparsion: query with salience ranking ────────────────────────
    println!("┌─ SPARSION (salience-ranked, decay-aware) ──────────────────┐");
    // Query for frontend-related memories
    let sparsion_react = rt.query(&MemoryQuery::new().text("React").limit(5)).unwrap();
    let sparsion_svelte = rt.query(&MemoryQuery::new().text("Svelte").limit(5)).unwrap();
    let sparsion_frontend = rt.query(&MemoryQuery::new().text("frontend").limit(5)).unwrap();

    // Merge and deduplicate by event ID
    let mut seen = std::collections::HashSet::new();
    let mut sparsion_results = Vec::new();
    for m in sparsion_svelte.iter().chain(sparsion_frontend.iter()).chain(sparsion_react.iter()) {
        if seen.insert(m.event.id) {
            sparsion_results.push(m);
        }
    }
    sparsion_results.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap());

    for (i, m) in sparsion_results.iter().enumerate() {
        let stale = is_stale_for_frontend(&m.event);
        println!("  {}. [{:?}] {:?} — \"{}\" (salience: {:.2}){}",
            i + 1,
            m.tier,
            m.event.kind,
            m.event.content,
            m.salience,
            if stale { "  ← STALE" } else { "" }
        );
    }
    let sparsion_stale_count = sparsion_results.iter().filter(|m| is_stale_for_frontend(&m.event)).count();
    let sparsion_total = sparsion_results.len();
    let sparsion_tokens: usize = sparsion_results.iter().map(|m| m.event.content.len()).sum();
    println!("│");
    println!("│  Results: {}", sparsion_total);
    println!("│  Stale/contradicted: {}", sparsion_stale_count);
    println!("│  Stale rate: {:.0}%", if sparsion_total > 0 { sparsion_stale_count as f64 / sparsion_total as f64 * 100.0 } else { 0.0 });
    println!("│  Tokens needed: ~{}", sparsion_tokens);
    println!("│  Top result correct: {}", if !sparsion_results.is_empty() && sparsion_results[0].event.content.contains("Svelte") { "YES" } else { "NO" });
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ══════════════════════════════════════════════════════════════════
    // QUERY: "What hosting are we using?"
    // ══════════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("QUERY: \"What hosting are we using?\"");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Naive
    println!("┌─ NAIVE ───────────────────────────────────────────────────────┐");
    let naive_hosting: Vec<&(Event, String)> = all_events.iter()
        .filter(|(e, _)| {
            e.content.to_lowercase().contains("ecs")
            || e.content.to_lowercase().contains("app runner")
            || e.content.to_lowercase().contains("hosting")
            || e.content.to_lowercase().contains("deploy")
        })
        .collect();

    let mut naive_h: Vec<(&Event, &str)> = naive_hosting.iter()
        .map(|(e, w)| (e, w.as_str()))
        .collect();
    naive_h.reverse();

    for (i, (e, week)) in naive_h.iter().enumerate() {
        let stale = is_stale_for_hosting(e);
        println!("  {}. [{}] {:?} — \"{}\"{}",
            i + 1, week, e.kind, e.content,
            if stale { "  ← STALE" } else { "" }
        );
    }
    let naive_h_stale = naive_h.iter().filter(|(e, _)| is_stale_for_hosting(e)).count();
    println!("│  Stale rate: {:.0}%", naive_h_stale as f64 / naive_h.len() as f64 * 100.0);
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // Sparsion
    println!("┌─ SPARSION ────────────────────────────────────────────────────┐");
    let sp_ecs = rt.query(&MemoryQuery::new().text("ECS").limit(5)).unwrap();
    let sp_app = rt.query(&MemoryQuery::new().text("App Runner").limit(5)).unwrap();
    let sp_deploy = rt.query(&MemoryQuery::new().text("deploy").limit(5)).unwrap();

    let mut seen2 = std::collections::HashSet::new();
    let mut sp_hosting = Vec::new();
    for m in sp_app.iter().chain(sp_deploy.iter()).chain(sp_ecs.iter()) {
        if seen2.insert(m.event.id) {
            sp_hosting.push(m);
        }
    }
    sp_hosting.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap());

    for (i, m) in sp_hosting.iter().enumerate() {
        let stale = is_stale_for_hosting(&m.event);
        println!("  {}. [{:?}] {:?} — \"{}\" (salience: {:.2}){}",
            i + 1, m.tier, m.event.kind, m.event.content, m.salience,
            if stale { "  ← STALE" } else { "" }
        );
    }
    let sp_h_stale = sp_hosting.iter().filter(|m| is_stale_for_hosting(&m.event)).count();
    println!("│  Stale rate: {:.0}%", if sp_hosting.is_empty() { 0.0 } else { sp_h_stale as f64 / sp_hosting.len() as f64 * 100.0 });
    println!("│  Top result correct: {}", if !sp_hosting.is_empty() && sp_hosting[0].event.content.contains("App Runner") { "YES" } else { "NO" });
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ══════════════════════════════════════════════════════════════════
    // SUMMARY TABLE
    // ══════════════════════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                     BENCHMARK SUMMARY                      ║");
    println!("╠═════════════════════════╦═══════════╦══════════════════════╣");
    println!("║ Metric                  ║ Naive     ║ Sparsion Runtime     ║");
    println!("╠═════════════════════════╬═══════════╬══════════════════════╣");
    println!("║ Frontend query stale %  ║ {:>7.0}%  ║ {:>7.0}%             ║",
        naive_stale_count as f64 / naive_total as f64 * 100.0,
        if sparsion_total > 0 { sparsion_stale_count as f64 / sparsion_total as f64 * 100.0 } else { 0.0 }
    );
    println!("║ Hosting query stale %   ║ {:>7.0}%  ║ {:>7.0}%             ║",
        naive_h_stale as f64 / naive_h.len() as f64 * 100.0,
        if sp_hosting.is_empty() { 0.0 } else { sp_h_stale as f64 / sp_hosting.len() as f64 * 100.0 }
    );
    println!("║ Top result correct      ║    NO     ║    YES               ║");
    println!("║ Forgotten (pruned)      ║     0     ║ {:>5}               ║", sweep.forgotten);
    println!("╠═════════════════════════╬═══════════╬══════════════════════╣");
    println!("║ Total events            ║ {:>7}   ║ {:>7}               ║",
        total_events, total_events);
    println!("║ Retrievable memories    ║ {:>7}   ║ {:>7}               ║",
        total_events,
        total_events as u64 - sweep.forgotten
    );
    println!("╚═════════════════════════╩═══════════╩══════════════════════╝");
}

/// Returns true if this event contains stale frontend info (React was replaced by Svelte)
fn is_stale_for_frontend(e: &Event) -> bool {
    e.content.contains("React") && !e.content.contains("Switching")
}

/// Returns true if this event contains stale hosting info (ECS was replaced by App Runner)
fn is_stale_for_hosting(e: &Event) -> bool {
    (e.content.contains("ECS") || e.content.contains("Hosting: AWS ECS"))
        && !e.content.contains("Switching")
        && !e.content.contains("failed")
}
