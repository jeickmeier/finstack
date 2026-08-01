//! Strict wire types for portfolio materialization bundles.

use crate::book::{Book, BookId};
use crate::position::PositionUnit;
use crate::types::{AttributeValue, Entity, EntityId, PositionId};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::wire::PercentageQuantityWire;
use finstack_quant_valuations::instruments::{InstrumentEnvelope, MarketDependencies};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Sole supported portfolio-materialization contract marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
pub enum PortfolioMaterializationSchema {
    /// Canonical v1 materialization contract.
    #[serde(rename = "finstack_quant.portfolio_materialization/1")]
    Materialization,
}

impl PortfolioMaterializationSchema {
    /// The exact marker required by every persisted materialization envelope.
    pub const CURRENT: Self = Self::Materialization;
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
    pub schema: PortfolioMaterializationSchema,
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
    pub base_currency: Currency,
    /// Valuation date for the materialized portfolio.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub as_of: Date,
    /// Entities keyed by their stable IDs in deterministic order.
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub entities: IndexMap<EntityId, Entity>,
    /// Optional book hierarchy keyed by stable book IDs.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
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
    /// Full typed, strict instrument envelope.
    #[cfg_attr(feature = "ts_export", ts(type = "unknown"))]
    pub envelope: InstrumentEnvelope,
    /// Optional producer dependency claim, checked against runtime extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub dependencies: Option<MarketDependencies>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
enum MaterializedPositionWire {
    Percentage(PercentageMaterializedPositionWire),
    NonPercentage(NonPercentageMaterializedPositionWire),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PercentageMaterializedPositionWire {
    #[schemars(with = "String")]
    id: PositionId,
    #[schemars(with = "String")]
    entity_id: EntityId,
    instrument_id: String,
    artifact_id: String,
    quantity: PercentageQuantityWire,
    unit: PercentagePositionUnitWire,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    attributes: IndexMap<String, AttributeValue>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    meta: IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NonPercentageMaterializedPositionWire {
    #[schemars(with = "String")]
    id: PositionId,
    #[schemars(with = "String")]
    entity_id: EntityId,
    instrument_id: String,
    artifact_id: String,
    quantity: f64,
    unit: NonPercentagePositionUnitWire,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    attributes: IndexMap<String, AttributeValue>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    meta: IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PercentagePositionUnitWire {
    Percentage,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
enum NonPercentagePositionUnitWire {
    Named(NonPercentagePositionUnitName),
    Notional { notional: Option<Currency> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum NonPercentagePositionUnitName {
    Units,
    FaceValue,
}

/// Lightweight position referencing a unique instrument artifact.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[serde(from = "MaterializedPositionWire")]
#[schemars(with = "MaterializedPositionWire")]
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
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub unit: PositionUnit,
    /// Position attributes used for grouping, filtering, and constraints.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub attributes: IndexMap<String, AttributeValue>,
    /// Extension metadata retained with the position.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl From<MaterializedPositionWire> for MaterializedPosition {
    fn from(value: MaterializedPositionWire) -> Self {
        match value {
            MaterializedPositionWire::Percentage(position) => Self {
                id: position.id,
                entity_id: position.entity_id,
                instrument_id: position.instrument_id,
                artifact_id: position.artifact_id,
                quantity: position.quantity.into_inner(),
                unit: PositionUnit::Percentage,
                attributes: position.attributes,
                meta: position.meta,
            },
            MaterializedPositionWire::NonPercentage(position) => Self {
                id: position.id,
                entity_id: position.entity_id,
                instrument_id: position.instrument_id,
                artifact_id: position.artifact_id,
                quantity: position.quantity,
                unit: match position.unit {
                    NonPercentagePositionUnitWire::Named(NonPercentagePositionUnitName::Units) => {
                        PositionUnit::Units
                    }
                    NonPercentagePositionUnitWire::Named(
                        NonPercentagePositionUnitName::FaceValue,
                    ) => PositionUnit::FaceValue,
                    NonPercentagePositionUnitWire::Notional { notional } => {
                        PositionUnit::Notional(notional)
                    }
                },
                attributes: position.attributes,
                meta: position.meta,
            },
        }
    }
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
