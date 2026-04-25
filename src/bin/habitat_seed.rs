//! Seed the injection database from existing Habitat data sources.

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-seed requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    if let Err(e) = run_seed() {
        eprintln!("habitat-seed: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "sqlite")]
fn run_seed() -> Result<(), Box<dyn std::error::Error>> {
    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m2_schema::m06_schema;

    let source = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    let config = Config::load(None);
    let conn = m06_schema::open_database(&config.database.path)?;

    if matches!(source.as_str(), "chains" | "all") {
        println!("Seeded {} causal chains", seed_chains(&conn)?);
    }
    if matches!(source.as_str(), "trajectory" | "all") {
        println!("Seeded {} trajectory points", seed_trajectory(&conn)?);
    }
    if matches!(source.as_str(), "workstreams" | "all") {
        println!("Seeded {} workstreams", seed_workstreams(&conn)?);
    }
    if matches!(source.as_str(), "patterns" | "all") {
        println!("Seeded {} patterns", seed_patterns(&conn)?);
    }

    if !["all", "chains", "trajectory", "workstreams", "patterns"].contains(&source.as_str()) {
        eprintln!("Unknown source: {source}");
        eprintln!("Usage: habitat-seed [all|chains|trajectory|workstreams|patterns]");
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(feature = "sqlite")]
fn is_duplicate(e: &habitat_injection::m1_foundation::m02_errors::SchemaError) -> bool {
    let msg = e.to_string();
    msg.contains("UNIQUE") || msg.contains("duplicate")
}

#[cfg(feature = "sqlite")]
fn seed_chains(conn: &rusqlite::Connection) -> Result<u32, Box<dyn std::error::Error>> {
    use habitat_injection::m2_schema::m07_causal_chain::{
        find_by_label, insert_chain, reinforce_chain,
    };

    let chains = chain_data();
    let mut count = 0u32;
    for (ctype, label, origin, desc) in &chains {
        if find_by_label(conn, label)?.is_some() {
            reinforce_chain(conn, label, *origin)?;
        } else {
            insert_chain(conn, *origin, ctype, label, desc)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(feature = "sqlite")]
fn chain_data() -> Vec<(&'static str, &'static str, u32, &'static str)> {
    vec![
        ("bug", "BUG-001-devenv-stop", 1, "devenv stop does not kill processes — leaves rogue port occupants"),
        ("bug", "BUG-008-me-eventbus", 8, "ME EventBus has 0 external publishers — bridge never fires"),
        ("bug", "BUG-034-povm-write-only", 99, "POVM writes succeed but reads return stale — raw_http_post Ok(0) root"),
        ("bug", "BUG-055-systemd", 55, "systemd unit files not yet created for Habitat services"),
        ("bug", "BUG-058-layers-bc", 58, "DevOps V3 Layers B+C carry-forward from S102"),
        ("bug", "BUG-064i-pathway-update", 108, "pathway update discarded in POVM bridge"),
        ("trap", "trap-cp-alias", 1, "cp is aliased to interactive — use /usr/bin/cp -f"),
        ("trap", "trap-pkill-exit-144", 1, "pkill exits 144 — kills && chains"),
        ("trap", "trap-curl-sf-pipe", 1, "curl -sf silences errors when piping — use curl -s"),
        ("trap", "trap-synthex-health-path", 1, "SYNTHEX health is /api/health NOT /health"),
        ("trap", "trap-me-port-8180", 1, "ME V2 is port 8180 NOT 8080"),
        ("trap", "trap-rm-tsv-not-json", 1, "RM is TSV only — NEVER send JSON to :8130"),
        ("trap", "trap-povm-hydrate-broken", 1, "POVM /hydrate broken — use /pathways"),
        ("trap", "trap-orac-cwd", 1, "ORAC must start from orac-sidecar/ CWD"),
        ("trap", "trap-pswarm-port-10002", 1, "Pswarm V2 is port 10002 NOT 10001"),
        ("trap", "trap-focus-next-pane", 1, "focus-next-pane wraps — use directional move-focus"),
        ("trap", "trap-sandbox-kills-children", 1, "Sandbox kills children — daemons via habitat-start only"),
        ("trap", "trap-stash-pop-blind", 1, "git stash pop on wrong stash wipes edits — list first"),
        ("pattern", "convergence-trap-ralph", 71, "RALPH parameter oscillation — recurs under similar conditions"),
        ("plan", "daemon-phase-g-blocked", 107, "synthex-v2 Phase G blocked on v1 streaming gate"),
        ("plan", "habitat-injection-cli", 110, "Build CLI binaries for habitat-injection Phase 1"),
        ("plan", "comms-layer-v3", 108, "Comms Layer v3 — 10/16 shipped, WS-6+ pending"),
    ]
}

#[cfg(feature = "sqlite")]
fn seed_trajectory(conn: &rusqlite::Connection) -> Result<u32, Box<dyn std::error::Error>> {
    use habitat_injection::m2_schema::m08_trajectory::insert_point;

    let points = trajectory_data();
    let mut count = 0u32;
    for &(session, fitness, r, thermal, ltp_ltd, healthy, delta, achievement) in &points {
        match insert_point(conn, session, fitness, r, thermal, ltp_ltd, healthy, delta, achievement) {
            Ok(()) => count += 1,
            Err(e) if is_duplicate(&e) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(count)
}

#[cfg(feature = "sqlite")]
#[allow(clippy::type_complexity)]
fn trajectory_data() -> Vec<(u32, f64, f64, f64, f64, u32, &'static str, Option<&'static str>)> {
    vec![
        (103, 0.576, 0.0, 0.27, 4.5, 11, "SYNTHEX v2 architecture sealed", Some("8 ADRs, 16-part deployment")),
        (104, 0.580, 0.0, 0.27, 4.5, 11, "fitness +0.004 after V8 synergy", Some("9th bridge m35i V8")),
        (105, 0.600, 0.0, 0.27, 4.5, 11, "fitness +0.020 after L1-L3 impl", Some("18/60 modules, 535 tests")),
        (106, 0.660, 0.0, 0.272, 9.88, 11, "fitness +0.060 after L7+L8 seal", Some("60/60 modules, 2693 tests")),
        (107, 0.664, 0.0, 0.272, 4.5, 11, "fitness +0.004 after daemon wireup", Some("14 commits, real PGO cycle")),
        (108, 0.669, 0.0, 0.244, 4.5, 11, "fitness +0.005 after Watcher persona", Some("WCP v1, R13 elapsed")),
        (110, 0.664, 0.0, 0.273, 4.5, 9, "fitness -0.005, library complete", Some("27 modules, 1696 tests")),
    ]
}

#[cfg(feature = "sqlite")]
fn seed_workstreams(conn: &rusqlite::Connection) -> Result<u32, Box<dyn std::error::Error>> {
    use habitat_injection::m2_schema::m09_workstream::insert_workstream;

    let items = workstream_data();
    let mut count = 0u32;
    for (ws_id, title, status, session, resume) in &items {
        match insert_workstream(conn, ws_id, title, status, *session, resume) {
            Ok(()) => count += 1,
            Err(e) if is_duplicate(&e) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(count)
}

#[cfg(feature = "sqlite")]
fn workstream_data() -> Vec<(&'static str, &'static str, &'static str, u32, &'static str)> {
    vec![
        ("comms-layer-v3", "Comms Layer Unification v3", "active", 110,
         "10/16 shipped. Next: WS-6 habitat-wire"),
        ("habitat-injection-phase1", "habitat-injection Phase 1 CLI", "active", 110,
         "Library complete (1696 tests). Build CLI binaries"),
        ("daemon-phase-g", "synthex-v2 Phase G Shadow Window", "blocked", 108,
         "External gate: v1 active streaming absent"),
        ("wezterm-migration", "WezTerm Migration", "deferred", 77,
         "WezTerm installed, config pending"),
        ("povm-phase2", "POVM-001 Phase 2 /hydrate fix", "deferred", 101,
         "POVM /hydrate returns counts only, not entries"),
        ("systemd-units", "BUG-055 systemd unit files", "deferred", 102,
         "Create systemd units for all 12 services"),
    ]
}

#[cfg(feature = "sqlite")]
fn seed_patterns(conn: &rusqlite::Connection) -> Result<u32, Box<dyn std::error::Error>> {
    use habitat_injection::m2_schema::m10_pattern::insert_pattern;

    let items = pattern_data();
    let mut count = 0u32;
    for (pid, cat, desc, anti) in &items {
        match insert_pattern(conn, pid, cat, desc, *anti) {
            Ok(()) => count += 1,
            Err(e) if is_duplicate(&e) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(count)
}

#[cfg(feature = "sqlite")]
fn pattern_data() -> Vec<(&'static str, &'static str, &'static str, Option<&'static str>)> {
    vec![
        ("sqlite-state-query", "procedural", "Use sqlite3 for state retrieval — 125x token reduction", Some("MCP read_graph for state queries")),
        ("quality-gate-chain", "procedural", "check && clippy && pedantic && test — mandatory before commit", Some("Skipping quality gate stages")),
        ("health-probe-curl", "procedural", "curl -s -o /dev/null -w '%{http_code}' — never curl -sf when piping", Some("curl -sf silences errors")),
        ("four-surface-persistence", "procedural", "Persist plans at 4 surfaces with bidirectional links", Some("Single-surface persistence")),
        ("verify-before-ship", "feedback", "Re-verify any claim before shipping", Some("Trusting unverified claims")),
        ("read-only-forensics", "feedback", "Default to read-only when investigating — probe don't touch", Some("Modifying live services")),
        ("research-first", "feedback", "grep exact signatures before modifying", Some("Modifying without reading")),
        ("enumerate-before-scoring", "feedback", "Run enumeration + decompose BEFORE scoring", Some("Scoring before enumeration")),
        ("preserve-list-discipline", "feedback", "Blanket commands rebuild their own filter", Some("Trusting upstream exclusion lists")),
        ("binary-deployment-cp", "trap", "/usr/bin/cp -f for deployment — cp alias is interactive", Some("Bare cp for deployment")),
        ("never-test-fit", "feedback", "Proceed seamlessly, fix inline, stop only if blocked", Some("Stopping at every obstacle")),
        ("cascade-to-fleet", "procedural", "Dispatch to idle fleet panes for parallel work", Some("Sequential single-pane work")),
        ("context-poisoning-banned", "feedback", "Never cite context pressure to reduce scope", Some("Using context limits as excuse")),
        ("no-phase-collapse", "feedback", "Each phase gets own impl/QG/deploy/verify cycle", Some("Merging phases under pressure")),
        ("excellence-over-speed", "feedback", "Wait for all data before synthesizing", Some("Rushing with incomplete data")),
        ("god-tier-engineering", "feedback", "Never suppress warnings or take least resistance", Some("Suppressing warnings")),
        ("direct-team-comms", "feedback", "Address peers by seat+pane, explicit verification", Some("Broadcasting without verification")),
        ("sycophancy-mitigation", "feedback", "C1-C5 checks, min 3 weaknesses before praise", Some("Praising without analysis")),
    ]
}
