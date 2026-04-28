//! Habitat Memory Service — self-managing daemon for injection.db.
//!
//! Thread-per-connection HTTP server with background workers:
//! - Watchdog: periodic health check + self-heal (5 min interval)
//! - Timer: periodic cache rebuild (6 hour interval)
//! - Orphan cleanup on startup
//!
//! Endpoints:
//!   GET  `/health`        — cached health from watchdog (no DB query)
//!   GET  `/cache`         — raw `injection_cache` payload
//!   GET  `/status`        — live DB health query
//!   POST `/rebuild`       — trigger lightweight cache rebuild
//!   POST `/backup`        — trigger online backup
//!   POST `/heal`          — trigger `check_and_heal`
//!   POST `/tool-use-tick` — increment `PostToolUse` counter

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-memory requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    if let Err(e) = run_service() {
        eprintln!("habitat-memory: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "sqlite")]
fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m1_foundation::m05_constants::{
        AUTO_CONSOLIDATE_INTERVAL_SECS, CACHE_REBUILD_SECS, WATCHDOG_CHECK_INTERVAL_SECS,
    };
    use habitat_injection::m2_schema::m06_schema;
    use habitat_injection::m4_consolidation::{m26_backup_clone, m27_auto_consolidate, m28_health_watchdog};

    let port: u16 = std::env::var("HABITAT_MEMORY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8140);

    let config = Config::load(None);
    let db_path = config.database.path.clone();
    let backup_path = m26_backup_clone::backup_path(&db_path);

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = m06_schema::open_database(&db_path)?;
    let tables = m06_schema::list_tables(&conn)?;
    let version = m06_schema::schema_version(&conn)?;
    drop(conn);

    eprintln!(
        "habitat-memory: DB ready at {} ({} tables, v{version})",
        db_path.display(),
        tables.len(),
    );

    // Orphan cleanup on startup
    m26_backup_clone::cleanup_orphans(&db_path);

    // Initial backup if none exists
    if !backup_path.exists() && m26_backup_clone::create_backup(&db_path).is_err() {
        eprintln!("habitat-memory: initial backup creation failed");
    }

    // Shared stop flag for graceful shutdown
    let stop = Arc::new(AtomicBool::new(false));

    // Background worker: watchdog (5 min checks)
    let watchdog = m28_health_watchdog::start_watchdog(
        db_path.clone(),
        backup_path.clone(),
        Duration::from_secs(WATCHDOG_CHECK_INTERVAL_SECS),
        Duration::from_secs(CACHE_REBUILD_SECS),
        &stop,
    );

    // Background worker: consolidation timer (6 hour cache rebuilds)
    let timer_handle = m27_auto_consolidate::start_timer(
        db_path.clone(),
        Duration::from_secs(AUTO_CONSOLIDATE_INTERVAL_SECS),
        stop.clone(),
    );

    eprintln!("habitat-memory: workers started (watchdog + timer)");

    // TCP listener — thread per connection
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    listener.set_nonblocking(false)?;
    eprintln!("habitat-memory: listening on 127.0.0.1:{port}");

    for stream in listener.incoming() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let Ok(stream) = stream else { continue };

        let db = db_path.clone();
        let bak = backup_path.clone();
        let cached_health = watchdog.cached_health.clone();
        std::thread::spawn(move || {
            handle_connection(stream, &db, &bak, &cached_health);
        });
    }

    // Graceful shutdown
    stop.store(true, Ordering::Relaxed);
    watchdog.shutdown();
    let _ = timer_handle.join();
    eprintln!("habitat-memory: shutdown complete");

    Ok(())
}

#[cfg(feature = "sqlite")]
fn handle_connection(
    mut stream: std::net::TcpStream,
    db_path: &std::path::Path,
    backup_path: &std::path::Path,
    cached_health: &std::sync::Arc<std::sync::RwLock<habitat_injection::m4_consolidation::m25_self_heal::CacheHealth>>,
) {
    use std::io::{BufRead, Write};

    let mut reader = std::io::BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    let response = route_request(&request_line, db_path, backup_path, cached_health);
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[cfg(feature = "sqlite")]
fn route_request(
    request_line: &str,
    db_path: &std::path::Path,
    backup_path: &std::path::Path,
    cached_health: &std::sync::Arc<std::sync::RwLock<habitat_injection::m4_consolidation::m25_self_heal::CacheHealth>>,
) -> String {
    let is_post = request_line.starts_with("POST");

    if request_line.contains("/health") {
        return handle_health(cached_health);
    }
    if request_line.contains("/cache") {
        return handle_cache(db_path);
    }
    if request_line.contains("/status") {
        return handle_status(db_path);
    }
    if is_post && request_line.contains("/rebuild") {
        return handle_rebuild(db_path);
    }
    if is_post && request_line.contains("/backup") {
        return handle_backup(db_path);
    }
    if is_post && request_line.contains("/heal") {
        return handle_heal(db_path, backup_path);
    }
    if is_post && request_line.contains("/tool-use-tick") {
        return handle_tool_use_tick(db_path);
    }

    json_response(404, r#"{"error":"not found"}"#)
}

#[cfg(feature = "sqlite")]
fn handle_health(
    cached: &std::sync::Arc<std::sync::RwLock<habitat_injection::m4_consolidation::m25_self_heal::CacheHealth>>,
) -> String {
    let health = cached
        .read()
        .map(|h| h.clone())
        .unwrap_or_default();
    let body = serde_json::to_string(&health).unwrap_or_else(|_| {
        r#"{"status":"healthy","service":"habitat-memory"}"#.to_string()
    });
    json_response(200, &body)
}

#[cfg(feature = "sqlite")]
fn handle_cache(db_path: &std::path::Path) -> String {
    use habitat_injection::m2_schema::m06_schema;

    let conn = match m06_schema::open_database(db_path) {
        Ok(c) => c,
        Err(e) => return json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    };

    let key = habitat_injection::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
    match conn.query_row(
        "SELECT payload FROM injection_cache WHERE section = ?1",
        rusqlite::params![key],
        |r| r.get::<_, String>(0),
    ) {
        Ok(payload) => {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                payload.len(),
                payload
            )
        }
        Err(_) => json_response(404, r#"{"error":"no cache entry"}"#),
    }
}

#[cfg(feature = "sqlite")]
fn handle_status(db_path: &std::path::Path) -> String {
    use habitat_injection::m4_consolidation::m25_self_heal;
    use habitat_injection::m4_consolidation::m26_backup_clone;

    let backup_path = m26_backup_clone::backup_path(db_path);
    match m25_self_heal::check_and_heal(db_path, &backup_path, 86400) {
        Ok(health) => {
            let body = serde_json::to_string(&health)
                .unwrap_or_else(|_| r#"{"status":"unknown"}"#.to_string());
            json_response(200, &body)
        }
        Err(e) => json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    }
}

#[cfg(feature = "sqlite")]
fn handle_rebuild(db_path: &std::path::Path) -> String {
    use habitat_injection::m2_schema::m06_schema;
    use habitat_injection::m3_injection::m13b_cache_light;

    let conn = match m06_schema::open_database(db_path) {
        Ok(c) => c,
        Err(e) => return json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    };
    match m13b_cache_light::rebuild_cache_light(&conn) {
        Ok(result) => {
            let body = format!(
                r#"{{"rebuilt":true,"tokens":{},"sections":{},"elapsed_ms":{}}}"#,
                result.token_count, result.sections_rendered, result.elapsed_ms
            );
            json_response(200, &body)
        }
        Err(e) => json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    }
}

#[cfg(feature = "sqlite")]
fn handle_backup(db_path: &std::path::Path) -> String {
    use habitat_injection::m4_consolidation::m26_backup_clone;

    match m26_backup_clone::create_backup(db_path) {
        Ok(result) => {
            let body = format!(
                r#"{{"created":true,"path":"{}","size_bytes":{},"elapsed_ms":{}}}"#,
                result.path.display(),
                result.size_bytes,
                result.elapsed_ms
            );
            json_response(200, &body)
        }
        Err(e) => json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    }
}

#[cfg(feature = "sqlite")]
fn handle_heal(db_path: &std::path::Path, backup_path: &std::path::Path) -> String {
    use habitat_injection::m4_consolidation::m25_self_heal;

    match m25_self_heal::check_and_heal(db_path, backup_path, 86400) {
        Ok(health) => {
            let body = serde_json::to_string(&health)
                .unwrap_or_else(|_| r#"{"healed":true}"#.to_string());
            json_response(200, &body)
        }
        Err(e) => json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    }
}

#[cfg(feature = "sqlite")]
fn handle_tool_use_tick(db_path: &std::path::Path) -> String {
    use habitat_injection::m4_consolidation::m27_auto_consolidate;

    match m27_auto_consolidate::tick_tool_use(db_path) {
        Ok(Some(result)) => {
            let body = format!(
                r#"{{"ticked":true,"rebuilt":true,"tokens":{},"elapsed_ms":{}}}"#,
                result.token_count, result.elapsed_ms
            );
            json_response(200, &body)
        }
        Ok(None) => json_response(200, r#"{"ticked":true,"rebuilt":false}"#),
        Err(e) => json_response(500, &format!(r#"{{"error":"{e}"}}"#)),
    }
}

#[cfg(feature = "sqlite")]
fn json_response(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}
