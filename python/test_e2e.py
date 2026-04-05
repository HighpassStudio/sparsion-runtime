"""End-to-end test for Sparsion Python SDK."""

import os
import tempfile
from sparsion import Runtime


def make_db():
    """Create a temp db path."""
    d = tempfile.mkdtemp()
    return os.path.join(d, "test.db")


def test_record_and_query():
    """Record events, query them back, verify salience ordering."""
    rt = Runtime(make_db())

    # Record mix of event types
    rt.record("user", "decision", "Use PostgreSQL", importance="high")
    rt.record("user", "observation", "Build takes 12s")
    rt.record("user", "correction", "Switch to Svelte", importance="critical")
    rt.record("user", "user_action", "Set up CI pipeline")

    assert rt.count() == 4

    # Query all
    memories = rt.query(limit=10)
    assert len(memories) == 4

    # Verify salience ordering — correction should be first
    assert memories[0]["content"] == "Switch to Svelte", (
        f"Expected correction first, got: {memories[0]['content']}"
    )

    # Verify descending salience order
    saliences = [m["salience"] for m in memories]
    assert saliences == sorted(saliences, reverse=True), (
        f"Memories should be sorted by salience descending: {saliences}"
    )

    print(f"  record_and_query: PASS ({len(memories)} memories, top salience: {memories[0]['salience']:.2f})")


def test_sweep():
    """Verify sweep runs and returns stats."""
    rt = Runtime(make_db())

    rt.record("user", "observation", "trivial note")
    rt.record("user", "decision", "important choice", importance="high")

    result = rt.sweep()
    assert result["total_processed"] == 2
    assert "promoted" in result
    assert "demoted" in result
    assert "forgotten" in result

    print(f"  sweep: PASS (processed {result['total_processed']} memories)")


def test_query_filters():
    """Verify text and tier filters work."""
    rt = Runtime(make_db())

    rt.record("user", "decision", "Use React for frontend", importance="high")
    rt.record("user", "decision", "Use PostgreSQL for database", importance="high")
    rt.record("user", "observation", "Build is slow")

    # Text filter
    react_memories = rt.query(text="React")
    assert len(react_memories) == 1
    assert "React" in react_memories[0]["content"]

    # Tier filter — high importance decisions should be Hot
    hot_memories = rt.query(tier="hot")
    assert len(hot_memories) == 2, f"Expected 2 hot memories, got {len(hot_memories)}"

    print(f"  query_filters: PASS (text filter: 1 result, tier filter: {len(hot_memories)} results)")


def test_repetition():
    """Repeated events should score higher due to occurrence counting."""
    rt = Runtime(make_db())

    rt.record("user", "observation", "recurring pattern")
    rt.record("user", "observation", "recurring pattern")
    rt.record("user", "observation", "recurring pattern")

    memories = rt.query(text="recurring")
    saliences = [m["salience"] for m in memories]

    # Latest occurrence should have highest salience (more occurrences)
    assert len(saliences) == 3
    assert saliences[0] > saliences[-1], (
        f"Most repeated should score highest: {saliences}"
    )

    print(f"  repetition: PASS (saliences: {[f'{s:.2f}' for s in saliences]})")


if __name__ == "__main__":
    print("=== Sparsion Python SDK — End-to-End Tests ===\n")

    test_record_and_query()
    test_sweep()
    test_query_filters()
    test_repetition()

    print("\nAll tests passed.")
