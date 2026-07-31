pub mod duplicate_a;
pub mod duplicate_b;
mod nested_private;

pub use nested_private::NestedResult;

#[derive(::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
pub struct AliasTarget {
    pub value: String,
}

pub type LocalAliasEnvelope = AliasTarget;

pub struct CrossFileResult {
    pub value: String,
}

pub struct DomainVersion {
    pub version: u16,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionedDocument {
    #[serde(default)]
    pub schema_version: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerdeVersionedDocument {
    pub schema_version: String,
}

#[cfg_attr(
    feature = "optional",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct FeatureOnlyResult {
    pub value: String,
}

#[cfg_attr(not(feature = "optional"), derive(serde::Serialize))]
pub struct DefaultDerivedResult {
    pub value: String,
}

#[cfg_attr(feature = "enabled", derive(serde::Deserialize))]
pub struct EnabledByDefaultResult {
    pub value: String,
}

#[cfg_attr(
    all(
        feature = "enabled",
        any(not(feature = "optional"), feature = "missing")
    ),
    derive(serde::Serialize)
)]
pub struct NestedPredicateResult {
    pub value: String,
}

#[derive(finstack_quant_valuations_macros::FocusedPricingOverrides)]
#[pricing_overrides(skip_deserialize)]
pub struct SkipDeserializeSpec {
    pub value: String,
}

pub struct ManualMarker {
    pub schema_version: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DomainDefaultVersion {
    #[serde(default)]
    pub version: u16,
}

pub struct CfgManualResult {
    pub value: String,
}

pub struct CfgExternalImplResult {
    pub value: String,
}
