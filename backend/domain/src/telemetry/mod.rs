//! Telemetry helpers shared across crates.
//!
//! Currently exposes the external-HTTP span + duration helper used by all
//! outbound HTTP clients. See [`external_http`].

pub mod external_http;
