//! Canonical bytes, hash, and insertion-order invariance for factor-model envelopes.

use finstack_quant_core::contract::LoadLimits;
use finstack_quant_core::{content_hash, to_canonical_bytes};
use finstack_quant_factor_model::credit::hierarchy::CreditFactorModel;
use finstack_quant_factor_model::FactorModelConfigEnvelope;

fn config_bytes(overrides: &str) -> Vec<u8> {
    let matrix: serde_json::Value =
        serde_json::from_str(include_str!("data/contract_version_matrix.json"))
            .expect("contract matrix parses");
    let mut base = matrix["config"]["base"].clone();
    base["config"]["bump_size"] = serde_json::json!({"overrides": {}});
    serde_json::to_string(&base)
        .expect("factor-model fixture serializes")
        .replace(r#""overrides":{}"#, &format!(r#""overrides":{overrides}"#))
        .into_bytes()
}

#[test]
fn factor_model_envelope_has_golden_canonical_bytes_hash_and_order_invariance() {
    let first_bytes = config_bytes(r#"{"rates:usd":2.0,"credit:acme":1.0}"#);
    let second_bytes = config_bytes(r#"{"credit:acme":1.0,"rates:usd":2.0}"#);
    FactorModelConfigEnvelope::from_slice_strict(&first_bytes, &LoadLimits::default())
        .expect("first factor model is strictly valid");
    FactorModelConfigEnvelope::from_slice_strict(&second_bytes, &LoadLimits::default())
        .expect("second factor model is strictly valid");
    let first: FactorModelConfigEnvelope =
        serde_json::from_slice(&first_bytes).expect("first envelope parses");
    let second: FactorModelConfigEnvelope =
        serde_json::from_slice(&second_bytes).expect("second envelope parses");

    let canonical = to_canonical_bytes(&first).expect("factor model canonicalizes");
    assert_eq!(
        canonical,
        include_bytes!("data/canonical/factor_model_config.json")
    );
    assert_eq!(
        first.content_hash().expect("factor model hashes"),
        include_str!("data/canonical/factor_model_config.sha256").trim()
    );
    assert_eq!(
        canonical,
        to_canonical_bytes(&second).expect("reverse-order factor model canonicalizes")
    );
}

#[test]
fn credit_factor_model_has_exact_canonical_bytes_and_hash() {
    let source =
        include_bytes!("../../valuations/tests/schema_fixtures/credit_factor_model_v1.json");
    let (model, report) = CreditFactorModel::from_slice_strict(source, &LoadLimits::default())
        .expect("strict credit factor model");
    assert!(!report.has_errors());
    let canonical = to_canonical_bytes(&model).expect("credit factor model canonicalizes");
    let hash = content_hash(&model).expect("credit factor model hashes");
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("canonical");
    let json_path = directory.join("credit_factor_model.json");
    let hash_path = directory.join("credit_factor_model.sha256");

    if std::env::var_os("FQ_UPDATE_CANONICAL_GOLDENS").is_some() {
        std::fs::create_dir_all(&directory).expect("create canonical fixture directory");
        std::fs::write(&json_path, &canonical)
            .expect("write credit factor model canonical fixture");
        std::fs::write(&hash_path, format!("{hash}\n"))
            .expect("write credit factor model hash fixture");
    }

    assert_eq!(
        canonical,
        std::fs::read(&json_path).expect("read credit factor model canonical fixture")
    );
    assert_eq!(
        hash,
        std::fs::read_to_string(&hash_path)
            .expect("read credit factor model hash fixture")
            .trim()
    );
}
