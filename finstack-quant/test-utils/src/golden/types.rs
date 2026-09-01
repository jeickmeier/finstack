//! Core types for the golden test framework.
//!
//! This module defines the data structures used for loading and validating
//! golden test fixtures across all finstack crates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A golden test suite containing metadata and test cases.
///
/// This is the canonical JSON structure for golden fixtures:
///
/// ```json
/// {
///   "meta": {
///     "suite_id": "my_suite",
///     "description": "...",
///     "reference_source": { "name": "...", ... },
///     "generated": { "at": "...", "by": "..." },
///     "status": "certified",
///     "schema_version": 1
///   },
///   "cases": [ ... ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenSuite<T> {
    /// Suite-level metadata including provenance.
    pub meta: SuiteMeta,
    /// Test cases in this suite.
    pub cases: Vec<T>,
}

/// Suite-level metadata with provenance information.
///
/// All golden fixtures must include provenance to document where and when
/// the expected values were generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteMeta {
    /// Unique identifier for this suite.
    pub suite_id: String,

    /// Human-readable description of what this suite tests.
    #[serde(default)]
    pub description: String,

    /// Reference source for expected values (e.g., ISDA, QuantLib, Excel).
    #[serde(default)]
    pub reference_source: ReferenceSource,

    /// Information about how/when this suite was generated.
    #[serde(default)]
    pub generated: GeneratedInfo,

    /// Information about validation of expected values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validated: Option<ValidatedInfo>,

    /// Suite status: "certified", "provisional", "pending_validation".
    #[serde(default = "default_status")]
    pub status: String,

    /// Required schema version. Only version `1` is accepted.
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,

    /// Extensible metadata bag for future additions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_status() -> String {
    "unknown".to_string()
}

fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version == 1 {
        Ok(version)
    } else {
        Err(serde::de::Error::custom(format!(
            "unsupported schema_version {version}; expected 1"
        )))
    }
}

impl Default for SuiteMeta {
    fn default() -> Self {
        Self {
            suite_id: String::new(),
            description: String::new(),
            reference_source: ReferenceSource::default(),
            generated: GeneratedInfo::default(),
            validated: None,
            status: default_status(),
            schema_version: 1,
            extra: HashMap::new(),
        }
    }
}

/// Reference source for expected values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReferenceSource {
    /// Name of the reference source (required).
    pub name: String,

    /// Version of the reference implementation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Vendor or organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,

    /// URL for more information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Extensible metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Information about how/when the golden data was generated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneratedInfo {
    /// ISO 8601 timestamp of generation.
    pub at: String,

    /// Tool or script that generated the data.
    pub by: String,

    /// Command used to regenerate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Environment information (python version, OS, etc.).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, String>,
}

/// Information about validation of expected values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidatedInfo {
    /// When the validation was performed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,

    /// Who performed the validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,

    /// Validation method (e.g., "manual spot-check", "automated comparison").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Additional notes about validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Tolerance specification for numeric comparisons.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum Tolerance {
    /// Absolute tolerance (e.g., 0.01 means |actual - expected| < 0.01).
    #[serde(rename = "abs")]
    Abs(f64),

    /// Relative tolerance as a fraction (e.g., 0.001 means 0.1% relative error).
    #[serde(rename = "rel")]
    Rel(f64),

    /// Basis points tolerance (1 bp = 0.0001).
    #[serde(rename = "bp")]
    Bps(f64),

    /// Percentage tolerance (e.g., 0.1 means 0.1% relative error).
    #[serde(rename = "pct")]
    Pct(f64),
}

impl Tolerance {
    fn bp_error(actual: f64, expected: f64) -> f64 {
        (actual - expected).abs() * 10_000.0
    }

    /// The tolerance's own numeric threshold, in its variant's units.
    fn value(&self) -> f64 {
        match self {
            Tolerance::Abs(tol)
            | Tolerance::Rel(tol)
            | Tolerance::Bps(tol)
            | Tolerance::Pct(tol) => *tol,
        }
    }

    /// Check if actual is within tolerance of expected.
    pub fn is_within(&self, actual: f64, expected: f64) -> bool {
        self.compute_error(actual, expected) <= self.value()
    }

    /// Compute the error between actual and expected.
    pub fn compute_error(&self, actual: f64, expected: f64) -> f64 {
        match self {
            Tolerance::Abs(_) => (actual - expected).abs(),
            Tolerance::Rel(_) => {
                if expected.abs() < 1e-15 {
                    actual.abs()
                } else {
                    ((actual - expected) / expected).abs()
                }
            }
            Tolerance::Bps(_) => Self::bp_error(actual, expected),
            Tolerance::Pct(_) => {
                if expected.abs() < 1e-15 {
                    actual.abs()
                } else {
                    ((actual - expected) / expected).abs() * 100.0
                }
            }
        }
    }
}

/// An expected value that can be exact (with tolerance) or a range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Expectation {
    /// Exact value with optional tolerance.
    Exact {
        /// Expected value.
        value: f64,
        /// Tolerance for comparison.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tolerance: Option<Tolerance>,
        /// Optional notes about this expectation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },
    /// Range constraint (min <= actual <= max).
    Range {
        /// Minimum allowed value (inclusive).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        /// Maximum allowed value (inclusive).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        /// Optional notes about this expectation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },
}

impl Expectation {
    /// Check if actual satisfies this expectation.
    pub fn is_satisfied(&self, actual: f64) -> bool {
        match self {
            Expectation::Exact {
                value, tolerance, ..
            } => {
                if let Some(tol) = tolerance {
                    tol.is_within(actual, *value)
                } else {
                    // Scale-aware exact comparison: relative tolerance with absolute floor
                    (actual - value).abs() <= (value.abs() * f64::EPSILON * 8.0).max(1e-15)
                }
            }
            Expectation::Range { min, max, .. } => {
                let above_min = min.is_none_or(|m| actual >= m);
                let below_max = max.is_none_or(|m| actual <= m);
                above_min && below_max
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_abs() {
        let tol = Tolerance::Abs(0.01);
        assert!(tol.is_within(1.005, 1.0));
        assert!(!tol.is_within(1.02, 1.0));
    }

    #[test]
    fn test_tolerance_rel() {
        let tol = Tolerance::Rel(0.01); // 1% relative
        assert!(tol.is_within(1.005, 1.0));
        assert!(!tol.is_within(1.02, 1.0));
    }

    #[test]
    fn test_tolerance_bp() {
        let tol = Tolerance::Bps(0.5);
        // 0.00004 * 10_000 = 0.4 bp < 0.5 bp tolerance
        assert!(tol.is_within(100.00004, 100.0));
        // 0.00006 * 10_000 = 0.6 bp > 0.5 bp tolerance
        assert!(!tol.is_within(100.00006, 100.0));
    }

    #[test]
    fn test_tolerance_bp_on_decimal_rates() {
        let tol = Tolerance::Bps(0.5);
        assert!(tol.is_within(0.05004, 0.05000));
        assert!(!tol.is_within(0.05006, 0.05000));
        assert!((tol.compute_error(0.05004, 0.05000) - 0.4).abs() < 1e-12);
    }

    #[test]
    fn test_tolerance_pct() {
        let tol = Tolerance::Pct(1.0); // 1%
        assert!(tol.is_within(1.005, 1.0));
        assert!(!tol.is_within(1.02, 1.0));
    }

    #[test]
    fn test_expectation_exact() {
        let exp = Expectation::Exact {
            value: 100.0,
            tolerance: Some(Tolerance::Abs(0.5)),
            notes: None,
        };
        assert!(exp.is_satisfied(100.3));
        assert!(!exp.is_satisfied(100.6));
    }

    #[test]
    fn test_expectation_range() {
        let exp = Expectation::Range {
            min: Some(0.0),
            max: Some(100.0),
            notes: None,
        };
        assert!(exp.is_satisfied(50.0));
        assert!(!exp.is_satisfied(-1.0));
        assert!(!exp.is_satisfied(101.0));
    }

    #[test]
    fn expectation_deserializes_exact_and_range_shapes() -> Result<(), serde_json::Error> {
        let exact: Expectation =
            serde_json::from_str(r#"{"value":100.0,"tolerance":{"type":"abs","value":0.5}}"#)?;
        assert!(exact.is_satisfied(100.3));
        assert!(!exact.is_satisfied(100.6));

        let range: Expectation = serde_json::from_str(r#"{"min":0.0,"max":100.0}"#)?;
        assert!(range.is_satisfied(50.0));
        assert!(!range.is_satisfied(101.0));

        Ok(())
    }

    #[test]
    fn expectation_deserializes_all_tolerance_units() -> Result<(), serde_json::Error> {
        let cases = [
            (
                r#"{"value":1.0,"tolerance":{"type":"abs","value":0.01}}"#,
                1.005,
                1.02,
            ),
            (
                r#"{"value":100.0,"tolerance":{"type":"rel","value":0.01}}"#,
                100.5,
                102.0,
            ),
            (
                r#"{"value":0.05,"tolerance":{"type":"bp","value":0.5}}"#,
                0.05004,
                0.05006,
            ),
            (
                r#"{"value":100.0,"tolerance":{"type":"pct","value":1.0}}"#,
                100.5,
                102.0,
            ),
        ];

        for (json, within, outside) in cases {
            let expectation: Expectation = serde_json::from_str(json)?;
            assert!(expectation.is_satisfied(within), "fixture: {json}");
            assert!(!expectation.is_satisfied(outside), "fixture: {json}");
        }

        Ok(())
    }

    #[test]
    fn expectation_rejects_legacy_tolerance_fields() {
        for field in [
            "tolerance_abs",
            "tolerance_rel",
            "tolerance_bp",
            "tolerance_pct",
        ] {
            let json = format!(r#"{{"value":1.0,"{field}":0.1}}"#);
            assert!(
                serde_json::from_str::<Expectation>(&json).is_err(),
                "legacy field {field} must be rejected"
            );
        }
    }

    #[test]
    fn expectation_rejects_ambiguous_tolerances() {
        let json = r#"{"value":1.0,"tolerance_abs":0.1,"tolerance_bp":1.0}"#;
        assert!(serde_json::from_str::<Expectation>(json).is_err());
    }

    #[test]
    fn expectation_rejects_mixed_exact_and_range_fields() {
        let json = r#"{"value":1.0,"min":0.0,"max":2.0}"#;
        assert!(serde_json::from_str::<Expectation>(json).is_err());
    }

    #[test]
    fn expectation_rejects_duplicate_tolerance_fields() {
        let json = r#"{
            "value": 1.0,
            "tolerance": {"type": "abs", "value": 0.1},
            "tolerance": {"type": "bp", "value": 1.0}
        }"#;
        assert!(serde_json::from_str::<Expectation>(json).is_err());
    }

    #[test]
    fn test_suite_meta_deserialize() {
        let json = r#"{
            "suite_id": "test",
            "description": "Test suite",
            "reference_source": { "name": "ISDA", "version": "1.8.2" },
            "generated": { "at": "2025-01-15", "by": "test.py" },
            "status": "certified",
            "schema_version": 1,
            "extra": { "custom_field": "value" }
        }"#;
        let result = serde_json::from_str::<SuiteMeta>(json);
        assert!(result.is_ok(), "Should parse SuiteMeta from JSON");
        if let Ok(meta) = result {
            assert_eq!(meta.suite_id, "test");
            assert_eq!(meta.reference_source.name, "ISDA");
            assert_eq!(meta.reference_source.version, Some("1.8.2".to_string()));
            assert!(meta.extra.contains_key("custom_field"));
        }
    }

    #[test]
    fn suite_meta_requires_schema_version_one() {
        let base = serde_json::json!({
            "suite_id": "test",
            "reference_source": { "name": "manual" },
            "generated": { "at": "2025-01-15", "by": "test" },
            "status": "certified",
            "schema_version": 1
        });
        assert!(
            serde_json::from_value::<SuiteMeta>(base.clone()).is_ok(),
            "schema version one must deserialize"
        );

        let mut missing = base.clone();
        let removed = missing
            .as_object_mut()
            .and_then(|object| object.remove("schema_version"));
        assert!(
            removed.is_some(),
            "test fixture must contain schema_version"
        );
        assert!(serde_json::from_value::<SuiteMeta>(missing).is_err());

        let mut future = base;
        future["schema_version"] = serde_json::json!(2);
        assert!(serde_json::from_value::<SuiteMeta>(future).is_err());
    }
}
