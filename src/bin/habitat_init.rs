//! One-time database setup for the habitat-injection system.

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-init requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    use std::path::PathBuf;

    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m2_schema::m06_schema;

    fn run() -> Result<(), Box<dyn std::error::Error>> {
        let db_path: PathBuf = std::env::args().nth(1).map_or_else(
            || Config::load(None).database.path,
            PathBuf::from,
        );

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = m06_schema::open_database(&db_path)?;
        let tables = m06_schema::list_tables(&conn)?;
        let version = m06_schema::schema_version(&conn)?;

        println!(
            "Created injection.db at {} ({} tables, schema v{version})",
            db_path.display(),
            tables.len(),
        );

        Ok(())
    }

    if let Err(e) = run() {
        eprintln!("habitat-init: {e}");
        std::process::exit(1);
    }
}
