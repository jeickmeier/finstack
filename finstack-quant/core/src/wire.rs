//! Canonical serde representations whose domain storage cannot describe its
//! JSON contract directly to `schemars`.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::Date;

/// Numeric schema revision for contracts whose sole supported revision is v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct SchemaVersion(#[schemars(range(min = 1, max = 1))] pub u32);

impl SchemaVersion {
    /// Canonical numeric revision used by every v1-only wire contract.
    pub const CURRENT: Self = Self(1);

    /// Canonical numeric revision as a primitive integer.
    pub const VALUE: u32 = 1;
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let version = u32::deserialize(deserializer)?;
        if version == Self::VALUE {
            Ok(Self(version))
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported schema_version {version}; expected 1"
            )))
        }
    }
}

impl From<SchemaVersion> for u32 {
    fn from(value: SchemaVersion) -> Self {
        value.0
    }
}

/// ISO 8601 calendar date encoded as a `YYYY-MM-DD` JSON string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct DateWire(#[schemars(with = "String", extend("format" = "date"))] pub Date);

impl From<Date> for DateWire {
    fn from(value: Date) -> Self {
        Self(value)
    }
}

impl From<DateWire> for Date {
    fn from(value: DateWire) -> Self {
        value.0
    }
}

/// Exact decimal encoded only as a JSON string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct DecimalWire(
    #[serde(with = "rust_decimal::serde::str")]
    #[schemars(with = "String", regex(pattern = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$"))]
    pub Decimal,
);

impl From<Decimal> for DecimalWire {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<DecimalWire> for Decimal {
    fn from(value: DecimalWire) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn date_schema_has_date_format() {
        let schema = serde_json::to_value(schemars::schema_for!(DateWire)).expect("schema");
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "date");
    }

    #[test]
    fn decimal_wire_is_string_only() {
        let decimal: DecimalWire = serde_json::from_value(json!("-1.25e+2")).expect("decimal");
        assert_eq!(decimal.0, Decimal::new(-125, 0));
        assert!(serde_json::from_value::<DecimalWire>(json!(1.25)).is_err());

        let schema = serde_json::to_value(schemars::schema_for!(DecimalWire)).expect("schema");
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["pattern"], r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$");
    }

    #[test]
    fn schema_version_accepts_only_numeric_one() {
        assert_eq!(
            serde_json::from_value::<SchemaVersion>(json!(1)).expect("v1"),
            SchemaVersion::CURRENT
        );
        assert!(serde_json::from_value::<SchemaVersion>(json!(0)).is_err());
        assert!(serde_json::from_value::<SchemaVersion>(json!(2)).is_err());
        assert!(serde_json::from_value::<SchemaVersion>(json!("1")).is_err());

        let schema = serde_json::to_value(schemars::schema_for!(SchemaVersion)).expect("schema");
        assert_eq!(schema["minimum"], 1);
        assert_eq!(schema["maximum"], 1);
    }
}
