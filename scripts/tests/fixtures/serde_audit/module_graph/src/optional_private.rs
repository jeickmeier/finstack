#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct OptionalFeatureEnvelope {
    pub value: String,
}
