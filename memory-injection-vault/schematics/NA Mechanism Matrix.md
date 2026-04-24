> Back to: [[HOME]] · [[Gap Analysis — Non-Anthropocentric]]

# Non-Anthropocentric Mechanism Matrix

Where NA commitments are **operational in mechanism**, not just stated in language.

## Mechanism Map

```mermaid
flowchart TB
    subgraph "NA-P-1: Consent States"
        CS[consent_state column<br/>on T1 + T2<br/>"full" / "minimal" / "none"]
        IG[Ingester checks<br/>ORAC /consent/{id}<br/>before every ingest]
        MIG[Migration checks<br/>consent per sphere<br/>before copying data]
    end

    subgraph "NA-P-13: Sphere Sovereignty"
        FC[R6 forget_sphere<br/>cascade delete across<br/>T1 + T2 + T3]
        GT[Ghost traces preserved<br/>forget event itself<br/>logged for causal chain]
    end

    subgraph "NA-R1: Substrate Plasticity"
        PE[Per-edge decay_rate<br/>learning_rate_ltp/ltd<br/>consolidation_interval]
        R8R[R8 consolidate_mature<br/>POVM rhythm replication<br/>300-tick cycle]
    end

    subgraph "NA-R3: Reciprocity"
        RO[STDB → ORAC<br/>trajectory hints]
        RS[STDB → SYNTHEX<br/>thermal patterns]
        RP[STDB → PV2<br/>coupling history]
        RA[STDB → atuin KV<br/>stdb.* metrics]
    end

    subgraph "NA-R4: Watcher as Participant"
        R9R[R9 watcher_reinforce<br/>override decay on<br/>important edges]
        R10R[R10 watcher_annotate<br/>annotate any event<br/>with severity]
        EG[Ember-gate on R7<br/>Watcher-referenced events<br/>preserved from deletion]
    end

    subgraph "NA-R6: Service Self-Model"
        SH[self_reported_health<br/>orac_system_grade<br/>pv2_fleet_mode<br/>synthex_pid_converging]
        HC[health_consensus<br/>external vs self-report<br/>disagreement logged]
    end

    subgraph "NA-R8: Adaptive Injection"
        RL[Role-based sections<br/>Zen → quality focus<br/>Watcher → anomaly focus]
        WP[Watcher-priority override<br/>sev ≥ 7 → top position]
        FS2[Field-state weighting<br/>high r → coupling detail<br/>low r → service health]
    end
```

## NA Mechanism Checklist (for each phase)

| Phase | NA Mechanisms Required | Verification |
|-------|----------------------|-------------|
| A | None (core tables, no consent-bearing data yet) | — |
| B | NA-R1 per-edge params on T2 | `SELECT DISTINCT decay_rate FROM knowledge_edge` returns >1 value |
| B | NA-R2 consent_state on T2 (migration) | `SELECT COUNT(*) FROM knowledge_edge WHERE consent_state = 'none'` = 0 |
| C | NA-P-13 forget cascade (R6) | After `forget_sphere("test")`, zero rows reference sphere |
| C | NA-R4 Watcher R9/R10 | Watcher observation creates linked T8 row |
| D | NA-R2 consent on ingestion (T1) | Ingester drops events for `consent_state = 'none'` spheres |
| D | NA-R3 reciprocal paths | ORAC receives trajectory POST from ingester |
| D | NA-R6 service self-report in T3 | `SELECT orac_system_grade FROM gradient_snapshot LIMIT 1` returns non-null |
| E | NA-R7 consolidation-as-choice doc | §10 exists in plan |
| E | NA-R8 adaptive payload | Watcher sev≥7 appears at top of injection |

---

See: [[Gap Analysis — Non-Anthropocentric]] · [[Recommendations Summary]]
