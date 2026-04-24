> Back to: [[HOME]]

# Current State — Memory Substrates

## The Six Memory Substrates (live)

| # | Substrate | Access | Shape | Live Rows | Signal |
|---|-----------|--------|-------|-----------|--------|
| 1 | Auto-Memory | FS read (MEMORY.md + *.md) | Markdown w/ YAML frontmatter | ~50 files | High — hand-curated, always loaded |
| 2 | Tracking DBs (21) | `sqlite3` | 10 live / 11 dead | ~1,800 rows | Medium — rich schemas, poor reinforcement |
| 3 | POVM Engine | HTTP `:8125` | Hebbian pathways (pre→post→weight) | 3,554 | High — densest graph |
| 4 | Reasoning Memory | HTTP `:8130` TSV | Key-value TSV | ~2,000 | Medium — heartbeat only |
| 5 | VMS | HTTP `:8120` | Semantic memories, 12D tensor | 1,881 | Low — write pond, morphogenic_cycle=0 |
| 6 | Obsidian Vault | FS read | 215 markdown notes | 215 | High — canonical, human-authored |

## What's Missing (from S101 Roadmap)

| Gap | Why it Matters |
|-----|----------------|
| **Trajectory** | Know fitness is 0.664, not that it climbed from 0.5 |
| **Active workstreams** | In-flight / blocked / queued / held |
| **Causal chains** | "Why did sphere-7 restart?" — unanswerable today |
| **Active trap state** | 18 traps, don't know which are live |
| **Session narrative arc** | Last 3 sessions' feel + carry-forward |
| **Pattern reinforcement** | 141 patterns, only 1 ever reinforced >1× |

## The Five Data Patterns in Live Tracking DBs

| Pattern | Shape | STDB Table |
|---------|-------|-----------|
| **Event Log** | `(id, timestamp, type, source, data_json)` | [[T1 — HabitatEvent]] |
| **Weighted Graph** | `(source, target, weight, reinforcement_count)` | [[T2 — KnowledgeEdge]] |
| **Metric Sample** | `(id, service_id, metric_name, value, timestamp)` | [[T3 — GradientSnapshot]] |
| **Entity Registry** | `(id, name, status, config_json)` | [[T4 — SessionRecord]], [[T5 — Workstream]] |
| **Relationship** | `(system_1, system_2, score, latency, success_rate)` | [[T2 — KnowledgeEdge]] |

## POVM Namespace Distribution (3554 pathways)

| Namespace | Count |
|-----------|-------|
| bare (numeric IDs) | 2687 |
| synthex_* | 371 |
| orac_* | 61 |
| habitat_* | 55 |
| fleet_* | 22 |
| pioneer_* | 21 |
| session_* | 19 |
| claude_* | 16 |
| meta_* | 16 |
| obsidian_* | 15 |

---

See: [[Bootstrap Chain — Current vs Target]] · [[Migration Strategy]]
