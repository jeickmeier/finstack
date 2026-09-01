//! Golden fixture loading and comparison helpers.
//!
//! Golden suites pair externally sourced reference values with provenance and
//! explicit comparison tolerances. They are intended for crate tests rather
//! than runtime APIs.
//!
//! # Fixture contract
//!
//! Fixtures deserialize as [`GoldenSuite<T>`](GoldenSuite) from a canonical v1
//! JSON envelope:
//!
//! ```json
//! {
//!   "meta": {
//!     "suite_id": "pricing_vectors",
//!     "reference_source": { "name": "QuantLib" },
//!     "generated": {
//!       "at": "2026-08-02T12:00:00Z",
//!       "by": "scripts/generate_vectors.py"
//!     },
//!     "status": "certified",
//!     "schema_version": 1
//!   },
//!   "cases": []
//! }
//! ```
//!
//! `meta.schema_version` must be `1`; `cases` is decoded as the caller's case
//! type. `meta.extra` carries suite-specific metadata without changing the
//! envelope.
//!
//! # Provenance
//!
//! Every fixture records:
//!
//! - `meta.reference_source.name`: source of the expected values;
//! - `meta.generated.at`: generation timestamp;
//! - `meta.generated.by`: generating tool or script;
//! - `meta.status`: `certified`, `provisional`, or `pending_validation`.
//!
//! # API
//!
//! The data model comprises [`GoldenSuite`], [`SuiteMeta`],
//! [`ReferenceSource`], [`GeneratedInfo`], [`ValidatedInfo`], [`Expectation`],
//! and [`Tolerance`]. [`load_suite_from_path`] and [`load_suite_from_str`] load
//! the envelope; [`golden_path`] and `golden_path!` resolve crate-local fixture
//! paths. [`GoldenAssert`] compares results while retaining suite and case
//! context for diagnostics.
//!
//! # Example
//!
//! ```no_run
//! use finstack_quant_test_utils::golden::{load_suite_from_path, Expectation, GoldenAssert};
//! use finstack_quant_test_utils::golden_path;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct VarianceCase {
//!     id: String,
//!     inputs: Vec<f64>,
//!     expected: Expectation,
//! }
//!
//! fn variance(xs: &[f64]) -> f64 {
//!     let mean = xs.iter().sum::<f64>() / xs.len() as f64;
//!     xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let suite = load_suite_from_path::<VarianceCase>(golden_path!("data/variance.json"))?;
//! for case in &suite.cases {
//!     let check = GoldenAssert::new(&suite.meta, &case.id);
//!     check.expected("variance", variance(&case.inputs), &case.expected)?;
//! }
//! # Ok(())
//! # }
//! ```

mod compare;
mod loader;
mod types;

pub use types::{
    Expectation, GeneratedInfo, GoldenSuite, ReferenceSource, SuiteMeta, Tolerance, ValidatedInfo,
};

pub use loader::{golden_path, load_suite_from_path, load_suite_from_str};

pub use compare::GoldenAssert;
