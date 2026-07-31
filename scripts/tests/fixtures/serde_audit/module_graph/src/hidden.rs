pub struct HiddenSpec {
    pub value: String,
}

#[derive(serde::Serialize)]
pub struct ReexportedResult {
    pub value: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CashflowResult {
    pub value: String,
}

#[derive(serde::Serialize)]
pub struct AliasOnlyResult {
    pub value: String,
}
