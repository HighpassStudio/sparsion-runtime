"""
Sparsion — Temporal memory runtime for AI systems.

State for AI. Remember what matters, forget the rest.

Usage:
    from sparsion import Runtime

    rt = Runtime("./my_agent.db")
    rt.record("user", "decision", "Switch to React for the frontend", importance="high")
    rt.record("user", "observation", "Build time is 45 seconds")

    memories = rt.query(text="React", limit=5)
    for m in memories:
        print(f"[{m['tier']}] {m['content']} (salience: {m['salience']:.2f})")

    # Run decay sweep to age memories
    result = rt.sweep()
    print(f"Swept {result['total_processed']} memories, forgot {result['forgotten']}")
"""

from sparsion.sparsion_runtime import Runtime

__all__ = ["Runtime"]
__version__ = "0.3.0"
