//! CLI for injection database backup operations: create, verify, and swap.

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-backup requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    if let Err(e) = run() {
        eprintln!("habitat-backup: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "sqlite")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m4_consolidation::m26_backup_clone;

    let args: Vec<String> = std::env::args().collect();
    let config = Config::load(None);
    let db_path = config.database.path;
    let backup_path = m26_backup_clone::backup_path(&db_path);

    match args.get(1).map(String::as_str) {
        Some("--create") => {
            let result = m26_backup_clone::create_backup(&db_path)?;
            println!(
                "Backup created: {} ({} bytes, {}ms)",
                result.path.display(),
                result.size_bytes,
                result.elapsed_ms
            );
        }
        Some("--verify") => {
            let path = args.get(2).map_or(&backup_path, |p| {
                // Leak intentionally avoided: use backup_path as default
                Box::leak(Box::new(std::path::PathBuf::from(p)))
            });
            let ok = m26_backup_clone::verify_integrity(path)?;
            if ok {
                println!("Integrity check passed: {}", path.display());
            } else {
                eprintln!("Integrity check FAILED: {}", path.display());
                std::process::exit(2);
            }
        }
        Some("--swap") => {
            if !backup_path.exists() {
                return Err(format!("no backup at {}", backup_path.display()).into());
            }
            m26_backup_clone::swap_backup_to_primary(&db_path, &backup_path)?;
            println!(
                "Swapped: {} → {}",
                backup_path.display(),
                db_path.display()
            );
        }
        _ => {
            eprintln!("usage: habitat-backup --create | --verify [PATH] | --swap");
            std::process::exit(1);
        }
    }

    Ok(())
}
