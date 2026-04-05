# AI memory is broken. We built one that forgets.

Every agent framework has the same problem with memory: it doesn't forget.

Context windows reset between sessions. RAG and vector DBs store everything with equal weight and grow until they're noisy. So when your project changes direction two weeks in, the AI still pulls up week-one decisions like they're current.

## What this actually looks like

**Week 1:** You tell the agent "we're using React for the frontend."

**Week 2:** You switch. "Moving to Svelte, React bundle is too big."

**Week 4:** You ask "what's our frontend stack?"

A normal retrieval system hands back both answers. React and Svelte sit side by side with equal weight. Nothing in the system knows one replaced the other. So the agent might reference React, Svelte, or some confused mix of both.

We kept running into this while building agent tooling, and it became clear the issue isn't retrieval quality — it's that these systems have no concept of time or obsolescence.

## The numbers

We ran a 4-week simulated project through both systems. 24 events total — decisions, corrections, errors, repeated observations. Two major direction changes mid-project.

| | Naive | Sparsion |
|--|-------|----------|
| **Top result correct** | No | **Yes** |
| Pruned stale memories | 0 | 2 |
| Retrievable at week 4 | 24 | 22 |

Naive retrieval puts a stale entry on top. Sparsion puts the correction first — salience 1.65 vs 0.55 for the outdated original.

## What Sparsion actually does

It treats memory as a lifecycle instead of a log.

```
Events → Salience Scoring → Hot → Warm → Cold → Forgotten
```

- Old memories weaken over time (exponential decay, configurable half-life)
- Repeated events get stronger (log-frequency)
- You can flag things as critical — those survive 4x longer
- Corrections score 3x higher than observations by default
- Anything below a salience floor gets dropped from retrieval entirely

A critical correction enters the system at salience 13.18. A throwaway observation enters at 0.77. After six weeks with no reinforcement, the observation is gone. The correction is still there.

## Try it

```python
from sparsion import Runtime

rt = Runtime("agent_memory.db")

# Week 1
rt.record("user", "decision", "Frontend framework: React", importance="high")

# Week 2
rt.record("user", "correction", "Switching to Svelte — React bundle too large", importance="critical")

# Query
memories = rt.query(text="frontend", limit=3)
for m in memories:
    print(f"[{m['tier']}] {m['content']} (salience: {m['salience']:.2f})")
# [Hot] Switching to Svelte — React bundle too large (salience: 13.18)
# [Hot] Frontend framework: React (salience: 4.39)

# Age everything
result = rt.sweep()
print(f"Forgot {result['forgotten']} stale memories")
```

## Under the hood

Rust core, Python bindings via PyO3/maturin, SQLite for storage. No model dependency — salience scoring is heuristic for now.

```
Rust core
  ├── Event store (SQLite)
  ├── Salience scorer
  ├── Tier manager (hot/warm/cold)
  ├── Decay engine
  └── Ranked retrieval
       ↓
  PyO3 → Python SDK (pip install sparsion)
```

Tests: 12 Rust unit, 5 integration (deterministic time via MockClock), 4 Python end-to-end.

## What's in v0.1

- Temporal decay with configurable half-life
- Reinforcement through repetition
- Importance hints (low/normal/high/critical)
- Event type weighting — corrections > decisions > errors > actions > observations
- Tier migration and forgetting loop through storage
- Python SDK

## What's coming

- Plugging into real agent workflows
- Bigger benchmarks, longer time horizons
- Contradiction-aware updates
- LangChain memory backend

If you're building agents and keep hitting stale context problems, I'd like to hear about your use case.

---

**Sparsion Runtime** — github.com/HighpassStudio/sparsion-runtime
