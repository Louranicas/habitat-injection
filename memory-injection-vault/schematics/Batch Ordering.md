> Back to: [[HOME]] · [[System Topology]]

# Batch Ordering — devenv Start Sequence

```mermaid
graph LR
    subgraph "Batch 1 — No Dependencies"
        STDB[SpaceTimeDB :3000]
        POVM[POVM :8125]
        DEV[DevOps V3 :8082]
        CS[CodeSynthor V8 :8110]
    end

    subgraph "Batch 2 — Needs B1"
        ING[STDB Ingester :3001]
        SX1[SyntheX v1 :8090]
        SX2[SyntheX v2 :8091]
        ME[ME V2 :8180]
        PSW[Pswarm V2 :10002]
    end

    subgraph "Batch 3 — Needs B2"
        TL[Tool Library :8105]
        RM[RM :8130]
    end

    subgraph "Batch 4 — Needs B3"
        VMS[VMS :8120]
        PV2[PV2 :8132]
        ORC[ORAC :8133]
        HNC[Nerve :8083]
    end

    ING --> STDB
    SX1 --> POVM
    ME --> POVM
    ORC --> PV2
    ORC --> POVM
    PV2 --> SX1
    HNC --> ORC
```

**STDB is Batch 1** — it has no dependencies and must be available before the ingester (Batch 2) starts polling.

**Ingester is Batch 2** — depends only on STDB. It starts polling ORAC/PV2/SYNTHEX as soon as they're up (circuit breaker pattern for services not yet started).

---

See: [[System Topology]] · [[Sidecar Architecture]]
