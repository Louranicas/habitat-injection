//! `m04_traits` — Core traits defining the contract between layers.
//!
//! Four traits define the system's behavioural boundaries:
//! - [`Injectable`] — produce a text payload under a token budget
//! - [`Consolidatable`] — write back state after a session
//! - [`Queryable`] — execute queries and return rows
//! - [`Decayable`] — apply Hebbian decay to weights
//!
//! ## Layer
//!
//! `m1_foundation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m01_types::SessionId`] — session identifier
//! - [`crate::m1_foundation::m01_types::TokenBudget`] — injection budget
//! - [`crate::m1_foundation::m02_errors::HabitatError`] — error propagation
//!
//! ## Invariants
//!
//! - All trait errors propagate as [`HabitatError`] — consumers use `?`.
//! - [`Queryable`] provides default impls for `query_one` and `count`;
//!   implementors only need `query`.
//! - [`MemorySubstrate`] is blanket-implemented — no manual impl needed.

use crate::m1_foundation::m01_types::{SessionId, TokenBudget};
use crate::m1_foundation::m02_errors::HabitatError;

/// A row returned from a query — column name to string value.
///
/// Intentionally simple: the query layer parses typed values from these strings.
/// This avoids pulling rusqlite types into the trait boundary.
pub type Row = std::collections::BTreeMap<String, String>;

/// Produces a text payload for context-window injection.
pub trait Injectable {
    /// Render this data source into a text payload that fits within `budget`.
    ///
    /// Returns the rendered string and the approximate token count consumed.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Injection` if rendering fails.
    fn inject(&self, budget: TokenBudget) -> std::result::Result<(String, u32), HabitatError>;
}

/// Writes back state after a session closes.
pub trait Consolidatable {
    /// Consolidate state for the completed `session`.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Consolidation` on failure.
    fn consolidate(&mut self, session: SessionId) -> std::result::Result<(), HabitatError>;
}

/// Executes queries and returns rows.
pub trait Queryable {
    /// Execute a SQL query and return matching rows.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Query` on failure.
    fn query(&self, sql: &str) -> std::result::Result<Vec<Row>, HabitatError>;

    /// Execute a SQL query and return the first row, if any.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Query` on execution failure (not on empty results).
    fn query_one(&self, sql: &str) -> std::result::Result<Option<Row>, HabitatError> {
        let mut rows = self.query(sql)?;
        Ok(if rows.is_empty() {
            None
        } else {
            Some(rows.swap_remove(0))
        })
    }

    /// Execute a SQL query and return a scalar count.
    ///
    /// Expects the query to return exactly one row with one column.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Query` on failure or if the result is not parseable as u64.
    fn count(&self, sql: &str) -> std::result::Result<u64, HabitatError> {
        let row = self.query_one(sql)?.ok_or_else(|| {
            crate::m1_foundation::m02_errors::QueryError::NoResults {
                query: sql.to_string(),
            }
        })?;
        let val = row
            .values()
            .next()
            .ok_or_else(|| crate::m1_foundation::m02_errors::QueryError::NoResults {
                query: sql.to_string(),
            })?;
        val.parse::<u64>().map_err(|e| {
            crate::m1_foundation::m02_errors::QueryError::ParseFailed {
                column: "count".to_string(),
                reason: e.to_string(),
            }
            .into()
        })
    }
}

/// Applies Hebbian decay to weighted items.
pub trait Decayable {
    /// Decay all eligible weights by `rate` (multiplicative: `w *= rate`).
    ///
    /// Returns the number of items decayed.
    ///
    /// # Errors
    ///
    /// Returns `HabitatError::Consolidation` on failure.
    fn decay(&mut self, rate: f64) -> std::result::Result<u32, HabitatError>;
}

/// Marker trait for types that support both injection and consolidation.
///
/// Automatically implemented for anything that is both `Injectable` and `Consolidatable`.
///
/// Note: [`Consolidatable::consolidate`] requires `&mut self`. Trait-object consumers
/// must hold `&mut dyn MemorySubstrate` (or `Box<dyn MemorySubstrate>`) to call it.
pub trait MemorySubstrate: Injectable + Consolidatable {}
impl<T: Injectable + Consolidatable> MemorySubstrate for T {}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Mock implementations for testing --

    struct MockInjectable {
        content: String,
        tokens: u32,
    }

    impl Injectable for MockInjectable {
        fn inject(&self, budget: TokenBudget) -> std::result::Result<(String, u32), HabitatError> {
            if budget.as_u32() < self.tokens {
                return Err(crate::m1_foundation::m02_errors::InjectionError::BudgetExhausted {
                    budget: budget.as_u32(),
                    used: self.tokens,
                    section: "mock".into(),
                }
                .into());
            }
            Ok((self.content.clone(), self.tokens))
        }
    }

    struct MockConsolidatable {
        consolidated: bool,
        last_session: Option<SessionId>,
    }

    impl Consolidatable for MockConsolidatable {
        fn consolidate(&mut self, session: SessionId) -> std::result::Result<(), HabitatError> {
            self.consolidated = true;
            self.last_session = Some(session);
            Ok(())
        }
    }

    struct MockQueryable {
        rows: Vec<Row>,
    }

    impl Queryable for MockQueryable {
        fn query(&self, _sql: &str) -> std::result::Result<Vec<Row>, HabitatError> {
            Ok(self.rows.clone())
        }
    }

    struct MockDecayable {
        count: u32,
    }

    impl Decayable for MockDecayable {
        fn decay(&mut self, _rate: f64) -> std::result::Result<u32, HabitatError> {
            Ok(self.count)
        }
    }

    struct MockSubstrate {
        content: String,
        consolidated: bool,
    }

    impl Injectable for MockSubstrate {
        fn inject(&self, _budget: TokenBudget) -> std::result::Result<(String, u32), HabitatError> {
            Ok((self.content.clone(), 10))
        }
    }

    impl Consolidatable for MockSubstrate {
        fn consolidate(&mut self, _session: SessionId) -> std::result::Result<(), HabitatError> {
            self.consolidated = true;
            Ok(())
        }
    }

    // -- Injectable tests --

    #[test]
    fn injectable_within_budget() {
        let inj = MockInjectable {
            content: "test payload".into(),
            tokens: 50,
        };
        let (text, used) = inj.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(text, "test payload");
        assert_eq!(used, 50);
    }

    #[test]
    fn injectable_exceeds_budget() {
        let inj = MockInjectable {
            content: "big payload".into(),
            tokens: 200,
        };
        let result = inj.inject(TokenBudget::new(100));
        assert!(result.is_err());
    }

    #[test]
    fn injectable_zero_budget() {
        let inj = MockInjectable {
            content: "any".into(),
            tokens: 1,
        };
        assert!(inj.inject(TokenBudget::new(0)).is_err());
    }

    #[test]
    fn injectable_exact_budget() {
        let inj = MockInjectable {
            content: "exact".into(),
            tokens: 100,
        };
        let (_, used) = inj.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(used, 100);
    }

    // -- Consolidatable tests --

    #[test]
    fn consolidatable_records_session() {
        let mut con = MockConsolidatable {
            consolidated: false,
            last_session: None,
        };
        con.consolidate(SessionId::new(109)).unwrap();
        assert!(con.consolidated);
        assert_eq!(con.last_session, Some(SessionId::new(109)));
    }

    #[test]
    fn consolidatable_multiple_sessions() {
        let mut con = MockConsolidatable {
            consolidated: false,
            last_session: None,
        };
        con.consolidate(SessionId::new(1)).unwrap();
        con.consolidate(SessionId::new(2)).unwrap();
        assert_eq!(con.last_session, Some(SessionId::new(2)));
    }

    // -- Queryable tests --

    #[test]
    fn queryable_returns_rows() {
        let mut row = Row::new();
        row.insert("id".into(), "1".into());
        row.insert("label".into(), "test".into());
        let q = MockQueryable {
            rows: vec![row.clone()],
        };
        let result = q.query("SELECT * FROM test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["label"], "test");
    }

    #[test]
    fn queryable_empty_results() {
        let q = MockQueryable { rows: vec![] };
        let result = q.query("SELECT * FROM empty").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn queryable_query_one_some() {
        let mut row = Row::new();
        row.insert("count".into(), "42".into());
        let q = MockQueryable {
            rows: vec![row],
        };
        let result = q.query_one("SELECT count(*) FROM test").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap()["count"], "42");
    }

    #[test]
    fn queryable_query_one_none() {
        let q = MockQueryable { rows: vec![] };
        let result = q.query_one("SELECT * FROM empty").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn queryable_count_success() {
        let mut row = Row::new();
        row.insert("count".into(), "100".into());
        let q = MockQueryable {
            rows: vec![row],
        };
        let count = q.count("SELECT count(*) FROM test").unwrap();
        assert_eq!(count, 100);
    }

    #[test]
    fn queryable_count_empty_errors() {
        let q = MockQueryable { rows: vec![] };
        assert!(q.count("SELECT count(*) FROM empty").is_err());
    }

    #[test]
    fn queryable_count_parse_error() {
        let mut row = Row::new();
        row.insert("count".into(), "not_a_number".into());
        let q = MockQueryable {
            rows: vec![row],
        };
        assert!(q.count("SELECT count(*) FROM test").is_err());
    }

    #[test]
    fn queryable_multiple_rows() {
        let rows: Vec<Row> = (0..5)
            .map(|i| {
                let mut r = Row::new();
                r.insert("id".into(), i.to_string());
                r
            })
            .collect();
        let q = MockQueryable { rows };
        let result = q.query("SELECT id FROM test").unwrap();
        assert_eq!(result.len(), 5);
    }

    // -- Decayable tests --

    #[test]
    fn decayable_returns_count() {
        let mut d = MockDecayable { count: 15 };
        let decayed = d.decay(0.95).unwrap();
        assert_eq!(decayed, 15);
    }

    #[test]
    fn decayable_zero_count() {
        let mut d = MockDecayable { count: 0 };
        let decayed = d.decay(0.95).unwrap();
        assert_eq!(decayed, 0);
    }

    // -- MemorySubstrate tests --

    #[test]
    fn memory_substrate_injectable_and_consolidatable() {
        let mut sub = MockSubstrate {
            content: "substrate data".into(),
            consolidated: false,
        };

        let (text, _) = sub.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(text, "substrate data");

        sub.consolidate(SessionId::new(110)).unwrap();
        assert!(sub.consolidated);
    }

    fn accepts_substrate(_s: &dyn MemorySubstrate) {}

    #[test]
    fn memory_substrate_trait_object() {
        let sub = MockSubstrate {
            content: "test".into(),
            consolidated: false,
        };
        accepts_substrate(&sub);
    }

    // -- Row type tests --

    #[test]
    fn row_is_btreemap() {
        let mut row = Row::new();
        row.insert("a".into(), "1".into());
        row.insert("b".into(), "2".into());
        assert_eq!(row.len(), 2);
    }

    #[test]
    fn row_ordered_by_key() {
        let mut row = Row::new();
        row.insert("z".into(), "last".into());
        row.insert("a".into(), "first".into());
        let keys: Vec<&String> = row.keys().collect();
        assert_eq!(keys[0], "a");
        assert_eq!(keys[1], "z");
    }

    #[test]
    fn row_serde_roundtrip() {
        let mut row = Row::new();
        row.insert("id".into(), "42".into());
        row.insert("name".into(), "test".into());
        let json = serde_json::to_string(&row).unwrap();
        let back: Row = serde_json::from_str(&json).unwrap();
        assert_eq!(row, back);
    }

    // -- Injectable edge cases --

    #[test]
    fn injectable_empty_content() {
        let inj = MockInjectable {
            content: String::new(),
            tokens: 0,
        };
        let (text, used) = inj.inject(TokenBudget::new(100)).unwrap();
        assert!(text.is_empty());
        assert_eq!(used, 0);
    }

    #[test]
    fn injectable_large_content_within_budget() {
        let content = "x".repeat(5000);
        let inj = MockInjectable {
            content: content.clone(),
            tokens: 500,
        };
        let (text, _) = inj.inject(TokenBudget::new(1000)).unwrap();
        assert_eq!(text.len(), 5000);
    }

    #[test]
    fn injectable_error_is_habitat_error() {
        let inj = MockInjectable {
            content: "x".into(),
            tokens: 200,
        };
        let err = inj.inject(TokenBudget::new(10)).unwrap_err();
        assert_eq!(err.kind(), crate::m1_foundation::m02_errors::ErrorKind::Injection);
    }

    #[test]
    fn injectable_budget_boundary_one_under() {
        let inj = MockInjectable {
            content: "x".into(),
            tokens: 101,
        };
        assert!(inj.inject(TokenBudget::new(100)).is_err());
    }

    #[test]
    fn injectable_budget_boundary_one_over() {
        let inj = MockInjectable {
            content: "x".into(),
            tokens: 99,
        };
        assert!(inj.inject(TokenBudget::new(100)).is_ok());
    }

    // -- Consolidatable edge cases --

    #[test]
    fn consolidatable_session_zero() {
        let mut con = MockConsolidatable {
            consolidated: false,
            last_session: None,
        };
        con.consolidate(SessionId::new(0)).unwrap();
        assert_eq!(con.last_session, Some(SessionId::new(0)));
    }

    #[test]
    fn consolidatable_idempotent() {
        let mut con = MockConsolidatable {
            consolidated: false,
            last_session: None,
        };
        let s = SessionId::new(50);
        con.consolidate(s).unwrap();
        con.consolidate(s).unwrap();
        assert_eq!(con.last_session, Some(s));
        assert!(con.consolidated);
    }

    #[test]
    fn consolidatable_high_session_number() {
        let mut con = MockConsolidatable {
            consolidated: false,
            last_session: None,
        };
        con.consolidate(SessionId::new(u32::MAX)).unwrap();
        assert_eq!(con.last_session, Some(SessionId::new(u32::MAX)));
    }

    // -- Queryable edge cases --

    #[test]
    fn queryable_query_one_returns_first_of_many() {
        let rows: Vec<Row> = (0..3)
            .map(|i| {
                let mut r = Row::new();
                r.insert("id".into(), i.to_string());
                r
            })
            .collect();
        let q = MockQueryable { rows };
        let result = q.query_one("SELECT id").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn queryable_count_with_zero() {
        let mut row = Row::new();
        row.insert("count".into(), "0".into());
        let q = MockQueryable { rows: vec![row] };
        let count = q.count("SELECT count(*)").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn queryable_count_with_large_number() {
        let mut row = Row::new();
        row.insert("count".into(), "999999999".into());
        let q = MockQueryable { rows: vec![row] };
        let count = q.count("SELECT count(*)").unwrap();
        assert_eq!(count, 999_999_999);
    }

    #[test]
    fn queryable_count_negative_errors() {
        let mut row = Row::new();
        row.insert("count".into(), "-1".into());
        let q = MockQueryable { rows: vec![row] };
        assert!(q.count("SELECT count(*)").is_err());
    }

    #[test]
    fn queryable_count_float_errors() {
        let mut row = Row::new();
        row.insert("count".into(), "3.14".into());
        let q = MockQueryable { rows: vec![row] };
        assert!(q.count("SELECT count(*)").is_err());
    }

    #[test]
    fn queryable_count_empty_string_errors() {
        let mut row = Row::new();
        row.insert("count".into(), String::new());
        let q = MockQueryable { rows: vec![row] };
        assert!(q.count("SELECT count(*)").is_err());
    }

    #[test]
    fn queryable_count_error_kind_is_query() {
        let q = MockQueryable { rows: vec![] };
        let err = q.count("SELECT count(*)").unwrap_err();
        assert_eq!(err.kind(), crate::m1_foundation::m02_errors::ErrorKind::Query);
    }

    #[test]
    fn queryable_large_result_set() {
        let rows: Vec<Row> = (0..1000)
            .map(|i| {
                let mut r = Row::new();
                r.insert("id".into(), i.to_string());
                r
            })
            .collect();
        let q = MockQueryable { rows };
        assert_eq!(q.query("SELECT id").unwrap().len(), 1000);
    }

    // -- Decayable edge cases --

    #[test]
    fn decayable_large_count() {
        let mut d = MockDecayable { count: u32::MAX };
        assert_eq!(d.decay(0.95).unwrap(), u32::MAX);
    }

    // -- MemorySubstrate tests --

    #[test]
    fn memory_substrate_inject_then_consolidate() {
        let mut sub = MockSubstrate {
            content: "pre-consolidation".into(),
            consolidated: false,
        };
        let (text, _) = sub.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(text, "pre-consolidation");
        assert!(!sub.consolidated);

        sub.consolidate(SessionId::new(42)).unwrap();
        assert!(sub.consolidated);

        let (text2, _) = sub.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(text2, "pre-consolidation");
    }

    #[test]
    fn memory_substrate_boxed_trait_object() {
        let sub: Box<dyn MemorySubstrate> = Box::new(MockSubstrate {
            content: "boxed".into(),
            consolidated: false,
        });
        let (text, _) = sub.inject(TokenBudget::new(100)).unwrap();
        assert_eq!(text, "boxed");
    }

    // -- Failing mock --

    struct FailingInjectable;
    impl Injectable for FailingInjectable {
        fn inject(&self, _budget: TokenBudget) -> std::result::Result<(String, u32), HabitatError> {
            Err(crate::m1_foundation::m02_errors::InjectionError::Timeout {
                elapsed_ms: 200,
                budget_ms: 100,
            }
            .into())
        }
    }

    struct FailingConsolidatable;
    impl Consolidatable for FailingConsolidatable {
        fn consolidate(&mut self, _session: SessionId) -> std::result::Result<(), HabitatError> {
            Err(crate::m1_foundation::m02_errors::ConsolidationError::DecayFailed(
                "simulated".into(),
            )
            .into())
        }
    }

    struct FailingQueryable;
    impl Queryable for FailingQueryable {
        fn query(&self, _sql: &str) -> std::result::Result<Vec<Row>, HabitatError> {
            Err(crate::m1_foundation::m02_errors::QueryError::ExecutionFailed(
                "simulated".into(),
            )
            .into())
        }
    }

    struct FailingDecayable;
    impl Decayable for FailingDecayable {
        fn decay(&mut self, _rate: f64) -> std::result::Result<u32, HabitatError> {
            Err(crate::m1_foundation::m02_errors::ConsolidationError::DecayFailed(
                "simulated".into(),
            )
            .into())
        }
    }

    #[test]
    fn failing_injectable_returns_error() {
        let f = FailingInjectable;
        assert!(f.inject(TokenBudget::new(9999)).is_err());
    }

    #[test]
    fn failing_consolidatable_returns_error() {
        let mut f = FailingConsolidatable;
        assert!(f.consolidate(SessionId::new(1)).is_err());
    }

    #[test]
    fn failing_queryable_query_returns_error() {
        let f = FailingQueryable;
        assert!(f.query("SELECT 1").is_err());
    }

    #[test]
    fn failing_queryable_query_one_returns_error() {
        let f = FailingQueryable;
        assert!(f.query_one("SELECT 1").is_err());
    }

    #[test]
    fn failing_queryable_count_returns_error() {
        let f = FailingQueryable;
        assert!(f.count("SELECT count(*)").is_err());
    }

    #[test]
    fn failing_decayable_returns_error() {
        let mut f = FailingDecayable;
        assert!(f.decay(0.95).is_err());
    }

    // -- Row additional tests --

    #[test]
    fn row_empty() {
        let row = Row::new();
        assert!(row.is_empty());
    }

    #[test]
    fn row_get_nonexistent_key() {
        let row = Row::new();
        assert!(row.get("missing").is_none());
    }

    #[test]
    fn row_overwrite_value() {
        let mut row = Row::new();
        row.insert("key".into(), "old".into());
        row.insert("key".into(), "new".into());
        assert_eq!(row["key"], "new");
        assert_eq!(row.len(), 1);
    }

    #[test]
    fn row_with_numeric_string_values() {
        let mut row = Row::new();
        row.insert("int".into(), "42".into());
        row.insert("float".into(), "3.14".into());
        row.insert("negative".into(), "-1".into());
        assert_eq!(row["int"].parse::<i64>().unwrap(), 42);
        assert!((row["float"].parse::<f64>().unwrap() - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn row_with_empty_values() {
        let mut row = Row::new();
        row.insert("empty".into(), String::new());
        row.insert("null_repr".into(), "NULL".into());
        assert!(row["empty"].is_empty());
        assert_eq!(row["null_repr"], "NULL");
    }

    #[test]
    fn row_iteration_order() {
        let mut row = Row::new();
        row.insert("c".into(), "3".into());
        row.insert("a".into(), "1".into());
        row.insert("b".into(), "2".into());
        let keys: Vec<&String> = row.keys().collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }
}
