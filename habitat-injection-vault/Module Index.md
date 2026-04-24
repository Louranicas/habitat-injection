> Back to: [[HOME]] · [[MASTER INDEX]]

# Module Index

> 24 modules across 6 layers | Quality target: 50+ tests per module

## Registry

| # | Module | Layer | File | Test Kind | Feature Gate | Depends On |
|---|--------|-------|------|-----------|-------------|------------|
| 01 | m01_types | [[L1 Foundation]] | `m1_foundation/m01_types.rs` | unit | — | — |
| 02 | m02_errors | [[L1 Foundation]] | `m1_foundation/m02_errors.rs` | unit | — | m01 |
| 03 | m03_config | [[L1 Foundation]] | `m1_foundation/m03_config.rs` | unit | — | m01 |
| 04 | m04_traits | [[L1 Foundation]] | `m1_foundation/m04_traits.rs` | unit | — | m01, m02 |
| 05 | m05_constants | [[L1 Foundation]] | `m1_foundation/m05_constants.rs` | unit | — | — |
| 06 | m06_schema | [[L2 Schema & Persistence]] | `m2_schema/m06_schema.rs` | unit | — | m01, m02 |
| 07 | m07_causal_chain | [[L2 Schema & Persistence]] | `m2_schema/m07_causal_chain.rs` | unit | — | m01, m02, m06 |
| 08 | m08_trajectory | [[L2 Schema & Persistence]] | `m2_schema/m08_trajectory.rs` | unit | — | m01, m02, m06 |
| 09 | m09_workstream | [[L2 Schema & Persistence]] | `m2_schema/m09_workstream.rs` | unit | — | m01, m02, m06 |
| 10 | m10_pattern | [[L2 Schema & Persistence]] | `m2_schema/m10_pattern.rs` | unit | — | m01, m02, m06 |
| 10b | m10b_checkpoint | [[L2 Schema & Persistence]] | `m2_schema/m10b_checkpoint.rs` | unit | — | m01, m02, m06, m07 |
| 11 | m11_parallel_query | [[L3 Injection Engine]] | `m3_injection/m11_parallel_query.rs` | unit | — | m01, m02, m07-m10 |
| 12 | m12_prose_renderer | [[L3 Injection Engine]] | `m3_injection/m12_prose_renderer.rs` | unit | — | m01, m02 |
| 13 | m13_fallback | [[L3 Injection Engine]] | `m3_injection/m13_fallback.rs` | unit | — | m01, m02 |
| 14 | m14_consent_filter | [[L3 Injection Engine]] | `m3_injection/m14_consent_filter.rs` | unit | — | m01 |
| 15 | m15_checkpoint_ingest | [[L4 Consolidation Engine]] | `m4_consolidation/m15_checkpoint_ingest.rs` | unit | — | m01, m02, m07-m10b |
| 15b | m15b_trajectory_capture | [[L4 Consolidation Engine]] | `m4_consolidation/m15b_trajectory_capture.rs` | unit | — | m01, m02, m08 |
| 16 | m16_hebbian_engine | [[L4 Consolidation Engine]] | `m4_consolidation/m16_hebbian_engine.rs` | unit | — | m01, m02, m05, m07, m10 |
| 17 | m17_cache_builder | [[L4 Consolidation Engine]] | `m4_consolidation/m17_cache_builder.rs` | unit | — | m01, m02, m06, m12 |
| 18 | m18_atuin_cache | [[L4 Consolidation Engine]] | `m4_consolidation/m18_atuin_cache.rs` | unit | — | m01, m02 |
| 19 | m19_preset_queries | [[L5 Query & Browser]] | `m5_query/m19_preset_queries.rs` | unit | — | m01, m02, m07-m10b |
| 20 | m20_raw_query | [[L5 Query & Browser]] | `m5_query/m20_raw_query.rs` | unit | — | m01, m02, m06 |
| 21 | m21_fzf_browser | [[L5 Query & Browser]] | `m5_query/m21_fzf_browser.rs` | unit | — | m01, m19 |
| 21b | m21b_scripts_engine | [[L5 Query & Browser]] | `m5_query/m21b_scripts_engine.rs` | unit | — | m01, m02, m06 |
| 22 | m22_stdb_module | [[L6 SpaceTimeDB Migration]] | `m6_stdb/m22_stdb_module.rs` | unit | `stdb` | m01 |
| 23 | m23_ingester | [[L6 SpaceTimeDB Migration]] | `m6_stdb/m23_ingester.rs` | integration | `ingester` | m01, m02, m22 |
| 24 | m24_migration | [[L6 SpaceTimeDB Migration]] | `m6_stdb/m24_migration.rs` | integration | `stdb` | m01, m02, m06, m22 |

## Property Test Targets
- m01_types — newtype invariants (weight clamping, Display round-trip)
- m10_pattern — Hebbian convergence (weight stays in [0,1])
- m16_hebbian_engine — decay/reinforce cycle monotonicity
