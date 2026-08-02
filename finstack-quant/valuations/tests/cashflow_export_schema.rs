//! JSON Schema coverage for the exported instrument cashflow envelope.

use finstack_quant_valuations::instruments::cashflow_export::InstrumentCashflowEnvelope;

#[test]
fn instrument_cashflow_schema_describes_envelope_and_rows() {
    let schema = serde_json::to_value(schemars::schema_for!(InstrumentCashflowEnvelope))
        .expect("schema serializes");

    let properties = schema["properties"]
        .as_object()
        .expect("root schema has properties");
    for field in [
        "instrument_id",
        "currency",
        "model",
        "as_of",
        "discount_curve_id",
        "flows",
        "total_pv",
        "reconciles_with_base_value",
    ] {
        assert!(
            properties.contains_key(field),
            "missing envelope field {field}"
        );
    }

    let rendered = serde_json::to_string(&schema).expect("schema renders");
    for row_field in ["discount_factor", "survival_probability", "pv"] {
        assert!(
            rendered.contains(&format!("\"{row_field}\"")),
            "schema must describe row field {row_field}"
        );
    }
}
