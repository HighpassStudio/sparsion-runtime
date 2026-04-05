"""Sparsion CLI — record, query, sweep, and inspect agent memory."""

import argparse
import os
import sys

from sparsion import Runtime

DEFAULT_DB = os.path.expanduser("~/.sparsion/memory.db")


def get_runtime(db_path=None, policy=None):
    path = db_path or os.environ.get("SPARSION_DB", DEFAULT_DB)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    return Runtime(path, policy=policy)


def cmd_record(args):
    rt = get_runtime(args.db, args.policy)
    event_id = rt.record(args.source, args.kind, args.content, importance=args.importance)
    print(event_id)


def cmd_query(args):
    rt = get_runtime(args.db, args.policy)
    memories = rt.query(
        text=args.text,
        source=args.source,
        tier=args.tier,
        min_salience=args.min_salience,
        limit=args.limit,
    )
    if not memories:
        print("No memories found.")
        return

    for m in memories:
        tier = m["tier"]
        sal = m["salience"]
        content = m["content"]
        src = m["source"]
        ts = m["timestamp"][:10]
        print(f"  [{tier:<4}] {sal:>6.2f}  {src}/{content}  ({ts})")


def cmd_sweep(args):
    rt = get_runtime(args.db, args.policy)
    result = rt.sweep()
    print(f"Swept {result['total_processed']} memories: "
          f"{result['demoted']} demoted, {result['forgotten']} forgotten, "
          f"{result['promoted']} promoted")


def cmd_inspect(args):
    rt = get_runtime(args.db, args.policy)
    count = rt.count()
    print(f"Total events: {count}")
    print()

    for tier_name in ["hot", "warm", "cold"]:
        memories = rt.query(tier=tier_name, limit=100)
        if memories:
            print(f"-- {tier_name.upper()} ({len(memories)}) --")
            for m in memories:
                print(f"  {m['salience']:>6.2f}  [{m['source']}] {m['content']}")
            print()


def cmd_context(args):
    """Output top memories as context for an agent prompt."""
    rt = get_runtime(args.db, args.policy)
    memories = rt.query(
        text=args.text,
        source=args.source,
        limit=args.limit,
    )
    if not memories:
        return

    lines = []
    for m in memories:
        tier = m["tier"]
        content = m["content"]
        lines.append(f"[{tier}] {content}")

    print("\n".join(lines))


def main():
    parser = argparse.ArgumentParser(prog="sparsion", description="Temporal memory for AI agents")
    parser.add_argument("--db", help=f"Database path (default: {DEFAULT_DB})")
    parser.add_argument("--policy", choices=["balanced", "coding", "knowledge", "assistant"],
                        help="Domain policy preset")
    sub = parser.add_subparsers(dest="command")

    # record
    p_rec = sub.add_parser("record", help="Record an event")
    p_rec.add_argument("source", help="Event source (e.g. user, agent, tool)")
    p_rec.add_argument("kind", choices=["user_action", "observation", "decision", "error", "correction"])
    p_rec.add_argument("content", help="Event content")
    p_rec.add_argument("-i", "--importance", default="normal", choices=["low", "normal", "high", "critical"])

    # query
    p_q = sub.add_parser("query", help="Query memories")
    p_q.add_argument("-t", "--text", help="Text filter")
    p_q.add_argument("-s", "--source", help="Source filter")
    p_q.add_argument("--tier", choices=["hot", "warm", "cold"])
    p_q.add_argument("--min-salience", type=float)
    p_q.add_argument("-n", "--limit", type=int, default=10)

    # sweep
    sub.add_parser("sweep", help="Run decay sweep")

    # inspect
    sub.add_parser("inspect", help="Show memory state by tier")

    # context
    p_ctx = sub.add_parser("context", help="Output top memories as agent context")
    p_ctx.add_argument("-t", "--text", help="Text filter")
    p_ctx.add_argument("-s", "--source", help="Source filter")
    p_ctx.add_argument("-n", "--limit", type=int, default=10)

    args = parser.parse_args()

    if args.command == "record":
        cmd_record(args)
    elif args.command == "query":
        cmd_query(args)
    elif args.command == "sweep":
        cmd_sweep(args)
    elif args.command == "inspect":
        cmd_inspect(args)
    elif args.command == "context":
        cmd_context(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
