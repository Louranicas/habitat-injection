//! Validation helpers for `SpaceTimeDB` WASM module mirror types.
//!
//! Contains [`validate_event`], [`validate_edge`], and
//! [`default_learning_params`].

use super::{
    enums::EdgeType,
    tables::{HabitatEvent, KnowledgeEdge},
};

/// Default Hebbian learning parameters for new knowledge edges.
///
/// Returns `(ltp_rate, ltd_rate, decay_rate)`:
/// - `ltp_rate` — 0.05 (conservative upward reinforcement)
/// - `ltd_rate` — 0.03 (slower depression than potentiation)
/// - `decay_rate` — 0.95 (5% decay per decay cycle, ~Ebbinghaus approximation)
#[must_use]
pub fn default_learning_params() -> (f64, f64, f64) {
    (0.05, 0.03, 0.95)
}

/// Validate a [`HabitatEvent`].
///
/// # Errors
///
/// Returns a human-readable error string when:
/// - `severity` > 10
/// - `confidence` is outside [0.0, 1.0] or is NaN
/// - `event_type` is empty
/// - `source_service` is empty
#[must_use = "validation result must be checked — ignoring it skips safety checks"]
pub fn validate_event(event: &HabitatEvent) -> Result<(), String> {
    if event.severity > 10 {
        return Err(format!(
            "severity {} exceeds maximum 10",
            event.severity
        ));
    }
    if event.confidence.is_nan() || !(0.0..=1.0).contains(&event.confidence) {
        return Err(format!(
            "confidence {} is not in [0.0, 1.0]",
            event.confidence
        ));
    }
    if event.event_type.is_empty() {
        return Err("event_type must not be empty".to_string());
    }
    if event.source_service.is_empty() {
        return Err("source_service must not be empty".to_string());
    }
    Ok(())
}

/// Validate a [`KnowledgeEdge`].
///
/// # Errors
///
/// Returns a human-readable error string when:
/// - `weight` is outside [0.0, 1.0] or is NaN
/// - `edge_type` is not one of the recognised [`EdgeType`] values
/// - `source_id` is empty
/// - `target_id` is empty
#[must_use = "validation result must be checked — ignoring it skips safety checks"]
pub fn validate_edge(edge: &KnowledgeEdge) -> Result<(), String> {
    if edge.weight.is_nan() || !(0.0..=1.0).contains(&edge.weight) {
        return Err(format!("weight {} is not in [0.0, 1.0]", edge.weight));
    }
    if EdgeType::parse(&edge.edge_type).is_none() {
        return Err(format!("unknown edge_type {:?}", edge.edge_type));
    }
    if edge.source_id.is_empty() {
        return Err("source_id must not be empty".to_string());
    }
    if edge.target_id.is_empty() {
        return Err("target_id must not be empty".to_string());
    }
    Ok(())
}
