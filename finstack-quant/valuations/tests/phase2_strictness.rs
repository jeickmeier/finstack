//! Phase 2 strict persisted calibration-step contract regressions.

use finstack_quant_valuations::calibration::api::schema::{
    CalibrationStep, DiscountCurveParams, StepParams,
};

#[test]
fn calibration_step_rejects_unknown_flattened_fields() {
    let params: DiscountCurveParams = serde_json::from_value(serde_json::json!({
        "curve_id": "USD-OIS",
        "currency": "USD",
        "base_date": "2026-01-01"
    }))
    .expect("default discount parameters deserialize");
    let step = CalibrationStep {
        id: "usd-curve".into(),
        quote_set: "usd-quotes".into(),
        params: StepParams::Discount(params),
    };
    let mut value = serde_json::to_value(step).expect("step serializes");
    value
        .as_object_mut()
        .expect("step is an object")
        .insert("__nonexistent__".into(), serde_json::json!(true));

    let error =
        serde_json::from_value::<CalibrationStep>(value).expect_err("unknown fields must fail");
    assert!(
        error
            .to_string()
            .contains("unknown field `__nonexistent__`"),
        "{error}"
    );
}
