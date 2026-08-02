//! Shared numeric conversion helpers.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

/// Serialize a finite, strictly positive `f64` through the canonical wire type.
pub(crate) fn serialize_positive_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    finstack_quant_core::wire::PositiveF64Wire::try_from(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Deserialize a finite, non-negative `f64` through the canonical wire type.
pub(crate) fn deserialize_non_negative_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::NonNegativeF64Wire::deserialize(deserializer)
        .map(finstack_quant_core::wire::NonNegativeF64Wire::into_inner)
}

/// Serialize a finite, non-negative `f64` through the canonical wire type.
pub(crate) fn serialize_non_negative_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    finstack_quant_core::wire::NonNegativeF64Wire::try_from(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Deserialize a finite value in the closed interval `[0, 1]`.
pub(crate) fn deserialize_closed_unit_interval_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::ClosedUnitIntervalF64Wire::deserialize(deserializer)
        .map(finstack_quant_core::wire::ClosedUnitIntervalF64Wire::into_inner)
}

/// Serialize a finite value in the closed interval `[0, 1]`.
pub(crate) fn serialize_closed_unit_interval_f64<S>(
    value: &f64,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    finstack_quant_core::wire::ClosedUnitIntervalF64Wire::try_from(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Deserialize a finite probability in the open interval `(0, 1)`.
pub(crate) fn deserialize_open_unit_interval_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::OpenUnitIntervalF64Wire::deserialize(deserializer)
        .map(finstack_quant_core::wire::OpenUnitIntervalF64Wire::into_inner)
}

/// Serialize a finite probability in the open interval `(0, 1)`.
pub(crate) fn serialize_open_unit_interval_f64<S>(
    value: &f64,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    finstack_quant_core::wire::OpenUnitIntervalF64Wire::try_from(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Deserialize a finite correlation coefficient in `[-1, 1]`.
pub(crate) fn deserialize_correlation<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    finstack_quant_core::wire::CorrelationWire::deserialize(deserializer)
        .map(finstack_quant_core::wire::CorrelationWire::into_inner)
}

/// Serialize a finite correlation coefficient in `[-1, 1]`.
pub(crate) fn serialize_correlation<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    finstack_quant_core::wire::CorrelationWire::try_from(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Deserialize an optional finite, strictly positive `f64`.
pub(crate) fn deserialize_optional_positive_f64<'de, D>(
    deserializer: D,
) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<finstack_quant_core::wire::PositiveF64Wire>::deserialize(deserializer)
        .map(|value| value.map(finstack_quant_core::wire::PositiveF64Wire::into_inner))
}

/// Serialize an optional finite, strictly positive `f64`.
pub(crate) fn serialize_optional_positive_f64<S>(
    value: &Option<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value
        .map(finstack_quant_core::wire::PositiveF64Wire::try_from)
        .transpose()
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}
