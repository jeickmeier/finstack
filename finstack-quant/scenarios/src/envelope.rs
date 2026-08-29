//! Versioned persistence envelope for scenario specifications.

use finstack_quant_core::contract::{
    deserialize_json_value, parse_json_value, ContractDescriptor, ContractError, Diagnostic,
    LoadLimits, LoadPhase, Severity, ValidationReport,
};
use serde::{Deserialize, Serialize};

use crate::ScenarioSpec;

/// Persistence contract for [`ScenarioEnvelope`].
pub const SCENARIO_CONTRACT: ContractDescriptor =
    ContractDescriptor::new("finstack_quant.scenario");

/// Exact schema marker accepted by [`ScenarioEnvelope`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum ScenarioSchema {
    /// The sole supported scenario contract.
    #[serde(rename = "finstack_quant.scenario/1")]
    Scenario,
}

impl ScenarioSchema {
    /// The exact marker required by every persisted scenario envelope.
    pub const CURRENT: Self = Self::Scenario;
}

/// Versioned wrapper used when persisting a scenario specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ScenarioEnvelope {
    /// Exact scenario contract marker.
    pub schema: ScenarioSchema,
    /// Bare scenario specification used by in-process APIs.
    pub scenario: ScenarioSpec,
}

impl ScenarioEnvelope {
    /// Wrap a scenario with the current persistence schema marker.
    ///
    /// # Arguments
    ///
    /// * `scenario` - Bare scenario specification to persist.
    #[must_use]
    pub fn new(scenario: ScenarioSpec) -> Self {
        Self {
            schema: ScenarioSchema::CURRENT,
            scenario,
        }
    }

    /// Compute the versioned SHA-256 hash of this envelope's canonical JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the scenario contains a non-finite number or cannot
    /// be serialized to the core canonical JSON representation.
    pub fn content_hash(&self) -> finstack_quant_core::Result<String> {
        finstack_quant_core::canonical::content_hash(self)
    }

    /// Load and validate a persisted scenario envelope.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Complete UTF-8 JSON encoding of a scenario envelope.
    /// * `limits` - Resource policy bounding input size, JSON depth, and
    ///   retained diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] for malformed JSON, resource-limit failures,
    /// missing, malformed, or unsupported schema markers, invalid envelope
    /// shape, or failed scenario semantic validation.
    pub fn from_slice_strict(
        bytes: &[u8],
        limits: &LoadLimits,
    ) -> Result<(ScenarioSpec, ValidationReport), ContractError> {
        let value = parse_json_value(bytes, limits)?;
        let schema = match value.get("schema") {
            Some(schema) => Some(deserialize_json_value::<String>(schema.clone(), limits)?),
            None => None,
        };
        SCENARIO_CONTRACT.parse_schema_strict(schema.as_deref(), "/schema", limits)?;
        let envelope: Self = deserialize_json_value(value, limits)?;
        envelope.scenario.validate().map_err(|error| {
            let mut report = ValidationReport::default();
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "contract/semantic-invalid",
                    LoadPhase::Semantic,
                    Severity::Error,
                    error.to_string(),
                )
                .with_contract(SCENARIO_CONTRACT.id),
            );
            ContractError::Report(Box::new(report))
        })?;
        Ok((envelope.scenario, ValidationReport::default()))
    }
}
