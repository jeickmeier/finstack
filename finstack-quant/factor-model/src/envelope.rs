//! Versioned persistence envelope for factor-model configuration.

use finstack_quant_core::contract::{
    deserialize_json_value, parse_json_value, ContractDescriptor, ContractError, Diagnostic,
    LoadLimits, LoadPhase, Severity, ValidationReport,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::FactorModelConfig;

/// Persistence contract for [`FactorModelConfigEnvelope`].
pub const FACTOR_MODEL_CONFIG_CONTRACT: ContractDescriptor =
    ContractDescriptor::new("finstack_quant.factor_model_config", 1);

fn factor_model_config_schema_marker(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let marker = FACTOR_MODEL_CONFIG_CONTRACT.schema_string();
    schemars::json_schema!({
        "type": "string",
        "const": marker,
    })
}

/// Versioned wrapper used when persisting factor-model configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactorModelConfigEnvelope {
    /// Exact factor-model configuration contract marker.
    #[schemars(schema_with = "factor_model_config_schema_marker")]
    pub schema: String,
    /// Bare configuration used by in-process analysis APIs.
    pub config: FactorModelConfig,
}

impl FactorModelConfigEnvelope {
    /// Wrap a factor-model configuration with the current schema marker.
    ///
    /// # Arguments
    ///
    /// * `config` - Bare factor-model configuration to persist.
    #[must_use]
    pub fn new(config: FactorModelConfig) -> Self {
        Self {
            schema: FACTOR_MODEL_CONFIG_CONTRACT.schema_string(),
            config,
        }
    }

    /// Compute the versioned SHA-256 hash of this envelope's canonical JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration contains a non-finite number or
    /// cannot be serialized to the core canonical JSON representation.
    pub fn content_hash(&self) -> finstack_quant_core::Result<String> {
        finstack_quant_core::canonical::content_hash(self)
    }

    /// Load and validate a persisted factor-model configuration envelope.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Complete UTF-8 JSON encoding of a factor-model envelope.
    /// * `limits` - Resource policy bounding input size, JSON depth, and
    ///   retained diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] for malformed JSON, resource-limit failures,
    /// missing, malformed, or unsupported schema markers, invalid envelope
    /// shape, or factor-universe validation failures.
    pub fn from_slice_strict(
        bytes: &[u8],
        limits: &LoadLimits,
    ) -> Result<(FactorModelConfig, ValidationReport), ContractError> {
        let value = parse_json_value(bytes, limits)?;
        let schema = match value.get("schema") {
            Some(schema) => Some(deserialize_json_value::<String>(schema.clone(), limits)?),
            None => None,
        };
        FACTOR_MODEL_CONFIG_CONTRACT.parse_schema_strict(schema.as_deref(), "/schema", limits)?;
        let envelope: Self = deserialize_json_value(value, limits)?;
        envelope
            .config
            .validate_matching_factor_ids()
            .and_then(|()| envelope.config.risk_measure.validate())
            .map_err(|error| semantic_error(error, limits))?;
        Ok((envelope.config, ValidationReport::default()))
    }
}

fn semantic_error(error: finstack_quant_core::Error, limits: &LoadLimits) -> ContractError {
    let mut report = ValidationReport::default();
    report.push_bounded(
        limits,
        Diagnostic::new(
            "contract/semantic-invalid",
            LoadPhase::Semantic,
            Severity::Error,
            error.to_string(),
        )
        .with_contract(FACTOR_MODEL_CONFIG_CONTRACT.id),
    );
    ContractError::Report(Box::new(report))
}
