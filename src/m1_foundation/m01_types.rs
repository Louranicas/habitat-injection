//! `m01_types` — Core newtypes for cross-module boundaries.
//!
//! Every typed concept that crosses a module boundary gets a newtype here.
//! Raw `String`/`u32`/`u64`/`f64` at module boundaries is banned.
//!
//! ## Layer
//!
//! `m1_foundation`
//!
//! ## Dependencies
//!
//! None — this is the bottom of the dependency graph.
//!
//! ## Invariants
//!
//! - [`PatternWeight`] and [`Confidence`] are clamped to `[0.0, 1.0]` at
//!   construction; NaN is rejected. Downstream code trusts these bounds.
//! - [`Severity`] is clamped to `[0, 10]` at construction.
//! - [`WorkstreamId`] and [`PatternId`] reject empty strings.
//! - [`ConsentLevel`] serialises as `PascalCase` (`"Emit"`) matching the SQL
//!   `CHECK` constraint on every table.
//! - All newtypes use `#[serde(transparent)]` for zero-overhead serialisation.

use std::fmt;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SessionId
// ---------------------------------------------------------------------------

/// Session identifier — maps to habitat session numbers (S001, S002, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct SessionId(u32);

impl SessionId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S{:03}", self.0)
    }
}

impl From<u32> for SessionId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

// ---------------------------------------------------------------------------
// WorkstreamId
// ---------------------------------------------------------------------------

/// Workstream identifier — a named, in-flight work item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct WorkstreamId(String);

impl WorkstreamId {
    /// Returns `None` if `id` is empty.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let s = id.into();
        if s.is_empty() {
            None
        } else {
            Some(Self(s))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkstreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ChainId
// ---------------------------------------------------------------------------

/// Causal chain identifier — links events via `causal_parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ChainId(u64);

impl ChainId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C{}", self.0)
    }
}

impl From<u64> for ChainId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

// ---------------------------------------------------------------------------
// PatternId
// ---------------------------------------------------------------------------

/// Pattern identifier — a named learned pattern (e.g. "session-071-convergence-trap").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct PatternId(String);

impl PatternId {
    /// Returns `None` if `id` is empty.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let s = id.into();
        if s.is_empty() {
            None
        } else {
            Some(Self(s))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PatternId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ConsentLevel
// ---------------------------------------------------------------------------

/// Consent level per NA-R2: controls what data a sphere allows.
///
/// Serialises as `"Emit"` / `"Store"` / `"Forget"` — matching the
/// `CHECK(consent IN ('Emit', 'Store', 'Forget'))` constraint in every
/// `SQLite` table from the deliberation consensus schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentLevel {
    /// Full data capture — events, edges, gradients all stored and injected.
    #[default]
    Emit,
    /// Store data but do not inject into context windows.
    Store,
    /// Delete/redact all data for this sphere (NA-P-13 cascade).
    Forget,
}

impl ConsentLevel {
    /// Parse from the SQL string representation.
    ///
    /// Returns `None` for unrecognised values (e.g. `"emit"` instead of `"Emit"`).
    #[must_use]
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "Emit" => Some(Self::Emit),
            "Store" => Some(Self::Store),
            "Forget" => Some(Self::Forget),
            _ => None,
        }
    }

    /// Returns the SQL-compatible string representation.
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Store => "Store",
            Self::Forget => "Forget",
        }
    }

    /// Whether this level permits injection into context windows.
    #[must_use]
    pub const fn permits_injection(&self) -> bool {
        matches!(self, Self::Emit)
    }

    /// Whether this level permits persistent storage.
    #[must_use]
    pub const fn permits_storage(&self) -> bool {
        matches!(self, Self::Emit | Self::Store)
    }
}

impl fmt::Display for ConsentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_sql())
    }
}


// ---------------------------------------------------------------------------
// ChainType
// ---------------------------------------------------------------------------

/// Classification of a causal chain entry.
///
/// Matches `CHECK(chain_type IN ('bug', 'trap', 'plan', 'pattern'))` in the
/// `causal_chain` table from deliberation consensus schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    /// A software defect (BUG-NNN pattern).
    Bug,
    /// A recurring pitfall that wastes effort if not recognised.
    Trap,
    /// A multi-session plan or initiative.
    Plan,
    /// A recurring behavioural or architectural pattern.
    Pattern,
}

impl ChainType {
    /// Parse from the SQL string representation.
    #[must_use]
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "bug" => Some(Self::Bug),
            "trap" => Some(Self::Trap),
            "plan" => Some(Self::Plan),
            "pattern" => Some(Self::Pattern),
            _ => None,
        }
    }

    /// Returns the SQL-compatible string representation.
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::Trap => "trap",
            Self::Plan => "plan",
            Self::Pattern => "pattern",
        }
    }
}

impl fmt::Display for ChainType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_sql())
    }
}

// ---------------------------------------------------------------------------
// PatternCategory
// ---------------------------------------------------------------------------

/// Classification of a reinforced pattern.
///
/// Matches `CHECK(category IN ('procedural', 'semantic', 'trap', 'feedback'))`
/// in the `reinforced_pattern` table from deliberation consensus schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternCategory {
    /// How-to knowledge — build sequences, deployment steps, tool usage.
    Procedural,
    /// Factual knowledge — architecture decisions, invariants, constraints.
    Semantic,
    /// Anti-patterns to avoid — known failure modes.
    Trap,
    /// User feedback and collaboration preferences.
    Feedback,
}

impl PatternCategory {
    /// Parse from the SQL string representation.
    #[must_use]
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "procedural" => Some(Self::Procedural),
            "semantic" => Some(Self::Semantic),
            "trap" => Some(Self::Trap),
            "feedback" => Some(Self::Feedback),
            _ => None,
        }
    }

    /// Returns the SQL-compatible string representation.
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Procedural => "procedural",
            Self::Semantic => "semantic",
            Self::Trap => "trap",
            Self::Feedback => "feedback",
        }
    }
}

impl fmt::Display for PatternCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_sql())
    }
}

// ---------------------------------------------------------------------------
// PatternWeight
// ---------------------------------------------------------------------------

/// Clamped f64 in [0.0, 1.0] — used for edge weights, pattern strengths.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PatternWeight(f64);

/// Errors returned when constructing a [`PatternWeight`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternWeightError {
    /// The value was NaN.
    IsNan,
}

impl fmt::Display for PatternWeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsNan => f.write_str("pattern weight must not be NaN"),
        }
    }
}

impl std::error::Error for PatternWeightError {}

impl PatternWeight {
    /// Clamps `value` to [0.0, 1.0].
    ///
    /// # Errors
    ///
    /// Returns [`PatternWeightError::IsNan`] if `value` is NaN.
    pub fn new(value: f64) -> Result<Self, PatternWeightError> {
        if value.is_nan() {
            return Err(PatternWeightError::IsNan);
        }
        Ok(Self(value.clamp(0.0, 1.0)))
    }

    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }

    /// Reinforce: weight += rate * (1.0 - weight). Saturates at 1.0.
    ///
    /// If `rate` is NaN, returns `self` unchanged to preserve the NaN-free invariant.
    #[must_use]
    pub fn reinforce(self, rate: f64) -> Self {
        if rate.is_nan() {
            return self;
        }
        let new = self.0 + rate * (1.0 - self.0);
        Self(new.clamp(0.0, 1.0))
    }

    /// Decay: weight *= factor. Saturates at 0.0.
    ///
    /// If `factor` is NaN, returns `self` unchanged to preserve the NaN-free invariant.
    #[must_use]
    pub fn decay(self, factor: f64) -> Self {
        if factor.is_nan() {
            return self;
        }
        let new = self.0 * factor;
        Self(new.clamp(0.0, 1.0))
    }

    /// Below the prune threshold?
    #[must_use]
    pub fn is_below_threshold(self, threshold: f64) -> bool {
        self.0 < threshold
    }

    /// Zero weight.
    pub const ZERO: Self = Self(0.0);

    /// Maximum weight.
    pub const MAX: Self = Self(1.0);
}

impl fmt::Display for PatternWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl Eq for PatternWeight {}

// f64 can't derive Ord; total_cmp is correct because new() rejects NaN.
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for PatternWeight {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for PatternWeight {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// TokenBudget
// ---------------------------------------------------------------------------

/// Token budget for injection payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TokenBudget(u32);

impl TokenBudget {
    #[must_use]
    pub const fn new(tokens: u32) -> Self {
        Self(tokens)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Subtract `used` tokens, saturating at zero.
    #[must_use]
    pub const fn consume(self, used: u32) -> Self {
        Self(self.0.saturating_sub(used))
    }

    /// Is there any budget remaining?
    #[must_use]
    pub const fn is_exhausted(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for TokenBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}tok", self.0)
    }
}

impl From<u32> for TokenBudget {
    fn from(tokens: u32) -> Self {
        Self(tokens)
    }
}

// ---------------------------------------------------------------------------
// WorkstreamStatus
// ---------------------------------------------------------------------------

/// Status of an in-flight workstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkstreamStatus {
    Active,
    Blocked,
    Completed,
    Deferred,
}

impl fmt::Display for WorkstreamStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Blocked => f.write_str("blocked"),
            Self::Completed => f.write_str("completed"),
            Self::Deferred => f.write_str("deferred"),
        }
    }
}

// ---------------------------------------------------------------------------
// ThermalClass
// ---------------------------------------------------------------------------

/// Thermal classification for knowledge edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalClass {
    Critical,
    Hot,
    Warm,
    Cool,
    Cold,
}

impl fmt::Display for ThermalClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => f.write_str("critical"),
            Self::Hot => f.write_str("hot"),
            Self::Warm => f.write_str("warm"),
            Self::Cool => f.write_str("cool"),
            Self::Cold => f.write_str("cold"),
        }
    }
}

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Event severity, clamped to 0–10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Severity(u8);

impl Severity {
    #[must_use]
    pub const fn new(value: u8) -> Self {
        if value > 10 {
            Self(10)
        } else {
            Self(value)
        }
    }

    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Severity 7+ triggers Watcher observation creation.
    #[must_use]
    pub const fn triggers_watcher(self) -> bool {
        self.0 >= 7
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/10", self.0)
    }
}

impl From<u8> for Severity {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// Confidence score, clamped to [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(f64);

/// Error returned when constructing a [`Confidence`] with NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceError;

impl fmt::Display for ConfidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("confidence must not be NaN")
    }
}

impl std::error::Error for ConfidenceError {}

impl Confidence {
    /// Clamps `value` to [0.0, 1.0].
    ///
    /// # Errors
    ///
    /// Returns [`ConfidenceError`] if `value` is NaN.
    pub fn new(value: f64) -> Result<Self, ConfidenceError> {
        if value.is_nan() {
            return Err(ConfidenceError);
        }
        Ok(Self(value.clamp(0.0, 1.0)))
    }

    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

impl Eq for Confidence {}

// f64 can't derive Ord; total_cmp is correct because new() rejects NaN.
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Confidence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Confidence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- SessionId --

    #[test]
    fn session_id_display() {
        assert_eq!(SessionId::new(1).to_string(), "S001");
        assert_eq!(SessionId::new(109).to_string(), "S109");
        assert_eq!(SessionId::new(0).to_string(), "S000");
    }

    #[test]
    fn session_id_roundtrip() {
        let id = SessionId::new(42);
        assert_eq!(id.as_u32(), 42);
    }

    #[test]
    fn session_id_from_u32() {
        let id: SessionId = 99u32.into();
        assert_eq!(id.as_u32(), 99);
    }

    #[test]
    fn session_id_ordering() {
        assert!(SessionId::new(1) < SessionId::new(2));
        assert_eq!(SessionId::new(5), SessionId::new(5));
    }

    #[test]
    fn session_id_serde_transparent() {
        let id = SessionId::new(108);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "108");
        let back: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn session_id_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SessionId::new(1));
        set.insert(SessionId::new(1));
        assert_eq!(set.len(), 1);
    }

    // -- WorkstreamId --

    #[test]
    fn workstream_id_valid() {
        let ws = WorkstreamId::new("comms-layer-v3");
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().as_str(), "comms-layer-v3");
    }

    #[test]
    fn workstream_id_empty_rejected() {
        assert!(WorkstreamId::new("").is_none());
    }

    #[test]
    fn workstream_id_display() {
        let ws = WorkstreamId::new("WS-6").unwrap();
        assert_eq!(ws.to_string(), "WS-6");
    }

    #[test]
    fn workstream_id_from_string() {
        let ws = WorkstreamId::new(String::from("test")).unwrap();
        assert_eq!(ws.as_str(), "test");
    }

    #[test]
    fn workstream_id_serde_roundtrip() {
        let ws = WorkstreamId::new("habitat-wire").unwrap();
        let json = serde_json::to_string(&ws).unwrap();
        let back: WorkstreamId = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, back);
    }

    #[test]
    fn workstream_id_ordering() {
        let a = WorkstreamId::new("aaa").unwrap();
        let b = WorkstreamId::new("bbb").unwrap();
        assert!(a < b);
    }

    // -- ChainId --

    #[test]
    fn chain_id_display() {
        assert_eq!(ChainId::new(12329).to_string(), "C12329");
    }

    #[test]
    fn chain_id_roundtrip() {
        let id = ChainId::new(999);
        assert_eq!(id.as_u64(), 999);
    }

    #[test]
    fn chain_id_from_u64() {
        let id: ChainId = 42u64.into();
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn chain_id_serde_transparent() {
        let id = ChainId::new(12345);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "12345");
        let back: ChainId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    // -- PatternId --

    #[test]
    fn pattern_id_valid() {
        let p = PatternId::new("session-071-convergence-trap");
        assert!(p.is_some());
    }

    #[test]
    fn pattern_id_empty_rejected() {
        assert!(PatternId::new("").is_none());
    }

    #[test]
    fn pattern_id_display() {
        let p = PatternId::new("cp-alias").unwrap();
        assert_eq!(p.to_string(), "cp-alias");
    }

    #[test]
    fn pattern_id_serde_roundtrip() {
        let p = PatternId::new("test-pattern").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: PatternId = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // -- ConsentLevel --

    #[test]
    fn consent_level_display_matches_sql() {
        assert_eq!(ConsentLevel::Emit.to_string(), "Emit");
        assert_eq!(ConsentLevel::Store.to_string(), "Store");
        assert_eq!(ConsentLevel::Forget.to_string(), "Forget");
    }

    #[test]
    fn consent_level_default_is_emit() {
        assert_eq!(ConsentLevel::default(), ConsentLevel::Emit);
    }

    #[test]
    fn consent_level_serde_roundtrip() {
        for level in [ConsentLevel::Emit, ConsentLevel::Store, ConsentLevel::Forget] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ConsentLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn consent_level_serde_pascal_case() {
        assert_eq!(serde_json::to_string(&ConsentLevel::Emit).unwrap(), "\"Emit\"");
        assert_eq!(serde_json::to_string(&ConsentLevel::Store).unwrap(), "\"Store\"");
        assert_eq!(serde_json::to_string(&ConsentLevel::Forget).unwrap(), "\"Forget\"");
    }

    #[test]
    fn consent_level_from_sql() {
        assert_eq!(ConsentLevel::from_sql("Emit"), Some(ConsentLevel::Emit));
        assert_eq!(ConsentLevel::from_sql("Store"), Some(ConsentLevel::Store));
        assert_eq!(ConsentLevel::from_sql("Forget"), Some(ConsentLevel::Forget));
        assert_eq!(ConsentLevel::from_sql("emit"), None);
        assert_eq!(ConsentLevel::from_sql(""), None);
        assert_eq!(ConsentLevel::from_sql("EMIT"), None);
    }

    #[test]
    fn consent_level_as_sql() {
        assert_eq!(ConsentLevel::Emit.as_sql(), "Emit");
        assert_eq!(ConsentLevel::Store.as_sql(), "Store");
        assert_eq!(ConsentLevel::Forget.as_sql(), "Forget");
    }

    #[test]
    fn consent_level_permits_injection() {
        assert!(ConsentLevel::Emit.permits_injection());
        assert!(!ConsentLevel::Store.permits_injection());
        assert!(!ConsentLevel::Forget.permits_injection());
    }

    #[test]
    fn consent_level_permits_storage() {
        assert!(ConsentLevel::Emit.permits_storage());
        assert!(ConsentLevel::Store.permits_storage());
        assert!(!ConsentLevel::Forget.permits_storage());
    }

    // -- PatternWeight --

    #[test]
    fn pattern_weight_clamps_high() {
        let w = PatternWeight::new(1.5).unwrap();
        assert!((w.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_clamps_low() {
        let w = PatternWeight::new(-0.5).unwrap();
        assert!((w.as_f64()).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_nan_rejected() {
        assert!(PatternWeight::new(f64::NAN).is_err());
    }

    #[test]
    fn pattern_weight_normal() {
        let w = PatternWeight::new(0.75).unwrap();
        assert!((w.as_f64() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_reinforce() {
        let w = PatternWeight::new(0.5).unwrap();
        let r = w.reinforce(0.1);
        // 0.5 + 0.1 * (1.0 - 0.5) = 0.55
        assert!((r.as_f64() - 0.55).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_reinforce_at_max() {
        let w = PatternWeight::MAX;
        let r = w.reinforce(0.1);
        assert!((r.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_decay() {
        let w = PatternWeight::new(0.8).unwrap();
        let d = w.decay(0.95);
        assert!((d.as_f64() - 0.76).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_decay_at_zero() {
        let w = PatternWeight::ZERO;
        let d = w.decay(0.95);
        assert!((d.as_f64()).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_is_below_threshold() {
        let w = PatternWeight::new(0.04).unwrap();
        assert!(w.is_below_threshold(0.05));
        assert!(!w.is_below_threshold(0.04));
    }

    #[test]
    fn pattern_weight_display() {
        let w = PatternWeight::new(0.7).unwrap();
        assert_eq!(w.to_string(), "0.7000");
    }

    #[test]
    fn pattern_weight_ordering() {
        let a = PatternWeight::new(0.3).unwrap();
        let b = PatternWeight::new(0.7).unwrap();
        assert!(a < b);
        assert_eq!(a, a);
    }

    #[test]
    fn pattern_weight_serde_transparent() {
        let w = PatternWeight::new(0.42).unwrap();
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, "0.42");
        let back: PatternWeight = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn pattern_weight_constants() {
        assert!((PatternWeight::ZERO.as_f64()).abs() < f64::EPSILON);
        assert!((PatternWeight::MAX.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_reinforce_converges() {
        let mut w = PatternWeight::new(0.0).unwrap();
        for _ in 0..100 {
            w = w.reinforce(0.1);
        }
        assert!(w.as_f64() > 0.99);
    }

    #[test]
    fn pattern_weight_decay_converges_to_zero() {
        let mut w = PatternWeight::new(1.0).unwrap();
        for _ in 0..1000 {
            w = w.decay(0.95);
        }
        assert!(w.as_f64() < 0.001);
    }

    #[test]
    fn pattern_weight_reinforce_nan_rate_preserves() {
        let w = PatternWeight::new(0.5).unwrap();
        let r = w.reinforce(f64::NAN);
        assert!((r.as_f64() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_weight_decay_nan_factor_preserves() {
        let w = PatternWeight::new(0.8).unwrap();
        let d = w.decay(f64::NAN);
        assert!((d.as_f64() - 0.8).abs() < f64::EPSILON);
    }

    // -- TokenBudget --

    #[test]
    fn token_budget_display() {
        assert_eq!(TokenBudget::new(1100).to_string(), "1100tok");
    }

    #[test]
    fn token_budget_consume() {
        let b = TokenBudget::new(1100);
        let after = b.consume(200);
        assert_eq!(after.as_u32(), 900);
    }

    #[test]
    fn token_budget_consume_saturates() {
        let b = TokenBudget::new(100);
        let after = b.consume(200);
        assert_eq!(after.as_u32(), 0);
        assert!(after.is_exhausted());
    }

    #[test]
    fn token_budget_exhausted() {
        assert!(TokenBudget::new(0).is_exhausted());
        assert!(!TokenBudget::new(1).is_exhausted());
    }

    #[test]
    fn token_budget_from_u32() {
        let b: TokenBudget = 500u32.into();
        assert_eq!(b.as_u32(), 500);
    }

    #[test]
    fn token_budget_serde_transparent() {
        let b = TokenBudget::new(1100);
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "1100");
        let back: TokenBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    // -- WorkstreamStatus --

    #[test]
    fn workstream_status_display() {
        assert_eq!(WorkstreamStatus::Active.to_string(), "active");
        assert_eq!(WorkstreamStatus::Blocked.to_string(), "blocked");
        assert_eq!(WorkstreamStatus::Completed.to_string(), "completed");
        assert_eq!(WorkstreamStatus::Deferred.to_string(), "deferred");
    }

    #[test]
    fn workstream_status_serde_roundtrip() {
        for s in [
            WorkstreamStatus::Active,
            WorkstreamStatus::Blocked,
            WorkstreamStatus::Completed,
            WorkstreamStatus::Deferred,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: WorkstreamStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    // -- ThermalClass --

    #[test]
    fn thermal_class_display() {
        assert_eq!(ThermalClass::Critical.to_string(), "critical");
        assert_eq!(ThermalClass::Hot.to_string(), "hot");
        assert_eq!(ThermalClass::Warm.to_string(), "warm");
        assert_eq!(ThermalClass::Cool.to_string(), "cool");
        assert_eq!(ThermalClass::Cold.to_string(), "cold");
    }

    #[test]
    fn thermal_class_serde_roundtrip() {
        for tc in [
            ThermalClass::Critical,
            ThermalClass::Hot,
            ThermalClass::Warm,
            ThermalClass::Cool,
            ThermalClass::Cold,
        ] {
            let json = serde_json::to_string(&tc).unwrap();
            let back: ThermalClass = serde_json::from_str(&json).unwrap();
            assert_eq!(tc, back);
        }
    }

    // -- Severity --

    #[test]
    fn severity_clamps_at_10() {
        assert_eq!(Severity::new(15).as_u8(), 10);
    }

    #[test]
    fn severity_normal() {
        assert_eq!(Severity::new(7).as_u8(), 7);
    }

    #[test]
    fn severity_triggers_watcher() {
        assert!(Severity::new(7).triggers_watcher());
        assert!(Severity::new(10).triggers_watcher());
        assert!(!Severity::new(6).triggers_watcher());
        assert!(!Severity::new(0).triggers_watcher());
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::new(5).to_string(), "5/10");
    }

    #[test]
    fn severity_from_u8() {
        let s: Severity = 8u8.into();
        assert_eq!(s.as_u8(), 8);
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::new(3) < Severity::new(7));
    }

    #[test]
    fn severity_serde_transparent() {
        let s = Severity::new(9);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "9");
        let back: Severity = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    // -- Confidence --

    #[test]
    fn confidence_clamps_high() {
        let c = Confidence::new(1.5).unwrap();
        assert!((c.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_clamps_low() {
        let c = Confidence::new(-0.5).unwrap();
        assert!((c.as_f64()).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_nan_rejected() {
        assert!(Confidence::new(f64::NAN).is_err());
    }

    #[test]
    fn confidence_display() {
        let c = Confidence::new(0.875).unwrap();
        assert_eq!(c.to_string(), "0.875");
    }

    #[test]
    fn confidence_ordering() {
        let a = Confidence::new(0.3).unwrap();
        let b = Confidence::new(0.9).unwrap();
        assert!(a < b);
    }

    #[test]
    fn confidence_serde_transparent() {
        let c = Confidence::new(0.5).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "0.5");
        let back: Confidence = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    // -- ChainType --

    #[test]
    fn chain_type_display_matches_sql() {
        assert_eq!(ChainType::Bug.to_string(), "bug");
        assert_eq!(ChainType::Trap.to_string(), "trap");
        assert_eq!(ChainType::Plan.to_string(), "plan");
        assert_eq!(ChainType::Pattern.to_string(), "pattern");
    }

    #[test]
    fn chain_type_from_sql() {
        assert_eq!(ChainType::from_sql("bug"), Some(ChainType::Bug));
        assert_eq!(ChainType::from_sql("trap"), Some(ChainType::Trap));
        assert_eq!(ChainType::from_sql("plan"), Some(ChainType::Plan));
        assert_eq!(ChainType::from_sql("pattern"), Some(ChainType::Pattern));
        assert_eq!(ChainType::from_sql("Bug"), None);
        assert_eq!(ChainType::from_sql(""), None);
    }

    #[test]
    fn chain_type_as_sql_roundtrips_from_sql() {
        for ct in [ChainType::Bug, ChainType::Trap, ChainType::Plan, ChainType::Pattern] {
            assert_eq!(ChainType::from_sql(ct.as_sql()), Some(ct));
        }
    }

    #[test]
    fn chain_type_serde_roundtrip() {
        for ct in [ChainType::Bug, ChainType::Trap, ChainType::Plan, ChainType::Pattern] {
            let json = serde_json::to_string(&ct).unwrap();
            let back: ChainType = serde_json::from_str(&json).unwrap();
            assert_eq!(ct, back);
        }
    }

    // -- PatternCategory --

    #[test]
    fn pattern_category_display_matches_sql() {
        assert_eq!(PatternCategory::Procedural.to_string(), "procedural");
        assert_eq!(PatternCategory::Semantic.to_string(), "semantic");
        assert_eq!(PatternCategory::Trap.to_string(), "trap");
        assert_eq!(PatternCategory::Feedback.to_string(), "feedback");
    }

    #[test]
    fn pattern_category_from_sql() {
        assert_eq!(PatternCategory::from_sql("procedural"), Some(PatternCategory::Procedural));
        assert_eq!(PatternCategory::from_sql("semantic"), Some(PatternCategory::Semantic));
        assert_eq!(PatternCategory::from_sql("trap"), Some(PatternCategory::Trap));
        assert_eq!(PatternCategory::from_sql("feedback"), Some(PatternCategory::Feedback));
        assert_eq!(PatternCategory::from_sql("Procedural"), None);
        assert_eq!(PatternCategory::from_sql(""), None);
    }

    #[test]
    fn pattern_category_as_sql_roundtrips_from_sql() {
        for pc in [
            PatternCategory::Procedural,
            PatternCategory::Semantic,
            PatternCategory::Trap,
            PatternCategory::Feedback,
        ] {
            assert_eq!(PatternCategory::from_sql(pc.as_sql()), Some(pc));
        }
    }

    #[test]
    fn pattern_category_serde_roundtrip() {
        for pc in [
            PatternCategory::Procedural,
            PatternCategory::Semantic,
            PatternCategory::Trap,
            PatternCategory::Feedback,
        ] {
            let json = serde_json::to_string(&pc).unwrap();
            let back: PatternCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(pc, back);
        }
    }

    // -- Wire-format pinning tests --

    #[test]
    fn chain_type_serde_lowercase_wire_format() {
        assert_eq!(serde_json::to_string(&ChainType::Bug).unwrap(), "\"bug\"");
        assert_eq!(serde_json::to_string(&ChainType::Trap).unwrap(), "\"trap\"");
        assert_eq!(serde_json::to_string(&ChainType::Plan).unwrap(), "\"plan\"");
        assert_eq!(serde_json::to_string(&ChainType::Pattern).unwrap(), "\"pattern\"");
    }

    #[test]
    fn pattern_category_serde_lowercase_wire_format() {
        assert_eq!(serde_json::to_string(&PatternCategory::Procedural).unwrap(), "\"procedural\"");
        assert_eq!(serde_json::to_string(&PatternCategory::Semantic).unwrap(), "\"semantic\"");
        assert_eq!(serde_json::to_string(&PatternCategory::Trap).unwrap(), "\"trap\"");
        assert_eq!(serde_json::to_string(&PatternCategory::Feedback).unwrap(), "\"feedback\"");
    }
}
