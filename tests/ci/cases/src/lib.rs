//! Shared helpers for CI test cases.
//!
//! The crate itself does not expose functionality; each test lives under
//! `tests/` and can depend on `test_utils` directly.

/// Placeholder to ensure the crate compiles even without exported items.
#[allow(dead_code)]
pub fn __keep_crate_alive() {}
