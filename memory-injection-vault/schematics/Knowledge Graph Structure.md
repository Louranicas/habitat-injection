> Back to: [[HOME]] · [[T2 — KnowledgeEdge]] · [[Migration Strategy]]

# Knowledge Graph Structure

## Edge Type Distribution (Post-Migration)

```mermaid
pie title KnowledgeEdge by edge_type (~3,934 edges)
    "povm" : 3554
    "learned_pattern" : 141
    "hebbian" : 109
    "synergy" : 89
    "orchestration" : 29
    "cross_agent" : 12
```

## Namespace Clusters

```mermaid
mindmap
  root((KnowledgeEdge<br/>3,934 edges))
    POVM Origin (3,554)
      bare numeric (2,687)
        sphere coupling pairs
        pane coordination
      synthex_v2_* (371)
        daemon plan
        watcher persona
        bridge configs
      orac_* (61)
        RALPH parameters
        emergence detectors
      habitat_* (55)
        comms unification
        cross-service
      fleet_* (22)
        pane coordination
        task dispatch
    SQLite Origin (380)
      learned_pattern (141)
        session learnings
        best practices
        architecture
      hebbian (109)
        neural pathways
        STDP weights
      synergy (89)
        system pairs
        integration health
      orchestration (29)
        module graph
        tool chains
      cross_agent (12)
        Zen learnings
        fleet patterns
```

## Learning Dynamics (NA-R1 Per-Edge)

```mermaid
flowchart TD
    subgraph "POVM-Origin Edges"
        PE[decay_rate: 0.02/day<br/>consolidation_interval: 300 ticks<br/>Replicates POVM /consolidate cycle]
    end
    
    subgraph "Hebbian-Origin Edges"
        HE[decay_rate: per-edge stdp_rate<br/>learning_rate_ltp: 0.1<br/>learning_rate_ltd: 0.05<br/>timing_window: 20ms]
    end
    
    subgraph "Learned-Pattern Edges"
        LP[decay_rate: 0.005/day<br/>reinforcement-driven<br/>Slow decay, strong when used]
    end
    
    subgraph "Synergy Edges"
        SE[decay_rate: 0.01/day<br/>success_rate weighted<br/>Healthy pairs decay slower]
    end
    
    R5[R5 run_decay<br/>Every 6h] -->|reads per-edge rate| PE & HE & LP & SE
    R8[R8 consolidate<br/>Every 300 ticks] -->|POVM-origin only| PE
    R2[R2 reinforce_edge<br/>On RALPH cycle] -->|increments count| LP & HE
```

## Thermal Classification View

Derived from the `v3_pattern_view` pattern in the existing `hebbian_pulse.db`:

| thermal_class | Weight Range | Meaning | Decay Behaviour |
|--------------|-------------|---------|-----------------|
| `critical` | > 0.9 | Core system pathway | Decay suspended |
| `hot` | 0.7 - 0.9 | Actively reinforced | Normal decay |
| `warm` | 0.5 - 0.7 | Moderate use | Normal decay |
| `cool` | 0.3 - 0.5 | Fading | Accelerated decay |
| `cold` | < 0.3 | Candidate for pruning | Pruned if < 0.05 |

---

See: [[T2 — KnowledgeEdge]] · [[Reducers]] · [[Gap Analysis — Non-Anthropocentric]]
