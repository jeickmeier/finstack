//! Golden test framework for finstack_quant.
//!
//! This module provides a unified framework for loading and validating golden
//! test fixtures across all finstack crates. Golden tests compare implementation
//! results against known reference values from external sources (ISDA, QuantLib,
//! Excel, etc.).
//!
//! # Overview
//!
//! The framework consists of:
//!
//! - **Types** (`types`): Data structures for suites, cases, metadata, and tolerances
//! - **Loaders** (`loader`): Functions to load suites from files or strings
//! - **Comparisons** (`compare`): Assertion helpers with actionable error messages
//!
//! The stable public surface is the assertion/comparison layer (`assert_abs`,
//! `assert_expected_f64`, [`GoldenAssert`], [`Expectation`], and [`Tolerance`]). Loader and
//! fixture metadata helpers remain public for existing tests, but they are
//! intended primarily for crate-internal golden suites rather than runtime APIs.
//!
//! # Fixture Format
//!
//! Golden fixtures use a canonical JSON structure with provenance metadata:
//!
//! ```json
//! {
//!   "meta": {
//!     "suite_id": "cds_isda_vectors",
//!     "description": "ISDA CDS Standard Model reference vectors",
//!     "reference_source": {
//!       "name": "ISDA CDS Standard Model",
//!       "version": "1.8.2"
//!     },
//!     "generated": {
//!       "at": "2025-01-15T12:34:56Z",
//!       "by": "scripts/generate_cds_golden_vectors.py"
//!     },
//!     "status": "certified",
//!     "schema_version": 1
//!   },
//!   "cases": [
//!     {
//!       "id": "isda_5y_flat_100bp",
//!       "inputs": { ... },
//!       "expected": { ... }
//!     }
//!   ]
//! }
//! ```
//!
//! # Usage
//!
//! ## Loading fixtures
//!
//! ```ignore
//! use finstack_quant_test_utils::golden::load_suite_from_path;
//! use finstack_quant_test_utils::golden_path;
//!
//! // Load from file
//! let path = golden_path!("data/my_suite.json");
//! let suite = load_suite_from_path::<MyTestCase>(&path)?;
//!
//! for case in &suite.cases {
//!     // Run tests...
//! }
//! ```
//!
//! ## Making assertions
//!
//! ```ignore
//! use finstack_quant_test_utils::golden::{assert_abs, GoldenAssert};
//!
//! // Simple assertion
//! assert_abs("suite", "case", "price", actual, expected, 0.01)?;
//!
//! // With assertion builder
//! let assert = GoldenAssert::new(&suite.meta, &case.id);
//! assert.abs("price", actual_price, expected_price, 0.01)?;
//! assert.expected("spread", actual_spread, &case.expected.spread)?;
//! ```
//!
//! ## Path macros
//!
//! ```ignore
//! use finstack_quant_test_utils::golden_path;
//!
//! // Get paths relative to calling crate's CARGO_MANIFEST_DIR
//! let path = golden_path!("data/my_suite.json");  // <crate>/tests/golden/data/my_suite.json
//! ```
//!
//! # Directory Structure
//!
//! Each crate maintains its own golden fixtures:
//!
//! ```text
//! <crate>/tests/golden/
//! ├── README.md           # Provenance documentation
//! ├── data/
//! │   ├── suite_a.json    # Golden suite files
//! │   └── suite_b.json
//! └── schemas/            # Optional JSON schemas
//!     └── suite_a.schema.json
//! ```
//!
//! # Provenance Requirements
//!
//! Every golden fixture must document:
//!
//! - **Where**: `meta.reference_source.name` - source of expected values
//! - **When**: `meta.generated.at` - generation timestamp
//! - **How**: `meta.generated.by` - tool/script that generated the data
//! - **Status**: `meta.status` - "certified", "provisional", or "pending_validation"
//!
//! The `meta.extra` field allows adding custom metadata without schema changes.

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
