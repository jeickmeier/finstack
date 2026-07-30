//! Strict wire types for portfolio materialization bundles.

use crate::book::{Book, BookId};
use crate::position::PositionUnit;
use crate::types::{AttributeValue, Entity, EntityId, PositionId};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::types::{CurveId, PriceId};
use finstack_quant_valuations::instruments::{
    FxPair, InstrumentCurves, MarketDependencies, VolatilityDependency,
};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

fn materialization_schema_marker(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string",
        "const": super::PORTFOLIO_MATERIALIZATION_CONTRACT.schema_string(),
    })
}

/// Producer-supplied market-dependency claim for an instrument artifact.
///
/// The alias names the dependency payload's role in the materialization
/// contract while retaining the canonical valuations representation.
pub type MarketDependenciesSpec = MarketDependencies;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictEntityWire {
    id: EntityId,
    name: Option<String>,
    #[serde(default)]
    tags: IndexMap<String, String>,
    #[serde(default)]
    meta: IndexMap<String, serde_json::Value>,
}

impl From<StrictEntityWire> for Entity {
    fn from(value: StrictEntityWire) -> Self {
        Self {
            id: value.id,
            name: value.name,
            tags: value.tags,
            meta: value.meta,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictBookWire {
    id: BookId,
    name: Option<String>,
    parent_id: Option<BookId>,
    position_ids: Vec<PositionId>,
    child_book_ids: Vec<BookId>,
    #[serde(default)]
    tags: IndexMap<String, String>,
    #[serde(default)]
    meta: IndexMap<String, serde_json::Value>,
}

impl From<StrictBookWire> for Book {
    fn from(value: StrictBookWire) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parent_id: value.parent_id,
            position_ids: value.position_ids,
            child_book_ids: value.child_book_ids,
            tags: value.tags,
            meta: value.meta,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictInstrumentCurvesWire {
    discount_curves: Vec<CurveId>,
    forward_curves: Vec<CurveId>,
    credit_curves: Vec<CurveId>,
    inflation_curves: Vec<CurveId>,
}

impl From<StrictInstrumentCurvesWire> for InstrumentCurves {
    fn from(value: StrictInstrumentCurvesWire) -> Self {
        Self {
            discount_curves: value.discount_curves.into_iter().collect(),
            forward_curves: value.forward_curves.into_iter().collect(),
            credit_curves: value.credit_curves.into_iter().collect(),
            inflation_curves: value.inflation_curves.into_iter().collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictVolatilityDependencyWire {
    surface_id: CurveId,
    underlying_id: Option<PriceId>,
    reference_strike: Option<f64>,
}

impl From<StrictVolatilityDependencyWire> for VolatilityDependency {
    fn from(value: StrictVolatilityDependencyWire) -> Self {
        Self::new(
            value.surface_id,
            value.underlying_id,
            value.reference_strike,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictFxPairWire {
    base: Currency,
    quote: Currency,
}

impl From<StrictFxPairWire> for FxPair {
    fn from(value: StrictFxPairWire) -> Self {
        Self::new(value.base, value.quote)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictMarketDependenciesWire {
    curves: StrictInstrumentCurvesWire,
    spot_ids: Vec<String>,
    volatility_dependencies: Vec<StrictVolatilityDependencyWire>,
    fx_pairs: Vec<StrictFxPairWire>,
    series_ids: Vec<String>,
}

impl From<StrictMarketDependenciesWire> for MarketDependencies {
    fn from(value: StrictMarketDependenciesWire) -> Self {
        Self {
            curves: value.curves.into(),
            spot_ids: value.spot_ids,
            volatility_dependencies: value
                .volatility_dependencies
                .into_iter()
                .map(Into::into)
                .collect(),
            fx_pairs: value.fx_pairs.into_iter().map(Into::into).collect(),
            series_ids: value.series_ids,
        }
    }
}

fn deserialize_entities<'de, D>(deserializer: D) -> Result<IndexMap<EntityId, Entity>, D::Error>
where
    D: Deserializer<'de>,
{
    IndexMap::<EntityId, StrictEntityWire>::deserialize(deserializer).map(|values| {
        values
            .into_iter()
            .map(|(id, entity)| (id, entity.into()))
            .collect()
    })
}

fn deserialize_books<'de, D>(deserializer: D) -> Result<IndexMap<BookId, Book>, D::Error>
where
    D: Deserializer<'de>,
{
    IndexMap::<BookId, StrictBookWire>::deserialize(deserializer).map(|values| {
        values
            .into_iter()
            .map(|(id, book)| (id, book.into()))
            .collect()
    })
}

pub(super) fn deserialize_dependencies(
    raw: &RawValue,
) -> serde_json::Result<MarketDependenciesSpec> {
    serde_json::from_str::<StrictMarketDependenciesWire>(raw.get()).map(Into::into)
}

fn deserialize_optional_dependencies<'de, D>(
    deserializer: D,
) -> Result<Option<MarketDependenciesSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StrictMarketDependenciesWire>::deserialize(deserializer)
        .map(|value| value.map(Into::into))
}

/// Strict, versioned portfolio materialization bundle.
///
/// Unlike [`crate::portfolio::PortfolioSpec`], this database-oriented format
/// stores each instrument artifact once and lets ordered lightweight positions
/// reference the artifact by ID.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PortfolioMaterializationEnvelope {
    /// Exact materialization contract marker.
    #[schemars(schema_with = "materialization_schema_marker")]
    pub schema: String,
    /// Portfolio fields that do not contain runtime instrument trait objects.
    pub portfolio: PortfolioHeader,
    /// Unique strict instrument envelopes referenced by positions.
    pub instruments: Vec<InstrumentArtifact>,
    /// Positions in the order required by the reconstructed portfolio.
    pub positions: Vec<MaterializedPosition>,
    /// Optional producer and compiler version provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materializer: Option<MaterializerInfo>,
}

/// Portfolio fields shared by every materialized position.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PortfolioHeader {
    /// Stable portfolio identifier.
    pub id: String,
    /// Optional human-readable portfolio name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Reporting currency used for portfolio aggregation.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub base_ccy: Currency,
    /// Valuation date for the materialized portfolio.
    #[schemars(with = "String")]
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub as_of: Date,
    /// Entities keyed by their stable IDs in deterministic order.
    #[serde(deserialize_with = "deserialize_entities")]
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub entities: IndexMap<EntityId, Entity>,
    /// Optional book hierarchy keyed by stable book IDs.
    #[serde(
        default,
        skip_serializing_if = "IndexMap::is_empty",
        deserialize_with = "deserialize_books"
    )]
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub books: IndexMap<BookId, Book>,
    /// Portfolio-level grouping and classification tags.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, string>"))]
    pub tags: IndexMap<String, String>,
    /// Extension metadata retained as part of the persisted bundle.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// One unique, content-addressed instrument artifact.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InstrumentArtifact {
    /// Immutable producer revision ID or content-addressed artifact ID.
    pub artifact_id: String,
    /// Optional claimed canonical digest, verified before decoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Full strict [`finstack_quant_valuations::instruments::InstrumentEnvelope`]
    /// JSON retained without constructing a duplicate generic JSON tree during
    /// the outer bundle parse.
    #[schemars(with = "serde_json::Value")]
    #[cfg_attr(feature = "ts_export", ts(type = "unknown"))]
    pub envelope: Box<RawValue>,
    /// Optional producer dependency claim, checked against runtime extraction.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_dependencies"
    )]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub dependencies: Option<MarketDependenciesSpec>,
}

/// Lightweight position referencing a unique instrument artifact.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MaterializedPosition {
    /// Stable position identifier.
    #[schemars(with = "String")]
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub id: PositionId,
    /// Stable ID of the entity that owns the position.
    #[schemars(with = "String")]
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub entity_id: EntityId,
    /// Instrument identifier exposed by portfolio lookup and reports.
    pub instrument_id: String,
    /// Artifact ID resolved against
    /// [`PortfolioMaterializationEnvelope::instruments`].
    pub artifact_id: String,
    /// Signed holding quantity interpreted according to [`PositionUnit`].
    pub quantity: f64,
    /// Scaling convention applied to `quantity`.
    #[schemars(with = "serde_json::Value")]
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub unit: PositionUnit,
    /// Position attributes used for grouping, filtering, and constraints.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub attributes: IndexMap<String, AttributeValue>,
    /// Extension metadata retained with the position.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Producer and compiler version stamps for reproducibility.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MaterializerInfo {
    /// Stable producer implementation name.
    pub producer: String,
    /// Version of the producer that assembled the bundle.
    pub producer_version: String,
    /// Optional version of the artifact compiler used by the producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler_version: Option<String>,
    /// Finstack version targeted by the compiled artifacts.
    pub finstack_version: String,
}
