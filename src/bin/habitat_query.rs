//! Interactive memory browser — preset queries, raw SQL, fzf mode.

#[cfg(not(feature = "sqlite"))]
fn main() {
    eprintln!("habitat-query requires the `sqlite` feature");
    std::process::exit(1);
}

#[cfg(feature = "sqlite")]
fn main() {
    use habitat_injection::m1_foundation::m03_config::Config;
    use habitat_injection::m2_schema::m06_schema;
    use habitat_injection::m5_query::{m19_preset_queries, m20_raw_query};

    fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            print_usage();
            return Ok(());
        }

        let config = Config::load(None);
        let conn = m06_schema::open_database(&config.database.path)?;
        let command = args[1].as_str();

        let output = dispatch(&conn, command, &args)?;
        if let Some(text) = output {
            print!("{text}");
        }
        Ok(())
    }

    fn dispatch(
        conn: &rusqlite::Connection,
        command: &str,
        args: &[String],
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        match command {
            "trajectory" => Ok(Some(m19_preset_queries::query_trajectory(conn, parse_limit(args, 10))?)),
            "chains" => Ok(Some(m19_preset_queries::query_chains(conn, parse_limit(args, 20))?)),
            "workstreams" => Ok(Some(m19_preset_queries::query_workstreams(conn)?)),
            "patterns" => Ok(Some(m19_preset_queries::query_patterns(conn, parse_limit(args, 20))?)),
            "summary" => Ok(Some(m19_preset_queries::query_summary(conn)?)),
            "--interactive" | "-i" => {
                run_interactive(conn)?;
                Ok(None)
            }
            "--help" | "-h" | "help" => {
                print_usage();
                Ok(None)
            }
            sql if sql.to_ascii_uppercase().starts_with("SELECT") => {
                Ok(Some(m20_raw_query::execute_raw_formatted(conn, sql)?))
            }
            other => {
                if let Ok(out) = m19_preset_queries::query_preset(conn, other) {
                    Ok(Some(out))
                } else {
                    eprintln!("Unknown command: {other}");
                    print_usage();
                    Ok(None)
                }
            }
        }
    }

    fn parse_limit(args: &[String], default: usize) -> usize {
        for (i, arg) in args.iter().enumerate() {
            if (arg == "--limit" || arg == "-n") && args.get(i + 1).is_some() {
                return args[i + 1].parse().unwrap_or(default);
            }
        }
        default
    }

    fn run_interactive(conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
        use habitat_injection::m5_query::m21_fzf_browser::is_fzf_available;

        if !is_fzf_available() {
            eprintln!("fzf not found — falling back to summary view");
            print!("{}", m19_preset_queries::query_summary(conn)?);
            return Ok(());
        }

        let input = "trajectory\nchains\nworkstreams\npatterns\nsummary";
        let mut child = std::process::Command::new("fzf")
            .args(["--header", "Select a view:", "--height", "10"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            let _ = stdin.write_all(input.as_bytes());
        }

        let output = child.wait_with_output()?;
        let choice = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !choice.is_empty() {
            print!("{}", m19_preset_queries::query_preset(conn, &choice)?);
        }

        Ok(())
    }

    fn print_usage() {
        println!("habitat-query — Interactive injection DB browser\n");
        println!("USAGE:");
        println!("  habitat-query trajectory          Last 10 session fitness arcs");
        println!("  habitat-query chains              Unresolved chains by frequency");
        println!("  habitat-query workstreams          Active + blocked workstreams");
        println!("  habitat-query patterns            Top 20 patterns by weight");
        println!("  habitat-query summary             One-line counts");
        println!("  habitat-query \"SELECT ...\"         Raw SQL passthrough");
        println!("  habitat-query --interactive        fzf browser\n");
        println!("OPTIONS:");
        println!("  --limit N, -n N                   Limit results (default varies)");
    }

    if let Err(e) = run() {
        eprintln!("habitat-query: {e}");
        std::process::exit(1);
    }
}
