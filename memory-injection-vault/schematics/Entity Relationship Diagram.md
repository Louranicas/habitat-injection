> Back to: [[HOME]] · [[MASTER INDEX]]

# Entity Relationship Diagram

## STDB Table Relationships

```mermaid
erDiagram
    HabitatEvent ||--o{ HabitatEvent : "causal_parent"
    HabitatEvent }o--|| SessionRecord : "session_id"
    HabitatEvent ||--o| WatcherObservation : "caused_by_event"

    KnowledgeEdge ||--o{ KnowledgeEdge : "bidirectional pair"

    GradientSnapshot }o--|| SessionRecord : "session_id"

    Workstream }o--|| SessionRecord : "session_id"

    ServiceHealth {
        u64 id PK
        string service_id
        u16 port
        string health_status
        string circuit_state
        timestamp timestamp
    }

    HabitatEvent {
        u64 id PK
        string event_type
        string source_service
        string sphere_id
        u64 causal_parent FK
        u8 severity
        f64 confidence
        string payload_json
        string session_id FK
        u64 tick
        timestamp timestamp
    }

    KnowledgeEdge {
        u64 id PK
        string source_id
        string target_id
        string edge_type
        string namespace
        f64 weight
        u32 reinforcement_count
        string thermal_class
        f64 decay_rate
        timestamp last_reinforced
    }

    GradientSnapshot {
        u64 id PK
        string source
        f64 temperature
        f64 ralph_fitness
        string ralph_phase
        f64 pv2_r
        string system_grade
        string session_id FK
        timestamp timestamp
    }

    SessionRecord {
        string session_id PK
        u32 session_number
        string persona
        string model
        f64 fitness_start
        f64 fitness_end
        string status
        timestamp started_at
    }

    Workstream {
        string id PK
        string service_name
        string goal
        string status
        f64 confidence
        string session_id FK
        timestamp updated_at
    }

    TrapState {
        string trap_name PK
        bool is_active
        u32 trigger_count
        timestamp last_checked
    }

    WatcherObservation {
        string observation_id PK
        string observer_role
        string anomaly_class
        u8 severity
        u64 caused_by_event FK
        timestamp timestamp
    }
```

## Key Relationships

| From | To | Cardinality | Link Field |
|------|----|-------------|------------|
| HabitatEvent | HabitatEvent | Self-referential (causal chain) | `causal_parent → id` |
| HabitatEvent | SessionRecord | Many-to-one | `session_id` |
| WatcherObservation | HabitatEvent | Many-to-one | `caused_by_event → id` |
| GradientSnapshot | SessionRecord | Many-to-one | `session_id` |
| Workstream | SessionRecord | Many-to-one | `session_id` |

## Table Independence

- **TrapState** and **ServiceHealth** are independent — no foreign keys to other tables
- **KnowledgeEdge** is self-contained — source_id/target_id are free-text identifiers, not FK references
- **SessionRecord** is the temporal anchor — most tables reference it via `session_id`

---

See: [[T1 — HabitatEvent]] · [[T4 — SessionRecord]] · [[T8 — WatcherObservation]]
