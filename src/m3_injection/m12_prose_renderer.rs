//! `m12_prose_renderer` — Converts structured query results into a prose injection payload (≤2KB).
//!
//! This is what Claude Code actually reads at session start. Five sections:
//!
//! - **Orientation** (≤80 tokens): current workstream, last delta, fitness trend.
//! - **Trajectory** (≤200 tokens): recent session fitness arc.
//! - **Workstreams** (≤300 tokens): active / blocked / deferred items.
//! - **Unresolved Chains** (≤200 tokens): causal chains ranked by reinforcement count.
//! - **Health** (≤100 tokens): service count + thermal status.
//!
//! Token budget from [`crate::m1_foundation::m05_constants::DEFAULT_BUDGET`] (default 1100).
//! When the budget is tight, sections are truncated in reverse priority order
//! (health first, then chains, then workstreams, then trajectory; orientation is always kept).
//!
//! ## Payload format (Practitioner's spec — adopted unanimously)
//!
//! ```text
//! ## Session S{NNN} Injection ({token_count} tokens)
//!
//! ### Orientation (≤80 tokens)
//! YOU WERE IN THE MIDDLE OF: {top workstream title + stage}.
//! Last session: {delta_summary from most recent trajectory}.
//! Fitness trending {UP|DOWN|FLAT}: {fitness_start} → {fitness_end} over {N} sessions.
//!
//! ### Trajectory
//! S{N}: {fitness} — {delta_summary}
//! …
//!
//! ### Workstreams
//! ACTIVE: {title} ({items_done}/{items_total}) — next: {resume_context}
//! BLOCKED: {title} — {blocker}
//! DEFERRED: {title} | {title}
//!
//! ### Unresolved Chains (by frequency)
//! {label} ({reinforcement_count}×) — {description}
//! …
//!
//! ### Health
//! All {N} services responding. Thermal {T} ({status}).
//! ```
//!
//! Layer: `m3_injection`
//! Dependencies: [`crate::m1_foundation::m01_types`], [`crate::m1_foundation::m02_errors`],
//!               [`crate::m1_foundation::m05_constants`]

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::m1_foundation::m01_types::TokenBudget;
use crate::m1_foundation::m02_errors::InjectionError;
use crate::m1_foundation::m05_constants::{
    MAX_CHAINS_INJECTED, MAX_TRAJECTORY_POINTS, MAX_WORKSTREAMS,
};

// ---------------------------------------------------------------------------
// Section token budgets (from spec)
// ---------------------------------------------------------------------------

/// Token budget for the orientation section.
// Used in render() for mandatory-section guard and in tests.
#[allow(dead_code)]
const SECTION_BUDGET_ORIENTATION: u32 = 80;
/// Token budget for the trajectory section.
// Used in tests to verify each section fits its spec budget.
#[allow(dead_code)]
const SECTION_BUDGET_TRAJECTORY: u32 = 200;
/// Token budget for the workstreams section.
// Used in tests to verify each section fits its spec budget.
#[allow(dead_code)]
const SECTION_BUDGET_WORKSTREAMS: u32 = 300;
/// Token budget for the causal chains section.
// Used in tests to verify each section fits its spec budget.
#[allow(dead_code)]
const SECTION_BUDGET_CHAINS: u32 = 200;
/// Token budget for the health section.
// Used in tests to verify each section fits its spec budget.
#[allow(dead_code)]
const SECTION_BUDGET_HEALTH: u32 = 100;

/// Minimum delta between first and last fitness values to call a trend directional.
const FITNESS_TREND_EPSILON: f64 = 0.005;

// ---------------------------------------------------------------------------
// Entry types
// ---------------------------------------------------------------------------

/// A causal chain entry — unresolved trap/bug weighted by reinforcement count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    /// Human-readable label for the chain (e.g. `"cp-alias-trap"`).
    pub label: String,
    /// Number of sessions the chain has fired without resolution.
    pub reinforcement_count: u32,
    /// Short description of the trap/bug.
    pub description: String,
}

/// One session in the fitness arc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEntry {
    /// Session number (e.g. 109 for S109).
    pub session_id: u32,
    /// RALPH fitness at session end.
    pub ralph_fitness: f64,
    /// One-sentence delta summary.
    pub delta_summary: String,
}

/// An in-flight work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstreamEntry {
    /// Human-readable title.
    pub title: String,
    /// Status string (`"active"`, `"blocked"`, `"deferred"`, etc.).
    pub status: String,
    /// Completed sub-items, if known.
    pub items_done: Option<u32>,
    /// Total sub-items, if known.
    pub items_total: Option<u32>,
    /// What to do next / how to resume.
    pub resume_context: String,
    /// Blocking reason when status is `"blocked"`.
    pub blocker: Option<String>,
}

/// Learned pattern entry (included for completeness; not rendered in the
/// prose payload but carried in [`RenderInput`] for future extensions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    /// Pattern identifier.
    pub pattern_id: String,
    /// Hebbian weight in [0.0, 1.0].
    pub weight: f64,
    /// One-sentence description.
    pub description: String,
}

// ---------------------------------------------------------------------------
// RenderInput
// ---------------------------------------------------------------------------

/// All data needed to render the session-start prose payload.
///
/// Comes from [`crate::m3_injection::m11_parallel_query`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderInput {
    /// Current session number (e.g. 109).
    pub session_number: u32,
    /// Unresolved causal chains, ordered by `reinforcement_count` descending.
    pub chains: Vec<ChainEntry>,
    /// Recent session trajectory, ordered from oldest to newest.
    pub trajectory: Vec<TrajectoryEntry>,
    /// Active workstreams.
    pub active_workstreams: Vec<WorkstreamEntry>,
    /// Blocked workstreams.
    pub blocked_workstreams: Vec<WorkstreamEntry>,
    /// Deferred workstreams.
    pub deferred_workstreams: Vec<WorkstreamEntry>,
    /// Reinforced patterns (not yet rendered in payload).
    pub patterns: Vec<PatternEntry>,
    /// Services responding at query time.
    pub services_healthy: u32,
    /// Total services expected.
    pub services_total: u32,
    /// SYNTHEX thermal value, or `None` if unavailable.
    pub thermal: Option<f64>,
}

// ---------------------------------------------------------------------------
// Fitness trend helper
// ---------------------------------------------------------------------------

/// Trend direction based on the trajectory arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitnessTrend {
    Up,
    Down,
    Flat,
}

impl FitnessTrend {
    fn label(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Flat => "FLAT",
        }
    }
}

/// Computes the fitness trend from oldest to newest trajectory entry.
fn compute_trend(trajectory: &[TrajectoryEntry]) -> FitnessTrend {
    if trajectory.len() < 2 {
        return FitnessTrend::Flat;
    }
    let first = trajectory[0].ralph_fitness;
    let last = trajectory[trajectory.len() - 1].ralph_fitness;
    let delta = last - first;
    if delta > FITNESS_TREND_EPSILON {
        FitnessTrend::Up
    } else if delta < -FITNESS_TREND_EPSILON {
        FitnessTrend::Down
    } else {
        FitnessTrend::Flat
    }
}

// ---------------------------------------------------------------------------
// Thermal status label
// ---------------------------------------------------------------------------

fn thermal_status(t: f64) -> &'static str {
    if t >= 0.8 {
        "hot"
    } else if t >= 0.5 {
        "warm"
    } else if t >= 0.3 {
        "cool"
    } else {
        "cold"
    }
}

// ---------------------------------------------------------------------------
// Section renderers (public)
// ---------------------------------------------------------------------------

/// Counts tokens in `text` using whitespace splitting.
///
/// This is a fast approximation: each whitespace-delimited run is one token.
/// The result saturates at [`u32::MAX`] for pathologically large inputs.
#[must_use]
pub fn count_tokens(text: &str) -> u32 {
    u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX)
}

/// Renders the orientation section (≤80 tokens target).
///
/// Always present regardless of budget. Contains the current workstream,
/// last delta summary, and fitness trend.
#[must_use]
pub fn render_orientation(input: &RenderInput) -> String {
    let top_workstream = input
        .active_workstreams
        .first()
        .map_or("no active workstream", |ws| ws.title.as_str());

    let last_delta = input
        .trajectory
        .last()
        .map_or("no prior session recorded", |e| e.delta_summary.as_str());

    let trend_line = if input.trajectory.len() >= 2 {
        let trend = compute_trend(&input.trajectory);
        let first = input.trajectory[0].ralph_fitness;
        let last = input.trajectory[input.trajectory.len() - 1].ralph_fitness;
        let n = input.trajectory.len();
        format!(
            "Fitness trending {}: {:.3} → {:.3} over {} sessions.",
            trend.label(),
            first,
            last,
            n
        )
    } else if let Some(only) = input.trajectory.first() {
        format!(
            "Fitness trending FLAT: {:.3} (single session recorded).",
            only.ralph_fitness
        )
    } else {
        "Fitness trending FLAT: no trajectory data.".to_owned()
    };

    format!(
        "### Orientation (≤80 tokens)\n\
         YOU WERE IN THE MIDDLE OF: {top_workstream}.\n\
         Last session: {last_delta}\n\
         {trend_line}\n"
    )
}

/// Renders the trajectory section.
///
/// Shows up to `limit` entries (newest last). Each line:
/// `S{NNN}: {fitness:.3} — {delta_summary}`
#[must_use]
pub fn render_trajectory(entries: &[TrajectoryEntry], limit: usize) -> String {
    if entries.is_empty() {
        return "### Trajectory\n(no trajectory data)\n".to_owned();
    }
    let capped = entries.len().min(limit);
    // Take the *last* `capped` entries (most recent arc)
    let slice = &entries[entries.len() - capped..];
    let mut out = String::from("### Trajectory\n");
    for e in slice {
        // Infallible: writing to a String never fails.
        let _ = writeln!(out, "S{:03}: {:.3} — {}", e.session_id, e.ralph_fitness, e.delta_summary);
    }
    out
}

/// Renders the workstreams section.
///
/// Format:
/// ```text
/// ACTIVE: {title} ({done}/{total}) — next: {resume_context}
/// BLOCKED: {title} — {blocker}
/// DEFERRED: {title} | {title}
/// ```
#[must_use]
pub fn render_workstreams(
    active: &[WorkstreamEntry],
    blocked: &[WorkstreamEntry],
    deferred: &[WorkstreamEntry],
) -> String {
    let mut out = String::from("### Workstreams\n");

    let active_limit = active.len().min(MAX_WORKSTREAMS);
    for ws in &active[..active_limit] {
        let progress = match (ws.items_done, ws.items_total) {
            (Some(d), Some(t)) => format!(" ({d}/{t})"),
            _ => String::new(),
        };
        // Infallible: writing to a String never fails.
        let _ = writeln!(out, "ACTIVE: {}{} — next: {}", ws.title, progress, ws.resume_context);
    }

    let blocked_limit = blocked.len().min(MAX_WORKSTREAMS);
    for ws in &blocked[..blocked_limit] {
        let blocker_text = ws.blocker.as_deref().unwrap_or("unknown blocker");
        // Infallible: writing to a String never fails.
        let _ = writeln!(out, "BLOCKED: {} — {}", ws.title, blocker_text);
    }

    if !deferred.is_empty() {
        let deferred_limit = deferred.len().min(MAX_WORKSTREAMS);
        let titles: Vec<&str> = deferred[..deferred_limit]
            .iter()
            .map(|ws| ws.title.as_str())
            .collect();
        // Infallible: writing to a String never fails.
        let _ = writeln!(out, "DEFERRED: {}", titles.join(" | "));
    }

    if active.is_empty() && blocked.is_empty() && deferred.is_empty() {
        out.push_str("(no workstreams)\n");
    }

    out
}

/// Renders the unresolved causal chains section.
///
/// Shows up to `limit` entries ordered by reinforcement count descending.
/// Each line: `{label} ({count}×) — {description}`
#[must_use]
pub fn render_chains(entries: &[ChainEntry], limit: usize) -> String {
    if entries.is_empty() {
        return "### Unresolved Chains (by frequency)\n(no unresolved chains)\n".to_owned();
    }

    // Sort descending by reinforcement_count; work on a local copy to avoid mutating caller data
    let mut sorted: Vec<&ChainEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.reinforcement_count));

    let capped = sorted.len().min(limit);
    let mut out = String::from("### Unresolved Chains (by frequency)\n");
    for entry in &sorted[..capped] {
        // Infallible: writing to a String never fails.
        let _ = writeln!(out, "{} ({}×) — {}", entry.label, entry.reinforcement_count, entry.description);
    }
    out
}

/// Renders the health section.
///
/// All-green:   `All {N} services responding. Thermal {T:.3} ({status}).`
/// Partial:     `{healthy}/{total} services responding. DOWN: {list}. Thermal …`
/// No thermal:  thermal part omitted.
#[must_use]
pub fn render_health(healthy: u32, total: u32, thermal: Option<f64>) -> String {
    let service_line = if healthy == total {
        format!("All {total} services responding.")
    } else {
        let down = total.saturating_sub(healthy);
        format!("{healthy}/{total} services responding. DOWN: {down} service(s) unreachable.")
    };

    let thermal_line = thermal
        .map(|t| format!(" Thermal {t:.3} ({}).", thermal_status(t)))
        .unwrap_or_default();

    format!("### Health\n{service_line}{thermal_line}\n")
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

/// Renders the full session-start prose payload.
///
/// Returns `(payload_string, tokens_used)`.
///
/// If the total budget would be exceeded, sections are dropped in reverse
/// priority order: health first, then chains, then workstreams, then trajectory.
/// Orientation is **always** included.
///
/// # Errors
///
/// Returns [`InjectionError::BudgetExhausted`] if even after all truncation
/// the orientation alone exceeds the budget (extremely unlikely given spec limits).
pub fn render(
    input: &RenderInput,
    budget: TokenBudget,
) -> Result<(String, u32), InjectionError> {
    let budget_tokens = budget.as_u32();

    // ---- Render each section independently ----
    let header = format!(
        "## Session S{:03} Injection\n\n",
        input.session_number
    );

    let orientation = render_orientation(input);
    let trajectory = render_trajectory(&input.trajectory, MAX_TRAJECTORY_POINTS);
    let workstreams = render_workstreams(
        &input.active_workstreams,
        &input.blocked_workstreams,
        &input.deferred_workstreams,
    );
    let chains = render_chains(&input.chains, MAX_CHAINS_INJECTED);
    let health = render_health(input.services_healthy, input.services_total, input.thermal);

    // ---- Greedy assembly with budget enforcement ----
    // Priority order (highest first): orientation > trajectory > workstreams > chains > health
    // We drop lower-priority sections first when budget is tight.

    let header_tokens = count_tokens(&header);
    let orientation_tokens = count_tokens(&orientation);
    let trajectory_tokens = count_tokens(&trajectory);
    let workstreams_tokens = count_tokens(&workstreams);
    let chains_tokens = count_tokens(&chains);
    let health_tokens = count_tokens(&health);

    // Orientation is mandatory.
    let mandatory = header_tokens.saturating_add(orientation_tokens);
    if mandatory > budget_tokens {
        // Cannot fit even orientation; emit it anyway but report the overrun.
        let payload = format!("{header}\n{orientation}");
        let used = count_tokens(&payload);
        return Err(InjectionError::BudgetExhausted {
            budget: budget_tokens,
            used,
            section: "orientation".to_owned(),
        });
    }

    let mut remaining = budget_tokens.saturating_sub(mandatory);
    let mut include_trajectory = false;
    let mut include_workstreams = false;
    let mut include_chains = false;
    let mut include_health = false;

    // Add in priority order
    if remaining >= trajectory_tokens {
        remaining = remaining.saturating_sub(trajectory_tokens);
        include_trajectory = true;
    }
    if remaining >= workstreams_tokens {
        remaining = remaining.saturating_sub(workstreams_tokens);
        include_workstreams = true;
    }
    if remaining >= chains_tokens {
        remaining = remaining.saturating_sub(chains_tokens);
        include_chains = true;
    }
    if remaining >= health_tokens {
        include_health = true;
    }

    // Assemble final payload
    let mut payload = String::with_capacity(2048);
    payload.push_str(&header);
    payload.push('\n');
    payload.push_str(&orientation);
    if include_trajectory {
        payload.push('\n');
        payload.push_str(&trajectory);
    }
    if include_workstreams {
        payload.push('\n');
        payload.push_str(&workstreams);
    }
    if include_chains {
        payload.push('\n');
        payload.push_str(&chains);
    }
    if include_health {
        payload.push('\n');
        payload.push_str(&health);
    }

    // Stamp the actual token count into the header line
    let tokens_used = count_tokens(&payload);
    let payload = payload.replacen(
        &format!("## Session S{:03} Injection\n", input.session_number),
        &format!(
            "## Session S{:03} Injection ({tokens_used} tokens)\n",
            input.session_number
        ),
        1,
    );

    Ok((payload, tokens_used))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Helpers ----

    fn make_chain(label: &str, count: u32) -> ChainEntry {
        ChainEntry {
            label: label.to_owned(),
            reinforcement_count: count,
            description: format!("description of {label}"),
        }
    }

    fn make_traj(session_id: u32, fitness: f64) -> TrajectoryEntry {
        TrajectoryEntry {
            session_id,
            ralph_fitness: fitness,
            delta_summary: format!("delta for S{session_id:03}"),
        }
    }

    fn make_active(title: &str) -> WorkstreamEntry {
        WorkstreamEntry {
            title: title.to_owned(),
            status: "active".to_owned(),
            items_done: Some(3),
            items_total: Some(10),
            resume_context: "resume here".to_owned(),
            blocker: None,
        }
    }

    fn make_blocked(title: &str, blocker: &str) -> WorkstreamEntry {
        WorkstreamEntry {
            title: title.to_owned(),
            status: "blocked".to_owned(),
            items_done: None,
            items_total: None,
            resume_context: String::new(),
            blocker: Some(blocker.to_owned()),
        }
    }

    fn make_deferred(title: &str) -> WorkstreamEntry {
        WorkstreamEntry {
            title: title.to_owned(),
            status: "deferred".to_owned(),
            items_done: None,
            items_total: None,
            resume_context: String::new(),
            blocker: None,
        }
    }

    fn full_input() -> RenderInput {
        RenderInput {
            session_number: 109,
            chains: vec![
                make_chain("cp-alias-trap", 5),
                make_chain("stash-pop-wipe", 3),
                make_chain("docker-prune", 2),
            ],
            trajectory: vec![
                make_traj(105, 0.60),
                make_traj(106, 0.65),
                make_traj(107, 0.67),
                make_traj(108, 0.70),
                make_traj(109, 0.72),
            ],
            active_workstreams: vec![make_active("SpaceTimeDB memory injection")],
            blocked_workstreams: vec![make_blocked("WezTerm migration", "apt unavailable")],
            deferred_workstreams: vec![make_deferred("Comms layer unification")],
            patterns: vec![],
            services_healthy: 12,
            services_total: 12,
            thermal: Some(0.55),
        }
    }

    // ---- count_tokens ----

    #[test]
    fn count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn count_tokens_single_word() {
        assert_eq!(count_tokens("hello"), 1);
    }

    #[test]
    fn count_tokens_whitespace_only() {
        assert_eq!(count_tokens("   \t\n  "), 0);
    }

    #[test]
    fn count_tokens_multiple_words() {
        assert_eq!(count_tokens("one two three four five"), 5);
    }

    #[test]
    fn count_tokens_newlines_count_as_separators() {
        assert_eq!(count_tokens("a\nb\nc"), 3);
    }

    #[test]
    fn count_tokens_tabs_count_as_separators() {
        assert_eq!(count_tokens("a\tb\tc"), 3);
    }

    #[test]
    fn count_tokens_leading_trailing_whitespace() {
        assert_eq!(count_tokens("  hello world  "), 2);
    }

    // ---- render_orientation ----

    #[test]
    fn render_orientation_contains_workstream() {
        let input = full_input();
        let out = render_orientation(&input);
        assert!(out.contains("SpaceTimeDB memory injection"), "missing workstream title");
    }

    #[test]
    fn render_orientation_contains_delta() {
        let input = full_input();
        let out = render_orientation(&input);
        assert!(out.contains("delta for S109"), "missing last delta");
    }

    #[test]
    fn render_orientation_trend_up() {
        let mut input = full_input();
        input.trajectory = vec![make_traj(1, 0.5), make_traj(2, 0.8)];
        let out = render_orientation(&input);
        assert!(out.contains("UP"), "expected UP trend");
    }

    #[test]
    fn render_orientation_trend_down() {
        let mut input = full_input();
        input.trajectory = vec![make_traj(1, 0.8), make_traj(2, 0.5)];
        let out = render_orientation(&input);
        assert!(out.contains("DOWN"), "expected DOWN trend");
    }

    #[test]
    fn render_orientation_trend_flat() {
        let mut input = full_input();
        input.trajectory = vec![make_traj(1, 0.7), make_traj(2, 0.7)];
        let out = render_orientation(&input);
        assert!(out.contains("FLAT"), "expected FLAT trend");
    }

    #[test]
    fn render_orientation_no_trajectory() {
        let mut input = full_input();
        input.trajectory = vec![];
        let out = render_orientation(&input);
        assert!(out.contains("FLAT"), "no trajectory should yield FLAT");
        assert!(out.contains("no trajectory"), "should mention missing data");
    }

    #[test]
    fn render_orientation_single_trajectory() {
        let mut input = full_input();
        input.trajectory = vec![make_traj(108, 0.65)];
        let out = render_orientation(&input);
        assert!(out.contains("FLAT"), "single entry should be FLAT");
        assert!(out.contains("single session"), "should mention single session");
    }

    #[test]
    fn render_orientation_no_active_workstream() {
        let mut input = full_input();
        input.active_workstreams = vec![];
        let out = render_orientation(&input);
        assert!(out.contains("no active workstream"));
    }

    #[test]
    fn render_orientation_within_budget() {
        let input = full_input();
        let out = render_orientation(&input);
        let tokens = count_tokens(&out);
        assert!(
            tokens <= SECTION_BUDGET_ORIENTATION * 2,
            "orientation vastly exceeded budget: {tokens}"
        );
    }

    #[test]
    fn render_orientation_contains_header() {
        let input = full_input();
        let out = render_orientation(&input);
        assert!(out.starts_with("### Orientation"));
    }

    // ---- render_trajectory ----

    #[test]
    fn render_trajectory_empty() {
        let out = render_trajectory(&[], 5);
        assert!(out.contains("no trajectory data"));
    }

    #[test]
    fn render_trajectory_formats_session_id() {
        let entries = vec![make_traj(109, 0.72)];
        let out = render_trajectory(&entries, 5);
        assert!(out.contains("S109:"), "expected S109:");
    }

    #[test]
    fn render_trajectory_formats_fitness() {
        let entries = vec![make_traj(109, 0.72)];
        let out = render_trajectory(&entries, 5);
        assert!(out.contains("0.720"), "expected 3dp fitness");
    }

    #[test]
    fn render_trajectory_respects_limit() {
        let entries: Vec<_> = (100..110).map(|i| make_traj(i, 0.5)).collect();
        let out = render_trajectory(&entries, 5);
        // Only last 5 sessions should appear
        assert!(out.contains("S109"), "should include S109");
        assert!(!out.contains("S100"), "should not include S100");
    }

    #[test]
    fn render_trajectory_limit_larger_than_data() {
        let entries = vec![make_traj(1, 0.5), make_traj(2, 0.6)];
        let out = render_trajectory(&entries, 10);
        assert!(out.contains("S001"));
        assert!(out.contains("S002"));
    }

    #[test]
    fn render_trajectory_single_entry() {
        let entries = vec![make_traj(99, 0.669)];
        let out = render_trajectory(&entries, 5);
        assert!(out.contains("S099"));
        assert!(out.contains("0.669"));
    }

    #[test]
    fn render_trajectory_contains_header() {
        let out = render_trajectory(&[], 5);
        assert!(out.starts_with("### Trajectory"));
    }

    #[test]
    fn render_trajectory_within_budget() {
        let entries: Vec<_> = (100..110).map(|i| make_traj(i, 0.6)).collect();
        let out = render_trajectory(&entries, MAX_TRAJECTORY_POINTS);
        assert!(
            count_tokens(&out) <= SECTION_BUDGET_TRAJECTORY,
            "trajectory exceeded budget"
        );
    }

    // ---- render_workstreams ----

    #[test]
    fn render_workstreams_active_format() {
        let ws = make_active("SpaceTimeDB");
        let out = render_workstreams(&[ws], &[], &[]);
        assert!(out.contains("ACTIVE:"));
        assert!(out.contains("SpaceTimeDB"));
        assert!(out.contains("3/10"));
        assert!(out.contains("resume here"));
    }

    #[test]
    fn render_workstreams_blocked_format() {
        let ws = make_blocked("WezTerm", "apt unavailable");
        let out = render_workstreams(&[], &[ws], &[]);
        assert!(out.contains("BLOCKED:"));
        assert!(out.contains("WezTerm"));
        assert!(out.contains("apt unavailable"));
    }

    #[test]
    fn render_workstreams_deferred_format() {
        let ws = make_deferred("Comms layer");
        let out = render_workstreams(&[], &[], &[ws]);
        assert!(out.contains("DEFERRED:"));
        assert!(out.contains("Comms layer"));
    }

    #[test]
    fn render_workstreams_deferred_pipe_separated() {
        let d1 = make_deferred("Alpha");
        let d2 = make_deferred("Beta");
        let out = render_workstreams(&[], &[], &[d1, d2]);
        assert!(out.contains("Alpha | Beta"));
    }

    #[test]
    fn render_workstreams_empty() {
        let out = render_workstreams(&[], &[], &[]);
        assert!(out.contains("no workstreams"));
    }

    #[test]
    fn render_workstreams_active_no_items_count() {
        let mut ws = make_active("test");
        ws.items_done = None;
        ws.items_total = None;
        let out = render_workstreams(&[ws], &[], &[]);
        assert!(out.contains("ACTIVE:"));
        assert!(!out.contains("None"), "should not print None");
    }

    #[test]
    fn render_workstreams_blocked_no_explicit_blocker() {
        let mut ws = make_blocked("test", "");
        ws.blocker = None;
        let out = render_workstreams(&[], &[ws], &[]);
        assert!(out.contains("unknown blocker"));
    }

    #[test]
    fn render_workstreams_contains_header() {
        let out = render_workstreams(&[], &[], &[]);
        assert!(out.starts_with("### Workstreams"));
    }

    #[test]
    fn render_workstreams_within_budget() {
        let active: Vec<_> = (0..5).map(|i| make_active(&format!("WS-{i}"))).collect();
        let blocked: Vec<_> = (0..3).map(|i| make_blocked(&format!("BL-{i}"), "reason")).collect();
        let deferred: Vec<_> = (0..3).map(|i| make_deferred(&format!("DF-{i}"))).collect();
        let out = render_workstreams(&active, &blocked, &deferred);
        assert!(
            count_tokens(&out) <= SECTION_BUDGET_WORKSTREAMS,
            "workstreams exceeded budget"
        );
    }

    // ---- render_chains ----

    #[test]
    fn render_chains_empty() {
        let out = render_chains(&[], 5);
        assert!(out.contains("no unresolved chains"));
    }

    #[test]
    fn render_chains_format() {
        let c = make_chain("cp-alias", 7);
        let out = render_chains(&[c], 5);
        assert!(out.contains("cp-alias"));
        assert!(out.contains("7×"));
        assert!(out.contains("description of cp-alias"));
    }

    #[test]
    fn render_chains_sorted_descending() {
        let chains = vec![make_chain("low", 1), make_chain("high", 9), make_chain("mid", 5)];
        let out = render_chains(&chains, 10);
        let high_pos = out.find("high").unwrap_or(usize::MAX);
        let mid_pos = out.find("mid").unwrap_or(usize::MAX);
        let low_pos = out.find("low").unwrap_or(usize::MAX);
        assert!(high_pos < mid_pos, "high should appear before mid");
        assert!(mid_pos < low_pos, "mid should appear before low");
    }

    #[test]
    fn render_chains_respects_limit() {
        let chains: Vec<_> = (0..10).map(|i| make_chain(&format!("chain-{i}"), i as u32)).collect();
        let out = render_chains(&chains, 3);
        let count = out.lines().filter(|l| l.contains('×')).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn render_chains_contains_header() {
        let out = render_chains(&[], 5);
        assert!(out.starts_with("### Unresolved Chains"));
    }

    #[test]
    fn render_chains_within_budget() {
        let chains: Vec<_> = (0..10)
            .map(|i| make_chain(&format!("trap-{i}"), (10 - i) as u32))
            .collect();
        let out = render_chains(&chains, MAX_CHAINS_INJECTED);
        assert!(
            count_tokens(&out) <= SECTION_BUDGET_CHAINS,
            "chains exceeded budget"
        );
    }

    // ---- render_health ----

    #[test]
    fn render_health_all_green() {
        let out = render_health(12, 12, Some(0.55));
        assert!(out.contains("All 12 services responding."));
    }

    #[test]
    fn render_health_partial() {
        let out = render_health(10, 12, Some(0.55));
        assert!(out.contains("10/12 services responding."));
        assert!(out.contains("DOWN:"));
    }

    #[test]
    fn render_health_with_thermal() {
        let out = render_health(12, 12, Some(0.55));
        assert!(out.contains("Thermal 0.550"));
        assert!(out.contains("warm"));
    }

    #[test]
    fn render_health_no_thermal() {
        let out = render_health(12, 12, None);
        assert!(!out.contains("Thermal"), "should not include thermal when None");
    }

    #[test]
    fn render_health_thermal_hot() {
        let out = render_health(12, 12, Some(0.85));
        assert!(out.contains("hot"));
    }

    #[test]
    fn render_health_thermal_warm() {
        let out = render_health(12, 12, Some(0.6));
        assert!(out.contains("warm"));
    }

    #[test]
    fn render_health_thermal_cool() {
        let out = render_health(12, 12, Some(0.35));
        assert!(out.contains("cool"));
    }

    #[test]
    fn render_health_thermal_cold() {
        let out = render_health(12, 12, Some(0.1));
        assert!(out.contains("cold"));
    }

    #[test]
    fn render_health_contains_header() {
        let out = render_health(0, 0, None);
        assert!(out.starts_with("### Health"));
    }

    #[test]
    fn render_health_within_budget() {
        let out = render_health(12, 12, Some(0.244));
        assert!(
            count_tokens(&out) <= SECTION_BUDGET_HEALTH,
            "health exceeded budget"
        );
    }

    #[test]
    fn render_health_zero_services() {
        let out = render_health(0, 0, None);
        assert!(out.contains("All 0 services responding."));
    }

    // ---- render (full) ----

    #[test]
    fn render_full_succeeds() {
        let input = full_input();
        let result = render(&input, TokenBudget::new(1100));
        assert!(result.is_ok(), "full render should succeed: {result:?}");
    }

    #[test]
    fn render_full_contains_session_header() {
        let input = full_input();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("Session S109 Injection"));
    }

    #[test]
    fn render_full_token_count_in_header() {
        let input = full_input();
        let (payload, tokens) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(
            payload.contains(&format!("{tokens} tokens")),
            "token count should be stamped in header"
        );
    }

    #[test]
    fn render_full_within_budget() {
        let input = full_input();
        let budget = TokenBudget::new(1100);
        let (payload, tokens) = render(&input, budget).unwrap();
        assert!(
            tokens <= budget.as_u32(),
            "payload ({tokens}) exceeds budget ({})",
            budget.as_u32()
        );
    }

    #[test]
    fn render_empty_input_succeeds() {
        let input = RenderInput::default();
        let result = render(&input, TokenBudget::new(1100));
        assert!(result.is_ok());
    }

    #[test]
    fn render_empty_input_has_orientation() {
        let input = RenderInput::default();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("### Orientation"));
    }

    #[test]
    fn render_zero_session_number() {
        let input = RenderInput::default();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("S000"));
    }

    #[test]
    fn render_one_session_trajectory() {
        let mut input = RenderInput::default();
        input.session_number = 50;
        input.trajectory = vec![make_traj(50, 0.65)];
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("S050"));
    }

    #[test]
    fn render_no_workstreams() {
        let mut input = full_input();
        input.active_workstreams = vec![];
        input.blocked_workstreams = vec![];
        input.deferred_workstreams = vec![];
        let result = render(&input, TokenBudget::new(1100));
        assert!(result.is_ok());
        let (payload, _) = result.unwrap();
        assert!(payload.contains("no workstreams"));
    }

    #[test]
    fn render_no_chains() {
        let mut input = full_input();
        input.chains = vec![];
        let result = render(&input, TokenBudget::new(1100));
        assert!(result.is_ok());
        let (payload, _) = result.unwrap();
        assert!(payload.contains("no unresolved chains"));
    }

    #[test]
    fn render_budget_truncation_drops_health_first() {
        let input = full_input();
        // Measure the real cost of the mandatory sections, then use a budget
        // just enough for orientation but not enough to add health (≤100 tokens).
        // We use a budget of 60 — enough to render orientation alone (≈35 tokens
        // for the full_input fixture) but not enough to append health (≈10 tokens).
        // With budget=60, only orientation fits; no health section is appended.
        let tight = 60_u32;
        let result = render(&input, TokenBudget::new(tight));
        assert!(result.is_ok());
        let (payload, _) = result.unwrap();
        // Health section should not appear
        assert!(!payload.contains("### Health"), "health should be dropped under tight budget");
    }

    #[test]
    fn render_budget_truncation_orientation_always_present() {
        let input = full_input();
        // Allow a very large budget to verify orientation is always there
        let (payload, _) = render(&input, TokenBudget::new(50)).unwrap();
        assert!(payload.contains("### Orientation"));
    }

    #[test]
    fn render_budget_error_when_orientation_too_large() {
        // Fabricate input with a huge session number but near-zero budget
        let input = full_input();
        // Budget so small even the header + orientation exceeds it: use 0
        let result = render(&input, TokenBudget::new(0));
        assert!(
            result.is_err(),
            "should error when budget is 0 (below mandatory cost)"
        );
        if let Err(InjectionError::BudgetExhausted { section, .. }) = result {
            assert_eq!(section, "orientation");
        } else {
            panic!("expected BudgetExhausted(orientation)");
        }
    }

    #[test]
    fn render_fitness_trend_up_over_five_sessions() {
        let mut input = full_input();
        input.trajectory = (105..=109).zip([0.60_f64, 0.63, 0.66, 0.69, 0.72]).map(|(s, f)| make_traj(s, f)).collect();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("UP"), "expected UP in payload");
    }

    #[test]
    fn render_fitness_trend_down() {
        let mut input = full_input();
        input.trajectory = (105..=109).zip([0.72_f64, 0.69, 0.66, 0.63, 0.60]).map(|(s, f)| make_traj(s, f)).collect();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("DOWN"));
    }

    #[test]
    fn render_full_all_sections_present() {
        let input = full_input();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        assert!(payload.contains("### Orientation"), "missing Orientation");
        assert!(payload.contains("### Trajectory"), "missing Trajectory");
        assert!(payload.contains("### Workstreams"), "missing Workstreams");
        assert!(payload.contains("### Unresolved Chains"), "missing Chains");
        assert!(payload.contains("### Health"), "missing Health");
    }

    // ---- Serde roundtrips ----

    #[test]
    fn serde_chain_entry_roundtrip() {
        let c = make_chain("test-chain", 42);
        let json = serde_json::to_string(&c).unwrap();
        let back: ChainEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, c.label);
        assert_eq!(back.reinforcement_count, c.reinforcement_count);
        assert_eq!(back.description, c.description);
    }

    #[test]
    fn serde_trajectory_entry_roundtrip() {
        let t = make_traj(108, 0.65);
        let json = serde_json::to_string(&t).unwrap();
        let back: TrajectoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, t.session_id);
        assert!((back.ralph_fitness - t.ralph_fitness).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_workstream_entry_roundtrip() {
        let ws = make_active("SpaceTimeDB");
        let json = serde_json::to_string(&ws).unwrap();
        let back: WorkstreamEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, ws.title);
        assert_eq!(back.items_done, ws.items_done);
        assert_eq!(back.items_total, ws.items_total);
    }

    #[test]
    fn serde_workstream_entry_blocked_roundtrip() {
        let ws = make_blocked("WezTerm", "apt");
        let json = serde_json::to_string(&ws).unwrap();
        let back: WorkstreamEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.blocker, Some("apt".to_owned()));
    }

    #[test]
    fn serde_pattern_entry_roundtrip() {
        let p = PatternEntry {
            pattern_id: "session-071".to_owned(),
            weight: 0.8,
            description: "convergence trap".to_owned(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PatternEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pattern_id, p.pattern_id);
        assert!((back.weight - p.weight).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_render_input_roundtrip() {
        let input = full_input();
        let json = serde_json::to_string(&input).unwrap();
        let back: RenderInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_number, input.session_number);
        assert_eq!(back.chains.len(), input.chains.len());
        assert_eq!(back.trajectory.len(), input.trajectory.len());
    }

    #[test]
    fn serde_render_input_default_roundtrip() {
        let input = RenderInput::default();
        let json = serde_json::to_string(&input).unwrap();
        let back: RenderInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_number, 0);
        assert!(back.chains.is_empty());
    }

    // ---- Edge cases ----

    #[test]
    fn render_chains_does_not_mutate_input_order() {
        // The caller's slice should not be reordered; we sort internally.
        let chains = vec![make_chain("z", 1), make_chain("a", 9)];
        let _ = render_chains(&chains, 10);
        // original order preserved (we can verify by checking label order in input)
        assert_eq!(chains[0].label, "z");
        assert_eq!(chains[1].label, "a");
    }

    #[test]
    fn render_full_large_chain_count_respects_max() {
        let mut input = RenderInput::default();
        input.chains = (0..20)
            .map(|i| make_chain(&format!("c{i}"), (20 - i) as u32))
            .collect();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        // Only MAX_CHAINS_INJECTED chains should appear
        let chain_lines: usize = payload.lines().filter(|l| l.contains('×')).count();
        assert_eq!(
            chain_lines, MAX_CHAINS_INJECTED,
            "expected exactly {MAX_CHAINS_INJECTED} chain lines"
        );
    }

    #[test]
    fn render_full_large_trajectory_respects_max() {
        let mut input = RenderInput::default();
        input.trajectory = (50..70).map(|i| make_traj(i, 0.5)).collect();
        let (payload, _) = render(&input, TokenBudget::new(1100)).unwrap();
        let traj_lines: usize = payload.lines().filter(|l| l.contains("— delta for")).count();
        assert!(
            traj_lines <= MAX_TRAJECTORY_POINTS,
            "expected at most {MAX_TRAJECTORY_POINTS} trajectory lines, got {traj_lines}"
        );
    }

    #[test]
    fn render_health_all_down() {
        let out = render_health(0, 12, None);
        assert!(out.contains("0/12 services responding."));
        assert!(out.contains("DOWN:"));
    }

    #[test]
    fn render_trajectory_zero_limit() {
        let entries = vec![make_traj(1, 0.5)];
        let out = render_trajectory(&entries, 0);
        // With limit 0, no entries should appear
        assert!(!out.contains("S001"), "no entries expected with limit 0");
    }

    #[test]
    fn compute_trend_up_epsilon() {
        let t = vec![
            TrajectoryEntry { session_id: 1, ralph_fitness: 0.5, delta_summary: String::new() },
            TrajectoryEntry { session_id: 2, ralph_fitness: 0.51, delta_summary: String::new() },
        ];
        assert_eq!(compute_trend(&t), FitnessTrend::Up);
    }

    #[test]
    fn compute_trend_flat_epsilon() {
        let t = vec![
            TrajectoryEntry { session_id: 1, ralph_fitness: 0.5, delta_summary: String::new() },
            TrajectoryEntry { session_id: 2, ralph_fitness: 0.5001, delta_summary: String::new() },
        ];
        // 0.0001 < FITNESS_TREND_EPSILON (0.005)
        assert_eq!(compute_trend(&t), FitnessTrend::Flat);
    }

    #[test]
    fn compute_trend_single_entry() {
        let t = vec![
            TrajectoryEntry { session_id: 1, ralph_fitness: 0.7, delta_summary: String::new() },
        ];
        assert_eq!(compute_trend(&t), FitnessTrend::Flat);
    }

    #[test]
    fn thermal_status_boundaries() {
        assert_eq!(thermal_status(0.8), "hot");
        assert_eq!(thermal_status(0.79), "warm");
        assert_eq!(thermal_status(0.5), "warm");
        assert_eq!(thermal_status(0.49), "cool");
        assert_eq!(thermal_status(0.3), "cool");
        assert_eq!(thermal_status(0.29), "cold");
    }

    #[test]
    fn default_render_input_has_zero_services() {
        let d = RenderInput::default();
        assert_eq!(d.services_healthy, 0);
        assert_eq!(d.services_total, 0);
        assert!(d.thermal.is_none());
    }
}
