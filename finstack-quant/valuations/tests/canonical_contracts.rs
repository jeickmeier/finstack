//! Canonical bytes, hashes, and insertion-order invariance for valuation contracts.

use finstack_quant_core::contract::LoadLimits;
use finstack_quant_core::to_canonical_bytes;
use finstack_quant_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_quant_valuations::instruments::InstrumentEnvelope;

fn matrix() -> serde_json::Value {
    serde_json::from_str(include_str!("data/contract_version_matrix.json"))
        .expect("contract matrix parses")
}

fn document_with_replacement(contract: &str, from: &str, to: &str) -> Vec<u8> {
    serde_json::to_string(&matrix()[contract]["base"])
        .expect("base fixture serializes")
        .replace(from, to)
        .into_bytes()
}

#[test]
fn instrument_envelope_has_golden_canonical_bytes_hash_and_order_invariance() {
    let first_bytes = document_with_replacement(
        "instrument",
        r#""meta":{}"#,
        r#""meta":{"desk":"rates","source":"golden"}"#,
    );
    let second_bytes = document_with_replacement(
        "instrument",
        r#""meta":{}"#,
        r#""meta":{"source":"golden","desk":"rates"}"#,
    );
    InstrumentEnvelope::from_slice_strict(&first_bytes, &LoadLimits::default())
        .expect("first instrument is strictly valid");
    InstrumentEnvelope::from_slice_strict(&second_bytes, &LoadLimits::default())
        .expect("second instrument is strictly valid");
    let first: InstrumentEnvelope =
        serde_json::from_slice(&first_bytes).expect("first envelope parses");
    let second: InstrumentEnvelope =
        serde_json::from_slice(&second_bytes).expect("second envelope parses");

    let canonical = to_canonical_bytes(&first).expect("instrument canonicalizes");
    assert_eq!(canonical, include_bytes!("data/canonical/instrument.json"));
    assert_eq!(
        first.content_hash().expect("instrument hashes"),
        include_str!("data/canonical/instrument.sha256").trim()
    );
    assert_eq!(
        canonical,
        to_canonical_bytes(&second).expect("reverse-order instrument canonicalizes")
    );
}

#[test]
fn calibration_envelope_has_golden_canonical_bytes_hash_and_order_invariance() {
    let first_bytes = document_with_replacement(
        "calibration",
        r#""quote_sets":{}"#,
        r#""quote_sets":{"usd":[],"eur":[]}"#,
    );
    let second_bytes = document_with_replacement(
        "calibration",
        r#""quote_sets":{}"#,
        r#""quote_sets":{"eur":[],"usd":[]}"#,
    );
    let (first, first_report) =
        CalibrationEnvelope::from_slice_strict(&first_bytes, &LoadLimits::default())
            .expect("first calibration is strictly valid");
    let (second, second_report) =
        CalibrationEnvelope::from_slice_strict(&second_bytes, &LoadLimits::default())
            .expect("second calibration is strictly valid");
    assert!(!first_report.has_errors());
    assert!(!second_report.has_errors());

    let canonical = to_canonical_bytes(&first).expect("calibration canonicalizes");
    assert_eq!(canonical, include_bytes!("data/canonical/calibration.json"));
    assert_eq!(
        first.content_hash().expect("calibration hashes"),
        include_str!("data/canonical/calibration.sha256").trim()
    );
    assert_eq!(
        canonical,
        to_canonical_bytes(&second).expect("reverse-order calibration canonicalizes")
    );
}
