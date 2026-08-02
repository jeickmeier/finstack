#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct GlobEnvelope {
    pub value: String,
}
