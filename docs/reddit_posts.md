# Reddit Posts

## r/LocalLLaMA

**Title:** Built a memory engine that forgets stale info instead of storing everything forever

**Post:**

Week 1: tell agent "use React"
Week 2: "switch to Svelte"
Week 4: ask "what framework?" — get both back with equal weight.

Every memory system I've used has this problem. Old decisions never go away, so the agent can't tell what's current.

Built something where memories decay over time, corrections outrank old decisions, and stale stuff drops out of retrieval. 4-week sim: naive put outdated info on top, this put the correction first.

Rust core, SQLite, Python SDK.

https://github.com/HighpassStudio/sparsion-runtime

---

## r/LLMDevs

**Title:** Anyone else dealing with stale context in agent memory?

**Post:**

Same pattern keeps coming up: project direction changes, agent still pulls old info, references both old and new like they're equally valid.

Built a small runtime that decays memories over time and ranks corrections above original decisions. Anything stale enough gets dropped from queries.

Tested it against naive retrieval on a 4-week project — naive surfaced outdated info first, this surfaced the correction.

https://github.com/HighpassStudio/sparsion-runtime

How are you handling this? Manual pruning? Just living with it?

---

## r/MachineLearning

**Title:** Temporal decay for agent memory — benchmark results

**Post:**

Tested a simple idea: what if agent memory had exponential decay, event-type weighting (corrections > decisions > observations), and tier-based pruning?

24 events over a simulated 4-week project with 2 direction changes. Naive retrieval returned stale info as the top result. Decay-weighted retrieval returned the correction.

Rust + SQLite + Python SDK. Heuristic scoring, no model dependency.

https://github.com/HighpassStudio/sparsion-runtime

Details: https://dev.to/highpass_studio_382ce5641/ai-memory-is-broken-we-built-one-that-forgets-dmc
