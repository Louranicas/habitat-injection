//! Habitat Memory Service — lightweight daemon that ensures the injection DB
//! exists and serves a health endpoint for devenv integration.
//!
//! Starts by running `habitat-init` (idempotent), then listens on the
//! configured port. Responds to `/health` with DB stats.

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
    use std::io::{BufRead, Write};
    use std::net::TcpListener;

    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m2_schema::m06_schema;

    let port: u16 = std::env::var("HABITAT_MEMORY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8140);

    let config = Config::load(None);

    if let Some(parent) = config.database.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = m06_schema::open_database(&config.database.path)?;
    let tables = m06_schema::list_tables(&conn)?;
    let version = m06_schema::schema_version(&conn)?;
    drop(conn);

    eprintln!(
        "habitat-memory: DB ready at {} ({} tables, v{version})",
        config.database.path.display(),
        tables.len(),
    );

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    eprintln!("habitat-memory: listening on 127.0.0.1:{port}");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };

        let mut reader = std::io::BufReader::new(&stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            continue;
        }

        let response = if request_line.contains("/health") {
            health_response(&config.database.path)
        } else {
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
        };

        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    }

    Ok(())
}

#[cfg(feature = "sqlite")]
fn health_response(db_path: &std::path::Path) -> String {
    use habitat_injection::m2_schema::m06_schema;

    let body = match m06_schema::open_database(db_path) {
        Ok(conn) => {
            let chains = count_table(&conn, "causal_chain");
            let sessions = count_table(&conn, "session_trajectory");
            let patterns = count_table(&conn, "reinforced_pattern");
            let workstreams = count_table(&conn, "workstream");
            let version = m06_schema::schema_version(&conn).unwrap_or(0);
            format!(
                r#"{{"status":"healthy","service":"habitat-memory","version":"0.1.0","schema_version":{version},"chains":{chains},"sessions":{sessions},"patterns":{patterns},"workstreams":{workstreams},"db":"{}"}}"#,
                db_path.display()
            )
        }
        Err(e) => {
            format!(r#"{{"status":"unhealthy","error":"{e}"}}"#)
        }
    };

    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(feature = "sqlite")]
fn count_table(conn: &rusqlite::Connection, table: &str) -> u64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
        .unwrap_or(0)
}
