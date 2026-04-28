#![doc = include_str!("../../ai_docs/modules/L4_CONSOLIDATION_ENGINE.md")]
#![allow(clippy::doc_markdown)]

//! `m4_consolidation` layer modules.

pub mod m15_checkpoint_ingest;
pub mod m15b_trajectory_capture;
pub mod m16_hebbian_engine;
pub mod m17_cache_builder;
pub mod m18_atuin_cache;
pub mod m25_self_heal;
pub mod m26_backup_clone;
pub mod m27_auto_consolidate;
pub mod m28_health_watchdog;
