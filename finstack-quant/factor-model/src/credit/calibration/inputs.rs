use std::collections::BTreeMap;

use finstack_quant_core::dates::Date;
use finstack_quant_core::types::IssuerId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::credit::hierarchy::{GenericFactorSpec, IssuerTags};

/// Sparse issuer-spread history aligned to a sorted date grid.
///
/// `dates` is the sorted observation grid. `spreads[issuer]` has length
/// `dates.len()`; entries are `Some(spread)` when the issuer was observed at
/// that date and `None` otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryPanel {
    /// Observation dates (sorted ascending).
    #[serde(with = "finstack_quant_core::wire::dates")]
    #[schemars(with = "Vec<finstack_quant_core::wire::DateWire>")]
    pub dates: Vec<Date>,
    /// Per-issuer spread series aligned with [`dates`][Self::dates].
    pub spreads: BTreeMap<IssuerId, Vec<Option<f64>>>,
}

/// Point-in-time issuer tags at the calibration `as_of`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssuerTagPanel {
    /// Tag map keyed by issuer.
    pub tags: BTreeMap<IssuerId, IssuerTags>,
}

/// Generic (PC) factor reference and aligned values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenericFactorSeries {
    /// Reference (name + series_id) embedded into the artifact.
    pub spec: GenericFactorSpec,
    /// Generic factor values aligned with [`HistoryPanel::dates`].
    pub values: Vec<f64>,
}

/// All inputs the calibrator needs for a single calibration run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditCalibrationInputs {
    /// Sparse issuer-spread history.
    pub history_panel: HistoryPanel,
    /// Per-issuer hierarchy tags (point-in-time).
    pub issuer_tags: IssuerTagPanel,
    /// Generic factor series + spec.
    pub generic_factor: GenericFactorSeries,
    /// Calibration anchor date (must appear in `history_panel.dates`).
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub as_of: Date,
    /// Issuer spreads at `as_of` (level space).
    pub as_of_spreads: BTreeMap<IssuerId, f64>,
    /// Optional caller-supplied idiosyncratic vol overrides.
    ///
    /// Caller-supplied values take precedence over history, peer-proxy, and
    /// global-default adder-vol estimates.
    pub idiosyncratic_overrides: BTreeMap<IssuerId, f64>,
}
