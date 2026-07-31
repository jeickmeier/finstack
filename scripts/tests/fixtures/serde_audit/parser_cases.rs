use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct MultilineSpec {
    pub value: String,
}

#[derive(::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
pub enum QualifiedEnvelope {
    Empty,
}

#[derive(Debug)]
pub struct ManualResult {
    pub value: String,
}

impl serde::Serialize for ManualResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> serde::Deserialize<'de> for ManualResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            value: String::deserialize(deserializer)?,
        })
    }
}

impl schemars::JsonSchema for ManualResult {
    fn schema_name() -> String {
        "ManualResult".to_owned()
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(generator)
    }
}

#[derive(Serialize)]
#[serde(into = "String")]
pub struct SerializeOnlyResult {
    pub value: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MarkerOnlyType {
    #[serde(default)]
    pub schema_version: u32,
    pub value: String,
}

#[derive(Debug, finstack_quant_valuations_macros::FocusedPricingOverrides)]
pub struct MacroProvidedSpec {
    pub value: String,
}

#[cfg(test)]
mod tests {
    #[derive(serde::Serialize)]
    pub struct HiddenTestSpec {
        pub value: String,
    }
}
