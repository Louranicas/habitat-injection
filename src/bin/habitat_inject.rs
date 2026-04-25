//! `SessionStart` hook — produces <2KB injection payload to stdout.
//!
//! Exit 0 ALWAYS — never block session start.

fn main() {
    #[cfg(feature = "sqlite")]
    {
        use habitat_injection::m1_foundation::m03_config::Config;
        use habitat_injection::m3_injection::m13_fallback::execute_fallback_chain;

        let config = Config::load(None);
        let result = execute_fallback_chain(
            Some(&config.database.path),
            config.consolidation.cache_rebuild_secs,
        );
        print!("{}", result.payload);
    }

    #[cfg(not(feature = "sqlite"))]
    {
        print!("NO INJECTION STATE — sqlite feature not enabled.");
    }
}
