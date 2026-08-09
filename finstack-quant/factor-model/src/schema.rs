//! JSON Schema generation helpers for factor-model contracts.

use std::sync::OnceLock;

use serde_json::Value;

/// Stable base URI for factor-model-owned schemas.
///
/// This retains the published underscore URI used by the original credit
/// factor-model schemas even though the Rust crate name is hyphenated.
pub const FACTOR_MODEL_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/factor_model/1/";
/// Filename of the published factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_FILENAME: &str = "factor_model_config.schema.json";
/// Canonical title of the factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_TITLE: &str = "Finstack Quant Factor Model Configuration";
/// Canonical description of the factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_DESCRIPTION: &str =
    "Versioned factor-model configuration with typed factors, covariance, matching, pricing, and risk settings.";
/// Filename of the published credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_FILENAME: &str = "credit_factor_model.schema.json";
/// Canonical title of the credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_TITLE: &str = "CreditFactorModel";
/// Canonical description of the credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION: &str =
    "Fully self-contained credit factor hierarchy model artifact.";
/// Filename of the published credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME: &str = "credit_calibration_config.schema.json";
/// Canonical title of the credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE: &str = "CreditCalibrationConfig";
/// Canonical description of the credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION: &str =
    "Configuration for the deterministic credit factor-model calibrator.";
/// Filename of the published credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME: &str = "credit_calibration_inputs.schema.json";
/// Canonical title of the credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE: &str = "CreditCalibrationInputs";
/// Canonical description of the credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION: &str =
    "Typed issuer histories, tags, generic factor series, anchor date, and overrides for one credit calibration run.";

fn parse_schema(
    cache: &'static OnceLock<std::result::Result<Value, String>>,
    raw: &'static str,
    filename: &'static str,
) -> finstack_quant_core::Result<&'static Value> {
    cache
        .get_or_init(|| {
            serde_json::from_str(raw)
                .map_err(|error| format!("invalid factor-model schema JSON at {filename}: {error}"))
        })
        .as_ref()
        .map_err(|error| finstack_quant_core::Error::Internal(error.clone()))
}

/// Return the checked-in schema for [`crate::FactorModelConfigEnvelope`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn factor_model_config_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/factor_model_config.schema.json"),
        FACTOR_MODEL_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::hierarchy::CreditFactorModel`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_factor_model_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_factor_model.schema.json"),
        CREDIT_FACTOR_MODEL_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::calibration::CreditCalibrationConfig`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_calibration_config_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_calibration_config.schema.json"),
        CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::calibration::CreditCalibrationInputs`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_calibration_inputs_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_calibration_inputs.schema.json"),
        CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME,
    )
}




/// The three-issuer, six-date panel behind both credit examples.
///
/// Small enough to read end to end, and shared so the published inputs and the
/// published model are demonstrably the same calibration.
fn example_credit_inputs() -> finstack_quant_core::Result<
    crate::credit::calibration::CreditCalibrationInputs,
> {
    use crate::credit::calibration::{
        CreditCalibrationInputs, GenericFactorSeries, HistoryPanel, IssuerTagPanel,
    };
    use crate::credit::hierarchy::{GenericFactorSpec, IssuerTags};
    use std::collections::BTreeMap;

    let months = [
        finstack_quant_core::dates::Month::January,
        finstack_quant_core::dates::Month::February,
        finstack_quant_core::dates::Month::March,
        finstack_quant_core::dates::Month::April,
        finstack_quant_core::dates::Month::May,
        finstack_quant_core::dates::Month::June,
    ];
    let dates = months
        .into_iter()
        .map(|month| {
            finstack_quant_core::dates::Date::from_calendar_date(2024, month, 1).map_err(|error| {
                finstack_quant_core::Error::Internal(format!("example date: {error}"))
            })
        })
        .collect::<finstack_quant_core::Result<Vec<_>>>()?;

    let issuers = [
        ("ACME", "ig", [120.0, 125.0, 118.0, 130.0, 127.0, 122.0]),
        ("BOREAS", "ig", [95.0, 99.0, 92.0, 104.0, 101.0, 97.0]),
        ("CYGNUS", "hy", [340.0, 355.0, 331.0, 372.0, 364.0, 349.0]),
    ];

    let mut spreads = BTreeMap::new();
    let mut tags = BTreeMap::new();
    let mut as_of_spreads = BTreeMap::new();
    for (issuer, rating, series) in issuers {
        let id = finstack_quant_core::types::IssuerId::from(issuer);
        spreads.insert(id.clone(), series.iter().map(|value| Some(*value)).collect());
        let mut issuer_tags = BTreeMap::new();
        issuer_tags.insert("rating".to_string(), rating.to_string());
        tags.insert(id.clone(), IssuerTags(issuer_tags));
        as_of_spreads.insert(id, series[series.len() - 1]);
    }

    Ok(CreditCalibrationInputs {
        history_panel: HistoryPanel {
            dates: dates.clone(),
            spreads,
        },
        issuer_tags: IssuerTagPanel { tags },
        generic_factor: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "credit_pc1".to_string(),
                series_id: "CREDIT-PC1".to_string(),
            },
            values: vec![100.0, 104.0, 98.0, 109.0, 106.0, 102.0],
        },
        as_of: dates[dates.len() - 1],
        as_of_spreads,
        idiosyncratic_overrides: BTreeMap::new(),
    })
}

/// Canonical `CreditCalibrationInputs`: the shared panel.
fn credit_calibration_inputs_examples() -> finstack_quant_core::Result<Vec<serde_json::Value>> {
    let value = serde_json::to_value(example_credit_inputs()?).map_err(|error| {
        finstack_quant_core::Error::Internal(format!(
            "serialize credit calibration inputs example: {error}"
        ))
    })?;
    Ok(vec![value])
}

/// Canonical `CreditFactorModel`: the calibrator's own output for that panel.
///
/// Produced by running the calibration rather than hand-writing a result, so
/// the published example is exactly what a caller would get back. `CreditCalibrator`
/// is deterministic, which is what lets a computed artifact be checked in.
fn credit_factor_model_examples() -> finstack_quant_core::Result<Vec<serde_json::Value>> {
    let config = crate::credit::calibration::CreditCalibrationConfig::default();
    let model = crate::credit::calibration::CreditCalibrator::new(config)
        .calibrate(example_credit_inputs()?)?;
    let value = serde_json::to_value(&model).map_err(|error| {
        finstack_quant_core::Error::Internal(format!(
            "serialize credit factor model example: {error}"
        ))
    })?;
    Ok(vec![value])
}

/// A canonical one-factor model: a parallel USD rates factor.
///
/// One factor keeps the covariance matrix 1x1 and the mapping table a single
/// rule, which is enough to show how factors, their market mapping and the
/// dependency matcher fit together.
fn factor_model_config_examples() -> finstack_quant_core::Result<Vec<serde_json::Value>> {
    use crate::{FactorDefinition, FactorId, FactorType, MappingRule, MarketMapping, MatchingConfig};
    use finstack_quant_core::market_data::bumps::BumpUnits;

    let factor_id = FactorId::new("rates::usd::parallel");
    let factor = FactorDefinition {
        id: factor_id.clone(),
        factor_type: FactorType::Rates,
        market_mapping: MarketMapping::CurveParallel {
            curve_ids: vec!["USD-OIS".into()],
            units: BumpUnits::RateBp,
        },
        description: Some("Parallel shift of the USD OIS curve.".to_string()),
    };
    let covariance =
        crate::FactorCovarianceMatrix::new(vec![factor_id.clone()], vec![0.000_1])?;
    let matching = MatchingConfig::MappingTable(vec![MappingRule {
        dependency_filter: Default::default(),
        attribute_filter: Default::default(),
        factor_id,
    }]);

    let config = crate::FactorModelConfig {
        factors: vec![factor],
        covariance,
        matching,
        pricing_mode: crate::PricingMode::DeltaBased,
        risk_measure: Default::default(),
        bump_size: None,
        unmatched_policy: None,
    };
    let value = serde_json::to_value(crate::FactorModelConfigEnvelope::new(config)).map_err(
        |error| {
            finstack_quant_core::Error::Internal(format!(
                "serialize factor-model config example: {error}"
            ))
        },
    )?;
    Ok(vec![value])
}

/// Canonical `CreditCalibrationConfig`: the shipped defaults.
fn credit_calibration_config_examples() -> finstack_quant_core::Result<Vec<serde_json::Value>> {
    let value = serde_json::to_value(crate::credit::calibration::CreditCalibrationConfig::default())
        .map_err(|error| {
            finstack_quant_core::Error::Internal(format!(
                "serialize credit calibration config example: {error}"
            ))
        })?;
    Ok(vec![value])
}

/// The crate's complete schema registry, sorted by artifact path.
///
/// This lives in the library, not the generator binary, so the generator, the
/// contract tests and the bindings all render from one definition. Render an
/// entry with [`finstack_quant_core::schema::SchemaArtifact::generate`];
/// `generated_schema` produces only the raw derived document.
pub const ARTIFACTS: &[finstack_quant_core::schema::SchemaArtifact] = &[
    finstack_quant_core::schema::SchemaArtifact::new::<
        crate::credit::calibration::CreditCalibrationConfig,
    >(
        "schemas/factor_model/1/credit_calibration_config.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_calibration_config.schema.json",
        CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE,
        CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION,
    )
    .with_summary("Estimator choice and bounds for credit factor calibration.")
    .with_examples(credit_calibration_config_examples),
    finstack_quant_core::schema::SchemaArtifact::new::<
        crate::credit::calibration::CreditCalibrationInputs,
    >(
        "schemas/factor_model/1/credit_calibration_inputs.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_calibration_inputs.schema.json",
        CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE,
        CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION,
    )
    .with_summary("Observed spread panel feeding credit factor calibration.")
    .with_examples(credit_calibration_inputs_examples),
    finstack_quant_core::schema::SchemaArtifact::new::<crate::credit::hierarchy::CreditFactorModel>(
        "schemas/factor_model/1/credit_factor_model.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_factor_model.schema.json",
        CREDIT_FACTOR_MODEL_SCHEMA_TITLE,
        CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION,
    )
    .with_kind(finstack_quant_core::schema::SchemaKind::Input)
    .with_summary("Calibrated credit factor hierarchy, loadable as pricing input.")
    .with_examples(credit_factor_model_examples),
    finstack_quant_core::schema::SchemaArtifact::new::<crate::FactorModelConfigEnvelope>(
        "schemas/factor_model/1/factor_model_config.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/factor_model_config.schema.json",
        FACTOR_MODEL_SCHEMA_TITLE,
        FACTOR_MODEL_SCHEMA_DESCRIPTION,
    )
    .with_kind(finstack_quant_core::schema::SchemaKind::Input)
    .with_summary("Factor definitions, matchers and covariance settings.")
    .with_examples(factor_model_config_examples),
];
