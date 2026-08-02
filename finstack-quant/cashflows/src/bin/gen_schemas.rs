//! Generate the JSON Schemas owned by `finstack-quant-cashflows`.

use finstack_quant_cashflows::builder::{
    AmortizationSpec, DefaultModelSpec, FeeSpec, FixedCouponSpec, PrepaymentModelSpec,
    RecoveryModelSpec, ScheduleParams,
};
use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};
use std::path::Path;

const ARTIFACTS: &[SchemaArtifact] = &[
    SchemaArtifact::new::<FixedCouponSpec>(
        "schemas/cashflow/1/coupon_specs.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/coupon_specs.schema.json",
        "FixedCouponSpec",
        "Fixed and floating coupon specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<AmortizationSpec>(
        "schemas/cashflow/1/amortization_spec.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/amortization_spec.schema.json",
        "AmortizationSpec",
        "Cashflow amortization specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<ScheduleParams>(
        "schemas/cashflow/1/schedule_params.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/schedule_params.schema.json",
        "ScheduleParams",
        "Cashflow schedule construction parameters.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<FeeSpec>(
        "schemas/cashflow/1/fee_specs.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/fee_specs.schema.json",
        "FeeSpec",
        "Cashflow fee specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<DefaultModelSpec>(
        "schemas/cashflow/1/default_model_spec.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/default_model_spec.schema.json",
        "DefaultModelSpec",
        "Cashflow default-model specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<PrepaymentModelSpec>(
        "schemas/cashflow/1/prepayment_model_spec.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/prepayment_model_spec.schema.json",
        "PrepaymentModelSpec",
        "Cashflow prepayment-model specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
    SchemaArtifact::new::<RecoveryModelSpec>(
        "schemas/cashflow/1/recovery_model_spec.schema.json",
        "https://finstack_quant.dev/schemas/cashflow/1/recovery_model_spec.schema.json",
        "RecoveryModelSpec",
        "Cashflow recovery-model specification.",
    )
    .with_packager(finstack_quant_cashflows::schema::package_cashflow_schema),
];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/cashflow"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate cashflow schemas: {error}"));
}
