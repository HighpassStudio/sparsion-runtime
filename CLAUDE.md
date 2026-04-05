# Sparsion Runtime

Temporal memory runtime for AI systems. "State for AI."

## Architecture
- **Rust core** (`crates/`) — event store, salience scoring, decay engine, retrieval
- **Python SDK** (`python/sparsion/`) — PyO3/maturin bindings, ergonomic API
- **Storage**: SQLite behind trait interface (only impl for v0.1)

## Crates
- `runtime-types` — shared types (Event, MemoryTier, SalienceScore, etc.)
- `runtime-core` — traits (EventStore, SalienceScorer, DecayPolicy, MemoryRetriever) + heuristic impl
- `runtime-sqlite` — SQLite storage backend
- `runtime-ffi` — PyO3 bindings

## Build
```bash
cargo build                    # Rust core
cd python && maturin develop   # Python SDK
```

## Key Design Principles
- Forgetting is a feature, not a bug — decay is core to the architecture
- Salience scoring is heuristic in v0.1, model-based later
- Storage behind traits — SQLite now, custom format if needed later
- Python SDK should feel native, not like a thin wrapper
