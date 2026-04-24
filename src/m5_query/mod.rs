//! `m5_query` layer modules.

pub mod m19_preset_queries;
pub mod m20_raw_query;
pub mod m21_fzf_browser;
pub mod m21b_scripts_engine;

use crate::m1_foundation::m02_errors::QueryError;

/// Shared error-mapping helper for all L5 modules.
///
/// Converts any `Display`-implementing error into
/// [`QueryError::ExecutionFailed`].
#[allow(dead_code)]
#[inline]
pub(crate) fn query_err(e: impl std::fmt::Display) -> QueryError {
    QueryError::ExecutionFailed(e.to_string())
}
