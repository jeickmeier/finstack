//! Shared numeric conversion helpers.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;

/// Convert a decimal value to `f64` with an explicit validation error.
#[inline]
pub(crate) fn decimal_to_f64(value: Decimal, field_name: &str) -> finstack_quant_core::Result<f64> {
    value.to_f64().ok_or_else(|| {
        finstack_quant_core::Error::Validation(format!(
            "{} value {} cannot be converted to f64",
            field_name, value
        ))
    })
}

/// Deserialize a finite, strictly positive `f64` through the canonical wire type.
pub(crate) fn deserialize_positive_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::PositiveF64Wire::deserialize(deserializer)
        .map(finstack_quant_core::wire::PositiveF64Wire::into_inner)
}

/// Deserialize a finite probability in the open interval `(0, 1)`.
pub(crate) fn deserialize_open_unit_interval_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::OpenUnitIntervalF64Wire::deserialize(deserializer)
        .map(finstack_quant_core::wire::OpenUnitIntervalF64Wire::into_inner)
}

/// Deserialize a finite correlation coefficient in `[-1, 1]`.
pub(crate) fn deserialize_correlation<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::CorrelationWire::deserialize(deserializer)
        .map(finstack_quant_core::wire::CorrelationWire::into_inner)
}
