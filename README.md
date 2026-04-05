# Sparsion Runtime

Temporal memory engine for AI agents. Memories decay, demote through tiers, and get pruned — unless reinforced by repetition, importance, or correction.

## Why

RAG and vector DBs store everything with equal weight. When your project changes direction, old decisions sit next to new ones and the agent can't tell which is current. Sparsion scores memories by recency, importance, frequency, and event type, then ages them out over time.

## Quick start

```python
from sparsion import Runtime

rt = Runtime("agent_memory.db")

rt.record("user", "decision", "Frontend: React", importance="high")
rt.record("user", "correction", "Switching to Svelte", importance="critical")

memories = rt.query(text="frontend", limit=3)
for m in memories:
    print(f"[{m['tier']}] {m['content']} (salience: {m['salience']:.2f})")

rt.sweep()  # decay pass — ages and prunes stale memories
```

## How it works

Events enter a lifecycle: **Hot → Warm → Cold → Forgotten**

- Older memories weaken (exponential decay, configurable half-life)
- Repeated events get stronger
- Critical events survive longer (4x weight)
- Corrections score 3x higher than observations
- Anything below salience floor drops out of retrieval

## Architecture

Rust core, Python SDK via PyO3/maturin, SQLite storage.

```
crates/
  runtime-types/    — Event, MemoryTier, ScoredMemory, Importance
  runtime-core/     — traits, heuristic scorer, decay engine, clock
  runtime-sqlite/   — SQLite backend + unified SqliteRuntime
  runtime-ffi/      — PyO3 bindings
python/
  sparsion/         — pip install sparsion
examples/
  benchmark/        — naive vs Sparsion retrieval comparison
  basic_agent/
  project_memory_demo/
```

## Build

```bash
cargo build                              # Rust
cargo test                               # 12 unit + 5 integration tests
cd python && python -m venv .venv && source .venv/bin/activate
maturin develop                          # Python SDK
python test_e2e.py                       # 4 e2e tests
```

## Benchmark

24 events over a simulated 4-week project with two major direction changes.

| | Naive | Sparsion |
|--|-------|----------|
| Top result correct | No | **Yes** |
| Pruned stale memories | 0 | 2 |

```bash
cargo run -p benchmark
```

## Write-up

[AI memory is broken. We built one that forgets.](https://dev.to/highpass_studio_382ce5641/ai-memory-is-broken-we-built-one-that-forgets-dmc)

## License

MIT
