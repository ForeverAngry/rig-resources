//! ECS-shaped row helpers for security signal extraction.
//!
//! These helpers intentionally produce small, stable `rig-compose` signal
//! strings. Aggregators can lift row-level signals such as `auth.failure`
//! into higher-order signals such as `auth.failure.burst` before running the
//! security skill catalog.

use rig_compose::{InvestigationContext, Signal};

/// Common signal names consumed by the built-in security skills and pattern registry.
pub mod signals {
    pub const AUTH_SUCCESS: &str = "auth.success";
    pub const AUTH_FAILURE: &str = "auth.failure";
    pub const AUTH_FAILURE_BURST: &str = "auth.failure.burst";
    pub const BEACON_REGULAR: &str = "beacon.regular";
    pub const DNS_QUERY: &str = "dns.query";
    pub const ENTROPY_ANOMALOUS: &str = "entropy.anomalous";
    pub const FANOUT_HIGH: &str = "fanout.high";
    pub const LATERAL_MOVE: &str = "lateral.move";
    pub const NET_CONNECT: &str = "net.connect";
    pub const NETWORK_EGRESS: &str = "network.egress";
    pub const NETWORK_INGRESS: &str = "network.ingress";
    pub const PROCESS_SPAWN: &str = "process.spawn";
}

/// Classify one ECS-shaped JSON row into security signals.
///
/// The scanner is intentionally lightweight and allocation-minimal: it looks
/// for common ECS fields by string, without parsing the full JSON document.
/// Non-UTF-8 rows and rows without recognizable ECS fields return no signals.
pub fn ecs_security_signals(row: &[u8]) -> Vec<&'static str> {
    let Ok(text) = std::str::from_utf8(row) else {
        return Vec::new();
    };

    let category = find_string_field(text, "event.category");
    let action = find_string_field(text, "event.action");
    let outcome = find_string_field(text, "event.outcome");
    let direction = find_string_field(text, "network.direction");

    let mut out = Vec::new();

    if matches!(category.as_deref(), Some("authentication"))
        || matches!(
            action.as_deref(),
            Some("logon" | "login" | "user_login" | "ssh_login")
        )
    {
        match outcome.as_deref() {
            Some("success") => push_unique(&mut out, signals::AUTH_SUCCESS),
            _ => push_unique(&mut out, signals::AUTH_FAILURE),
        }
    }

    if matches!(category.as_deref(), Some("process"))
        || matches!(
            action.as_deref(),
            Some("process_start" | "process_exec" | "exec" | "spawn")
        )
    {
        push_unique(&mut out, signals::PROCESS_SPAWN);
    }

    if matches!(category.as_deref(), Some("network")) {
        push_unique(&mut out, signals::NET_CONNECT);
        match direction.as_deref() {
            Some("outbound" | "egress") => push_unique(&mut out, signals::NETWORK_EGRESS),
            Some("inbound" | "ingress") => push_unique(&mut out, signals::NETWORK_INGRESS),
            _ => {}
        }
    }

    if matches!(action.as_deref(), Some("dns_query" | "query")) {
        push_unique(&mut out, signals::DNS_QUERY);
    }

    if matches!(
        action.as_deref(),
        Some("lateral_move" | "psexec" | "wmiexec")
    ) {
        push_unique(&mut out, signals::LATERAL_MOVE);
    }

    out
}

/// Add row-level ECS security signals to `ctx`, skipping duplicates.
pub fn add_ecs_security_signals(ctx: &mut InvestigationContext, row: &[u8]) {
    for signal in ecs_security_signals(row) {
        if !ctx.has_signal(signal) {
            ctx.signals.push(Signal::new(signal));
        }
    }
}

fn push_unique(out: &mut Vec<&'static str>, signal: &'static str) {
    if !out.contains(&signal) {
        out.push(signal);
    }
}

fn find_string_field(s: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let start = s.find(&needle)?;
    let after = &s[start + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_auth_failure() {
        let row = br#"{"event.category":"authentication","event.outcome":"failure"}"#;
        let signals = ecs_security_signals(row);
        assert!(signals.contains(&signals::AUTH_FAILURE));
        assert!(!signals.contains(&signals::AUTH_SUCCESS));
    }

    #[test]
    fn extracts_lateral_chain_pieces() {
        let row = br#"{"event.category":"process","event.action":"psexec"}"#;
        let signals = ecs_security_signals(row);
        assert!(signals.contains(&signals::PROCESS_SPAWN));
        assert!(signals.contains(&signals::LATERAL_MOVE));
    }

    #[test]
    fn adds_signals_without_duplicates() {
        let row = br#"{"event.category":"network","network.direction":"outbound"}"#;
        let mut ctx = InvestigationContext::new("host", "edge");
        add_ecs_security_signals(&mut ctx, row);
        add_ecs_security_signals(&mut ctx, row);
        assert_eq!(
            ctx.signals
                .iter()
                .filter(|signal| signal.as_str() == signals::NET_CONNECT)
                .count(),
            1
        );
        assert!(ctx.has_signal(signals::NETWORK_EGRESS));
    }
}
