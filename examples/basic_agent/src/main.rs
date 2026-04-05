use sparsion_core::{EventStore, HeuristicScorer, SalienceScorer};
use sparsion_sqlite::SqliteEventStore;
use sparsion_types::{Event, EventKind, Importance};

fn main() {
    println!("=== Sparsion Runtime — Basic Agent Example ===\n");

    let store = SqliteEventStore::in_memory().expect("failed to create store");
    let scorer = HeuristicScorer::default();

    // Simulate an agent recording events
    let events = vec![
        Event::new("agent", EventKind::UserAction, "User asked about deployment options"),
        Event::new("agent", EventKind::Observation, "User prefers AWS over GCP"),
        Event::new("agent", EventKind::Decision, "Recommending ECS for container orchestration")
            .with_importance(Importance::High),
        Event::new("agent", EventKind::Error, "User rejected ECS — too complex for their team"),
        Event::new("agent", EventKind::Correction, "Switching recommendation to AWS App Runner — simpler")
            .with_importance(Importance::Critical),
    ];

    for event in &events {
        store.append(event).expect("failed to append");
        store.init_memory_state(event, &scorer).expect("failed to init state");

        let salience = scorer.score(event, 1);
        println!(
            "  [{:?}] {:?} — \"{}\" (salience: {:.2})",
            event.importance, event.kind, event.content, salience
        );
    }

    println!("\nTotal events: {}", store.count().unwrap());
    println!("\nThe correction (App Runner) scores highest — the agent adapts.");
}
