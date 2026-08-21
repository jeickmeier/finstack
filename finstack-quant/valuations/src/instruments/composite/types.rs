use crate::instruments::common_impl::dependencies::MarketDependencies;
use crate::instruments::{
    Attributes, Instrument, InstrumentEnvelope, InstrumentJson, InstrumentPricingOverrides,
    MetricPricingOverrides, PricingOptions, ScenarioPricingOverrides,
};
use crate::metrics::{is_additive_metric, MetricId};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{BusinessDayConvention, Date, ScheduleBuilder, Tenor};
use finstack_quant_core::expr::{CompiledExpr, EvalOpts, Expr, SimpleContext};
use finstack_quant_core::market_data::context::{MarketContext, MarketContextState};
use finstack_quant_core::math::summation::neumaier_sum;
use finstack_quant_core::money::{fx::FxQuery, Money};
use finstack_quant_core::types::InstrumentId;
use finstack_quant_core::{Error, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Maximum nesting depth of composite-in-composite trees, counting the root.
pub const MAX_COMPOSITE_DEPTH: usize = 8;
/// Maximum number of leg nodes permitted across one composite tree.
pub const MAX_COMPOSITE_LEGS: usize = 64;
const MIN_ABS_INPUT: f64 = 1.0e-12;

/// Runtime cache of boxed composite legs. Not serialized.
#[derive(Default)]
struct BoxedLegCache(OnceLock<Vec<Box<dyn Instrument>>>);

impl Clone for BoxedLegCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for BoxedLegCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoxedLegCache")
    }
}

struct VolatilityWeightingConfig<'a> {
    anchor_leg_id: &'a InstrumentId,
    anchor_quantity: f64,
    lookback: usize,
    min_observations: usize,
    annualization_factor: f64,
}

/// One self-contained leg in a composite specification.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeLegSpec {
    /// Stable identifier of the embedded instrument; must equal `instrument.id()`.
    pub instrument_id: InstrumentId,
    /// Canonical typed definition of the underlying instrument.
    pub instrument: Box<InstrumentJson>,
    /// Signed quantity for fixed weighting or signed target score for dynamic weighting.
    pub weight: f64,
}

impl CompositeLegSpec {
    /// Construct a self-contained composite leg.
    ///
    /// `weight` is the resolved quantity under [`WeightingMethod::FixedQuantity`]
    /// and the signed target score for every dynamic method. Validation requires
    /// a finite value with absolute magnitude greater than `1e-12`.
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Identifier that must equal `instrument.id()` after
    ///   the embedded definition is boxed.
    /// * `instrument` - Canonical typed underlying instrument; nested
    ///   composites are permitted within [`MAX_COMPOSITE_DEPTH`].
    /// * `weight` - Signed fixed quantity or dynamic target score; must be
    ///   finite and non-zero.
    #[must_use]
    pub fn new(
        instrument_id: impl Into<InstrumentId>,
        instrument: InstrumentJson,
        weight: f64,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            instrument: Box::new(instrument),
            weight,
        }
    }
}

/// Calendar cadence for automatic composite rebalancing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RebalanceFrequency {
    /// Every calendar day, adjusted by the configured business-day convention.
    Daily,
    /// Every seven calendar days from the schedule start.
    Weekly,
    /// Every calendar month from the schedule start.
    Monthly,
    /// Every three calendar months from the schedule start.
    Quarterly,
}

impl RebalanceFrequency {
    fn tenor(self) -> Result<Tenor> {
        match self {
            Self::Daily => Tenor::parse("1D"),
            Self::Weekly => Ok(Tenor::weekly()),
            Self::Monthly => Ok(Tenor::monthly()),
            Self::Quarterly => Ok(Tenor::quarterly()),
        }
    }
}

/// Rule controlling when dynamic quantities may be explicitly recalculated.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RebalanceRule {
    /// Rebalance only when the caller explicitly invokes `rebalance`.
    Manual,
    /// Rebalance on the supplied adjusted dates.
    Dates {
        /// Strictly increasing dates on which the new state becomes eligible.
        #[serde(with = "finstack_quant_core::wire::dates")]
        #[schemars(with = "Vec<finstack_quant_core::wire::DateWire>")]
        dates: Vec<Date>,
    },
    /// Generate dates from a calendar-aware cadence.
    Calendar {
        /// Unadjusted schedule start.
        #[serde(with = "finstack_quant_core::wire::date")]
        #[schemars(with = "finstack_quant_core::wire::DateWire")]
        start: Date,
        /// Optional final unadjusted schedule date.
        #[serde(default, with = "finstack_quant_core::wire::optional_date")]
        #[schemars(with = "Option<finstack_quant_core::wire::DateWire>")]
        end: Option<Date>,
        /// Daily, weekly, monthly, or quarterly cadence.
        frequency: RebalanceFrequency,
        /// Registered holiday-calendar identifier.
        calendar_id: String,
        /// Business-day adjustment applied to each generated date.
        business_day_convention: BusinessDayConvention,
    },
}

impl RebalanceRule {
    /// Validate ordering, calendar, and schedule bounds.
    ///
    /// Explicit dates must be strictly increasing. Calendar rules must have
    /// `end >= start` when an end date is supplied, and the named holiday
    /// calendar plus business-day convention must produce a valid schedule.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate or unordered explicit dates, invalid
    /// calendar ranges, or unknown calendar identifiers.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Manual => Ok(()),
            Self::Dates { dates } => {
                if dates.windows(2).any(|pair| pair[0] >= pair[1]) {
                    return Err(Error::Validation(
                        "composite rebalance dates must be strictly increasing".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Calendar {
                start,
                end,
                frequency,
                calendar_id,
                business_day_convention,
            } => {
                if end.is_some_and(|end| end < *start) {
                    return Err(Error::Validation(
                        "composite rebalance calendar end precedes start".to_string(),
                    ));
                }
                let horizon = end.unwrap_or(*start);
                let _ = ScheduleBuilder::new(*start, horizon)?
                    .frequency(frequency.tenor()?)
                    .adjust_with_id(*business_day_convention, calendar_id)
                    .build()?;
                Ok(())
            }
        }
    }

    /// Return eligible rebalance dates up to a supplied horizon.
    ///
    /// [`Self::Manual`] yields an empty list. Explicit dates are filtered to
    /// `date <= horizon`. Calendar dates are generated from `start` through
    /// `min(end, horizon)` (or `horizon` when `end` is omitted) and then
    /// business-day adjusted.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Latest date, inclusive, to include after business-day
    ///   adjustment; dates after this cutoff are omitted.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured calendar or generated schedule is invalid.
    pub fn dates_through(&self, horizon: Date) -> Result<Vec<Date>> {
        match self {
            Self::Manual => Ok(Vec::new()),
            Self::Dates { dates } => Ok(dates
                .iter()
                .copied()
                .take_while(|date| *date <= horizon)
                .collect()),
            Self::Calendar {
                start,
                end,
                frequency,
                calendar_id,
                business_day_convention,
            } => {
                if *start > horizon {
                    return Ok(Vec::new());
                }
                let schedule_end = end.map_or(horizon, |end| end.min(horizon));
                let schedule = ScheduleBuilder::new(*start, schedule_end)?
                    .frequency(frequency.tenor()?)
                    .adjust_with_id(*business_day_convention, calendar_id)
                    .build()?;
                Ok(schedule
                    .dates
                    .into_iter()
                    .filter(|date| *date <= horizon)
                    .collect())
            }
        }
    }
}

/// Policy used to resolve signed leg quantities at initialization or rebalance.
///
/// Each variant consumes the signed `weight` on [`CompositeLegSpec`] as either
/// the quantity itself or a target score. Resolution happens only in
/// [`CompositeSpec::initialize`] / [`CompositeSpec::initialize_fixed`] or
/// [`CompositeInstrument::rebalance`].
///
/// # Formulas
///
/// Let `w_i` be the leg score, `G` a positive reporting-currency gross
/// notional, `N_i` the absolute unit notional, `m_i` the unit metric, `s_i`
/// the (optionally neutralized) score, and `σ_i` annualized unit-P&L
/// volatility. With an anchor quantity `q_a`:
///
/// ```text
/// FixedQuantity:     q_i = w_i
/// NotionalWeighted:  q_i = sign(w_i) · G · |w_i| / Σ|w| / N_i
/// MetricWeighted:    q_i = (s_i / s_a) · (q_a · m_a) / m_i
/// VolatilityWeighted:q_i = (w_i / w_a) · q_a · σ_a / σ_i
/// ```
///
/// Neutralization rescales positive scores to sum to `+1` and negative scores
/// to sum to `-1` before metric weighting. User-defined expressions replace
/// these closed forms and must return a finite non-zero quantity per leg.
///
/// # References
///
/// - DV01-neutral and duration-weighted curve trades:
///   `docs/REFERENCES.md#tuckman-serrat-fixed-income`
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WeightingMethod {
    /// Use each leg's signed `weight` directly as its resolved quantity.
    FixedQuantity,
    /// Allocate signed scores across a fixed gross reporting-currency notional.
    NotionalWeighted {
        /// Positive gross notional in the composite reporting currency.
        gross_notional: Money,
    },
    /// Target relative per-leg contributions to a valuation metric.
    MetricWeighted {
        /// Metric used as the weighting measure, such as `dv01` or `delta`.
        metric: MetricId,
        /// Leg whose resolved quantity fixes the overall scale.
        anchor_leg_id: InstrumentId,
        /// Non-zero signed quantity assigned to the anchor leg.
        anchor_quantity: f64,
        /// Whether positive and negative scores are normalized to `+1` and `-1`.
        neutralize: bool,
    },
    /// Inverse annualized unit-P&L volatility weighting.
    VolatilityWeighted {
        /// Leg whose resolved quantity fixes the overall scale.
        anchor_leg_id: InstrumentId,
        /// Non-zero signed quantity assigned to the anchor leg.
        anchor_quantity: f64,
        /// Maximum number of most-recent P&L observations used.
        lookback: usize,
        /// Minimum P&L observations required for every active leg.
        min_observations: usize,
        /// Positive periods-per-year factor used to annualize sample volatility.
        annualization_factor: f64,
    },
    /// Deterministic expression-defined quantities.
    UserDefined {
        /// Metrics populated into the expression context for every leg.
        required_metrics: Vec<MetricId>,
        /// One scalar expression per leg, keyed by instrument identifier.
        ///
        /// Available columns: `as_of_days`, `leg.{id}.weight`,
        /// `leg.{id}.value`, `leg.{id}.fx_rate`, optional `leg.{id}.notional`,
        /// `leg.{id}.metric.{metric}` for each required metric, and
        /// `leg.{id}.volatility` when history has at least three observations
        /// (annualized with `sqrt(252)`).
        quantity_expressions: IndexMap<String, Expr>,
    },
}

impl WeightingMethod {
    /// Construct parallel-DV01-neutral weighting.
    ///
    /// Equivalent to [`Self::MetricWeighted`] with `metric = dv01` and
    /// `neutralize = true`. Positive and negative score groups are normalized
    /// separately so a steepener or butterfly can split wing risk.
    ///
    /// # Arguments
    ///
    /// * `anchor_leg_id` - Existing leg whose signed quantity fixes overall
    ///   scale; must be present on the specification.
    /// * `anchor_quantity` - Finite non-zero signed quantity assigned to the
    ///   anchor; `q_a · m_a` must have the same sign as the anchor score.
    #[must_use]
    pub fn dv01_neutral(anchor_leg_id: impl Into<InstrumentId>, anchor_quantity: f64) -> Self {
        Self::MetricWeighted {
            metric: MetricId::Dv01,
            anchor_leg_id: anchor_leg_id.into(),
            anchor_quantity,
            neutralize: true,
        }
    }

    /// Construct curve-neutral weighting, defined as parallel-DV01 neutrality.
    ///
    /// Alias of [`Self::dv01_neutral`]. Use for 2s10s, butterflies, and other
    /// curve packages whose wing split follows signed scores.
    ///
    /// # Arguments
    ///
    /// * `anchor_leg_id` - Existing curve leg whose signed quantity fixes
    ///   overall scale.
    /// * `anchor_quantity` - Finite non-zero signed quantity assigned to the
    ///   anchor; sign must match the anchor score.
    #[must_use]
    pub fn curve_neutral(anchor_leg_id: impl Into<InstrumentId>, anchor_quantity: f64) -> Self {
        Self::dv01_neutral(anchor_leg_id, anchor_quantity)
    }

    /// Construct delta-neutral weighting.
    ///
    /// Equivalent to [`Self::MetricWeighted`] with `metric = delta` and
    /// `neutralize = true`.
    ///
    /// # Arguments
    ///
    /// * `anchor_leg_id` - Existing delta-bearing leg whose signed quantity
    ///   fixes overall scale.
    /// * `anchor_quantity` - Finite non-zero signed quantity assigned to the
    ///   anchor; `q_a · Δ_a` must have the same sign as the anchor score.
    #[must_use]
    pub fn delta_neutral(anchor_leg_id: impl Into<InstrumentId>, anchor_quantity: f64) -> Self {
        Self::MetricWeighted {
            metric: MetricId::Delta,
            anchor_leg_id: anchor_leg_id.into(),
            anchor_quantity,
            neutralize: true,
        }
    }

    /// Construct modified-duration weighting without neutrality normalization.
    ///
    /// Equivalent to [`Self::MetricWeighted`] with `metric = duration_mod` and
    /// `neutralize = false`. Scores are used as raw relative weights.
    ///
    /// # Arguments
    ///
    /// * `anchor_leg_id` - Existing duration-bearing leg whose signed quantity
    ///   fixes overall scale.
    /// * `anchor_quantity` - Finite non-zero signed quantity assigned to the
    ///   anchor; `q_a · D_a` must have the same sign as the anchor score.
    #[must_use]
    pub fn duration_weighted(anchor_leg_id: impl Into<InstrumentId>, anchor_quantity: f64) -> Self {
        Self::MetricWeighted {
            metric: MetricId::DurationMod,
            anchor_leg_id: anchor_leg_id.into(),
            anchor_quantity,
            neutralize: false,
        }
    }

    /// Construct inverse unit-P&L-volatility weighting.
    ///
    /// Unit P&L is value change plus signed cashflows on `(t_{k-1}, t_k]`,
    /// converted to the composite reporting currency. Sample volatility uses
    /// Bessel's correction (`n - 1`) and is annualized by
    /// `sqrt(annualization_factor)`. History supplied to `initialize` or
    /// `rebalance` must be strictly increasing and must end on the effective
    /// date.
    ///
    /// # Arguments
    ///
    /// * `anchor_leg_id` - Existing leg whose signed quantity fixes overall
    ///   scale.
    /// * `anchor_quantity` - Finite non-zero signed quantity assigned to the
    ///   anchor; sign must match the anchor score.
    /// * `lookback` - Maximum number of most-recent unit-P&L observations
    ///   retained; must be at least `min_observations`.
    /// * `min_observations` - Minimum finite P&L observations required for
    ///   every active leg; must satisfy `lookback >= min_observations >= 2`.
    /// * `annualization_factor` - Positive periods-per-year factor, such as
    ///   `252.0` for daily observations.
    #[must_use]
    pub fn volatility_weighted(
        anchor_leg_id: impl Into<InstrumentId>,
        anchor_quantity: f64,
        lookback: usize,
        min_observations: usize,
        annualization_factor: f64,
    ) -> Self {
        Self::VolatilityWeighted {
            anchor_leg_id: anchor_leg_id.into(),
            anchor_quantity,
            lookback,
            min_observations,
            annualization_factor,
        }
    }
}

/// Immutable resolved quantity for one top-level composite leg.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCompositeLeg {
    /// Identifier of the corresponding leg specification.
    pub instrument_id: InstrumentId,
    /// Signed quantity held from the state's effective date until the next rebalance.
    pub quantity: f64,
}

/// Immutable holdings state used by pricing, risk, scenarios, and history.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeState {
    /// Date from which the resolved quantities are effective.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub effective_date: Date,
    /// Resolved top-level quantities in specification order.
    pub resolved_legs: Vec<ResolvedCompositeLeg>,
    /// Finite scalar inputs retained for rebalance audit and reproducibility.
    pub weighting_inputs: IndexMap<String, f64>,
}

/// One dated, complete market snapshot used by dynamic weighting and history.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeMarketObservation {
    /// Observation date; must be strictly increasing within a supplied history.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub date: Date,
    /// Complete materialized market state for the observation date.
    pub state: MarketContextState,
}

impl CompositeMarketObservation {
    /// Capture a market context as a dated immutable observation.
    ///
    /// The snapshot stores the complete [`MarketContextState`]. Dynamic
    /// weighting and history restore it rather than mutating the live market.
    ///
    /// # Arguments
    ///
    /// * `date` - Observation date; sequences must be strictly increasing and
    ///   volatility history must end on the rebalance date.
    /// * `market` - Complete market context whose curves, prices, and FX
    ///   matrix are materialized for this date.
    #[must_use]
    pub fn new(date: Date, market: &MarketContext) -> Self {
        Self {
            date,
            state: MarketContextState::from(market),
        }
    }

    pub(crate) fn restore(&self) -> Result<MarketContext> {
        MarketContext::try_from(self.state.clone())
    }
}

/// Unresolved composite definition and rebalance policy.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeSpec {
    /// Stable composite instrument identifier.
    pub id: InstrumentId,
    /// Currency in which value, P&L, additive risk, and capital are reported.
    pub reporting_currency: Currency,
    /// Positive return denominator per composite unit.
    pub capital: Money,
    /// Self-contained underlying instrument definitions.
    pub legs: Vec<CompositeLegSpec>,
    /// Rule used to resolve quantities at initialization and rebalance.
    pub weighting_method: WeightingMethod,
    /// Dates on which explicit dynamic resolution becomes eligible.
    pub rebalance_rule: RebalanceRule,
    /// Scenario-selection and reporting metadata.
    pub attributes: Attributes,
    /// Instrument-level pricing inputs shared with the normal valuation lifecycle.
    #[serde(default, skip_serializing_if = "InstrumentPricingOverrides::is_empty")]
    pub instrument_pricing_overrides: InstrumentPricingOverrides,
    /// Metric bump and calculation controls.
    #[serde(default, skip_serializing_if = "MetricPricingOverrides::is_empty")]
    pub metric_pricing_overrides: MetricPricingOverrides,
    /// Scenario-only price adjustments applied to the composite after leg aggregation.
    #[serde(default, skip_serializing_if = "ScenarioPricingOverrides::is_empty")]
    pub scenario_pricing_overrides: ScenarioPricingOverrides,
}

impl CompositeSpec {
    /// Construct an unresolved composite specification.
    ///
    /// The constructor does not validate. Call [`Self::validate`] or
    /// [`Self::initialize`] / [`Self::initialize_fixed`] before pricing.
    ///
    /// # Arguments
    ///
    /// * `id` - Stable composite identifier used for pricing, serialization,
    ///   and primitive exposure paths.
    /// * `reporting_currency` - Currency for capital, value, additive risk,
    ///   cashflows, P&L, and period returns.
    /// * `capital` - Positive return denominator per composite unit; amount
    ///   must be finite and denominated in `reporting_currency`.
    /// * `legs` - At least two self-contained signed legs with unique
    ///   identifiers that match their embedded instruments.
    /// * `weighting_method` - Policy used only during initialization or
    ///   explicit rebalance; pricing never re-solves quantities.
    /// * `rebalance_rule` - Manual, explicit-date, or calendar-aware rule
    ///   that marks when a new state becomes eligible.
    #[must_use]
    pub fn new(
        id: impl Into<InstrumentId>,
        reporting_currency: Currency,
        capital: Money,
        legs: Vec<CompositeLegSpec>,
        weighting_method: WeightingMethod,
        rebalance_rule: RebalanceRule,
    ) -> Self {
        Self {
            id: id.into(),
            reporting_currency,
            capital,
            legs,
            weighting_method,
            rebalance_rule,
            attributes: Attributes::new(),
            instrument_pricing_overrides: InstrumentPricingOverrides::default(),
            metric_pricing_overrides: MetricPricingOverrides::default(),
            scenario_pricing_overrides: ScenarioPricingOverrides::default(),
        }
    }

    /// Replace scenario-selection attributes.
    ///
    /// # Arguments
    ///
    /// * `attributes` - Tags and metadata copied onto the unresolved
    ///   specification and retained on every resolved instrument.
    #[must_use]
    pub fn with_attributes(mut self, attributes: Attributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// Validate the complete embedded instrument tree and weighting policy.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid capital, leg counts, identifiers, embedded
    /// instruments, scores, anchors, expressions, schedules, or resource limits.
    pub fn validate(&self) -> Result<()> {
        let mut seen = BTreeMap::<String, String>::new();
        let mut count = 0usize;
        self.validate_tree(0, &mut count, &mut seen)
    }

    fn validate_tree(
        &self,
        depth: usize,
        count: &mut usize,
        seen: &mut BTreeMap<String, String>,
    ) -> Result<()> {
        if depth >= MAX_COMPOSITE_DEPTH {
            return Err(Error::Validation(format!(
                "composite '{}' exceeds maximum nesting depth {MAX_COMPOSITE_DEPTH}",
                self.id
            )));
        }
        if self.legs.len() < 2 {
            return Err(Error::Validation(format!(
                "composite '{}' requires at least two legs",
                self.id
            )));
        }
        if !self.capital.amount().is_finite()
            || self.capital.amount() <= 0.0
            || self.capital.currency() != self.reporting_currency
        {
            return Err(Error::Validation(format!(
                "composite '{}' capital must be positive and denominated in {}",
                self.id, self.reporting_currency
            )));
        }
        self.rebalance_rule.validate()?;
        let mut local_ids = BTreeMap::<String, ()>::new();
        for leg in &self.legs {
            *count += 1;
            if *count > MAX_COMPOSITE_LEGS {
                return Err(Error::Validation(format!(
                    "composite '{}' exceeds maximum total leg count {MAX_COMPOSITE_LEGS}",
                    self.id
                )));
            }
            if !leg.weight.is_finite() || leg.weight.abs() <= MIN_ABS_INPUT {
                return Err(Error::Validation(format!(
                    "composite leg '{}' weight must be finite and non-zero",
                    leg.instrument_id
                )));
            }
            if local_ids
                .insert(leg.instrument_id.to_string(), ())
                .is_some()
            {
                return Err(Error::Validation(format!(
                    "composite '{}' contains duplicate leg id '{}'",
                    self.id, leg.instrument_id
                )));
            }
            let hash = InstrumentEnvelope::new((*leg.instrument).clone()).content_hash()?;
            if let Some(existing) = seen.get(leg.instrument_id.as_str()) {
                if existing != &hash {
                    return Err(Error::Validation(format!(
                        "instrument id '{}' is reused with conflicting definitions",
                        leg.instrument_id
                    )));
                }
            } else {
                seen.insert(leg.instrument_id.to_string(), hash);
            }
            let boxed = leg.instrument.as_ref().clone().into_boxed()?;
            if boxed.id() != leg.instrument_id.as_str() {
                return Err(Error::Validation(format!(
                    "composite leg id '{}' does not match embedded instrument id '{}'",
                    leg.instrument_id,
                    boxed.id()
                )));
            }
            if let InstrumentJson::Composite(nested) = leg.instrument.as_ref() {
                nested.spec.validate_tree(depth + 1, count, seen)?;
                nested.validate_state()?;
            }
        }
        self.validate_weighting()
    }

    fn validate_weighting(&self) -> Result<()> {
        let validate_anchor = |anchor: &InstrumentId, quantity: f64| -> Result<()> {
            if !quantity.is_finite() || quantity.abs() <= MIN_ABS_INPUT {
                return Err(Error::Validation(
                    "composite anchor quantity must be finite and non-zero".to_string(),
                ));
            }
            if !self.legs.iter().any(|leg| leg.instrument_id == *anchor) {
                return Err(Error::Validation(format!(
                    "composite anchor leg '{}' is not present",
                    anchor
                )));
            }
            Ok(())
        };
        match &self.weighting_method {
            WeightingMethod::FixedQuantity => Ok(()),
            WeightingMethod::NotionalWeighted { gross_notional } => {
                if !gross_notional.amount().is_finite()
                    || gross_notional.amount() <= 0.0
                    || gross_notional.currency() != self.reporting_currency
                {
                    return Err(Error::Validation(format!(
                        "gross notional must be positive and denominated in {}",
                        self.reporting_currency
                    )));
                }
                Ok(())
            }
            WeightingMethod::MetricWeighted {
                anchor_leg_id,
                anchor_quantity,
                neutralize,
                ..
            } => {
                validate_anchor(anchor_leg_id, *anchor_quantity)?;
                if *neutralize {
                    let positive = self.legs.iter().any(|leg| leg.weight > 0.0);
                    let negative = self.legs.iter().any(|leg| leg.weight < 0.0);
                    if !(positive && negative) {
                        return Err(Error::Validation(
                            "neutral composite weighting requires both positive and negative scores"
                                .to_string(),
                        ));
                    }
                }
                Ok(())
            }
            WeightingMethod::VolatilityWeighted {
                anchor_leg_id,
                anchor_quantity,
                lookback,
                min_observations,
                annualization_factor,
            } => {
                validate_anchor(anchor_leg_id, *anchor_quantity)?;
                if *lookback == 0
                    || *min_observations < 2
                    || *min_observations > *lookback
                    || !annualization_factor.is_finite()
                    || *annualization_factor <= 0.0
                {
                    return Err(Error::Validation(
                        "volatility weighting requires lookback >= min_observations >= 2 and a positive annualization factor"
                            .to_string(),
                    ));
                }
                Ok(())
            }
            WeightingMethod::UserDefined {
                quantity_expressions,
                ..
            } => {
                if quantity_expressions.len() != self.legs.len() {
                    return Err(Error::Validation(
                        "user-defined weighting requires exactly one expression per leg"
                            .to_string(),
                    ));
                }
                for leg in &self.legs {
                    let expression = quantity_expressions
                        .get(leg.instrument_id.as_str())
                        .ok_or_else(|| {
                            Error::Validation(format!(
                                "missing quantity expression for leg '{}'",
                                leg.instrument_id
                            ))
                        })?;
                    let _ = CompiledExpr::try_new_scalar(expression.clone())?;
                }
                Ok(())
            }
        }
    }

    /// Resolve a fixed-quantity specification without consulting market data.
    ///
    /// Each leg's `weight` becomes its frozen quantity. Dynamic weighting
    /// methods must use [`Self::initialize`] instead.
    ///
    /// Python and WASM expose only [`Self::initialize`]; that path also
    /// resolves [`WeightingMethod::FixedQuantity`] and does not require
    /// historical observations.
    ///
    /// # Arguments
    ///
    /// * `effective_date` - Date from which the fixed quantities are held
    ///   until the next explicit rebalance.
    ///
    /// # Errors
    ///
    /// Returns an error when the specification is invalid or uses a dynamic method.
    pub fn initialize_fixed(&self, effective_date: Date) -> Result<CompositeRebalanceResult> {
        self.validate()?;
        if !matches!(self.weighting_method, WeightingMethod::FixedQuantity) {
            return Err(Error::Validation(
                "initialize_fixed is available only for fixed-quantity composites".to_string(),
            ));
        }
        let quantities: Vec<f64> = self.legs.iter().map(|leg| leg.weight).collect();
        self.build_rebalance_result(effective_date, quantities, IndexMap::new(), None)
    }

    /// Resolve quantities using current and, when required, historical market data.
    ///
    /// Volatility weighting requires `history` to be strictly increasing and
    /// to end on `as_of`. User-defined expressions populate
    /// `leg.{id}.volatility` only when `history` has at least three
    /// observations (two unit-P&L increments), annualized with `sqrt(252)`.
    ///
    /// # Arguments
    ///
    /// * `market` - Complete current market used for unit values, additive
    ///   metrics, notionals, and reporting-currency FX conversion.
    /// * `as_of` - Effective date of the new immutable holdings state; no
    ///   later history observation is permitted.
    /// * `history` - Strictly increasing dated snapshots available through
    ///   `as_of`. Required for volatility weighting; optional otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input, missing market data, unsupported
    /// weighting measures, insufficient history, or non-finite resolved quantities.
    pub fn initialize(
        &self,
        market: &MarketContext,
        as_of: Date,
        history: &[CompositeMarketObservation],
    ) -> Result<CompositeRebalanceResult> {
        self.validate()?;
        validate_history(history, Some(as_of))?;
        let (quantities, inputs) = self.resolve_quantities(market, as_of, history)?;
        self.build_rebalance_result(as_of, quantities, inputs, None)
    }

    fn build_rebalance_result(
        &self,
        effective_date: Date,
        quantities: Vec<f64>,
        weighting_inputs: IndexMap<String, f64>,
        previous: Option<&CompositeInstrument>,
    ) -> Result<CompositeRebalanceResult> {
        if quantities.len() != self.legs.len() {
            return Err(Error::Internal(
                "composite quantity resolver returned the wrong leg count".to_string(),
            ));
        }
        let resolved_legs: Vec<ResolvedCompositeLeg> = self
            .legs
            .iter()
            .zip(quantities)
            .map(|(leg, quantity)| ResolvedCompositeLeg {
                instrument_id: leg.instrument_id.clone(),
                quantity,
            })
            .collect();
        if resolved_legs
            .iter()
            .any(|leg| !leg.quantity.is_finite() || leg.quantity.abs() <= MIN_ABS_INPUT)
        {
            return Err(Error::Validation(
                "composite resolved quantities must be finite and non-zero".to_string(),
            ));
        }
        let instrument = CompositeInstrument {
            spec: self.clone(),
            state: CompositeState {
                effective_date,
                resolved_legs,
                weighting_inputs,
            },
            boxed_legs: BoxedLegCache::default(),
        };
        instrument.validate_state()?;
        let trades = instrument.execution_trades(previous)?;
        Ok(CompositeRebalanceResult { instrument, trades })
    }

    fn resolve_quantities(
        &self,
        market: &MarketContext,
        as_of: Date,
        history: &[CompositeMarketObservation],
    ) -> Result<(Vec<f64>, IndexMap<String, f64>)> {
        match &self.weighting_method {
            WeightingMethod::FixedQuantity => Ok((
                self.legs.iter().map(|leg| leg.weight).collect(),
                IndexMap::new(),
            )),
            WeightingMethod::NotionalWeighted { gross_notional } => {
                self.resolve_notional_weighted(market, as_of, *gross_notional)
            }
            WeightingMethod::MetricWeighted {
                metric,
                anchor_leg_id,
                anchor_quantity,
                neutralize,
            } => self.resolve_metric_weighted(
                market,
                as_of,
                metric,
                anchor_leg_id,
                *anchor_quantity,
                *neutralize,
            ),
            WeightingMethod::VolatilityWeighted {
                anchor_leg_id,
                anchor_quantity,
                lookback,
                min_observations,
                annualization_factor,
            } => self.resolve_volatility_weighted(
                as_of,
                history,
                VolatilityWeightingConfig {
                    anchor_leg_id,
                    anchor_quantity: *anchor_quantity,
                    lookback: *lookback,
                    min_observations: *min_observations,
                    annualization_factor: *annualization_factor,
                },
            ),
            WeightingMethod::UserDefined {
                required_metrics,
                quantity_expressions,
            } => self.resolve_user_defined(
                market,
                as_of,
                history,
                required_metrics,
                quantity_expressions,
            ),
        }
    }

    fn resolve_notional_weighted(
        &self,
        market: &MarketContext,
        as_of: Date,
        gross_notional: Money,
    ) -> Result<(Vec<f64>, IndexMap<String, f64>)> {
        let score_total = self.legs.iter().map(|leg| leg.weight.abs()).sum::<f64>();
        if !score_total.is_finite() || score_total <= MIN_ABS_INPUT {
            return Err(Error::Validation(
                "composite absolute score total must be finite and non-zero".to_string(),
            ));
        }
        let mut notionals = Vec::with_capacity(self.legs.len());
        let mut inputs = IndexMap::new();
        for leg in &self.legs {
            let instrument = leg.instrument.as_ref().clone().into_boxed()?;
            let notional = instrument.notional().ok_or_else(|| {
                Error::Validation(format!(
                    "instrument '{}' does not expose a weighting notional",
                    leg.instrument_id
                ))
            })?;
            let converted = convert_amount(
                market,
                notional.amount(),
                notional.currency(),
                self.reporting_currency,
                as_of,
            )?
            .abs();
            if !converted.is_finite() || converted <= MIN_ABS_INPUT {
                return Err(Error::Validation(format!(
                    "instrument '{}' has zero or non-finite weighting notional",
                    leg.instrument_id
                )));
            }
            inputs.insert(format!("leg.{}.notional", leg.instrument_id), converted);
            notionals.push(converted);
        }
        let quantities = self
            .legs
            .iter()
            .zip(notionals)
            .map(|(leg, notional)| {
                leg.weight.signum() * gross_notional.amount() * leg.weight.abs()
                    / score_total
                    / notional
            })
            .collect();
        Ok((quantities, inputs))
    }

    fn resolve_metric_weighted(
        &self,
        market: &MarketContext,
        as_of: Date,
        metric: &MetricId,
        anchor_leg_id: &InstrumentId,
        anchor_quantity: f64,
        neutralize: bool,
    ) -> Result<(Vec<f64>, IndexMap<String, f64>)> {
        let scores = normalized_scores(&self.legs, neutralize)?;
        let mut measures = Vec::with_capacity(self.legs.len());
        let mut inputs = IndexMap::new();
        for leg in &self.legs {
            let instrument = leg.instrument.as_ref().clone().into_boxed()?;
            let result = instrument.price_with_metrics(
                market,
                as_of,
                std::slice::from_ref(metric),
                PricingOptions::default(),
            )?;
            let mut value = result.measures.get(metric).copied().ok_or_else(|| {
                Error::Validation(format!(
                    "metric '{}' is not available for composite leg '{}'",
                    metric, leg.instrument_id
                ))
            })?;
            if is_additive_metric(metric) {
                value = convert_amount(
                    market,
                    value,
                    result.value.currency(),
                    self.reporting_currency,
                    as_of,
                )?;
            }
            if !value.is_finite() || value.abs() <= MIN_ABS_INPUT {
                return Err(Error::Validation(format!(
                    "metric '{}' is zero or non-finite for composite leg '{}'",
                    metric, leg.instrument_id
                )));
            }
            inputs.insert(
                format!("leg.{}.metric.{}", leg.instrument_id, metric),
                value,
            );
            measures.push(value);
        }
        let anchor_index = leg_index(&self.legs, anchor_leg_id)?;
        let anchor_score = scores[anchor_index];
        let anchor_measure = measures[anchor_index];
        if (anchor_quantity * anchor_measure).signum() != anchor_score.signum() {
            return Err(Error::Validation(
                "anchor quantity and unit metric must have the anchor score's sign".to_string(),
            ));
        }
        let anchor_contribution = anchor_quantity * anchor_measure;
        let quantities = scores
            .iter()
            .zip(measures)
            .map(|(score, measure)| (score / anchor_score) * anchor_contribution / measure)
            .collect();
        Ok((quantities, inputs))
    }

    fn resolve_volatility_weighted(
        &self,
        as_of: Date,
        history: &[CompositeMarketObservation],
        config: VolatilityWeightingConfig<'_>,
    ) -> Result<(Vec<f64>, IndexMap<String, f64>)> {
        if history
            .last()
            .is_none_or(|observation| observation.date != as_of)
        {
            return Err(Error::Validation(
                "volatility weighting history must end on the rebalance date".to_string(),
            ));
        }
        let mut volatilities = Vec::with_capacity(self.legs.len());
        let mut inputs = IndexMap::new();
        for leg in &self.legs {
            let pnl = unit_pnl_series(leg.instrument.as_ref(), self.reporting_currency, history)?;
            let start = pnl.len().saturating_sub(config.lookback);
            let sample = &pnl[start..];
            if sample.len() < config.min_observations {
                return Err(Error::Validation(format!(
                    "leg '{}' has {} P&L observations; {} required",
                    leg.instrument_id,
                    sample.len(),
                    config.min_observations
                )));
            }
            let volatility = sample_std_dev(sample)? * config.annualization_factor.sqrt();
            if !volatility.is_finite() || volatility <= MIN_ABS_INPUT {
                return Err(Error::Validation(format!(
                    "leg '{}' has zero or non-finite unit-P&L volatility",
                    leg.instrument_id
                )));
            }
            inputs.insert(format!("leg.{}.volatility", leg.instrument_id), volatility);
            volatilities.push(volatility);
        }
        let anchor_index = leg_index(&self.legs, config.anchor_leg_id)?;
        let anchor_score = self.legs[anchor_index].weight;
        if config.anchor_quantity.signum() != anchor_score.signum() {
            return Err(Error::Validation(
                "anchor quantity must have the anchor score's sign".to_string(),
            ));
        }
        let anchor_volatility = volatilities[anchor_index];
        let quantities = self
            .legs
            .iter()
            .zip(volatilities)
            .map(|(leg, volatility)| {
                (leg.weight / anchor_score) * config.anchor_quantity * anchor_volatility
                    / volatility
            })
            .collect();
        Ok((quantities, inputs))
    }

    fn resolve_user_defined(
        &self,
        market: &MarketContext,
        as_of: Date,
        history: &[CompositeMarketObservation],
        required_metrics: &[MetricId],
        quantity_expressions: &IndexMap<String, Expr>,
    ) -> Result<(Vec<f64>, IndexMap<String, f64>)> {
        let mut columns = IndexMap::<String, f64>::new();
        columns.insert("as_of_days".to_string(), f64::from(as_of.to_julian_day()));
        for leg in &self.legs {
            let instrument = leg.instrument.as_ref().clone().into_boxed()?;
            let result = instrument.price_with_metrics(
                market,
                as_of,
                required_metrics,
                PricingOptions::default(),
            )?;
            let value = convert_amount(
                market,
                result.value.amount(),
                result.value.currency(),
                self.reporting_currency,
                as_of,
            )?;
            let fx_rate = convert_amount(
                market,
                1.0,
                result.value.currency(),
                self.reporting_currency,
                as_of,
            )?;
            columns.insert(format!("leg.{}.weight", leg.instrument_id), leg.weight);
            columns.insert(format!("leg.{}.value", leg.instrument_id), value);
            columns.insert(format!("leg.{}.fx_rate", leg.instrument_id), fx_rate);
            if let Some(notional) = instrument.notional() {
                let converted = convert_amount(
                    market,
                    notional.amount(),
                    notional.currency(),
                    self.reporting_currency,
                    as_of,
                )?;
                columns.insert(format!("leg.{}.notional", leg.instrument_id), converted);
            }
            for metric in required_metrics {
                let mut value = result.measures.get(metric).copied().ok_or_else(|| {
                    Error::Validation(format!(
                        "required metric '{}' is unavailable for leg '{}'",
                        metric, leg.instrument_id
                    ))
                })?;
                if is_additive_metric(metric) {
                    value *= fx_rate;
                }
                columns.insert(
                    format!("leg.{}.metric.{}", leg.instrument_id, metric),
                    value,
                );
            }
            if history.len() >= 3 {
                let pnl =
                    unit_pnl_series(leg.instrument.as_ref(), self.reporting_currency, history)?;
                let volatility = sample_std_dev(&pnl)? * 252.0_f64.sqrt();
                columns.insert(format!("leg.{}.volatility", leg.instrument_id), volatility);
            }
        }
        let names: Vec<String> = columns.keys().cloned().collect();
        let data: Vec<Vec<f64>> = columns.values().map(|value| vec![*value]).collect();
        let refs: Vec<&[f64]> = data.iter().map(Vec::as_slice).collect();
        let context = SimpleContext::new(names)?;
        let mut quantities = Vec::with_capacity(self.legs.len());
        for leg in &self.legs {
            let expression = quantity_expressions
                .get(leg.instrument_id.as_str())
                .ok_or_else(|| {
                    Error::Validation(format!(
                        "missing quantity expression for leg '{}'",
                        leg.instrument_id
                    ))
                })?;
            let compiled = CompiledExpr::try_new_scalar(expression.clone())?;
            let evaluated = compiled.eval(&context, &refs, EvalOpts::default())?;
            let quantity = evaluated.values.first().copied().ok_or_else(|| {
                Error::Validation(format!(
                    "quantity expression for leg '{}' returned no value",
                    leg.instrument_id
                ))
            })?;
            quantities.push(quantity);
        }
        Ok((quantities, columns))
    }
}

/// Priceable composite instrument containing an unresolved policy and immutable state.
///
/// Valuation and risk use `state` exactly as stored. Call [`Self::rebalance`]
/// to obtain a distinct instrument; this type does not mutate in place.
///
/// # Examples
///
/// ```
/// use finstack_quant_valuations::instruments::{CompositeInstrument, Instrument};
///
/// let composite = CompositeInstrument::example()?;
/// assert_eq!(composite.id(), "COMPOSITE-EXAMPLE");
/// assert_eq!(composite.state.resolved_legs.len(), 2);
/// # Ok::<(), finstack_quant_core::Error>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeInstrument {
    /// Economic definition and future rebalance policy.
    pub spec: CompositeSpec,
    /// Frozen quantities used for every valuation until explicit rebalance.
    pub state: CompositeState,
    /// Boxed legs materialized once per instance.
    #[serde(skip)]
    #[schemars(skip)]
    boxed_legs: BoxedLegCache,
}

impl CompositeInstrument {
    /// Create and validate a resolved composite instrument.
    ///
    /// # Arguments
    ///
    /// * `spec` - Self-contained composite definition, including future
    ///   rebalance policy.
    /// * `state` - Immutable resolved quantities, effective date, and finite
    ///   weighting-audit inputs; leg order must match `spec.legs`.
    ///
    /// # Errors
    ///
    /// Returns an error when the specification or state is inconsistent.
    pub fn new(spec: CompositeSpec, state: CompositeState) -> Result<Self> {
        let instrument = Self {
            spec,
            state,
            boxed_legs: BoxedLegCache::default(),
        };
        instrument.validate_invariants()?;
        Ok(instrument)
    }

    fn boxed_legs(&self) -> Result<&[Box<dyn Instrument>]> {
        if let Some(legs) = self.boxed_legs.0.get() {
            return Ok(legs.as_slice());
        }
        let legs = self
            .spec
            .legs
            .iter()
            .map(|leg| leg.instrument.as_ref().clone().into_boxed())
            .collect::<Result<Vec<_>>>()?;
        let _ = self.boxed_legs.0.set(legs);
        self.boxed_legs.0.get().map(Vec::as_slice).ok_or_else(|| {
            Error::Internal("boxed composite legs cache missing after initialization".into())
        })
    }

    /// Return a canonical fixed-quantity long/short equity example.
    ///
    /// Long `COMPOSITE-LONG` at 100 and short `COMPOSITE-SHORT` at 90, each
    /// with quantity `±1`, effective `2025-01-01`, and USD 100 capital.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated example fails validation.
    pub fn example() -> Result<Self> {
        let long = crate::instruments::Equity::new("COMPOSITE-LONG", "LONG", Currency::USD)
            .with_shares(1.0)
            .with_price(100.0);
        let short = crate::instruments::Equity::new("COMPOSITE-SHORT", "SHORT", Currency::USD)
            .with_shares(1.0)
            .with_price(90.0);
        let spec = CompositeSpec::new(
            "COMPOSITE-EXAMPLE",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                CompositeLegSpec::new("COMPOSITE-LONG", InstrumentJson::Equity(long), 1.0),
                CompositeLegSpec::new("COMPOSITE-SHORT", InstrumentJson::Equity(short), -1.0),
            ],
            WeightingMethod::FixedQuantity,
            RebalanceRule::Manual,
        );
        Ok(spec
            .initialize_fixed(time::macros::date!(2025 - 01 - 01))?
            .instrument)
    }

    fn validate_state(&self) -> Result<()> {
        if self.state.resolved_legs.len() != self.spec.legs.len() {
            return Err(Error::Validation(format!(
                "composite '{}' state leg count does not match its specification",
                self.spec.id
            )));
        }
        for (spec, state) in self.spec.legs.iter().zip(&self.state.resolved_legs) {
            if spec.instrument_id != state.instrument_id {
                return Err(Error::Validation(format!(
                    "composite '{}' state leg order or identifier mismatch",
                    self.spec.id
                )));
            }
            if !state.quantity.is_finite() || state.quantity.abs() <= MIN_ABS_INPUT {
                return Err(Error::Validation(format!(
                    "composite state quantity for '{}' must be finite and non-zero",
                    state.instrument_id
                )));
            }
        }
        if self
            .state
            .weighting_inputs
            .values()
            .any(|value| !value.is_finite())
        {
            return Err(Error::Validation(
                "composite state weighting inputs must be finite".to_string(),
            ));
        }
        Ok(())
    }

    /// Explicitly resolve a new immutable state and primitive execution deltas.
    ///
    /// The receiver is not mutated. Trades are net primitive quantity deltas
    /// from this state to the newly resolved state.
    ///
    /// # Arguments
    ///
    /// * `market` - Complete current market used to resolve dynamic quantities
    ///   and convert notionals, metrics, and FX into `reporting_currency`.
    /// * `as_of` - Effective date for the distinct returned state; no later
    ///   history observation is permitted.
    /// * `history` - Strictly increasing observations available through
    ///   `as_of`. Required for volatility weighting and must end on `as_of`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid market/history inputs or weighting failures.
    pub fn rebalance(
        &self,
        market: &MarketContext,
        as_of: Date,
        history: &[CompositeMarketObservation],
    ) -> Result<CompositeRebalanceResult> {
        self.spec.validate()?;
        validate_history(history, Some(as_of))?;
        let (quantities, inputs) = self.spec.resolve_quantities(market, as_of, history)?;
        self.spec
            .build_rebalance_result(as_of, quantities, inputs, Some(self))
    }

    /// Return frozen top-level quantities as execution deltas from an optional prior state.
    ///
    /// Nested composites are flattened first. Deltas smaller than `1e-12` in
    /// absolute value are omitted. A reused primitive identifier must carry
    /// the same definition in both states.
    ///
    /// # Arguments
    ///
    /// * `previous` - Prior resolved state used as the trade baseline, or
    ///   `None` to treat every current primitive quantity as an establishment
    ///   trade.
    ///
    /// # Errors
    ///
    /// Returns an error when either state is invalid or primitive definitions conflict.
    pub fn execution_trades(
        &self,
        previous: Option<&CompositeInstrument>,
    ) -> Result<Vec<CompositeTrade>> {
        let current = self.flatten_primitives()?;
        let prior = match previous {
            Some(previous) => previous.flatten_primitives()?,
            None => Vec::new(),
        };
        let mut deltas = BTreeMap::<String, (String, f64)>::new();
        let mut definitions = BTreeMap::<String, String>::new();
        for exposure in prior {
            register_execution_definition(&mut definitions, &exposure)?;
            let entry = deltas
                .entry(exposure.instrument_id.to_string())
                .or_insert((exposure.instrument_type, 0.0));
            entry.1 -= exposure.quantity;
        }
        for exposure in current {
            register_execution_definition(&mut definitions, &exposure)?;
            let entry = deltas
                .entry(exposure.instrument_id.to_string())
                .or_insert((exposure.instrument_type, 0.0));
            entry.1 += exposure.quantity;
        }
        Ok(deltas
            .into_iter()
            .filter_map(|(instrument_id, (instrument_type, quantity_delta))| {
                (quantity_delta.abs() > MIN_ABS_INPUT).then(|| CompositeTrade {
                    instrument_id: InstrumentId::new(instrument_id),
                    instrument_type,
                    quantity_delta,
                })
            })
            .collect())
    }

    /// Recursively flatten the frozen state into path-level primitive quantities.
    ///
    /// # Errors
    ///
    /// Returns an error when the composite tree is invalid or exceeds resource limits.
    pub fn flatten_primitives(&self) -> Result<Vec<PrimitiveExposure>> {
        self.validate_invariants()?;
        let mut out = Vec::new();
        let mut path = vec![self.spec.id.to_string()];
        flatten_composite(self, 1.0, &mut path, &mut out)?;
        Ok(out)
    }

    /// Price path-level primitives and return net and gross aggregates.
    ///
    /// Only additive metrics can be requested because a top-level value for
    /// yield, duration, implied volatility, or another non-linear measure would
    /// conceal instrument-specific meaning. Path values and additive measures
    /// are converted to `reporting_currency` on `as_of`.
    ///
    /// # Arguments
    ///
    /// * `market` - Complete market used to price every primitive and convert
    ///   native amounts into `reporting_currency`.
    /// * `as_of` - Valuation date and FX conversion date.
    /// * `metrics` - Additive risk metrics to aggregate; an empty slice
    ///   reports value only. Non-additive identifiers are rejected.
    ///
    /// # Errors
    ///
    /// Returns an error for non-additive metrics, invalid embedded instruments,
    /// missing market data, or a metric unsupported by every primitive.
    pub fn primitive_exposure_report(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<CompositeExposureReport> {
        self.primitive_exposure_report_with_options(
            market,
            as_of,
            metrics,
            PricingOptions::default(),
        )
    }

    pub(crate) fn primitive_exposure_report_with_options(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: PricingOptions,
    ) -> Result<CompositeExposureReport> {
        if let Some(metric) = metrics.iter().find(|metric| !is_additive_metric(metric)) {
            return Err(Error::Validation(format!(
                "metric '{}' is not additive and cannot be aggregated for composite '{}'",
                metric, self.spec.id
            )));
        }
        let flattened = self.flatten_primitives()?;
        let mut paths = Vec::with_capacity(flattened.len());
        let mut supported = BTreeMap::<String, usize>::new();
        let mut price_cache = BTreeMap::<String, crate::results::ValuationResult>::new();
        for mut exposure in flattened {
            let instrument_json = exposure.instrument.as_ref().ok_or_else(|| {
                Error::Internal("primitive exposure lost its runtime instrument".to_string())
            })?;
            let cache_key = InstrumentEnvelope::new(instrument_json.clone()).content_hash()?;
            let result = match price_cache.get(&cache_key) {
                Some(cached) => cached.clone(),
                None => {
                    let instrument = instrument_json.clone().into_boxed()?;
                    let priced =
                        instrument.price_with_metrics(market, as_of, metrics, options.clone())?;
                    price_cache.insert(cache_key, priced.clone());
                    priced
                }
            };
            let unit_value = convert_amount(
                market,
                result.value.amount(),
                result.value.currency(),
                self.spec.reporting_currency,
                as_of,
            )?;
            exposure.value =
                Money::new(unit_value * exposure.quantity, self.spec.reporting_currency);
            for (metric_id, value) in result.measures {
                if !is_additive_metric(&metric_id) {
                    continue;
                }
                let converted = convert_amount(
                    market,
                    value,
                    result.value.currency(),
                    self.spec.reporting_currency,
                    as_of,
                )?;
                exposure
                    .measures
                    .insert(metric_id.clone(), converted * exposure.quantity);
                *supported.entry(metric_id.to_string()).or_default() += 1;
            }
            paths.push(exposure);
        }
        for metric in metrics {
            let is_supported = supported.keys().any(|key| {
                key == metric.as_str()
                    || key
                        .strip_prefix(metric.as_str())
                        .is_some_and(|suffix| suffix.starts_with("::"))
            });
            if !is_supported {
                return Err(Error::Validation(format!(
                    "metric '{}' is unsupported by every primitive in composite '{}'",
                    metric, self.spec.id
                )));
            }
        }
        Ok(aggregate_primitive_paths(
            self.spec.reporting_currency,
            paths,
        ))
    }

    pub(crate) fn valuation_details_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: PricingOptions,
    ) -> Result<(IndexMap<MetricId, f64>, CompositeValuationDetails)> {
        let report =
            self.primitive_exposure_report_with_options(market, as_of, metrics, options.clone())?;
        let leg_results = self.top_level_leg_results(market, as_of, metrics, options)?;
        let mut measures = IndexMap::<MetricId, f64>::new();
        for path in &report.paths {
            for (metric, value) in &path.measures {
                *measures.entry(metric.clone()).or_default() += *value;
            }
        }
        Ok((
            measures,
            CompositeValuationDetails {
                state_effective_date: self.state.effective_date,
                reporting_currency: self.spec.reporting_currency,
                resolved_legs: self.state.resolved_legs.clone(),
                weighting_inputs: self.state.weighting_inputs.clone(),
                leg_results,
                exposure_report: report,
            },
        ))
    }

    fn top_level_leg_results(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: PricingOptions,
    ) -> Result<Vec<CompositeLegValuation>> {
        self.boxed_legs()?
            .iter()
            .zip(&self.spec.legs)
            .zip(&self.state.resolved_legs)
            .map(|((instrument, leg), resolved)| {
                let valuation =
                    instrument.price_with_metrics(market, as_of, metrics, options.clone())?;
                let native_value = Money::new(
                    valuation.value.amount() * resolved.quantity,
                    valuation.value.currency(),
                );
                let reporting_value = Money::new(
                    convert_amount(
                        market,
                        native_value.amount(),
                        native_value.currency(),
                        self.spec.reporting_currency,
                        as_of,
                    )?,
                    self.spec.reporting_currency,
                );
                Ok(CompositeLegValuation {
                    instrument_id: leg.instrument_id.clone(),
                    quantity: resolved.quantity,
                    native_value,
                    reporting_value,
                    valuation,
                })
            })
            .collect()
    }

    fn base_value_raw_impl(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        let values = self
            .boxed_legs()?
            .iter()
            .zip(&self.state.resolved_legs)
            .map(|(instrument, resolved)| {
                let (amount, currency) = instrument.value_raw_with_currency(market, as_of)?;
                let converted = convert_amount(
                    market,
                    amount,
                    currency,
                    self.spec.reporting_currency,
                    as_of,
                )?;
                Ok(converted * resolved.quantity)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(neumaier_sum(values))
    }
}

impl Instrument for CompositeInstrument {
    fn id(&self) -> &str {
        self.spec.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Composite
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.spec.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.spec.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn validate_invariants(&self) -> Result<()> {
        self.spec.validate()?;
        self.validate_state()
    }

    fn base_value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        Ok(Money::new(
            self.base_value_raw_impl(market, as_of)?,
            self.spec.reporting_currency,
        ))
    }

    fn base_value_raw(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        self.base_value_raw_impl(market, as_of)
    }

    fn base_value_raw_with_currency(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, Currency)> {
        Ok((
            self.base_value_raw_impl(market, as_of)?,
            self.spec.reporting_currency,
        ))
    }

    fn market_dependencies(&self) -> Result<MarketDependencies> {
        let mut dependencies = MarketDependencies::new();
        for (leg, instrument) in self.spec.legs.iter().zip(self.boxed_legs()?) {
            dependencies.merge(MarketDependencies::from_instrument_json(
                leg.instrument.as_ref(),
            )?);
            if let Some(notional) = instrument.notional() {
                if notional.currency() != self.spec.reporting_currency {
                    dependencies.add_fx_pair(notional.currency(), self.spec.reporting_currency);
                }
            }
        }
        Ok(dependencies)
    }

    fn valuation_details(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> Option<crate::results::ValuationDetails> {
        self.primitive_exposure_report(market, as_of, &[])
            .ok()
            .and_then(|exposure_report| {
                let leg_results = self
                    .top_level_leg_results(market, as_of, &[], PricingOptions::default())
                    .ok()?;
                Some(crate::results::ValuationDetails::Composite(
                    CompositeValuationDetails {
                        state_effective_date: self.state.effective_date,
                        reporting_currency: self.spec.reporting_currency,
                        resolved_legs: self.state.resolved_legs.clone(),
                        weighting_inputs: self.state.weighting_inputs.clone(),
                        leg_results,
                        exposure_report,
                    },
                ))
            })
    }

    fn get_instrument_pricing_overrides(&self) -> Option<&InstrumentPricingOverrides> {
        Some(&self.spec.instrument_pricing_overrides)
    }

    fn get_instrument_pricing_overrides_mut(&mut self) -> Option<&mut InstrumentPricingOverrides> {
        Some(&mut self.spec.instrument_pricing_overrides)
    }

    fn get_metric_pricing_overrides(&self) -> Option<&MetricPricingOverrides> {
        Some(&self.spec.metric_pricing_overrides)
    }

    fn get_metric_pricing_overrides_mut(&mut self) -> Option<&mut MetricPricingOverrides> {
        Some(&mut self.spec.metric_pricing_overrides)
    }

    fn get_scenario_pricing_overrides(&self) -> Option<&ScenarioPricingOverrides> {
        Some(&self.spec.scenario_pricing_overrides)
    }

    fn get_scenario_pricing_overrides_mut(&mut self) -> Option<&mut ScenarioPricingOverrides> {
        Some(&mut self.spec.scenario_pricing_overrides)
    }
}

crate::impl_empty_cashflow_provider!(
    CompositeInstrument,
    crate::cashflow::builder::CashflowRepresentation::Placeholder
);

/// Primitive execution delta produced by initialization or rebalance.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeTrade {
    /// Primitive instrument identifier.
    pub instrument_id: InstrumentId,
    /// Canonical primitive instrument type discriminator.
    pub instrument_type: String,
    /// Signed change in primitive quantity.
    pub quantity_delta: f64,
}

/// Result of resolving a new composite holdings state.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeRebalanceResult {
    /// Newly resolved, immutable, priceable composite.
    pub instrument: CompositeInstrument,
    /// Net primitive execution deltas required to reach the new state.
    pub trades: Vec<CompositeTrade>,
}

/// Path-level primitive exposure in a resolved composite tree.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveExposure {
    /// Composite and leg identifiers from the root to the primitive.
    pub path: Vec<String>,
    /// Primitive instrument identifier.
    pub instrument_id: InstrumentId,
    /// Canonical primitive instrument type discriminator.
    pub instrument_type: String,
    /// Signed primitive quantity after multiplying every nested state quantity.
    pub quantity: f64,
    /// Reporting-currency signed value for this path.
    pub value: Money,
    /// Reporting-currency additive risk measures for this path.
    pub measures: IndexMap<MetricId, f64>,
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) instrument: Option<InstrumentJson>,
}

impl PrimitiveExposure {
    /// Borrow the canonical primitive definition retained for recursive consumers.
    ///
    /// Runtime definitions are intentionally omitted from serialized exposure
    /// reports; callers use this accessor only while processing a live report.
    ///
    /// # Returns
    ///
    /// The embedded primitive definition when this path was created by a live
    /// composite decomposition.
    #[must_use]
    pub fn instrument_definition(&self) -> Option<&InstrumentJson> {
        self.instrument.as_ref()
    }
}

/// Net and gross exposure aggregated by primitive instrument identifier.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveAggregate {
    /// Primitive instrument identifier.
    pub instrument_id: InstrumentId,
    /// Canonical primitive instrument type discriminator.
    pub instrument_type: String,
    /// Algebraic sum of path quantities.
    pub net_quantity: f64,
    /// Sum of absolute path quantities.
    pub gross_quantity: f64,
    /// Algebraic reporting-currency value.
    pub net_value: Money,
    /// Sum of absolute reporting-currency path values.
    pub gross_value: Money,
    /// Algebraic additive risk by metric.
    pub net_measures: IndexMap<MetricId, f64>,
    /// Sum of absolute path risk by metric.
    pub gross_measures: IndexMap<MetricId, f64>,
}

/// Complete primitive decomposition with path, net, and gross views.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeExposureReport {
    /// Composite reporting currency.
    pub reporting_currency: Currency,
    /// Every primitive path before overlap netting.
    pub paths: Vec<PrimitiveExposure>,
    /// Primitive aggregates ordered by identifier.
    pub aggregates: Vec<PrimitiveAggregate>,
}

/// Rich structured details attached to composite valuation results.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeValuationDetails {
    /// Effective date of the frozen holdings state used for pricing.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub state_effective_date: Date,
    /// Currency of all reported values and additive risk measures.
    pub reporting_currency: Currency,
    /// Frozen top-level quantities used for this valuation.
    pub resolved_legs: Vec<ResolvedCompositeLeg>,
    /// Scalar inputs retained when the state was resolved.
    pub weighting_inputs: IndexMap<String, f64>,
    /// Native and reporting-currency results for every top-level leg.
    pub leg_results: Vec<CompositeLegValuation>,
    /// Recursive path-level and net/gross primitive exposures.
    pub exposure_report: CompositeExposureReport,
}

/// One top-level leg result retained in composite valuation details.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeLegValuation {
    /// Identifier of the top-level leg specification.
    pub instrument_id: InstrumentId,
    /// Signed frozen leg quantity applied to the unit valuation.
    pub quantity: f64,
    /// Quantity-scaled value in the underlying instrument's native currency.
    pub native_value: Money,
    /// Quantity-scaled value converted to the composite reporting currency.
    pub reporting_value: Money,
    /// Complete unit-instrument valuation, including nested details where applicable.
    pub valuation: crate::results::ValuationResult,
}

fn flatten_composite(
    composite: &CompositeInstrument,
    multiplier: f64,
    path: &mut Vec<String>,
    out: &mut Vec<PrimitiveExposure>,
) -> Result<()> {
    for (leg, state) in composite
        .spec
        .legs
        .iter()
        .zip(&composite.state.resolved_legs)
    {
        path.push(leg.instrument_id.to_string());
        let quantity = multiplier * state.quantity;
        match leg.instrument.as_ref() {
            InstrumentJson::Composite(nested) => {
                flatten_composite(nested, quantity, path, out)?;
            }
            primitive => out.push(PrimitiveExposure {
                path: path.clone(),
                instrument_id: leg.instrument_id.clone(),
                instrument_type: primitive.type_tag().to_string(),
                quantity,
                value: Money::new(0.0, composite.spec.reporting_currency),
                measures: IndexMap::new(),
                instrument: Some(primitive.clone()),
            }),
        }
        let _ = path.pop();
    }
    Ok(())
}

fn register_execution_definition(
    definitions: &mut BTreeMap<String, String>,
    exposure: &PrimitiveExposure,
) -> Result<()> {
    let definition = exposure.instrument_definition().ok_or_else(|| {
        Error::Internal("primitive exposure lost its runtime instrument".to_string())
    })?;
    let hash = InstrumentEnvelope::new(definition.clone()).content_hash()?;
    match definitions.get(exposure.instrument_id.as_str()) {
        Some(existing) if existing != &hash => Err(Error::Validation(format!(
            "primitive instrument id '{}' has conflicting definitions across composite states",
            exposure.instrument_id
        ))),
        Some(_) => Ok(()),
        None => {
            definitions.insert(exposure.instrument_id.to_string(), hash);
            Ok(())
        }
    }
}

fn aggregate_primitive_paths(
    reporting_currency: Currency,
    paths: Vec<PrimitiveExposure>,
) -> CompositeExposureReport {
    let mut aggregate = BTreeMap::<String, PrimitiveAggregate>::new();
    for path in &paths {
        let entry = aggregate
            .entry(path.instrument_id.to_string())
            .or_insert_with(|| PrimitiveAggregate {
                instrument_id: path.instrument_id.clone(),
                instrument_type: path.instrument_type.clone(),
                net_quantity: 0.0,
                gross_quantity: 0.0,
                net_value: Money::new(0.0, reporting_currency),
                gross_value: Money::new(0.0, reporting_currency),
                net_measures: IndexMap::new(),
                gross_measures: IndexMap::new(),
            });
        entry.net_quantity += path.quantity;
        entry.gross_quantity += path.quantity.abs();
        entry.net_value = Money::new(
            entry.net_value.amount() + path.value.amount(),
            reporting_currency,
        );
        entry.gross_value = Money::new(
            entry.gross_value.amount() + path.value.amount().abs(),
            reporting_currency,
        );
        for (metric, value) in &path.measures {
            *entry.net_measures.entry(metric.clone()).or_default() += *value;
            *entry.gross_measures.entry(metric.clone()).or_default() += value.abs();
        }
    }
    CompositeExposureReport {
        reporting_currency,
        paths,
        aggregates: aggregate.into_values().collect(),
    }
}

fn normalized_scores(legs: &[CompositeLegSpec], neutralize: bool) -> Result<Vec<f64>> {
    if !neutralize {
        return Ok(legs.iter().map(|leg| leg.weight).collect());
    }
    let positive = legs
        .iter()
        .filter(|leg| leg.weight > 0.0)
        .map(|leg| leg.weight)
        .sum::<f64>();
    let negative = legs
        .iter()
        .filter(|leg| leg.weight < 0.0)
        .map(|leg| leg.weight.abs())
        .sum::<f64>();
    if !positive.is_finite()
        || !negative.is_finite()
        || positive <= MIN_ABS_INPUT
        || negative <= MIN_ABS_INPUT
    {
        return Err(Error::Validation(
            "neutral weighting requires non-zero positive and negative score totals".to_string(),
        ));
    }
    Ok(legs
        .iter()
        .map(|leg| {
            if leg.weight > 0.0 {
                leg.weight / positive
            } else {
                leg.weight / negative
            }
        })
        .collect())
}

fn leg_index(legs: &[CompositeLegSpec], id: &InstrumentId) -> Result<usize> {
    legs.iter()
        .position(|leg| leg.instrument_id == *id)
        .ok_or_else(|| Error::Validation(format!("composite anchor leg '{}' is not present", id)))
}

fn convert_amount(
    market: &MarketContext,
    amount: f64,
    from: Currency,
    to: Currency,
    as_of: Date,
) -> Result<f64> {
    if from == to {
        return Ok(amount);
    }
    let rate = market.fx_required()?.rate(FxQuery::new(from, to, as_of))?;
    Ok(amount * rate.rate)
}

pub(crate) fn validate_history(
    history: &[CompositeMarketObservation],
    latest_allowed: Option<Date>,
) -> Result<()> {
    if history.windows(2).any(|pair| pair[0].date >= pair[1].date) {
        return Err(Error::Validation(
            "composite market observations must be strictly increasing".to_string(),
        ));
    }
    if latest_allowed.is_some_and(|latest| {
        history
            .last()
            .is_some_and(|observation| observation.date > latest)
    }) {
        return Err(Error::Validation(
            "composite history contains observations after the rebalance date".to_string(),
        ));
    }
    Ok(())
}

fn unit_pnl_series(
    instrument_json: &InstrumentJson,
    reporting_currency: Currency,
    history: &[CompositeMarketObservation],
) -> Result<Vec<f64>> {
    validate_history(history, None)?;
    if history.len() < 2 {
        return Ok(Vec::new());
    }
    let instrument = instrument_json.clone().into_boxed()?;
    let mut values = Vec::with_capacity(history.len());
    let mut markets = Vec::with_capacity(history.len());
    for observation in history {
        let market = observation.restore()?;
        let value = instrument.value(&market, observation.date)?;
        values.push(convert_amount(
            &market,
            value.amount(),
            value.currency(),
            reporting_currency,
            observation.date,
        )?);
        markets.push(market);
    }
    let mut pnl = Vec::with_capacity(history.len() - 1);
    for index in 1..history.len() {
        let start = history[index - 1].date;
        let end = history[index].date;
        let cashflows = instrument.dated_cashflows(&markets[index], start)?;
        let cashflow_total = cashflows
            .into_iter()
            .filter(|(date, _)| *date > start && *date <= end)
            .map(|(date, amount)| {
                convert_amount(
                    &markets[index],
                    amount.amount(),
                    amount.currency(),
                    reporting_currency,
                    date,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        pnl.push(values[index] - values[index - 1] + neumaier_sum(cashflow_total));
    }
    Ok(pnl)
}

pub(crate) fn cashflows_between(
    instrument_json: &InstrumentJson,
    quantity: f64,
    reporting_currency: Currency,
    market: &MarketContext,
    start: Date,
    end: Date,
) -> Result<f64> {
    let instrument = instrument_json.clone().into_boxed()?;
    let cashflows = instrument.dated_cashflows(market, start)?;
    let amounts = cashflows
        .into_iter()
        .filter(|(date, _)| *date > start && *date <= end)
        .map(|(date, amount)| {
            convert_amount(
                market,
                amount.amount(),
                amount.currency(),
                reporting_currency,
                date,
            )
            .map(|converted| converted * quantity)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(neumaier_sum(amounts))
}

fn sample_std_dev(values: &[f64]) -> Result<f64> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(Error::Validation(
            "unit-P&L volatility requires at least two finite observations".to_string(),
        ));
    }
    let mean = neumaier_sum(values.iter().copied()) / values.len() as f64;
    let variance = neumaier_sum(values.iter().map(|value| {
        let difference = *value - mean;
        difference * difference
    })) / (values.len() - 1) as f64;
    Ok(variance.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_quant_core::expr::UnaryOp;
    use finstack_quant_core::market_data::scalars::MarketScalar;
    use finstack_quant_core::money::fx::{FxMatrix, SimpleFxProvider};
    use std::sync::Arc;
    use time::macros::date;

    fn equity_leg(id: &str, shares: f64, price: f64, weight: f64) -> CompositeLegSpec {
        CompositeLegSpec::new(
            id,
            InstrumentJson::Equity(
                crate::instruments::Equity::new(id, id, Currency::USD)
                    .with_shares(shares)
                    .with_price(price),
            ),
            weight,
        )
    }

    #[test]
    fn fixed_composite_values_and_decomposes() -> Result<()> {
        let composite = CompositeInstrument::example()?;
        let value = composite.value(&MarketContext::new(), date!(2025 - 01 - 02))?;
        assert_eq!(value.currency(), Currency::USD);
        assert!((value.amount() - 10.0).abs() < 1.0e-9);

        let primitives = composite.flatten_primitives()?;
        assert_eq!(primitives.len(), 2);
        assert_eq!(primitives[0].quantity, 1.0);
        assert_eq!(primitives[1].quantity, -1.0);
        Ok(())
    }

    #[test]
    fn cross_currency_values_and_dependencies_use_reporting_fx() -> Result<()> {
        let spec = CompositeSpec::new(
            "USD-EUR",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                equity_leg("USD-LEG", 1.0, 100.0, 1.0),
                CompositeLegSpec::new(
                    "EUR-LEG",
                    InstrumentJson::Equity(
                        crate::instruments::Equity::new("EUR-LEG", "EUR-LEG", Currency::EUR)
                            .with_shares(1.0)
                            .with_price(100.0),
                    ),
                    1.0,
                ),
            ],
            WeightingMethod::FixedQuantity,
            RebalanceRule::Manual,
        );
        let composite = spec.initialize_fixed(date!(2025 - 01 - 01))?.instrument;
        let provider = Arc::new(SimpleFxProvider::new());
        provider.set_quote(Currency::EUR, Currency::USD, 1.2)?;
        let market = MarketContext::new().insert_fx(FxMatrix::new(provider));

        let dependencies = composite.market_dependencies()?;
        assert!(dependencies
            .fx_pairs
            .iter()
            .any(|pair| { pair.base == Currency::EUR && pair.quote == Currency::USD }));
        let result = composite.price_with_metrics(
            &market,
            date!(2025 - 01 - 02),
            &[],
            PricingOptions::default(),
        )?;
        assert_eq!(result.value.amount(), 220.0);
        let Some(crate::results::ValuationDetails::Composite(details)) = result.details else {
            return Err(Error::Internal(
                "cross-currency composite details are missing".to_string(),
            ));
        };
        assert_eq!(details.leg_results[1].native_value.amount(), 100.0);
        assert_eq!(
            details.leg_results[1].native_value.currency(),
            Currency::EUR
        );
        assert_eq!(details.leg_results[1].reporting_value.amount(), 120.0);
        Ok(())
    }

    #[test]
    fn neutral_scores_split_butterfly_wings() -> Result<()> {
        let legs = vec![
            CompositeLegSpec::new(
                "A",
                InstrumentJson::Equity(
                    crate::instruments::Equity::new("A", "A", Currency::USD)
                        .with_shares(1.0)
                        .with_price(1.0),
                ),
                -1.0,
            ),
            CompositeLegSpec::new(
                "B",
                InstrumentJson::Equity(
                    crate::instruments::Equity::new("B", "B", Currency::USD)
                        .with_shares(1.0)
                        .with_price(1.0),
                ),
                1.0,
            ),
            CompositeLegSpec::new(
                "C",
                InstrumentJson::Equity(
                    crate::instruments::Equity::new("C", "C", Currency::USD)
                        .with_shares(1.0)
                        .with_price(1.0),
                ),
                -3.0,
            ),
        ];
        assert_eq!(normalized_scores(&legs, true)?, vec![-0.25, 1.0, -0.75]);
        Ok(())
    }

    #[test]
    fn notional_weighting_normalizes_requested_gross() -> Result<()> {
        let spec = CompositeSpec::new(
            "NOTIONAL",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                equity_leg("A", 1.0, 100.0, 1.0),
                equity_leg("B", 2.0, 50.0, -3.0),
            ],
            WeightingMethod::NotionalWeighted {
                gross_notional: Money::new(300.0, Currency::USD),
            },
            RebalanceRule::Manual,
        );
        let resolved = spec.initialize(&MarketContext::new(), date!(2025 - 01 - 01), &[])?;
        assert_eq!(resolved.instrument.state.resolved_legs[0].quantity, 0.75);
        assert_eq!(resolved.instrument.state.resolved_legs[1].quantity, -2.25);
        let gross = resolved
            .instrument
            .state
            .resolved_legs
            .iter()
            .map(|leg| leg.quantity.abs() * 100.0)
            .sum::<f64>();
        assert!((gross - 300.0).abs() < 1.0e-12);
        Ok(())
    }

    #[test]
    fn delta_neutral_weighting_uses_unit_metrics_and_anchor_scale() -> Result<()> {
        let spec = CompositeSpec::new(
            "DELTA",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                equity_leg("A", 2.0, 100.0, 1.0),
                equity_leg("B", 4.0, 100.0, -1.0),
            ],
            WeightingMethod::delta_neutral("A", 1.0),
            RebalanceRule::Manual,
        );
        let resolved = spec.initialize(&MarketContext::new(), date!(2025 - 01 - 01), &[])?;
        assert_eq!(resolved.instrument.state.resolved_legs[0].quantity, 1.0);
        assert_eq!(resolved.instrument.state.resolved_legs[1].quantity, -0.5);
        assert!((1.0_f64 * 2.0 + -0.5 * 4.0).abs() < 1.0e-12);
        Ok(())
    }

    #[test]
    fn volatility_weighting_uses_one_unit_total_pnl() -> Result<()> {
        let legs = vec![
            CompositeLegSpec::new(
                "A",
                InstrumentJson::Equity(crate::instruments::Equity::new("A", "A", Currency::USD)),
                1.0,
            ),
            CompositeLegSpec::new(
                "B",
                InstrumentJson::Equity(crate::instruments::Equity::new("B", "B", Currency::USD)),
                -1.0,
            ),
        ];
        let spec = CompositeSpec::new(
            "VOL",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            legs,
            WeightingMethod::volatility_weighted("A", 1.0, 3, 3, 252.0),
            RebalanceRule::Manual,
        );
        let observations = [(100.0, 100.0), (102.0, 104.0), (99.0, 98.0), (103.0, 106.0)]
            .into_iter()
            .enumerate()
            .map(|(offset, (a, b))| {
                let date = date!(2025 - 01 - 01) + time::Duration::days(offset as i64);
                let market = MarketContext::new()
                    .insert_price("A", MarketScalar::Unitless(a))
                    .insert_price("B", MarketScalar::Unitless(b));
                CompositeMarketObservation::new(date, &market)
            })
            .collect::<Vec<_>>();
        let market = observations
            .last()
            .ok_or_else(|| Error::Internal("test history is empty".to_string()))?
            .restore()?;
        let resolved = spec.initialize(&market, date!(2025 - 01 - 04), &observations)?;
        assert!((resolved.instrument.state.resolved_legs[0].quantity - 1.0).abs() < 1.0e-12);
        assert!((resolved.instrument.state.resolved_legs[1].quantity + 0.5).abs() < 1.0e-12);
        Ok(())
    }

    #[test]
    fn user_defined_expressions_resolve_quantities() -> Result<()> {
        let expressions = IndexMap::from([
            ("A".to_string(), Expr::literal(2.0)),
            (
                "B".to_string(),
                Expr::unary_op(UnaryOp::Neg, Expr::literal(3.0)),
            ),
        ]);
        let spec = CompositeSpec::new(
            "EXPR",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                equity_leg("A", 1.0, 100.0, 1.0),
                equity_leg("B", 1.0, 100.0, -1.0),
            ],
            WeightingMethod::UserDefined {
                required_metrics: Vec::new(),
                quantity_expressions: expressions,
            },
            RebalanceRule::Manual,
        );
        let resolved = spec.initialize(&MarketContext::new(), date!(2025 - 01 - 01), &[])?;
        assert_eq!(resolved.instrument.state.resolved_legs[0].quantity, 2.0);
        assert_eq!(resolved.instrument.state.resolved_legs[1].quantity, -3.0);
        Ok(())
    }

    #[test]
    fn fixed_state_rejects_mismatched_identifier() -> Result<()> {
        let mut composite = CompositeInstrument::example()?;
        composite.state.resolved_legs[0].instrument_id = InstrumentId::new("WRONG");
        assert!(composite.validate_invariants().is_err());
        Ok(())
    }

    #[test]
    fn execution_rejects_conflicting_primitive_definitions_between_states() -> Result<()> {
        let previous = CompositeInstrument::example()?;
        let mut changed_spec = previous.spec.clone();
        let InstrumentJson::Equity(equity) = changed_spec.legs[0].instrument.as_mut() else {
            return Err(Error::Internal("expected equity example leg".to_string()));
        };
        equity.price_quote = Some(101.0);
        let current = changed_spec
            .initialize_fixed(date!(2025 - 01 - 02))?
            .instrument;
        let error = current
            .execution_trades(Some(&previous))
            .expect_err("same primitive ID with changed economics must be rejected");
        assert!(error.to_string().contains("conflicting definitions"));
        Ok(())
    }

    #[test]
    fn nested_composites_report_net_and_gross_repeated_primitives() -> Result<()> {
        let a = equity_leg("A", 1.0, 100.0, 1.0);
        let inner = CompositeSpec::new(
            "INNER",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![a.clone(), equity_leg("B", 1.0, 90.0, -1.0)],
            WeightingMethod::FixedQuantity,
            RebalanceRule::Manual,
        )
        .initialize_fixed(date!(2025 - 01 - 01))?
        .instrument;
        let outer = CompositeSpec::new(
            "OUTER",
            Currency::USD,
            Money::new(100.0, Currency::USD),
            vec![
                CompositeLegSpec::new("INNER", InstrumentJson::Composite(Box::new(inner)), 2.0),
                CompositeLegSpec::new("A", (*a.instrument).clone(), -1.0),
            ],
            WeightingMethod::FixedQuantity,
            RebalanceRule::Manual,
        )
        .initialize_fixed(date!(2025 - 01 - 01))?
        .instrument;

        let report =
            outer.primitive_exposure_report(&MarketContext::new(), date!(2025 - 01 - 02), &[])?;
        assert_eq!(report.paths.len(), 3);
        let a = report
            .aggregates
            .iter()
            .find(|aggregate| aggregate.instrument_id.as_str() == "A")
            .ok_or_else(|| Error::Internal("missing repeated primitive A".to_string()))?;
        assert_eq!(a.net_quantity, 1.0);
        assert_eq!(a.gross_quantity, 3.0);
        assert_eq!(a.net_value.amount(), 100.0);
        assert_eq!(a.gross_value.amount(), 300.0);
        Ok(())
    }

    #[test]
    fn metric_pricing_never_changes_resolved_quantities() -> Result<()> {
        let composite = CompositeInstrument::example()?;
        let before = serde_json::to_value(&composite.state).map_err(|error| {
            Error::Internal(format!("failed to serialize composite state: {error}"))
        })?;
        let result = composite.price_with_metrics(
            &MarketContext::new(),
            date!(2025 - 01 - 02),
            &[MetricId::Delta],
            PricingOptions::default(),
        )?;
        assert!(result.measures.contains_key(&MetricId::Delta));
        let Some(crate::results::ValuationDetails::Composite(details)) = result.details else {
            return Err(Error::Internal(
                "composite valuation did not retain structured details".to_string(),
            ));
        };
        assert_eq!(details.resolved_legs.len(), 2);
        assert_eq!(details.leg_results.len(), 2);
        assert_eq!(details.leg_results[0].native_value.amount(), 100.0);
        assert_eq!(details.leg_results[0].reporting_value.amount(), 100.0);
        assert_eq!(details.leg_results[1].native_value.amount(), -90.0);
        assert_eq!(
            details.leg_results[1].valuation.instrument_id,
            "COMPOSITE-SHORT"
        );
        let after = serde_json::to_value(&composite.state).map_err(|error| {
            Error::Internal(format!("failed to serialize composite state: {error}"))
        })?;
        assert_eq!(before, after);
        Ok(())
    }

    #[test]
    fn non_additive_metrics_are_rejected_at_composite_level() -> Result<()> {
        let composite = CompositeInstrument::example()?;
        let error = composite
            .primitive_exposure_report(
                &MarketContext::new(),
                date!(2025 - 01 - 02),
                &[MetricId::DurationMod],
            )
            .expect_err("modified duration is non-additive");
        assert!(error.to_string().contains("not additive"));
        Ok(())
    }

    #[test]
    fn composite_envelope_round_trips_through_strict_loader() -> Result<()> {
        let composite = CompositeInstrument::example()?;
        let envelope = InstrumentEnvelope::new(InstrumentJson::Composite(Box::new(composite)));
        let json = serde_json::to_vec(&envelope).map_err(|error| {
            Error::Internal(format!("failed to serialize composite envelope: {error}"))
        })?;
        let (loaded, report) = InstrumentEnvelope::from_slice_strict(
            &json,
            &finstack_quant_core::LoadLimits::default(),
        )
        .map_err(|error| Error::Validation(error.to_string()))?;
        assert!(!report.has_errors());
        assert_eq!(loaded.id(), "COMPOSITE-EXAMPLE");
        assert_eq!(loaded.key(), crate::pricer::InstrumentType::Composite);
        Ok(())
    }
}
