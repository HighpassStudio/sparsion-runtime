"""Sparsion Memory Dashboard — terminal-based memory inspection.

Usage:
    sparsion dashboard
    sparsion dashboard --top 5
"""

from __future__ import annotations

import os
from datetime import datetime, timezone

from sparsion import Runtime

DEFAULT_DB = os.path.expanduser("~/.sparsion/memory.db")


def format_age(timestamp_str: str) -> str:
    """Convert ISO timestamp to human-readable age."""
    try:
        ts = datetime.fromisoformat(timestamp_str.replace("Z", "+00:00"))
        now = datetime.now(timezone.utc)
        delta = now - ts
        hours = delta.total_seconds() / 3600

        if hours < 1:
            mins = int(delta.total_seconds() / 60)
            return f"{mins}m ago"
        elif hours < 24:
            return f"{int(hours)}h ago"
        elif hours < 168:
            return f"{int(hours / 24)}d ago"
        else:
            return f"{int(hours / 168)}w ago"
    except (ValueError, TypeError):
        return "?"


def bar(count: int, total: int, width: int = 30) -> str:
    """Render a simple ASCII bar."""
    if total == 0:
        return " " * width
    filled = int((count / total) * width)
    return "#" * filled + "." * (width - filled)


def render_dashboard(db_path: str = None, top: int = 5, policy: str = None):
    """Render the memory dashboard to stdout."""
    path = db_path or os.environ.get("SPARSION_DB", DEFAULT_DB)
    if not os.path.exists(path):
        print(f"No database found at {path}")
        return

    rt = Runtime(path, policy=policy)
    info = rt.inspect()
    total = info["total_events"]

    if total == 0:
        print("Empty memory. No events recorded.")
        return

    active = info["hot"] + info["warm"] + info["cold"]

    print()
    print(f"  SPARSION MEMORY DASHBOARD")
    print(f"  {path}")
    print(f"  {'-' * 50}")
    print()

    # Tier distribution
    print(f"  Total events:  {total}")
    print(f"  Active:        {active}")
    print(f"  Forgotten:     {info['forgotten']}")
    print()

    print(f"  HOT    [{bar(info['hot'], active)}] {info['hot']:>5}")
    print(f"  WARM   [{bar(info['warm'], active)}] {info['warm']:>5}")
    print(f"  COLD   [{bar(info['cold'], active)}] {info['cold']:>5}")
    print()

    # Top memories per tier
    for tier_name in ["hot", "warm", "cold"]:
        memories = rt.query(tier=tier_name, limit=top)
        if not memories:
            continue

        print(f"  -- {tier_name.upper()} (top {min(top, len(memories))}) --")
        for m in memories:
            age = format_age(m["timestamp"])
            overridden = " [overridden]" if m.get("is_overridden") else ""
            content = m["content"]
            if len(content) > 60:
                content = content[:57] + "..."
            print(f"    {m['salience']:>6.2f}  {age:>6}  {content}{overridden}")
        print()

    # Memory health
    if active > 0:
        hot_pct = info["hot"] / active * 100
        warm_pct = info["warm"] / active * 100
        cold_pct = info["cold"] / active * 100
        forget_pct = info["forgotten"] / total * 100 if total > 0 else 0

        print(f"  -- HEALTH --")
        print(f"    Hot:       {hot_pct:>5.1f}%")
        print(f"    Warm:      {warm_pct:>5.1f}%")
        print(f"    Cold:      {cold_pct:>5.1f}%")
        print(f"    Forgotten: {forget_pct:>5.1f}% (of all time)")

        if hot_pct > 80:
            print(f"    Note: most memories are Hot -- may need a sweep or shorter half-life")
        if cold_pct > 60:
            print(f"    Note: many memories are Cold -- consider running sweep to prune")
        if forget_pct > 90:
            print(f"    Note: >90% forgotten -- half-life may be too aggressive")

    print()
