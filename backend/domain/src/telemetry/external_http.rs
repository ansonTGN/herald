//! Shared external-HTTP observability helper (BE-D10).
//!
//! A single shared mechanism that all outbound HTTP clients
//! (`herald-infra-stripe`, `herald-infra-creem`, `herald-infra-wechat`,
//! `backend/infra/src/oauth/http_client.rs`, `backend/core/src/third/email.rs`)
//! call to produce a consistent `external.http` tracing span and a
//! `external.http.duration` histogram measurement.
//!
//! # Sensitive-data governance (`.ai/design/observability.md` §5.4)
//!
//! Only **low-cardinality, non-sensitive** attributes are ever recorded, on
//! both the span and the histogram:
//!
//! | attribute        | source                  | example            |
//! |------------------|-------------------------|--------------------|
//! | `external.host`  | URL host (domain only)  | `api.stripe.com`   |
//! | `http.request.method` | reqwest method     | `POST`            |
//!
//! The following are **never** recorded, by construction:
//!
//! - **Full URL** (path and query may carry tokens, ids, secrets —
//!   `out_trade_no`, `realm_id`, `code=...`, `session_id=...`). Only the bare
//!   host is ever passed in; [`external_http_span`] additionally defends by
//!   stripping anything after the first `/` or `?` if a caller slips a fuller
//!   string through.
//! - **Authorization / `x-api-key` / `Bearer` headers** — these clients all
//!   carry api keys or access tokens; the helper takes no header accessors.
//! - **Request/response bodies** — payloads (checkout form fields, email HTML,
//!   OAuth token exchanges) are not accepted by any function here.
//! - **`error.type`** — recorded only as a coarse category string by callers
//!   via the span's `error.type` field, never an exception message.
//!
//! # Baseline behavior
//!
//! Under the baseline (`traces_enabled=false`, BE-D03/BE-D04) the span is
//! still created in-process (cheap) but is not exported because no OTel traces
//! layer is installed. The histogram is always recorded; if no meter provider
//! has been set yet, `global::meter(...)` returns a no-op meter (acceptable —
//! BE-D04 sets the provider at main startup).

use opentelemetry::KeyValue;
use opentelemetry::global;
use tracing::Span;

/// Histogram instrument name for outbound HTTP request duration.
pub const EXTERNAL_HTTP_DURATION_METRIC: &str = "external.http.duration";

/// Meter name shared with the RED middleware (BE-D05) — kept consistent so all
/// Herald metrics land under one instrumentation scope.
const METER_NAME: &str = "herald-api";

/// Strip a URL or `host:port` string down to the bare host (no scheme, no
/// port, no path, no query).
///
/// Governance defense-in-depth: callers are expected to pass the host only,
/// but if a full URL leaks in we still reduce it to a domain so that no path
/// segment / query parameter (which may carry tokens or ids) can ever reach a
/// span attribute or metric label.
fn sanitize_host(raw: &str) -> &str {
    // Drop scheme.
    let after_scheme = match raw.find("://") {
        Some(idx) => &raw[idx + 3..],
        None => raw,
    };
    // Drop path and query.
    let host_end = after_scheme.find(['/', '?']).unwrap_or(after_scheme.len());
    let host_port = &after_scheme[..host_end];
    // Drop port (host:port -> host). Splitting on the last ':' is safe because
    // a bare host has no ':' and an IPv6 literal is rare for these external
    // APIs; if present, the bracketed form is preserved.
    match host_port.rfind(':') {
        Some(idx) if !host_port.starts_with('[') => &host_port[..idx],
        _ => host_port,
    }
}

/// Create a tracing span for an outbound HTTP request.
///
/// Records only `external.host` (domain, no path/query) and
/// `http.request.method`. See the module docs for the full governance stance.
///
/// The returned span is **not** entered; the caller wraps the actual reqwest
/// `send().await` in a `let _enter = span.enter();` (or uses `Instrument`) so
/// the span covers the network round-trip.
#[must_use = "the span must be entered/instrumented to time the request"]
pub fn external_http_span(host: &str, method: &str) -> Span {
    tracing::info_span!(
        "external.http",
        external.host = %sanitize_host(host),
        http.request.method = %method,
    )
}

/// Record an outbound HTTP request duration on the global meter.
///
/// `elapsed` is the wall-clock duration of the request (e.g. measured with
/// `std::time::Instant`). The only attribute recorded is `external.host`
/// (sanitized to the bare domain).
///
/// If no meter provider has been registered (e.g. before BE-D04 runs at main
/// startup), `global::meter(...)` returns a no-op meter and this call is
/// effectively free.
pub fn record_external_http_duration(host: &str, elapsed: std::time::Duration) {
    let meter = global::meter(METER_NAME);
    let histogram = meter
        .u64_histogram(EXTERNAL_HTTP_DURATION_METRIC)
        .with_unit("ms")
        .with_description("Outbound HTTP request duration (milliseconds).")
        .build();
    let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    // OTel `KeyValue` values must be `'static`; own the sanitized host.
    let host_owned = sanitize_host(host).to_string();
    histogram.record(
        elapsed_ms,
        &[KeyValue::new("external.host".to_string(), host_owned)],
    );
}

/// Convenience guard returned by [`timed_external_http_span`].
///
/// On drop it records the elapsed duration via
/// [`record_external_http_duration`]. Combine with `span.enter()` for full
/// coverage.
pub struct ExternalHttpTiming {
    span: Span,
    host: String,
    start: std::time::Instant,
}

impl ExternalHttpTiming {
    /// Borrow the underlying span so the caller can `enter()` it around the
    /// reqwest call.
    #[must_use]
    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl Drop for ExternalHttpTiming {
    fn drop(&mut self) {
        record_external_http_duration(&self.host, self.start.elapsed());
    }
}

/// Create a span + duration guard in one call.
///
/// The caller should enter the span (via `.span().clone().enter()` or by
/// instrumenting the future) so the span covers the network call; the
/// [`ExternalHttpTiming`] guard records the histogram on drop.
#[must_use = "the guard must be held across the reqwest send().await to time it"]
pub fn timed_external_http_span(host: &str, method: &str) -> ExternalHttpTiming {
    ExternalHttpTiming {
        span: external_http_span(host, method),
        host: sanitize_host(host).to_string(),
        start: std::time::Instant::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Brings `SubscriberExt::with` into scope so `registry().with(fmt_layer)`
    // compiles (same pattern as `herald-api::observability::metrics_extractor`).
    use tracing_subscriber::prelude::*;

    // =====================================================================
    // Pre-existing (authored by BE-D10, left unchanged): pure-function host
    // reduction + span-build-no-panic smoke test. These establish the
    // `sanitize_host` contract the field-level tests below build on.
    // =====================================================================

    #[test]
    fn sanitize_strips_scheme_path_query_and_port() {
        assert_eq!(sanitize_host("https://api.stripe.com"), "api.stripe.com");
        assert_eq!(
            sanitize_host("https://api.stripe.com/v1/checkout/sessions"),
            "api.stripe.com"
        );
        assert_eq!(
            sanitize_host("https://api.stripe.com?code=secret_token"),
            "api.stripe.com"
        );
        assert_eq!(
            sanitize_host("api.mch.weixin.qq.com"),
            "api.mch.weixin.qq.com"
        );
        assert_eq!(
            sanitize_host("https://example.com:8443/path?x=1"),
            "example.com"
        );
        assert_eq!(sanitize_host("api.resend.com/emails"), "api.resend.com");
    }

    #[test]
    fn span_does_not_carry_full_url() {
        let span = external_http_span("https://api.stripe.com/v1/events?types[]=secret", "GET");
        // tracing spans serialize attributes via their Display; we rely on the
        // sanitize_host unit test above for the host reduction and here only
        // assert the span is named correctly and can be entered without panic.
        let _enter = span.enter();
        // The span records external.host and http.request.method only — no
        // assertion on the raw value is possible without a subscriber, which
        // is the test slot's (BE-T04) responsibility.
    }

    // =====================================================================
    // BE-T04 (newly added): field-level span governance.
    //
    // The two tests above only assert `sanitize_host` in isolation and that
    // the span builds without panicking — neither inspects what the span
    // actually records on its fields. The tests below install a `tracing`
    // fmt-JSON subscriber, build an `external.http` span through the
    // production helper, emit one event inside it (so the fmt layer flushes
    // the span fields), and assert on the serialized JSON. This is the same
    // harness pattern used by the request_id span tests in
    // `herald-api::observability::metrics_extractor` (BE-T02).
    //
    // Covers: design `.ai/design/observability.md` §5.4 (external HTTP
    // target + attribute allow-list `external.host` domain, no path/query)
    // and §4.5 (governance: no full URL / api_key / token / payload).
    // =====================================================================

    /// In-memory `MakeWriter` capturing the fmt layer's bytes into a shared
    /// buffer. Canonical `MockWriter` pattern from the tracing-subscriber
    /// test suite (its `MockMakeWriter` is not publicly exported in 0.3.x).
    /// Mirrors BE-T02's harness verbatim to keep one pattern across the
    /// observability test slot.
    #[derive(Clone)]
    struct BufWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl BufWriter {
        fn new() -> Self {
            Self(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())))
        }
        fn take_string(&self) -> String {
            let mut g = self.0.lock().expect("buf writer mutex poisoned");
            String::from_utf8(std::mem::take(&mut *g)).expect("captured log is utf8")
        }
    }

    impl std::io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("buf writer mutex poisoned")
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// Build an `external.http` span through the production helper
    /// [`external_http_span`] with the given (malicious) `host` and `method`,
    /// emit one `info!` event inside it so the fmt JSON layer flushes the span
    /// fields, and return the buffered JSON. The event's message is a stable
    /// marker so the caller can locate the span-fields payload in the buffer.
    fn drive_span_through_json_subscriber(host: &str, method: &str) -> String {
        let buf = BufWriter::new();

        // Same shape as the request_id tests: registry + fmt JSON layer only.
        // No `tracing_opentelemetry` layer is installed — the contract under
        // test is which fields the span RECORDS, not how OTel would export it.
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(buf.clone()),
        );

        {
            let dispatch = tracing::Dispatch::new(subscriber);
            let _guard = tracing::dispatcher::set_default(&dispatch);
            let span = external_http_span(host, method);
            let _enter = span.enter();
            tracing::info!("external.http.span.emitted");
        }

        buf.take_string()
    }

    // ---------------------------------------------------------------------
    // Test (new): `external.host` field carries the bare domain only — no
    // path, no query, no token-shaped substring.
    // ---------------------------------------------------------------------

    /// User Story: Technical / governance invariant — outbound HTTP spans
    /// (Stripe, WeChat Pay, Resend email, OAuth token exchange) must NEVER
    /// record the request path or query, because those carry secrets and
    /// correlation ids that would be leaked to logs / traces: OAuth `code=`,
    /// WeChat `out_trade_no`, Stripe `types[]=`, `realm_id`, `session_id`
    /// (design §4.5, §5.4). The defense is layered: callers SHOULD pass the
    /// host only, but [`external_http_span`] must also strip a fuller string
    /// if one slips through. This test deliberately passes a full URL with a
    /// secret-bearing query and asserts the recorded `external.host` is the
    /// bare domain and contains NONE of the path/query/secret substrings.
    /// Covers: BE-D10 handoff "host 仅域名" + §5.4 allow-list + §4.5 governance.
    /// Failure here MUST be delegated to BE-D10 (production helper fix) — this
    /// item does not modify production code.
    #[test]
    fn external_http_span_strips_path_and_query_from_host() {
        let captured = drive_span_through_json_subscriber(
            "https://api.stripe.com/v1/charges?secret=sk_live_secret_token_xyz",
            "POST",
        );

        // The recorded `external.host` MUST be the bare domain.
        assert!(
            captured.contains("\"external.host\":\"api.stripe.com\""),
            "external.host MUST be the bare domain 'api.stripe.com'; got: {captured}"
        );
        // And MUST NOT carry any path/query/secret fragment.
        assert!(
            !captured.contains("/v1/charges"),
            "external.host leaked the request path '/v1/charges'; got: {captured}"
        );
        assert!(
            !captured.contains("secret="),
            "external.host leaked the query key 'secret='; got: {captured}"
        );
        assert!(
            !captured.contains("sk_live_secret_token_xyz"),
            "external.host leaked the secret value 'sk_live_...'; got: {captured}"
        );
    }

    // ---------------------------------------------------------------------
    // Test (new): no api_key / token / authorization / secret field name is
    // ever recorded on the span.
    // ---------------------------------------------------------------------

    /// User Story: Technical / governance invariant — these outbound clients
    /// (Stripe `Authorization: Bearer sk_...`, WeChat `mch_api_key`,
    /// Resend `x-api-key`, OAuth `client_secret` in the token-exchange body)
    /// all carry credentials. The span's recorded field set must be the
    /// allow-list (`external.host`, `http.request.method`) and NOTHING ELSE
    /// from the credential family. This test builds the span with a
    /// secret-laden host and asserts NONE of the credential-shaped field
    /// names appear as a recorded span attribute. If a future change adds a
    /// `token`/`api_key`/`secret`/`authorization` field to the
    /// `info_span!` macro, this test fails.
    /// Covers: BE-D10 handoff "api_key/token 绝不入 span" + §4.5 governance.
    #[test]
    fn external_http_span_excludes_api_key_and_token_fields() {
        let captured = drive_span_through_json_subscriber(
            // Secret-laden input that SHOULD be reduced to a bare host before
            // anything is recorded. If reduction fails, the path/query test
            // above catches it; here we additionally assert that even if the
            // helper were buggy, no CREDENTIAL field NAME is recorded.
            "https://api.resend.com?api_key=re_xxx&token=eyJhbGc",
            "POST",
        );

        // Credential-shaped field names must never appear as recorded span
        // attributes. (The input `api_key=` is a QUERY substring on `host`;
        // here we assert it is not serialized as its own span FIELD NAME.)
        for forbidden_field in [
            "\"api_key\"",
            "\"apikey\"",
            "\"token\"",
            "\"secret\"",
            "\"authorization\"",
            "\"auth\"",
            "\"bearer\"",
            "\"x-api-key\"",
            "\"password\"",
            "\"client_secret\"",
        ] {
            assert!(
                !captured.contains(forbidden_field),
                "external.http span must NOT record a '{forbidden_field}' field; got: {captured}"
            );
        }
        // And the credential VALUES from the query must not survive either.
        assert!(
            !captured.contains("re_xxx"),
            "external.http span leaked the api_key value 're_xxx'; got: {captured}"
        );
        assert!(
            !captured.contains("eyJhbGc"),
            "external.http span leaked the token value 'eyJhbGc...'; got: {captured}"
        );
    }

    // ---------------------------------------------------------------------
    // Test (new): `http.request.method` is recorded as a span field.
    // ---------------------------------------------------------------------

    /// User Story: Technical invariant — the outbound HTTP span records the
    /// request method (low cardinality: GET/POST/...) so ops can slice
    /// external-call latency/error by verb without scraping the reqwest
    /// call site. This test asserts the field EXISTS on the recorded span.
    /// Covers: BE-D10 handoff allow-list (`http.request.method`) + §5.4.
    #[test]
    fn external_http_span_records_method() {
        let captured = drive_span_through_json_subscriber("https://api.stripe.com", "POST");
        assert!(
            captured.contains("\"http.request.method\":\"POST\""),
            "external.http span MUST record http.request.method='POST'; got: {captured}"
        );
    }

    // ---------------------------------------------------------------------
    // Test (new): only the two allow-listed fields are recorded — no
    // `http.url`, no `url.full`, no `url.path`, no `url.query`.
    // ---------------------------------------------------------------------

    /// User Story: Technical / governance invariant — the OTel HTTP semantic
    /// conventions offer `http.url` / `url.full` / `url.path` / `url.query`
    /// attributes, and a well-meaning future contributor might add one. Any of
    /// these would re-introduce the full-URL leak the host reduction defends
    /// against. This test asserts the span records ONLY `external.host` and
    /// `http.request.method` from the URL family — none of the URL-bearing
    /// attribute names appears.
    /// Covers: BE-D10 handoff allow-list ("ONLY external.host +
    /// http.request.method") + §4.5 governance.
    #[test]
    fn external_http_span_has_no_url_path_or_query_attributes() {
        let captured = drive_span_through_json_subscriber("https://api.mch.weixin.qq.com", "GET");
        for forbidden_field in [
            "\"http.url\"",
            "\"url.full\"",
            "\"url.path\"",
            "\"url.query\"",
            "\"http.target\"",
            "\"http.scheme\"",
            "\"net.peer.name\"",
        ] {
            assert!(
                !captured.contains(forbidden_field),
                "external.http span must NOT record '{forbidden_field}' (only external.host + http.request.method); got: {captured}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Test (new): host reduction works for the 5 distinct external-HTTP
    // clients BE-D10 covers (Stripe / Creem / WeChat Pay / Resend email /
    // OAuth provider), locking the variety in.
    // ---------------------------------------------------------------------

    /// User Story: Technical invariant — the 5 outbound HTTP targets BE-D10
    /// instruments (Stripe, Creem, WeChat Pay, Resend, OAuth provider) all
    /// reduce to their bare public domain when passed through the helper, so
    /// low-cardinality `external.host` labels stay stable per provider. This
    /// guards against a regression that would (e.g.) keep a per-realm
    /// `out_trade_no` path segment on the WeChat host.
    /// Covers: BE-D10 handoff "host 仅域名（api.stripe.com 等）" + §5.4.
    #[test]
    fn external_http_span_reduces_all_client_hosts() {
        for (host_input, expected_bare) in [
            ("api.stripe.com", "api.stripe.com"),
            ("https://api.creem.io/v1/checkouts", "api.creem.io"),
            (
                "https://api.mch.weixin.qq.com/v3/pay/transactions/out_trade_no/abc123",
                "api.mch.weixin.qq.com",
            ),
            ("https://api.resend.com/emails", "api.resend.com"),
            (
                "https://accounts.google.com/o/oauth2/token?code=abc&state=xyz",
                "accounts.google.com",
            ),
        ] {
            let captured = drive_span_through_json_subscriber(host_input, "POST");
            assert!(
                captured.contains(&format!("\"external.host\":\"{expected_bare}\"")),
                "for input {host_input:?}: expected external.host='{expected_bare}'; got: {captured}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Test (new): recording the duration histogram does not panic, with or
    // without a global meter provider.
    // ---------------------------------------------------------------------

    /// User Story: Technical invariant — every outbound HTTP call records a
    /// duration on the global meter via [`record_external_http_duration`].
    /// At main startup the provider is set (BE-D04), but in unit tests (and
    /// briefly during main init) no provider is registered, so
    /// `global::meter(...)` returns a no-op meter. The call MUST NOT panic in
    /// either case — a panic here would crash the process on every outbound
    /// request. This test calls it both paths (bare host and full URL input)
    /// and asserts no panic; it does not inspect the recorded value (that is
    /// OTel internals, out of scope).
    /// Covers: BE-D10 handoff "duration histogram record 不 panic" + §6.1.
    #[test]
    fn record_external_http_duration_does_not_panic_without_provider() {
        // Bare host: the normal caller input.
        record_external_http_duration("api.stripe.com", std::time::Duration::from_millis(42));
        // Full URL slipped through: still must not panic, and must not leak
        // into a span — only the bare host reaches the histogram label.
        record_external_http_duration(
            "https://api.stripe.com/v1/charges?secret=sk_xxx",
            std::time::Duration::from_millis(99),
        );
        // Drop guard path: timed_external_http_span records on drop without panic.
        {
            let _timing = timed_external_http_span("api.resend.com", "POST");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
