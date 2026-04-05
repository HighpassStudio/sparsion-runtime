# Sparsion Runtime

Temporal memory engine for AI agents.

## Architecture
- **Rust core** (`crates/`) — event store, salience scoring, decay engine, retrieval
- **Python SDK** (`python/sparsion/`) — PyO3/maturin bindings
- **Storage**: SQLite behind trait interface (only impl for v0.1)

## Crates
- `runtime-types` — shared types (Event, MemoryTier, ScoredMemory, etc.)
- `runtime-core` — traits (EventStore, SalienceScorer, DecayPolicy, MemoryRetriever) + heuristic impl + Clock
- `runtime-sqlite` — SQLite backend, unified SqliteRuntime
- `runtime-ffi` — PyO3 bindings

## Build
```bash
cargo build                    # Rust core
cargo test                     # all tests
cd python && maturin develop   # Python SDK
```

## Design
- Decay is core — forgetting is intentional, not a bug
- Salience scoring is heuristic in v0.1
- Storage behind traits — SQLite now, swap later if needed
- Clock abstraction (SystemClock/MockClock) for deterministic tests
