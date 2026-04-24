> Back to: [[HOME]] · [[Session Estimates]] · [[MASTER INDEX]]

# Phase Timeline

## Gantt Chart

```mermaid
gantt
    title SpaceTimeDB Integration — Phase Timeline
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section Phase A: Deploy
    Pre-flight (build STDB)           :a0, 2026-04-28, 1d
    Core tables T1/T3/T4/T6           :a1, after a0, 2d
    Basic ingester (ORAC+PV2+SYNTHEX) :a2, after a0, 2d
    Kill-recovery test                :a3, after a1, 1d

    section Phase B: Knowledge Graph
    T2 KnowledgeEdge + NA-R1 params   :b1, after a3, 2d
    POVM migration (3554 pathways)    :b2, after b1, 1d
    SQLite migration (10 DBs)         :b3, after b1, 1d
    T5 Workstream + T7 TrapState      :b4, after b2, 1d
    Decay + consolidation reducers    :b5, after b4, 1d
    Verification checksums            :b6, after b3, 1d

    section Phase C: Causal
    ORAC triggered_by_tick patch      :c1, after b6, 1d
    5 causal linkage rules            :c2, after c1, 2d
    T8 WatcherObservation             :c3, after c1, 1d
    R6 forget_sphere cascade          :c4, after c2, 1d
    Watcher R9/R10 integration        :c5, after c3, 1d

    section Phase D: Integration
    ORAC hook bridge                  :d1, after c4, 1d
    PV2 /bus/ws subscription          :d2, after d1, 1d
    Atuin + Telegram + Obsidian       :d3, after d2, 2d
    NA-R2 consent gates               :d4, after d1, 1d
    NA-R3 reciprocal paths            :d5, after d2, 2d
    R7 retention reducer              :d6, after d3, 1d

    section Phase E: Bootstrap
    Injector CLI                      :e1, after d6, 2d
    ORAC SessionStart wiring          :e2, after e1, 1d
    Dead DB cleanup                   :e3, after e2, 1d
    E2E verification                  :e4, after e3, 1d

    section Milestones
    STDB capturing events             :milestone, after a2, 0d
    Knowledge graph migrated          :milestone, after b6, 0d
    Causal chains working             :milestone, after c4, 0d
    Full round-trip verified          :milestone, after d5, 0d
    Bootstrap revolution live         :milestone, after e4, 0d
```

## Critical Path

```mermaid
flowchart LR
    A[Phase A<br/>Deploy<br/>6-8h] --> B[Phase B<br/>Knowledge<br/>8-10h]
    B --> C[Phase C<br/>Causal<br/>6-8h]
    C --> D[Phase D<br/>Integration<br/>8-10h]
    D --> E[Phase E<br/>Bootstrap<br/>6-8h]

    A -.->|independently<br/>valuable| V1[Events captured<br/>from day 1]
    B -.->|independently<br/>valuable| V2[Unified graph<br/>queryable]
    C -.->|independently<br/>valuable| V3[Causal chains<br/>answerable]
    D -.->|independently<br/>valuable| V4[Full round-trip<br/>working]
    E -.->|the payoff| V5[<100ms bootstrap<br/>with everything]

    style E fill:#4a148c,color:#fff
    style V5 fill:#4a148c,color:#fff
```

Each phase is independently valuable. You don't need Phase E to benefit from Phase A's event capture. But Phase E is where the full vision lands.

---

See: [[Phase A — STDB Deploy]] · [[Phase E — Bootstrap Revolution]] · [[Session Estimates]]
