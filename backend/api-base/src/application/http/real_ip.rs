//! Trusted-proxy-aware real client IP configuration.
//!
//! Parsed from `[server]` config (`trusted_proxies` CIDR list + `real_ip_header`),
//! injected into request extensions by `create_router` via `axum::Extension`, and
//! read by the `ClientIp` extractor (`crate::application::http::auth::util`) to
//! decide whether forwarded headers are trustworthy.
//!
//! Security model: forwarded headers (`X-Forwarded-For`, `X-Real-IP`,
//! `CF-Connecting-IP`, ...) are only consulted when the connection's socket
//! peer falls inside one of the configured `trusted_proxies` CIDRs. A client
//! connecting directly (or via an untrusted hop) cannot influence the extracted
//! IP by setting those headers — they are ignored and the socket peer IP is
//! used. Empty `trusted_proxies` (default) trusts nothing, which is correct for
//! direct exposure.

use axum::http::HeaderName;
use ipnetwork::IpNetwork;

/// Canonical lower-cased name of the `X-Forwarded-For` header.
///
/// When `real_ip_header` equals this, the `ClientIp` extractor treats the header
/// as a right-to-left chain and applies the rightmost-untrusted algorithm,
/// rather than reading a single value as it does for `CF-Connecting-IP` /
/// `X-Real-IP`.
pub const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// Parsed trusted-proxy configuration consumed by the `ClientIp` extractor.
///
/// Constructed once at startup from `[server].trusted_proxies` +
/// `[server].real_ip_header` and shared across requests via `axum::Extension`.
#[derive(Debug, Clone)]
pub struct RealIpConfig {
    /// CIDRs whose socket peer addresses are allowed to set trusted forwarded
    /// headers. Empty = trust nothing (extractor uses socket IP only).
    pub trusted_proxies: Vec<IpNetwork>,
    /// Header a trusted proxy writes the real client IP into. See
    /// [`X_FORWARDED_FOR`] for the chain-mode special case.
    pub real_ip_header: HeaderName,
}

impl RealIpConfig {
    /// Build a `RealIpConfig` from raw config strings.
    ///
    /// Returns an error on the first invalid CIDR or header name so the caller
    /// (server startup) can fail loud rather than silently degrade to an
    /// insecure default.
    pub fn new(trusted_proxies: &[String], real_ip_header: &str) -> anyhow::Result<Self> {
        let mut nets = Vec::with_capacity(trusted_proxies.len());
        for raw in trusted_proxies {
            let trimmed = raw.trim();
            let net = trimmed
                .parse::<IpNetwork>()
                .map_err(|e| anyhow::anyhow!("invalid CIDR in trusted_proxies '{raw}': {e}"))?;
            nets.push(net);
        }
        let header = HeaderName::from_bytes(real_ip_header.as_bytes())
            .map_err(|e| anyhow::anyhow!("invalid real_ip_header '{real_ip_header}': {e}"))?;
        Ok(Self {
            trusted_proxies: nets,
            real_ip_header: header,
        })
    }

    /// Whether a socket peer IP is trusted (falls inside one of the configured
    /// CIDRs). Used by the extractor to gate forwarded-header trust.
    pub fn trusts(&self, ip: std::net::IpAddr) -> bool {
        self.trusted_proxies.iter().any(|net| net.contains(ip))
    }

    /// Whether `real_ip_header` is the standard `X-Forwarded-For` chain (vs a
    /// single-value header like `CF-Connecting-IP` / `X-Real-IP`).
    pub fn is_xff_chain(&self) -> bool {
        self.real_ip_header.as_str() == X_FORWARDED_FOR
    }
}

impl Default for RealIpConfig {
    /// Default = trust nothing; if trusted later, use X-Forwarded-For semantics.
    fn default() -> Self {
        Self {
            trusted_proxies: Vec::new(),
            real_ip_header: HeaderName::from_static(X_FORWARDED_FOR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WHY: an invalid CIDR must abort construction, not be silently skipped —
    /// a typo in production `trusted_proxies` would otherwise narrow the trust
    /// set without anyone noticing, or widen it if parsing fell back. Startup
    /// must fail loud.
    #[test]
    fn new_rejects_invalid_cidr() {
        let err = RealIpConfig::new(&["not-a-cidr".to_string()], "CF-Connecting-IP")
            .expect_err("invalid CIDR MUST error, not silently drop");
        let msg = format!("{err}");
        assert!(
            msg.contains("not-a-cidr"),
            "error should name the offending CIDR: {msg}"
        );
    }

    /// WHY: an invalid header name (control char / whitespace) must abort
    /// construction for the same reason — silent fallback would change trust
    /// semantics undetectably.
    #[test]
    fn new_rejects_invalid_header_name() {
        RealIpConfig::new(&[], "bad header").expect_err("invalid header name MUST error");
    }

    /// WHY: parses both v4 and v6 CIDRs so dual-stack deployments (e.g. IPv6
    /// Cloudflare ranges alongside v4) work from the same config field.
    #[test]
    fn new_parses_v4_and_v6_cidrs() {
        let cfg = RealIpConfig::new(
            &["10.0.0.0/8".to_string(), "2001:db8::/32".to_string()],
            "X-Real-IP",
        )
        .expect("valid v4 + v6 CIDRs parse");
        assert!(cfg.trusts("10.1.2.3".parse().unwrap()));
        assert!(cfg.trusts("2001:db8::1".parse().unwrap()));
        assert!(!cfg.trusts("8.8.8.8".parse().unwrap()));
    }

    /// WHY: header names are lower-cased by `HeaderName`, so the XFF special
    /// case must compare against the lower-cased form — a config written as
    /// "X-Forwarded-For" must still trigger chain mode.
    #[test]
    fn xff_header_detected_regardless_of_case() {
        for spelling in ["X-Forwarded-For", "x-forwarded-for", "X-FORWARDED-FOR"] {
            let cfg = RealIpConfig::new(&[], spelling).expect("valid header");
            assert!(
                cfg.is_xff_chain(),
                "spelling '{spelling}' should be detected as the XFF chain header"
            );
        }
    }

    /// WHY: a non-XFF header (CF-Connecting-IP / X-Real-IP) is read as a single
    /// value, not a chain — the extractor branch depends on this returning
    /// false, otherwise a CF header like "1.2.3.4" would be mis-parsed as a
    /// one-element chain (harmless) but a comma in any value would break.
    #[test]
    fn non_xff_header_is_single_value() {
        let cfg = RealIpConfig::new(&[], "CF-Connecting-IP").expect("valid header");
        assert!(!cfg.is_xff_chain());
    }
}
