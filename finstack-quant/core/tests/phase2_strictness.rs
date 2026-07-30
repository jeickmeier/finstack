//! Phase 2 nested strictness and finite-value deserialization regressions.

use finstack_quant_core::dates::{build_periods, Period};
use finstack_quant_core::market_data::hierarchy::MarketDataHierarchy;
use finstack_quant_core::market_data::scalars::MarketScalar;
use serde::de::value::Error as ValueError;
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor};
use serde::Deserialize;

struct UnitlessScalarDeserializer(f64);

impl<'de> serde::Deserializer<'de> for UnitlessScalarDeserializer {
    type Error = ValueError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom("expected MarketScalar enum"))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(UnitlessScalarEnumAccess(self.0))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct identifier ignored_any
    }
}

struct UnitlessScalarEnumAccess(f64);

impl<'de> EnumAccess<'de> for UnitlessScalarEnumAccess {
    type Error = ValueError;
    type Variant = UnitlessScalarVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize("unitless".into_deserializer())?;
        Ok((variant, UnitlessScalarVariantAccess(self.0)))
    }
}

struct UnitlessScalarVariantAccess(f64);

impl<'de> VariantAccess<'de> for UnitlessScalarVariantAccess {
    type Error = ValueError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Err(de::Error::custom("unitless requires a numeric value"))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.0.into_deserializer())
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom("unitless is a newtype variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom("unitless is a newtype variant"))
    }
}

#[test]
fn period_rejects_unknown_fields() {
    let period = build_periods("2025Q1..Q1", None)
        .expect("valid period plan")
        .periods
        .into_iter()
        .next()
        .expect("one period");
    let mut value = serde_json::to_value(period).expect("period serializes");
    value
        .as_object_mut()
        .expect("period is an object")
        .insert("__nonexistent__".into(), serde_json::json!(true));

    let error = serde_json::from_value::<Period>(value).expect_err("unknown fields must fail");
    assert!(error.to_string().contains("unknown field"), "{error}");
}

#[test]
fn market_scalar_shadow_conversion_rejects_every_non_finite_value() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = MarketScalar::deserialize(UnitlessScalarDeserializer(value))
            .expect_err("the serde shadow conversion must reject non-finite values");
        assert!(error.to_string().contains("finite"), "{error}");
    }
}

#[test]
fn market_hierarchy_rejects_unknown_raw_fields_recursively() {
    let top_level = serde_json::json!({
        "roots": {},
        "__nonexistent__": true
    });
    let nested = serde_json::json!({
        "roots": {
            "Rates": {
                "curves": [],
                "__nonexistent__": true
            }
        }
    });

    for value in [top_level, nested] {
        let error = serde_json::from_value::<MarketDataHierarchy>(value)
            .expect_err("unknown raw hierarchy fields must fail");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
}
