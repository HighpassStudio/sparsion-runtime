"""LangChain integration for Sparsion Runtime.

Provides SparsionChatMessageHistory — a temporal memory backend
where messages are scored by salience and stale ones decay over time.

Usage with RunnableWithMessageHistory (modern):

    from sparsion.langchain import SparsionChatMessageHistory

    def get_history(session_id: str):
        return SparsionChatMessageHistory("memory.db", session_id, policy="coding")

    with_history = RunnableWithMessageHistory(chain, get_history)

Usage with ConversationBufferMemory (legacy):

    from langchain.memory import ConversationBufferMemory

    history = SparsionChatMessageHistory("memory.db", "user-123")
    memory = ConversationBufferMemory(chat_memory=history)
"""

from __future__ import annotations

import json
import os
from typing import Sequence

try:
    from langchain_core.chat_history import BaseChatMessageHistory
    from langchain_core.messages import BaseMessage, messages_from_dict, message_to_dict
except ImportError:
    raise ImportError(
        "langchain-core is required for Sparsion LangChain integration. "
        "Install it with: pip install langchain-core"
    )

from sparsion import Runtime


class SparsionChatMessageHistory(BaseChatMessageHistory):
    """LangChain chat message history backed by Sparsion Runtime.

    Unlike standard message stores, Sparsion scores messages by salience
    (recency, importance, frequency, event type) and ages out stale ones.

    The `messages` property returns messages ordered by salience, not time.
    Messages below the salience floor are excluded automatically after sweep.
    """

    def __init__(
        self,
        db_path: str,
        session_id: str,
        policy: str | None = None,
        sweep_on_read: bool = True,
        max_messages: int = 50,
    ):
        """
        Args:
            db_path: Path to SQLite database file.
            session_id: Unique session identifier. Messages are scoped to this session.
            policy: Domain policy preset ("balanced", "coding", "knowledge", "assistant").
            sweep_on_read: If True, run a decay sweep before each read. Default True.
            max_messages: Maximum messages to return from the `messages` property.
        """
        self.session_id = session_id
        self.sweep_on_read = sweep_on_read
        self.max_messages = max_messages
        self._rt = Runtime(db_path, policy=policy)

    @property
    def messages(self) -> list[BaseMessage]:
        """Return messages ordered by salience (highest first).

        Runs a decay sweep first if sweep_on_read is True.
        Messages that have decayed below the forget threshold are excluded.
        """
        if self.sweep_on_read:
            self._rt.sweep()

        memories = self._rt.query(
            source=self.session_id,
            limit=self.max_messages,
        )

        result = []
        for m in memories:
            try:
                msg_data = json.loads(m["content"])
                msgs = messages_from_dict([msg_data])
                result.extend(msgs)
            except (json.JSONDecodeError, KeyError):
                # Not a serialized message — skip
                continue

        return result

    def add_message(self, message: BaseMessage) -> None:
        """Persist a message with temporal salience scoring.

        Human messages are recorded as 'user_action'.
        AI messages are recorded as 'observation'.
        System messages are recorded as 'decision' (high importance).
        """
        serialized = json.dumps(message_to_dict(message))
        kind, importance = self._classify_message(message)

        self._rt.record(
            self.session_id,
            kind,
            serialized,
            importance=importance,
        )

    def add_messages(self, messages: Sequence[BaseMessage]) -> None:
        """Bulk add messages."""
        for message in messages:
            self.add_message(message)

    def clear(self) -> None:
        """Clear is a no-op — Sparsion handles forgetting through decay.

        To force-forget everything, delete the database file.
        Calling clear() is intentionally a no-op because Sparsion's
        value is in managing memory lifecycle, not manual deletion.
        """
        pass

    def sweep(self) -> dict:
        """Manually trigger a decay sweep. Returns sweep statistics."""
        return self._rt.sweep()

    def inspect(self) -> dict:
        """Return tier counts for this runtime instance."""
        return self._rt.inspect()

    @staticmethod
    def _classify_message(message: BaseMessage) -> tuple[str, str]:
        """Map LangChain message types to Sparsion event kinds."""
        msg_type = message.type
        if msg_type == "human":
            return "user_action", "normal"
        elif msg_type == "ai":
            return "observation", "normal"
        elif msg_type == "system":
            return "decision", "high"
        elif msg_type == "tool":
            return "observation", "normal"
        else:
            return "observation", "low"
