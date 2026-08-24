//! WASM bindings for the calibration engine.
//!
//! Mirrors the Python `calibrate` / `validate_calibration_json` surface plus
//! Phase 4 diagnostics (`dryRun`, `dependencyGraphJson`).
//!
//! # Number safety
//!
//! Counts (`iterations`, `residual_evals`, `lm_jacobian_evals`) are embedded
//! inside the JSON result envelope rather than crossed as raw `usize`. JS's
//! `JSON.parse` reads them as IEEE-754 doubles; values above
//! `Number.MAX_SAFE_INTEGER` (2^53 − 1) would round silently. In practice
//! iteration counts stay under ~10⁴ for any non-pathological calibration, so
//! the [`crate::utils::check_js_safe_count`] guard is not threaded through
//! the JSON path. If a future getter exposes a raw `usize` across the
//! boundary (e.g. a `report.iterations() -> usize` accessor), route it
//! through that guard first.
//!
//! On error, all four functions throw a JS `Error` with `name =
//! "CalibrationEnvelopeError"` and a structured `cause` property carrying
//! the serialized `EnvelopeError` payload. Standard `try/catch (e)` exposes
//! both via `e.name` and `e.cause`.
//!
//! # Native (non-wasm32) builds
//!
//! `JsValue` is opaque on native targets: every non-`const` constructor
//! (`JsValue::from_str`, `js_sys::Error::new`, ...) is a `wasm-bindgen` stub
//! that aborts the process. So the `#[wasm_bindgen]` wrappers below are kept
//! *thin* and the diagnostic-bearing logic lives in `*_inner` helpers that
//! return the structured `EnvelopeError` / `ExecuteError` directly. Native
//! tests exercise those helpers and assert on the real diagnostic — the
//! `JsValue` boundary (where the structured error would otherwise collapse to
//! an opaque value) is crossed only at the `#[wasm_bindgen]` edge.

// `EnvelopeError` / `ExecuteError` are intentionally large structured errors
// (rich diagnostic payloads); boxing them would change their public API.
// The upstream `calibration::api::{engine, validate}` modules make the same
// allowance — keep the binding layer consistent.
#![allow(clippy::result_large_err)]

use crate::utils::{structured_js_error, to_js_value};
use finstack_quant_valuations::calibration::api::engine::{self, ExecuteError};
use finstack_quant_valuations::calibration::api::errors::EnvelopeError;
#[cfg(test)]
use finstack_quant_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_quant_valuations::calibration::api::schema::CalibrationResultEnvelope;
use finstack_quant_valuations::calibration::api::validate;
use wasm_bindgen::prelude::*;

/// Native-testable core of [`validate_calibration_json`].
///
/// Parses the envelope and returns its canonical (pretty-printed) form. A parse
/// failure surfaces a structured [`EnvelopeError`] preserving the full parse
/// diagnostic. The Rust calibration API owns the canonical serialization path,
/// keeping the WASM layer to error mapping only.
fn validate_calibration_json_inner(json: &str) -> Result<String, ExecuteError> {
    validate::validate_calibration_json(json).map_err(ExecuteError::from)
}

/// Validate a calibration plan JSON and return the canonical (pretty-printed) form.
/// @param json - Canonical JSON string defining the object to deserialize or normalize.
///
/// # Errors
///
/// Throws a JavaScript exception if `json` is malformed, its calibration
/// schema marker is missing, malformed, or unsupported, static envelope
/// validation fails, or the canonical envelope cannot be serialized.
#[wasm_bindgen(js_name = validateCalibrationJson)]
pub fn validate_calibration_json(json: &str) -> Result<String, JsValue> {
    validate_calibration_json_inner(json).map_err(execute_error_to_js)
}

/// Native-testable core of [`calibrate`].
///
/// Returns the `CalibrationResultEnvelope`, or an [`ExecuteError`] (which
/// carries the structured `EnvelopeError` payload when the failure is
/// envelope-related).
fn calibrate_inner(envelope_json: &str) -> Result<CalibrationResultEnvelope, ExecuteError> {
    let envelope = validate::parse_envelope(envelope_json)?;
    engine::execute_with_diagnostics(&envelope)
}

/// Execute a calibration plan and return the full result envelope.
///
/// Accepts a serialized `CalibrationEnvelope` (plan + quote sets + optional
/// flat `market_data` / `prior_market` lists) and returns a plain JavaScript
/// `CalibrationResultEnvelope` object — the shape `index.d.ts` has always
/// declared. Re-ingest a sub-document (e.g. `result.result.final_market`) with
/// `JSON.stringify`.
/// @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
///
/// # Errors
///
/// Throws a JavaScript exception if `envelopeJson` is malformed or violates
/// the calibration schema or static plan contract, market context construction
/// or a calibration step fails, a solver does not converge, or the result
/// envelope cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = calibrate)]
pub fn calibrate(envelope_json: &str) -> Result<JsValue, JsValue> {
    let result = calibrate_inner(envelope_json).map_err(execute_error_to_js)?;
    to_js_value(&result)
}

/// Pre-flight envelope validation without invoking the solver.
///
/// Returns a JSON-serialized `CalibrationValidationReport` listing every error found
/// plus the dependency graph. Microseconds.
/// @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
///
/// # Errors
///
/// Throws a JavaScript exception if `envelopeJson` is malformed, its schema
/// marker is missing, malformed, or unsupported, the envelope structure is
/// invalid, or the validation report cannot be serialized. Semantic findings
/// are returned in the report rather than thrown.
#[wasm_bindgen(js_name = dryRun)]
pub fn dry_run(envelope_json: &str) -> Result<String, JsValue> {
    validate::dry_run(envelope_json).map_err(|e| envelope_error_to_js(&e))
}

/// Returns the static dependency graph of a calibration plan as JSON.
/// @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
///
/// # Errors
///
/// Throws a JavaScript exception if `envelopeJson` is malformed, its schema
/// marker is missing, malformed, or unsupported, the envelope structure is
/// invalid, or the dependency graph cannot be serialized.
#[wasm_bindgen(js_name = dependencyGraphJson)]
pub fn dependency_graph_json(envelope_json: &str) -> Result<String, JsValue> {
    validate::dependency_graph_json(envelope_json).map_err(|e| envelope_error_to_js(&e))
}

/// Convert an [`EnvelopeError`] into a JS-side error value.
///
/// On `wasm32`, returns a JS `Error` with `name = "CalibrationEnvelopeError"`
/// and a structured `cause` property carrying the serialized payload.
///
/// On native targets `JsValue` cannot carry a string (every constructor is a
/// process-aborting `wasm-bindgen` stub), so this returns the opaque
/// `JsValue::NULL`. The diagnostic is **not** lost: native callers use the
/// `*_inner` helpers above, which return the structured error *before* this
/// lossy boundary conversion. This function is reached natively only at the
/// thin `#[wasm_bindgen]` edge, which native tests do not assert through.
fn envelope_error_to_js(err: &EnvelopeError) -> JsValue {
    let display = err.to_string();
    let cause_json = err.to_json();
    structured_js_error(
        "CalibrationEnvelopeError",
        &display,
        None,
        Some(&cause_json),
    )
}

/// Map every execution stage to the same structured JavaScript error contract.
fn execute_error_to_js(err: ExecuteError) -> JsValue {
    let details = err.details();
    let cause_json = err.to_json();
    structured_js_error(
        "CalibrationEnvelopeError",
        &details.cause,
        details.step_id.as_deref(),
        Some(&cause_json),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_quant_valuations::calibration::api::schema::{CalibrationPlan, CalibrationSchema};

    fn empty_envelope_json() -> String {
        let plan = CalibrationPlan {
            id: "empty".to_string(),
            description: None,
            quote_sets: Default::default(),
            steps: Vec::new(),
            settings: Default::default(),
        };
        let envelope = CalibrationEnvelope {
            schema_url: None,
            schema: finstack_quant_valuations::calibration::api::schema::CalibrationSchema::CURRENT,
            plan,
            market_data: Vec::new(),
            prior_market: Vec::new(),
        };
        serde_json::to_string(&envelope).expect("serialize")
    }

    #[test]
    fn validate_calibration_json_accepts_empty_plan() {
        let json = empty_envelope_json();
        let canonical = validate_calibration_json(&json).expect("validate");
        assert!(!canonical.is_empty());
    }

    #[test]
    fn validation_and_execution_reject_semantically_invalid_requests() {
        let mut value: serde_json::Value =
            serde_json::from_str(&empty_envelope_json()).expect("empty envelope parses");
        value["plan"]["steps"] = serde_json::json!([{
            "id": "discount_step",
            "quote_set": "missing_quotes",
            "kind": "discount",
            "curve_id": "USD-OIS",
            "currency": "USD",
            "base_date": "2026-05-08"
        }]);
        let json = value.to_string();

        for error in [
            validate_calibration_json_inner(&json)
                .expect_err("validation must reject undefined quote sets"),
            calibrate_inner(&json).expect_err("execution must reject undefined quote sets"),
        ] {
            let details = error.details();
            assert_eq!(details.stage.as_str(), "ingestion");
            assert_eq!(details.category, "undefined_quote_set");
            assert!(details.cause.contains("missing_quotes"));
        }
    }

    #[test]
    fn calibrate_empty_plan_succeeds() {
        // Drives the native-testable core: the `#[wasm_bindgen]` wrapper only
        // adds the `JsValue` conversion, which aborts on non-wasm32 targets.
        let json = empty_envelope_json();
        let result = calibrate_inner(&json).expect("execute");
        let parsed = serde_json::to_value(&result).expect("json");
        assert!(parsed.is_object());
        assert!(parsed.get("result").is_some());
    }

    #[test]
    fn dry_run_accepts_empty_plan() {
        let json = empty_envelope_json();
        let report_json = dry_run(&json).expect("dry_run");
        let parsed: serde_json::Value = serde_json::from_str(&report_json).expect("json");
        assert!(parsed.get("errors").is_some());
        assert!(parsed.get("dependency_graph").is_some());
    }

    #[test]
    fn dependency_graph_json_for_empty_plan() {
        let json = empty_envelope_json();
        let graph_json = dependency_graph_json(&json).expect("dep graph");
        let parsed: serde_json::Value = serde_json::from_str(&graph_json).expect("json");
        assert!(parsed.get("initial_ids").is_some());
        assert!(parsed.get("nodes").is_some());
    }

    #[test]
    fn dry_run_rejects_malformed_json() {
        // The `#[wasm_bindgen]` wrapper must still return `Err` (not panic) on
        // a native build; the diagnostic itself is asserted via the `*_inner`
        // helpers below.
        assert!(dry_run("not json").is_err());
    }

    #[test]
    fn validate_calibration_json_inner_preserves_parse_diagnostic() {
        let error = validate_calibration_json_inner("{ not valid json")
            .expect_err("malformed JSON must error");
        let details = error.details();
        assert_eq!(details.stage.as_str(), "ingestion");
        assert_eq!(details.category, "strict_load");
        assert!(!details.cause.is_empty());
        assert!(error.to_json().contains("\"stage\": \"ingestion\""));
    }

    #[test]
    fn validate_calibration_json_inner_never_returns_empty_object_fallback() {
        // A successfully validated envelope must round-trip to a non-trivial
        // canonical JSON — never the literal `"{}"` that the old silent
        // `unwrap_or_else` fallback would have produced on a (hypothetical)
        // re-serialization failure. This guards the regression: the success
        // path returns the real canonical form, and the error path (covered
        // by `validate_calibration_json_inner_preserves_parse_diagnostic`)
        // returns a structured error rather than a fake `"{}"` success.
        let json = empty_envelope_json();
        let canonical = validate_calibration_json_inner(&json).expect("validate");
        assert_ne!(
            canonical, "{}",
            "canonical envelope JSON must not collapse to an empty object"
        );
        let (reparsed, _report) = CalibrationEnvelope::from_slice_strict(
            canonical.as_bytes(),
            &finstack_quant_core::contract::LoadLimits::default(),
        )
        .expect("canonical JSON must round-trip strictly");
        assert_eq!(reparsed.schema, CalibrationSchema::Calibration);
    }

    #[test]
    fn calibrate_inner_preserves_malformed_envelope_diagnostic() {
        let error =
            calibrate_inner("{ this is not valid json").expect_err("malformed envelope must error");
        let details = error.details();
        let json = error.to_json();
        assert_eq!(details.stage.as_str(), "ingestion");
        assert_eq!(details.category, "strict_load");
        assert!(
            json.contains("strict_load") && json.contains("cause"),
            "diagnostic JSON should carry the structured load error, got: {json}"
        );
        assert!(
            !details.cause.is_empty(),
            "diagnostic cause must not be empty"
        );
    }
}
