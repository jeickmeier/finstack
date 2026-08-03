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
//! The data model comprises [`GoldenSuite`], [`SuiteMeta`], [`CaseMeta`],
//! [`ReferenceSource`], [`GeneratedInfo`], [`ValidatedInfo`], [`Expectation`],
//! and [`Tolerance`]. [`load_suite_from_path`] and [`load_suite_from_str`] load
//! the envelope; [`golden_path`] and `golden_path!` resolve crate-local fixture
//! paths. [`assert_abs`], [`assert_expected_f64`],
//! [`assert_within_tolerance`], and [`GoldenAssert`] compare results.
//!
//! # Example
//!
//! ```ignore
//! use finstack_quant_test_utils::golden::{load_suite_from_path, GoldenAssert};
//! use finstack_quant_test_utils::golden_path;
//!
//! let suite = load_suite_from_path::<VarianceCase>(golden_path!("data/variance.json"))?;
//! for case in &suite.cases {
//!     let check = GoldenAssert::new(&suite.meta, &case.id);
//!     check.expected("variance", calculate(&case.inputs), &case.expected)?;
//! }
//! ```

mod compare;
mod loader;
mod types;

// Re-export types
pub use types::{
    CaseMeta, Expectation, GeneratedInfo, GoldenSuite, ReferenceSource, SuiteMeta, Tolerance,
    ValidatedInfo,
};

// Re-export loaders
pub use loader::{golden_path, load_suite_from_path, load_suite_from_str};

// Re-export comparison utilities
pub use compare::{assert_abs, assert_expected_f64, assert_within_tolerance, GoldenAssert};
