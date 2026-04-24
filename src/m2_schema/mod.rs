//! `m2_schema` layer modules.

pub mod m06_schema;
pub mod m07_causal_chain;
pub mod m08_trajectory;
pub mod m09_workstream;
pub mod m10_pattern;
pub mod m10b_checkpoint;

use crate::m1_foundation::m02_errors::SchemaError;

/// Shared error-mapping helper for all L2 modules.
///
/// Converts any `Display`-implementing error into [`SchemaError::Sqlite`].
#[inline]
pub(crate) fn sqlite_err(e: impl std::fmt::Display) -> SchemaError {
    SchemaError::Sqlite(e.to_string())
}
