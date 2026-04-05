# AI memory is broken. We built one that forgets.

AI systems today either forget everything or remember everything.

Neither works.

Context windows forget between sessions. Vector databases and RAG pipelines store everything equally and grow forever. The result is familiar to anyone building agents: when a project changes direction, the AI keeps surfacing stale, contradicted information.

## A real example

**Week 1:** "Frontend framework: React"

**Week 2:** "Switching frontend from React to Svelte — smaller bundle"

**Week 4 query:** "What framework are we using?"

Naive retrieval returns both entries with equal weight. The original React decision still sits in memory alongside the Svelte correction. An agent consuming this context may reference React, Svelte, or both — because nothing in the memory system knows that one supersedes the other.

Here's what happens in practice:

## The benchmark

We simulated a 4-week software project with 24 events — observations, decisions, errors, corrections, and repeated signals. Two major direction changes occurred mid-project (frontend framework switch, hosting platform switch). At week 4, we queried both systems.

| Metric | Naive | Sparsion Runtime |
|--------|-------|-----------------|
| **Top result correct** | NO | **YES** |
| Forgotten (pruned) | 0 | 2 |
| Retrievable memories | 24 | 22 |

Naive memory returns a stale observation as its top result. Sparsion surfaces the correction — with a salience score of 1.65 vs 0.55 for the outdated original decision.

**When project direction changes, Sparsion adapts. Naive memory doesn't.**

## The insight

Human memory works because it forgets. Irrelevant details fade. Repeated patterns strengthen. Corrections override old beliefs. Outdated information disappears.

Current AI memory systems don't do any of this.

Sparsion Runtime is a temporal memory engine that remembers what matters and forgets the rest.

## How it works

Every event enters a lifecycle:

```
Events → Salience Scoring → Hot → Warm → Cold → Forgotten
```

Sparsion uses five mechanisms:

- **Temporal decay** — older memories weaken over time (exponential, configurable half-life)
- **Reinforcement** — repeated events strengthen memory traces (log-frequency weighting)
- **Importance hints** — critical events survive longer (4x weight vs normal)
- **Event type weighting** — corrections score 3x, decisions 2x, observations 0.7x
- **Selective forgetting** — memories below threshold are pruned from retrieval

The result: a memory system that evolves instead of accumulating.

In our benchmark, a critical correction starts at salience 13.18. A normal observation starts at 0.77. After 6 weeks without reinforcement, the observation is forgotten. The correction survives. That's the memory policy working as designed.

## Try it

```python
from sparsion import Runtime

rt = Runtime("agent_memory.db")

# Week 1
rt.record("user", "decision", "Frontend framework: React", importance="high")

# Week 2
rt.record("user", "correction", "Switching to Svelte — React bundle too large", importance="critical")

# Query at week 4
memories = rt.query(text="frontend", limit=3)
for m in memories:
    print(f"[{m['tier']}] {m['content']} (salience: {m['salience']:.2f})")
# [Hot] Switching to Svelte — React bundle too large (salience: 13.18)
# [Hot] Frontend framework: React (salience: 4.39)

# Run decay sweep
result = rt.sweep()
print(f"Forgot {result['forgotten']} stale memories")
```

## Architecture

Rust core with Python SDK (PyO3/maturin). SQLite storage. Heuristic salience scoring — no model dependency for v0.1.

```
Rust Core (sparsion-core)
  ├── Event store (SQLite)
  ├── Salience scorer (heuristic)
  ├── Memory tier manager (hot/warm/cold)
  ├── Decay engine
  └── Retrieval (salience-ranked)
       ↓
  PyO3 bindings
       ↓
  Python SDK (pip install sparsion)
```

12 Rust unit tests. 5 storage-backed integration tests with deterministic time (MockClock). 4 Python end-to-end tests.

## What's next

Sparsion Runtime v0.1 is available now:

- Temporal decay with configurable half-life
- Reinforcement through repetition
- Importance hints (low/normal/high/critical)
- Event type weighting (corrections > decisions > errors > actions > observations)
- Tier migration (hot → warm → cold → forgotten)
- Full forgetting loop through storage
- Python SDK with record/query/sweep

We're exploring:

- Real agent workflow integrations
- Larger benchmarks with longer time horizons
- Contradiction-aware belief updates (v0.2)
- LangChain memory backend

If you're building agents and hitting memory limits — stale context, growing token costs, agents that can't adapt — we'd like to hear from you.

---

**Sparsion Runtime** — AI that remembers what matters and forgets the rest.

GitHub: github.com/HighpassStudio/sparsion-runtime
