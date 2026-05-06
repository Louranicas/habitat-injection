//! `SessionStart` hook — produces <2KB injection payload to stdout.
//!
//! Exit 0 ALWAYS — never block session start.

/// Estimate token count from a string (roughly 1 token per 4 characters).
fn estimate_tokens(s: &str) -> u32 {
    u32::try_from(s.len() / 4).unwrap_or(u32::MAX)
}

/// Notify ORAC of the injection event (fire-and-forget via HTTP POST).
///
/// Sends a JSON notification to `http://localhost:8133/notify/injection`
/// with token count, tier label, and latency. Failure is silently swallowed.
fn notify_orac(token_count: u32, tier: &str, latency_ms: u64) {
    let body = serde_json::json!({
        "source": "habitat-inject",
        "token_count": token_count,
        "tier": tier,
        "latency_ms": latency_ms,
    });
    let _ = ureq::post("http://localhost:8133/notify/injection")
        .timeout(std::time::Duration::from_millis(500))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string());
}

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

        let token_count = estimate_tokens(&result.payload);
        let tier_label = result.tier.to_string();

        // Fire-and-forget ORAC notification (will not block session start)
        notify_orac(token_count, &tier_label, result.elapsed_ms);

        print!("{}", result.payload);
    }

    #[cfg(not(feature = "sqlite"))]
    {
        print!("NO INJECTION STATE — sqlite feature not enabled.");
    }
}
