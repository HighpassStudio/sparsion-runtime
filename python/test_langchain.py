"""Test Sparsion LangChain integration."""

import os
import tempfile

from langchain_core.messages import HumanMessage, AIMessage, SystemMessage

from sparsion.langchain import SparsionChatMessageHistory


def make_db():
    d = tempfile.mkdtemp()
    return os.path.join(d, "test_lc.db")


def test_add_and_retrieve():
    history = SparsionChatMessageHistory(make_db(), session_id="test-1")

    history.add_message(HumanMessage(content="What framework should we use?"))
    history.add_message(AIMessage(content="I recommend React for the frontend."))
    history.add_message(HumanMessage(content="Actually, switch to Svelte."))

    messages = history.messages
    assert len(messages) == 3, f"Expected 3 messages, got {len(messages)}"

    # All should be retrievable
    contents = [m.content for m in messages]
    assert "What framework should we use?" in contents
    assert "I recommend React for the frontend." in contents
    assert "Actually, switch to Svelte." in contents

    print(f"  add_and_retrieve: PASS ({len(messages)} messages)")


def test_message_types_classified():
    history = SparsionChatMessageHistory(make_db(), session_id="test-2")

    history.add_message(HumanMessage(content="human msg"))
    history.add_message(AIMessage(content="ai msg"))
    history.add_message(SystemMessage(content="system msg"))

    # System messages get 'decision' kind with 'high' importance — should score higher
    messages = history.messages
    assert len(messages) == 3

    # System message should be near the top (higher salience from decision+high)
    top_content = messages[0].content
    assert top_content == "system msg", (
        f"System message should rank highest, got: {top_content}"
    )

    print(f"  message_types: PASS (system message ranked first)")


def test_sweep_and_inspect():
    history = SparsionChatMessageHistory(make_db(), session_id="test-3")

    history.add_message(HumanMessage(content="hello"))
    history.add_message(AIMessage(content="hi there"))

    result = history.sweep()
    assert "total_processed" in result
    assert result["total_processed"] == 2

    info = history.inspect()
    assert info["total_events"] == 2

    print(f"  sweep_and_inspect: PASS (processed: {result['total_processed']})")


def test_session_isolation():
    db = make_db()
    h1 = SparsionChatMessageHistory(db, session_id="session-A")
    h2 = SparsionChatMessageHistory(db, session_id="session-B")

    h1.add_message(HumanMessage(content="session A message"))
    h2.add_message(HumanMessage(content="session B message"))

    msgs_a = h1.messages
    msgs_b = h2.messages

    a_contents = [m.content for m in msgs_a]
    b_contents = [m.content for m in msgs_b]

    assert "session A message" in a_contents
    assert "session B message" not in a_contents
    assert "session B message" in b_contents
    assert "session A message" not in b_contents

    print(f"  session_isolation: PASS (A: {len(msgs_a)}, B: {len(msgs_b)})")


def test_policy_support():
    history = SparsionChatMessageHistory(
        make_db(), session_id="test-4", policy="coding"
    )

    history.add_message(HumanMessage(content="coding context"))
    messages = history.messages
    assert len(messages) == 1

    print(f"  policy_support: PASS (coding policy active)")


def test_clear_is_noop():
    history = SparsionChatMessageHistory(make_db(), session_id="test-5")
    history.add_message(HumanMessage(content="persist this"))
    history.clear()  # should not delete

    messages = history.messages
    assert len(messages) == 1, "clear() should be a no-op — Sparsion manages lifecycle"

    print(f"  clear_noop: PASS (message survived clear)")


if __name__ == "__main__":
    print("=== Sparsion LangChain Integration Tests ===\n")

    test_add_and_retrieve()
    test_message_types_classified()
    test_sweep_and_inspect()
    test_session_isolation()
    test_policy_support()
    test_clear_is_noop()

    print("\nAll tests passed.")
