//! Post-session consolidation — Hebbian decay/reinforce, trajectory capture,
//! cache rebuild, atuin KV sync.

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-consolidate requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    if let Err(e) = run_consolidate() {
        eprintln!("habitat-consolidate: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "sqlite")]
fn run_consolidate() -> Result<(), Box<dyn std::error::Error>> {
    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m2_schema::m06_schema;
    use habitat_injection::m4_consolidation::m15b_trajectory_capture::capture_trajectory;
    use habitat_injection::m4_consolidation::m16_hebbian_engine::run_consolidation;
    use habitat_injection::m4_consolidation::m17_cache_builder::rebuild_cache;
    use habitat_injection::m4_consolidation::m18_atuin_cache::{
        write_injection_cache, write_kv, AtuinCacheEntry,
    };

    let args: Vec<String> = std::env::args().collect();
    let session = parse_session_arg(&args)?;
    let fired_patterns = parse_fired_patterns(&args);

    let config = Config::load(None);
    let conn = m06_schema::open_database(&config.database.path)?;

    let snapshot = fetch_health_snapshot();
    let capture = capture_trajectory(&conn, session, &snapshot)?;

    let pattern_refs: Vec<&str> = fired_patterns.iter().map(String::as_str).collect();
    let result = run_consolidation(&conn, session, &pattern_refs)?;

    let healthy = count_healthy_services();
    let cache = rebuild_cache(&conn, session, healthy, 11, Some(snapshot.thermal_t))?;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    write_injection_cache(&AtuinCacheEntry {
        payload: cache.payload.clone(),
        token_count: cache.token_count,
        session_number: session,
        timestamp_utc: now_secs.to_string(),
    });
    write_kv("habitat.last-session", &session.to_string());

    println!(
        "Consolidated S{session:03}: {} decayed, {} reinforced, {} pruned, {} auto-resolved | trajectory: {} | cache: {} tokens",
        result.patterns_decayed,
        result.patterns_reinforced,
        result.patterns_pruned,
        result.chains_auto_resolved,
        capture.delta_summary,
        cache.token_count,
    );

    Ok(())
}

#[cfg(feature = "sqlite")]
fn parse_session_arg(args: &[String]) -> Result<u32, Box<dyn std::error::Error>> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--session" && args.get(i + 1).is_some() {
            return args[i + 1]
                .parse::<u32>()
                .map_err(|e| format!("invalid session number: {e}").into());
        }
    }
    Err("usage: habitat-consolidate --session NUM [--fired-patterns P1,P2,...]".into())
}

#[cfg(feature = "sqlite")]
fn parse_fired_patterns(args: &[String]) -> Vec<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--fired-patterns" && args.get(i + 1).is_some() {
            return args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
        }
    }
    Vec::new()
}

#[cfg(feature = "sqlite")]
fn fetch_health_snapshot() -> habitat_injection::m4_consolidation::m15b_trajectory_capture::HealthSnapshot {
    use habitat_injection::m4_consolidation::m15b_trajectory_capture::HealthSnapshot;

    let json = run_curl(&["http://localhost:8133/health"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
    let fitness = v.get("ralph_fitness").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    let ltp = v.get("hebbian_ltp_total").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    let ltd = v.get("hebbian_ltd_total").and_then(serde_json::Value::as_f64).unwrap_or(1.0);
    let ratio = if ltd > 0.0 { ltp / ltd } else { ltp };

    HealthSnapshot {
        ralph_fitness: fitness,
        field_r: 0.0,
        thermal_t: fetch_thermal(),
        ltp_ltd_ratio: ratio,
        services_healthy: count_healthy_services(),
        key_achievement: None,
    }
}

#[cfg(feature = "sqlite")]
fn fetch_thermal() -> f64 {
    let json = run_curl(&["http://localhost:8090/v3/thermal"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
    v.get("temperature").and_then(serde_json::Value::as_f64).unwrap_or(0.0)
}

#[cfg(feature = "sqlite")]
fn run_curl(args: &[&str]) -> String {
    let mut cmd_args = vec!["-s", "-m", "2"];
    cmd_args.extend_from_slice(args);
    std::process::Command::new("curl")
        .args(&cmd_args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(feature = "sqlite")]
fn count_healthy_services() -> u32 {
    let ports: [u16; 10] = [8082, 8083, 8111, 8120, 8125, 8130, 8132, 8133, 8180, 10002];
    #[allow(clippy::cast_possible_truncation)]
    let mut count = ports.iter().filter(|p| probe_health(**p, "/health")).count() as u32;
    if probe_health(8090, "/api/health") {
        count += 1;
    }
    count
}

#[cfg(feature = "sqlite")]
fn probe_health(port: u16, path: &str) -> bool {
    let url = format!("http://localhost:{port}{path}");
    std::process::Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "-m", "1", &url])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "200")
        .unwrap_or(false)
}
