//! Serializable, lossless hazard-curve calibration replay data.

/// Exact valuation-layer inputs required to replay a hazard-curve calibration.
///
/// The core market-data crate stores these payloads without interpreting them;
/// the valuations crate deserializes them back into its canonical
/// `HazardCurveParams`, typed CDS quotes, and `CalibrationConfig`. Keeping the
/// complete serde payloads avoids replacing date pillars with rounded tenors or
/// silently substituting current defaults for the original solver policy.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HazardCalibrationRecipe {
    /// Exact serialized `HazardCurveParams` used for the original solve.
    pub hazard_params: serde_json::Value,
    /// Complete ordered set of typed CDS quote payloads used for the original solve.
    pub cds_quotes: Vec<serde_json::Value>,
    /// Exact serialized `CalibrationConfig`, including solver and validation policy.
    pub calibration_config: serde_json::Value,
}
