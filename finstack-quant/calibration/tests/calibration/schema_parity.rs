//! Calibration JSON-schema parity checks.

use serde_json::Value;

const CALIBRATION_STEP_KINDS: &[&str] = &[
    "base_correlation",
    "cap_floor_hull_white",
    "discount",
    "forward",
    "hazard",
    "hull_white",
    "inflation",
    "parametric",
    "student_t",
    "svi_surface",
    "swaption_vol",
    "vol_surface",
    "xccy_basis",
];

fn tagged_discriminators(schema: &Value) -> Vec<&str> {
    schema
        .get("oneOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|variant| {
            variant
                .pointer("/properties/kind/const")
                .and_then(Value::as_str)
        })
        .collect()
}

#[test]
fn calibration_step_kinds_match_checked_in_schema() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../schemas/calibration/1/calibration.schema.json"
    ))
    .expect("calibration schema JSON");
    let step_params = schema
        .pointer("/$defs/StepParams")
        .or_else(|| schema.pointer("/$defs/CalibrationStep"))
        .expect("StepParams schema definition");
    let mut actual = tagged_discriminators(step_params);
    let mut expected = CALIBRATION_STEP_KINDS.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
