> Back to: [[HOME]] · [[MASTER INDEX]]

# Schema Diagram

## Entity Relationships

```mermaid
erDiagram
    causal_chain {
        INTEGER id PK
        INTEGER origin_session
        INTEGER resolved_session
        TEXT chain_type
        TEXT label
        TEXT description
        INTEGER reinforcement_count
        INTEGER last_reinforced_session
        TEXT consent
        TEXT created_at
        TEXT updated_at
    }

    session_trajectory {
        INTEGER id PK
        INTEGER session_id UK
        REAL fitness
        REAL field_r
        REAL thermal_t
        REAL ltp_ltd_ratio
        INTEGER services_healthy
        TEXT delta_summary
        TEXT consent
        TIMESTAMP recorded_at
    }

    workstream {
        TEXT ws_id PK
        TEXT title
        TEXT status
        TEXT blocker
        INTEGER priority
        INTEGER last_touched_session
        INTEGER items_total
        INTEGER items_done
        TEXT resume_context
        TEXT consent
        TEXT created_at
        TEXT updated_at
    }

    reinforced_pattern {
        TEXT pattern_id PK
        TEXT category
        TEXT description
        TEXT anti_pattern
        REAL weight
        INTEGER hit_count
        INTEGER last_fired_session
        TEXT consent
        TEXT created_at
        TEXT updated_at
    }

    injection_cache {
        INTEGER id PK
        TEXT section UK
        TEXT content
        INTEGER token_count
        TIMESTAMP computed_at
    }

    session_checkpoint {
        INTEGER id PK
        INTEGER session_id
        TEXT label
        TEXT frontmatter_json
        TEXT accomplished
        TEXT in_progress
        TEXT blocked
        TEXT key_findings
        TEXT resume_instructions
        TEXT consent
        TIMESTAMP created_at
    }

    session_checkpoint ||--o{ causal_chain : "extracts chains from"
    session_checkpoint ||--o{ session_trajectory : "extracts trajectory from"
    session_checkpoint ||--o{ workstream : "extracts workstreams from"
    session_checkpoint ||--o{ reinforced_pattern : "extracts patterns from"
```

## Table Purposes

| Table | Rows (estimated) | Write Frequency | Read Frequency |
|-------|-----------------|----------------|----------------|
| `causal_chain` | ~50-200 | Per consolidation | Per injection |
| `session_trajectory` | ~1 per session | Per consolidation | Per injection |
| `workstream` | ~5-20 active | Per consolidation | Per injection |
| `reinforced_pattern` | ~50-200 | Per consolidation | Per injection |
| `injection_cache` | 5 (one per section) | Per consolidation | Per injection |
| `session_checkpoint` | ~1 per session | Per consolidation | On-demand query |

## DB Location

`~/.local/share/habitat/injection.db`
