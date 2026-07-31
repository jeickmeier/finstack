//! Generate the JSON Schemas owned by `finstack-quant-cashflows`.

use finstack_quant_cashflows::builder::{
    AmortizationSpec, DefaultModelSpec, FeeSpec, FixedCouponSpec, PrepaymentModelSpec,
    RecoveryModelSpec, ScheduleParams,
};
use finstack_quant_core::schema::{
    assemble_schema, postprocess_schema, write_schema as write_schema_file,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const COMMON_BASE: &str = "https://finstack_quant.dev/schemas/common/1/";
const CASHFLOW_BASE: &str = "https://finstack_quant.dev/schemas/cashflow/1/";

fn schema_dir() -> PathBuf {
    Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("schemas/cashflow/1")
}

fn external_ref(name: &str) -> Option<String> {
    let common = match name {
        "Attributes" => "attributes.schema.json",
        "BusinessDayConvention" => "business_day_convention.schema.json",
        "Currency" => "currency.schema.json",
        "DayCount" => "day_count.schema.json",
        "Id" => "id.schema.json",
        "Money" => "money.schema.json",
        "PricingOverrides" => "pricing_overrides.schema.json",
        "Tenor" => "tenor.schema.json",
        _ => "",
    };
    if !common.is_empty() {
        return Some(format!("{COMMON_BASE}{common}"));
    }
    let cashflow = match name {
        "DefaultModelSpec" => "default_model_spec.schema.json",
        "FeeSpec" => "fee_specs.schema.json",
        "FixedCouponSpec" => "coupon_specs.schema.json",
        "PrepaymentModelSpec" => "prepayment_model_spec.schema.json",
        "RecoveryModelSpec" => "recovery_model_spec.schema.json",
        "ScheduleParams" => "schedule_params.schema.json",
        _ => return None,
    };
    Some(format!("{CASHFLOW_BASE}{cashflow}"))
}

fn write_schema<T: schemars::JsonSchema>(name: &str, filename: &str) {
    let path = schema_dir().join(format!("{filename}.schema.json"));
    let existing: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| {
            json!({
                "$id": format!("{CASHFLOW_BASE}{filename}.schema.json"),
                "title": name,
                "description": format!("{name} specification")
            })
        });
    let mut generated = serde_json::to_value(schemars::schema_for!(T)).expect("serialize schema");
    postprocess_schema(&mut generated, external_ref);
    let output = assemble_schema(
        &existing,
        &generated,
        &["$id", "title", "description", "examples"],
        &[
            "type",
            "properties",
            "required",
            "$defs",
            "additionalProperties",
            "oneOf",
            "anyOf",
        ],
    )
    .unwrap_or_else(|error| panic!("assemble {name} schema: {error}"));
    write_schema_file(&path, &output)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}

fn main() {
    write_schema::<FixedCouponSpec>("coupon_specs", "coupon_specs");
    write_schema::<AmortizationSpec>("amortization_spec", "amortization_spec");
    write_schema::<ScheduleParams>("schedule_params", "schedule_params");
    write_schema::<FeeSpec>("fee_specs", "fee_specs");
    write_schema::<DefaultModelSpec>("default_model_spec", "default_model_spec");
    write_schema::<PrepaymentModelSpec>("prepayment_model_spec", "prepayment_model_spec");
    write_schema::<RecoveryModelSpec>("recovery_model_spec", "recovery_model_spec");
}
