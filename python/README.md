# Sparsion

Temporal memory runtime for AI systems. State for AI.

```python
from sparsion import Runtime

rt = Runtime("./agent.db")
rt.record("user", "decision", "Switch to Svelte", importance="critical")
rt.record("user", "observation", "Build takes 12s")

memories = rt.query(limit=5)
for m in memories:
    print(f"[{m['tier']}] {m['content']} (salience: {m['salience']:.2f})")

result = rt.sweep()
print(f"Swept {result['total_processed']} memories, forgot {result['forgotten']}")
```
