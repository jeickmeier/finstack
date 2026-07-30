//! Embedded portfolio defaults registries.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::liquidity::LiquidityConfig;
use finstack_quant_core::{ContractDescriptor, Error, Result};

const LIQUIDITY_DEFAULTS: &str = include_str!("../data/defaults/liquidity_defaults.v1.json");
pub(crate) const LIQUIDITY_DEFAULTS_CONTRACT: ContractDescriptor =
    ContractDescriptor::new("finstack_quant.portfolio.liquidity_defaults", 1);

static EMBEDDED_LIQUIDITY_DEFAULTS: OnceLock<Result<LiquidityDefaults>> = OnceLock::new();

/// Registry-backed portfolio liquidity defaults.
#[derive(Debug, Clone)]
pub struct LiquidityDefaults {
    /// Default liquidity configuration.
    pub default_config: LiquidityConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LiquidityDefaultsFile {
    schema: String,
    version: u32,
    default_config: LiquidityConfig,
}

/// Return the embedded liquidity defaults registry.
///
/// The defaults are parsed once and cached in a process-wide `OnceLock`. If
/// the first caller's parse fails, that error is cached: every subsequent
/// call returns a clone of the *same* error rather than re-parsing. The parse
/// is therefore deterministic — embedded JSON is a compile-time asset — so a
/// failure indicates a build-time defect, not a transient condition.
pub fn embedded_liquidity_defaults() -> Result<&'static LiquidityDefaults> {
    match EMBEDDED_LIQUIDITY_DEFAULTS.get_or_init(parse_liquidity_defaults) {
        Ok(defaults) => Ok(defaults),
        Err(err) => Err(err.clone()),
    }
}

/// Panic-on-failure access for `Default` implementations backed by embedded data.
///
/// Panics only if the embedded liquidity defaults fail to parse. Because the
/// parse result is cached in a `OnceLock`, this is deterministic: it panics
/// on the first call or never. The embedded JSON is a compile-time asset, so
/// a panic here means the binary itself is malformed.
#[must_use]
#[allow(clippy::expect_used)]
pub fn embedded_liquidity_defaults_or_panic() -> &'static LiquidityDefaults {
    embedded_liquidity_defaults()
        .expect("embedded portfolio liquidity defaults are compile-time assets")
}

fn parse_liquidity_defaults() -> Result<LiquidityDefaults> {
    let file: LiquidityDefaultsFile = serde_json::from_str(LIQUIDITY_DEFAULTS).map_err(|err| {
        Error::Validation(format!(
            "failed to parse embedded liquidity defaults: {err}"
        ))
    })?;
    liquidity_defaults_from_file(file)
}

fn liquidity_defaults_from_file(file: LiquidityDefaultsFile) -> Result<LiquidityDefaults> {
    let schema_version = LIQUIDITY_DEFAULTS_CONTRACT
        .parse_schema(&file.schema)
        .map_err(|error| {
            Error::Validation(format!(
                "invalid liquidity defaults schema {:?}; expected {:?}: {error}",
                file.schema,
                LIQUIDITY_DEFAULTS_CONTRACT.schema_string(),
            ))
        })?;
    let version = LIQUIDITY_DEFAULTS_CONTRACT
        .resolve(Some(file.version))
        .map_err(|error| Error::Validation(error.to_string()))?;
    if schema_version != version {
        return Err(Error::Validation(format!(
            "liquidity defaults schema version {schema_version} does not match version field \
             {version}"
        )));
    }
    validate_liquidity_config(&file.default_config)?;
    Ok(LiquidityDefaults {
        default_config: file.default_config,
    })
}

fn validate_liquidity_config(config: &LiquidityConfig) -> Result<()> {
    validate_positive("liquidity.participation_rate", config.participation_rate)?;
    validate_positive("liquidity.risk_aversion", config.risk_aversion)?;
    validate_positive("liquidity.holding_period", config.holding_period)?;
    validate_unit_interval("liquidity.confidence_level", config.confidence_level)?;
    validate_non_negative(
        "liquidity.endogenous_spread_coef",
        config.endogenous_spread_coef,
    )?;
    let mut prev = 0.0;
    for (idx, threshold) in config.tier_thresholds.iter().copied().enumerate() {
        validate_positive(&format!("liquidity.tier_thresholds[{idx}]"), threshold)?;
        if threshold <= prev {
            return Err(Error::Validation(format!(
                "liquidity tier threshold {idx} must be greater than prior threshold"
            )));
        }
        prev = threshold;
    }
    Ok(())
}

fn validate_positive(label: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(Error::Validation(format!("{label} must be positive")))
    }
}

fn validate_non_negative(label: &str, value: f64) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(Error::Validation(format!("{label} must be non-negative")))
    }
}

fn validate_unit_interval(label: &str, value: f64) -> Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(Error::Validation(format!("{label} must be in [0, 1]")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_value(value: serde_json::Value) -> Result<LiquidityDefaults> {
        let file: LiquidityDefaultsFile =
            serde_json::from_value(value).map_err(|error| Error::Validation(error.to_string()))?;
        liquidity_defaults_from_file(file)
    }

    #[test]
    fn liquidity_defaults_reject_missing_zero_future_and_malformed_contract_markers() {
        let base: serde_json::Value =
            serde_json::from_str(LIQUIDITY_DEFAULTS).expect("embedded fixture is JSON");

        let mut missing_schema = base.clone();
        missing_schema
            .as_object_mut()
            .expect("object")
            .remove("schema");
        assert!(load_value(missing_schema).is_err());

        let mut missing_version = base.clone();
        missing_version
            .as_object_mut()
            .expect("object")
            .remove("version");
        assert!(load_value(missing_version).is_err());

        for version in [0, 2] {
            let mut value = base.clone();
            value["version"] = serde_json::json!(version);
            assert!(load_value(value).is_err(), "version {version} must fail");
        }

        for schema in [
            "finstack_quant.portfolio.liquidity_defaults/0",
            "finstack_quant.portfolio.liquidity_defaults/2",
            "finstack_quant.portfolio.liquidity_defaults/not-a-version",
            "other.liquidity_defaults/1",
        ] {
            let mut value = base.clone();
            value["schema"] = serde_json::json!(schema);
            assert!(load_value(value).is_err(), "schema {schema} must fail");
        }
    }

    #[test]
    fn liquidity_defaults_version_matrix_fixture_drives_loader() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/data/liquidity_defaults_version_matrix.json"
        ))
        .expect("version matrix fixture parses");
        let base = fixture["base"].clone();
        let cases = fixture["cases"]
            .as_array()
            .expect("fixture contains contract cases");

        for case in cases {
            let name = case["name"].as_str().expect("case name");
            let mut document = base.clone();
            for field in ["schema", "version"] {
                match case.get(field) {
                    Some(serde_json::Value::Null) => {
                        document
                            .as_object_mut()
                            .expect("defaults object")
                            .remove(field);
                    }
                    Some(value) => document[field] = value.clone(),
                    None => {}
                }
            }
            let expected = case["expected"].as_str().expect("expected outcome");
            match load_value(document) {
                Ok(_) => assert_eq!(expected, "ok", "{name} unexpectedly loaded"),
                Err(error) => assert!(
                    error.to_string().contains(expected),
                    "{name}: expected {expected} in {error}"
                ),
            }
        }
    }
}
