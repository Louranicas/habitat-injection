//! Reducer type aliases for the `SpaceTimeDB` WASM module.
//!
//! These are **reference types only** — not executable STDB reducer code.
//! In the WASM module, each function would be decorated with
//! `#[spacetimedb::reducer]`.
//!
//! # R1–R10 Summaries
//!
//! | ID | Name | Schedule |
//! |----|------|---------|
//! | R1 | `ingest_event` | on-demand |
//! | R2 | `reinforce_edge` | on-demand |
//! | R3 | `capture_gradient` | every 60 s |
//! | R4 | `register_session` / `close_session` | on-demand |
//! | R5 | `run_decay` | every 6 h |
//! | R6 | `forget_sphere` | on-demand (NA-P-13) |
//! | R7 | `compact_old_events` | every 24 h |
//! | R8 | `consolidate_mature_edges` | every 300 ticks |
//! | R9 | `watcher_reinforce` | on-demand |
//! | R10 | `watcher_annotate_event` | on-demand |

use super::tables::HabitatEvent;

/// Function signature type for R1 `ingest_event`.
///
/// `(event: HabitatEvent) -> Result<(), String>`
pub type IngestEvent = fn(HabitatEvent) -> Result<(), String>;

/// Function signature type for R2 `reinforce_edge`.
///
/// `(source_id: &str, target_id: &str, edge_type: &str, namespace: &str) -> Result<(), String>`
pub type ReinforceEdge = fn(&str, &str, &str, &str) -> Result<(), String>;

/// Function signature type for R3 `capture_gradient`.
///
/// `() -> Result<(), String>`
pub type CaptureGradient = fn() -> Result<(), String>;

/// Function signature type for R4a `register_session`.
///
/// `(session_id: &str, session_number: u32, model: &str) -> Result<(), String>`
pub type RegisterSession = fn(&str, u32, &str) -> Result<(), String>;

/// Function signature type for R4b `close_session`.
///
/// `(session_id: &str) -> Result<(), String>`
pub type CloseSession = fn(&str) -> Result<(), String>;

/// Function signature type for R5 `run_decay`.
///
/// `() -> Result<(), String>`
pub type RunDecay = fn() -> Result<(), String>;

/// Function signature type for R6 `forget_sphere`.
///
/// `(sphere_id: &str) -> Result<(), String>`
pub type ForgetSphere = fn(&str) -> Result<(), String>;

/// Function signature type for R7 `compact_old_events`.
///
/// `() -> Result<(), String>`
pub type CompactOldEvents = fn() -> Result<(), String>;

/// Function signature type for R8 `consolidate_mature_edges`.
///
/// `() -> Result<(), String>`
pub type ConsolidateMatureEdges = fn() -> Result<(), String>;

/// Function signature type for R9 `watcher_reinforce`.
///
/// `(edge_id: u64) -> Result<(), String>`
pub type WatcherReinforce = fn(u64) -> Result<(), String>;

/// Function signature type for R10 `watcher_annotate_event`.
///
/// `(event_id: u64, anomaly_class: &str, severity: u8, metric_json: &str) -> Result<(), String>`
pub type WatcherAnnotateEvent = fn(u64, &str, u8, &str) -> Result<(), String>;
