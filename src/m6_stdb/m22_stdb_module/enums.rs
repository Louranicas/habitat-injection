//! Enumerations for the `SpaceTimeDB` WASM module mirror types.
//!
//! Contains [`EdgeType`], [`EventCategory`], and [`ConsentState`].

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// EdgeType
// ---------------------------------------------------------------------------

/// Valid values for [`super::tables::KnowledgeEdge::edge_type`].
///
/// In the WASM module these would be enforced via a STDB enum or `CHECK`
/// constraint on a `String` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Pattern learned from session history.
    LearnedPattern,
    /// Hebbian pathway from `hebbian_pulse.db`.
    Hebbian,
    /// Orchestration topology edge from `service_tracking.db`.
    Orchestration,
    /// POVM knowledge-graph pathway from port 8125.
    Povm,
    /// Cross-service synergy edge from `system_synergy.db`.
    Synergy,
    /// Cross-agent learning from `service_tracking.db`.
    CrossAgent,
}

impl EdgeType {
    /// Returns the canonical string representation used in STDB and `SQLite`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LearnedPattern => "learned_pattern",
            Self::Hebbian => "hebbian",
            Self::Orchestration => "orchestration",
            Self::Povm => "povm",
            Self::Synergy => "synergy",
            Self::CrossAgent => "cross_agent",
        }
    }

    /// Parse from the string representation.
    ///
    /// Returns `None` for unrecognised values.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "learned_pattern" => Some(Self::LearnedPattern),
            "hebbian" => Some(Self::Hebbian),
            "orchestration" => Some(Self::Orchestration),
            "povm" => Some(Self::Povm),
            "synergy" => Some(Self::Synergy),
            "cross_agent" => Some(Self::CrossAgent),
            _ => None,
        }
    }
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// EventCategory
// ---------------------------------------------------------------------------

/// Valid event type prefixes for [`super::tables::HabitatEvent::event_type`].
///
/// The `event_type` field is a free-form `String` in the STDB schema (to allow
/// future extensibility), but well-known prefixes are catalogued here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// `emergence.*` — ORAC emergence events.
    Emergence,
    /// `sphere.*` — PV2 sphere lifecycle events.
    Sphere,
    /// `thermal.*` — SYNTHEX thermal adjustment events.
    Thermal,
    /// `command.*` — Atuin pre/post-exec hooks.
    Command,
    /// `watcher.*` — Watcher observation events.
    Watcher,
    /// `session.*` — Session start/stop hooks.
    Session,
    /// `service.*` — Service health events.
    Service,
    /// `hebbian.*` — Hebbian STDP pulse events.
    Hebbian,
    /// Any other event not in the well-known set.
    Other,
}

impl EventCategory {
    /// Returns the dot-prefix for this category (without trailing dot).
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Emergence => "emergence",
            Self::Sphere => "sphere",
            Self::Thermal => "thermal",
            Self::Command => "command",
            Self::Watcher => "watcher",
            Self::Session => "session",
            Self::Service => "service",
            Self::Hebbian => "hebbian",
            Self::Other => "other",
        }
    }

    /// Classify an `event_type` string by its dot-prefix.
    #[must_use]
    pub fn classify(event_type: &str) -> Self {
        let prefix = event_type
            .split_once('.')
            .map_or(event_type, |(p, _)| p);
        match prefix {
            "emergence" => Self::Emergence,
            "sphere" => Self::Sphere,
            "thermal" => Self::Thermal,
            "command" => Self::Command,
            "watcher" => Self::Watcher,
            "session" => Self::Session,
            "service" => Self::Service,
            "hebbian" => Self::Hebbian,
            _ => Self::Other,
        }
    }
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.prefix())
    }
}

// ---------------------------------------------------------------------------
// ConsentState
// ---------------------------------------------------------------------------

/// Consent state values for sphere-gated operations (NA-R2).
///
/// These mirror the `ConsentLevel` type in `m01_types` but use the STDB
/// string encoding directly to avoid a cross-layer dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentState {
    /// Full data capture — events, edges, gradients all stored and injected.
    Emit,
    /// Store data but do not inject into context windows.
    Store,
    /// Delete/redact all data for this sphere (NA-P-13 cascade).
    Forget,
}

impl ConsentState {
    /// Returns the STDB-compatible string value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Store => "Store",
            Self::Forget => "Forget",
        }
    }

    /// Parse from the STDB string representation.
    ///
    /// Returns `None` for unrecognised values.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Emit" => Some(Self::Emit),
            "Store" => Some(Self::Store),
            "Forget" => Some(Self::Forget),
            _ => None,
        }
    }

    /// Whether injection into context windows is permitted.
    #[must_use]
    pub const fn permits_injection(self) -> bool {
        matches!(self, Self::Emit)
    }
}

impl std::fmt::Display for ConsentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<crate::m1_foundation::m01_types::ConsentLevel> for ConsentState {
    fn from(level: crate::m1_foundation::m01_types::ConsentLevel) -> Self {
        match level {
            crate::m1_foundation::m01_types::ConsentLevel::Emit => Self::Emit,
            crate::m1_foundation::m01_types::ConsentLevel::Store => Self::Store,
            crate::m1_foundation::m01_types::ConsentLevel::Forget => Self::Forget,
        }
    }
}

impl From<ConsentState> for crate::m1_foundation::m01_types::ConsentLevel {
    fn from(state: ConsentState) -> Self {
        match state {
            ConsentState::Emit => Self::Emit,
            ConsentState::Store => Self::Store,
            ConsentState::Forget => Self::Forget,
        }
    }
}
