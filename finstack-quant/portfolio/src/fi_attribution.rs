//! Campisi-style benchmark-relative fixed-income attribution.
//!
//! # Single-period decomposition
//!
//! Each position's period return is decomposed (Campisi 2000) into:
//!
//! ```text
//! carry_j     = yield_annual_j × Δt                    // income effect
//! treasury_j  = −MD_j × Δy_tsy,j                       // duration / curve effect
//! spread_j    = −SD_j × Δs_j                           // spread effect
//! selection_j = r_j − carry_j − treasury_j − spread_j  // residual
//! ```
//!
//! `Δs_j` ([`FiPositionSnapshot::delta_spread`]) is the *absolute* spread
//! change. Supplying a relative change `Δs / s` instead would overstate the
//! spread effect by a factor of `1 / s` and dump the difference into selection.
//!
//! # Why there is no DTS spread mode
//!
//! Duration-Times-Spread (Ben Dor et al. 2007) re-expresses credit exposure as
//! the product `D · s` against a *relative* spread change. Under that
//! convention the three quantities of interest read:
//!
//! ```text
//!               absolute                DTS
//! return        R = −D · Δs             R = −(D · s) · (Δs / s)
//! volatility    σ_R ≈ D · σ_absolute    σ_R ≈ (D · s) · σ_relative
//! hedge ratio   H = D₁ / D₂             H = (D₁ · s₁) / (D₂ · s₂)
//! ```
//!
//! (Barclays QPS, "Managing Credit Exposure of CDS Portfolios: Adjusting DTS
//! for Market Beta", 23 Jan 2024 — the "paradigm change from absolute to
//! relative spread changes" slide.)
//!
//! The **return** row is an algebraic identity: `−(D · s)(Δs / s) ≡ −D · Δs`.
//! DTS earns its keep only in the volatility and hedge-ratio rows, where
//! `D · s` is a standalone risk quantity multiplied by a relative spread
//! volatility that is empirically far more stable across issuers and rating
//! bands than an absolute one. Ex-post attribution is handed a *realized* `Δs`,
//! so there is no volatility to model and no hedge to size — both conventions
//! reduce to the same number. A mode switch here could therefore only ever
//! relabel an identical result, so this module exposes one spread convention.
//! DTS belongs in risk and hedging surfaces, not in this decomposition.
//!
//! A corollary: the spread *level* [`FiPositionSnapshot::spread`] never enters
//! the arithmetic — nothing divides by it — so zero and negative spread levels
//! (Bund asset swaps, negative OAS on deep-premium callables) are accepted as
//! ordinary inputs rather than rejected.
//!
//! # Benchmark-relative sector layer
//!
//! Positions are bucketed by `sector`. With `w_p,i`/`w_b,i` the sector
//! weights, `r_p,i`/`r_b,i` the weighted sector returns and `c_p,i`/`c_b,i`
//! the weighted sector component rates (carry, treasury, spread, selection):
//!
//! ```text
//! Allocation_i      = (w_p,i − w_b,i) · (r_b,i − r_b)          // Brinson-Fachler
//! ActiveCarry_i     = w_p,i · (carry_p,i − carry_b,i)
//! ActiveTreasury_i  = w_p,i · (treasury_p,i − treasury_b,i)
//! ActiveSpread_i    = w_p,i · (spread_p,i − spread_b,i)
//! Selection_i       = w_p,i · (selection_p,i − selection_b,i)
//! ```
//!
//! Because component rates sum to the sector return on each side, the five
//! effects telescope exactly to the active return:
//!
//! ```text
//! Σ_i [Alloc_i + w_p,i (r_p,i − r_b,i)]
//!   = Σ_i w_p,i r_p,i − Σ_i w_b,i r_b,i − r_b Σ_i (w_p,i − w_b,i)
//!   = r_p − r_b                        (weights sum to 1 on each side)
//! ```
//!
//! This is the two-way Brinson-Fachler form (interaction folded into the
//! within-sector effects at portfolio weight), which keeps the component
//! split exact; the three-way split used in [`crate::brinson`] would need a
//! per-component interaction bucket.
//!
//! # References
//!
//! * Campisi, S. (2000). "Primer on Fixed Income Performance Attribution."
//!   *Journal of Portfolio Management*, 26(4), 14–25.
//!   `docs/REFERENCES.md#campisi-2000`
//! * Ben Dor, A., Dynkin, L., Hyman, J., Houweling, P., van Leeuwen, E., &
//!   Penninga, O. (2007). "DTS (Duration Times Spread)." *Journal of
//!   Portfolio Management*, 33(2), 77–100 — source of the DTS convention
//!   deliberately *not* offered here; see "Why there is no DTS spread mode".
//!   `docs/REFERENCES.md#ben-dor-2007-dts`
//! * Brinson, G. P., & Fachler, N. (1985). "Measuring Non-US Equity Portfolio
//!   Performance." *Journal of Portfolio Management*, 11(3) — source of the
//!   `(w_p − w_b)(r_b,i − r_b)` allocation form used above.
//!   `docs/REFERENCES.md#brinson-fachler-1985`
//! * Carino, D. (1999). "Combining Attribution Effects over Time." *Journal of
//!   Performance Measurement*, 3(4), 5–14 — multi-period smoothing applied by
//!   [`campisi_carino_link`]. `docs/REFERENCES.md#carino-1999`

use crate::brinson::carino_coefficient;
use crate::error::{Error, Result};
use finstack_quant_core::math::summation::NeumaierAccumulator;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Tolerance for the requirement that weights sum to 1.0 on each side.
const WEIGHT_TOLERANCE: f64 = 1e-6;

/// Configuration for [`campisi_attribution`].
///
/// Deliberately a single field: the spread convention is not configurable, for
/// the reason set out under "Why there is no DTS spread mode" in the module
/// docs. `period_years` has no serde default — an omitted key fails closed
/// rather than silently assuming a period length.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiAttributionConfig {
    /// Length of the attribution period in years (e.g. `0.25` for a
    /// quarter). Scales `yield_annual` into the period carry.
    pub period_years: f64,
}

impl FiAttributionConfig {
    /// Create a config for the given period length.
    ///
    /// # Arguments
    ///
    /// * `period_years` - Attribution period length in years; must be finite
    ///   and positive.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_portfolio::fi_attribution::FiAttributionConfig;
    ///
    /// let config = FiAttributionConfig::new(0.25);
    /// assert_eq!(config.period_years, 0.25);
    /// ```
    pub fn new(period_years: f64) -> Self {
        Self { period_years }
    }
}

/// Plain-data snapshot of one position (or pre-aggregated bucket) for one
/// attribution period.
///
/// Weights are fractions of the whole portfolio (or benchmark) and must sum
/// to 1.0 within each side. Returns, yields and spreads are decimals
/// (`0.02` = 2 %); durations are in years.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiPositionSnapshot {
    /// Sector bucket label (industry / quality / any grouping).
    pub sector: String,
    /// Weight as a fraction of the whole side at period start.
    pub weight: f64,
    /// Realized total return for the period (decimal).
    pub total_return: f64,
    /// Annualized yield at period start (decimal; e.g. YTM).
    pub yield_annual: f64,
    /// Modified duration in years at period start.
    pub modified_duration: f64,
    /// Spread duration in years at period start.
    pub spread_duration: f64,
    /// Spread at period start (decimal; e.g. Z-spread or OAS).
    ///
    /// Carried for provenance and downstream reporting only — the
    /// decomposition never divides by it, so any finite value (including zero
    /// and negative levels) is accepted. See "Why there is no DTS spread mode"
    /// in the module docs.
    pub spread: f64,
    /// Change in the treasury/benchmark yield relevant to this position's
    /// duration bucket over the period (decimal).
    pub delta_treasury_yield: f64,
    /// Absolute change in the position's spread over the period (decimal).
    pub delta_spread: f64,
}

/// Assemble an [`FiPositionSnapshot`] from valuation metrics plus
/// caller-supplied period data.
///
/// Reads `"ytm"`, `"duration_mod"` and `"spread_duration"` (the canonical
/// valuations metric IDs) plus the caller-chosen spread metric (typically
/// `"z_spread"` or `"oas"`) from `metrics`. All four are decimals/years in the
/// registry, matching [`FiPositionSnapshot`]'s conventions. The remaining
/// fields — sector, weight, realized return and the period's treasury/spread
/// moves — are not valuation metrics and must be supplied by the caller (e.g.
/// from performance data and curve marks).
///
/// # Instrument coverage
///
/// `"spread_duration"` is registered for `InstrumentType::Bond` and the
/// structured-credit instrument types (CLO/ABS/RMBS/CMBS tranches). It is
/// derived from CS01 (`-CS01 / (NPV × 1bp)`), so the position must have been
/// priced with `MetricId::Cs01` available; requesting `MetricId::SpreadDuration`
/// pulls that dependency in automatically. Instrument types without a
/// registered CS01 — swaps, options, equities — cannot supply this helper's
/// inputs, and the returned error names the missing metric.
///
/// # Arguments
///
/// * `metrics` - Per-position metrics from [`crate::metrics::aggregate_metrics`]
///   output (a [`crate::metrics::PositionMetrics`] entry of `by_position`).
/// * `sector` - Sector bucket label for the attribution.
/// * `weight` - Position weight (fraction of the side, decimal).
/// * `total_return` - Realized period return (decimal).
/// * `delta_treasury_yield` - Treasury yield move for the position's bucket
///   (decimal).
/// * `delta_spread` - Absolute spread move (decimal).
/// * `spread_metric_id` - Metric ID to read the period-start spread from
///   (e.g. `"z_spread"`, `"oas"`, `"g_spread"`, `"discount_margin"`).
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] naming the first missing metric ID; the
/// metrics are checked in the order `"ytm"`, `"duration_mod"`,
/// `"spread_duration"`, `spread_metric_id`.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_core::currency::Currency;
/// use finstack_quant_portfolio::metrics::PositionMetrics;
/// use finstack_quant_portfolio::snapshot_from_position_metrics;
///
/// let mut metrics = indexmap::IndexMap::new();
/// metrics.insert("ytm".to_string(), 0.06);
/// metrics.insert("duration_mod".to_string(), 4.0);
/// metrics.insert("spread_duration".to_string(), 3.8);
/// metrics.insert("z_spread".to_string(), 0.015);
/// let position = PositionMetrics { currency: Currency::USD, metrics };
///
/// let snap = snapshot_from_position_metrics(
///     &position, "CORP", 0.30, 0.012, -0.001, 0.002, "z_spread",
/// )?;
/// assert_eq!(snap.sector, "CORP");
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
pub fn snapshot_from_position_metrics(
    metrics: &crate::metrics::PositionMetrics,
    sector: impl Into<String>,
    weight: f64,
    total_return: f64,
    delta_treasury_yield: f64,
    delta_spread: f64,
    spread_metric_id: &str,
) -> Result<FiPositionSnapshot> {
    let get = |id: &str| -> Result<f64> {
        metrics.metrics.get(id).copied().ok_or_else(|| {
            Error::invalid_input(format!(
                "Campisi snapshot requires metric '{id}' in PositionMetrics; \
                 request it via RequestedMetrics before aggregation"
            ))
        })
    };
    Ok(FiPositionSnapshot {
        sector: sector.into(),
        weight,
        total_return,
        yield_annual: get("ytm")?,
        modified_duration: get("duration_mod")?,
        spread_duration: get("spread_duration")?,
        spread: get(spread_metric_id)?,
        delta_treasury_yield,
        delta_spread,
    })
}

/// Absolute Campisi component contributions for one side (portfolio or
/// benchmark), each `Σ_j w_j × component_j`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiComponents {
    /// Income effect `Σ w · y · Δt`.
    pub carry: f64,
    /// Treasury/duration effect `Σ w · (−MD · Δy)`.
    pub treasury: f64,
    /// Spread effect `Σ w · (−SD · Δs)`.
    pub spread: f64,
    /// Residual selection `Σ w · (r − explained)`.
    pub selection: f64,
    /// Sum of the four components — equals the side's total return.
    pub total: f64,
}

/// Per-sector benchmark-relative effects.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiSectorEffect {
    /// Sector label (mirrors [`FiPositionSnapshot::sector`]).
    pub sector: String,
    /// Portfolio weight in the sector.
    pub portfolio_weight: f64,
    /// Benchmark weight in the sector.
    pub benchmark_weight: f64,
    /// Portfolio sector return (weighted, per unit of sector weight).
    pub portfolio_return: f64,
    /// Benchmark sector return (weighted, per unit of sector weight).
    pub benchmark_return: f64,
    /// Brinson-Fachler allocation `(w_p − w_b)(r_b,i − r_b)`.
    pub allocation: f64,
    /// Active income effect `w_p (carry_p,i − carry_b,i)`.
    pub active_carry: f64,
    /// Active duration positioning `w_p (treasury_p,i − treasury_b,i)`.
    pub active_treasury: f64,
    /// Active spread positioning `w_p (spread_p,i − spread_b,i)`.
    pub active_spread: f64,
    /// Security selection residual `w_p (selection_p,i − selection_b,i)`.
    pub selection: f64,
    /// Sum of the five effects — the sector's contribution to active return.
    pub total_active: f64,
}

/// Single-period Campisi benchmark-relative attribution result.
///
/// This type is *input-reachable*: [`campisi_carino_link`] consumes a slice of
/// these, and the Python/WASM bindings deserialize them from JSON. It therefore
/// denies unknown fields like the other inbound types in this module, so a
/// misspelled or stale key fails closed instead of being silently dropped.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiAttributionResult {
    /// Per-sector effects, in first-seen order (portfolio first, then
    /// benchmark-only sectors).
    pub sectors: Vec<FiSectorEffect>,
    /// Portfolio-side absolute Campisi split.
    pub portfolio_components: FiComponents,
    /// Benchmark-side absolute Campisi split.
    pub benchmark_components: FiComponents,
    /// Portfolio total return `Σ w_p,j r_p,j`.
    pub portfolio_return: f64,
    /// Benchmark total return `Σ w_b,j r_b,j`.
    pub benchmark_return: f64,
    /// Active return `portfolio_return − benchmark_return`.
    pub active_return: f64,
    /// Sum of sector allocation effects.
    pub total_allocation: f64,
    /// Sum of sector active carry effects.
    pub total_active_carry: f64,
    /// Sum of sector active treasury effects.
    pub total_active_treasury: f64,
    /// Sum of sector active spread effects.
    pub total_active_spread: f64,
    /// Sum of sector selection effects.
    pub total_selection: f64,
}

/// Report from reconciling the five effect totals against the active return,
/// mirroring [`crate::attribution`] reconciliation conventions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiReconciliationReport {
    /// `active_return − (allocation + carry + treasury + spread + selection)`.
    pub total_residual: f64,
    /// Whether the residual is within tolerance.
    pub is_reconciled: bool,
    /// Tolerance used for the check.
    pub tolerance: f64,
}

impl FiAttributionResult {
    /// Check that the five effect totals reconstruct the active return.
    ///
    /// The decomposition reconciles by construction (the selection component
    /// is a residual), so this is a floating-point sanity gate, not a model
    /// check; `1e-10` is an appropriate tolerance for return-space values.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Absolute tolerance in return units.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_portfolio::fi_attribution::{
    ///     campisi_attribution, FiAttributionConfig, FiPositionSnapshot,
    /// };
    ///
    /// let bucket = |sector: &str, weight: f64, r: f64, y: f64, md: f64| FiPositionSnapshot {
    ///     sector: sector.into(),
    ///     weight,
    ///     total_return: r,
    ///     yield_annual: y,
    ///     modified_duration: md,
    ///     spread_duration: 0.0,
    ///     spread: 0.0,
    ///     delta_treasury_yield: -0.001,
    ///     delta_spread: 0.0,
    /// };
    /// let portfolio = vec![bucket("GOVT", 1.0, 0.016, 0.04, 5.0)];
    /// let benchmark = vec![bucket("GOVT", 1.0, 0.015, 0.04, 5.5)];
    /// let result = campisi_attribution(&portfolio, &benchmark, &FiAttributionConfig::new(0.25))?;
    /// let report = result.reconciliation_check(1e-10);
    /// assert!(report.is_reconciled);
    /// # Ok::<(), finstack_quant_portfolio::Error>(())
    /// ```
    pub fn reconciliation_check(&self, tolerance: f64) -> FiReconciliationReport {
        let mut acc = NeumaierAccumulator::new();
        acc.add(self.total_allocation);
        acc.add(self.total_active_carry);
        acc.add(self.total_active_treasury);
        acc.add(self.total_active_spread);
        acc.add(self.total_selection);
        let total_residual = self.active_return - acc.total();
        FiReconciliationReport {
            total_residual,
            is_reconciled: total_residual.abs() <= tolerance,
            tolerance,
        }
    }
}

/// Per-position Campisi components (carry, treasury, spread) in return space.
///
/// Infallible: every input has already been checked for finiteness by
/// [`validate_snapshot`], and none of the three expressions divides by an
/// input. In particular the spread effect is linear in `delta_spread` and does
/// not touch the spread *level*, so `spread <= 0` needs no guard here.
fn position_components(s: &FiPositionSnapshot, config: &FiAttributionConfig) -> (f64, f64, f64) {
    let carry = s.yield_annual * config.period_years;
    let treasury = -s.modified_duration * s.delta_treasury_yield;
    let spread = -s.spread_duration * s.delta_spread;
    (carry, treasury, spread)
}

/// Validate finiteness of every numeric field of a snapshot.
fn validate_snapshot(s: &FiPositionSnapshot, side: &str) -> Result<()> {
    for (name, value) in [
        ("weight", s.weight),
        ("total_return", s.total_return),
        ("yield_annual", s.yield_annual),
        ("modified_duration", s.modified_duration),
        ("spread_duration", s.spread_duration),
        ("spread", s.spread),
        ("delta_treasury_yield", s.delta_treasury_yield),
        ("delta_spread", s.delta_spread),
    ] {
        if !value.is_finite() {
            return Err(Error::invalid_input(format!(
                "Campisi {side} input '{name}' for sector '{}' must be finite (got {value})",
                s.sector
            )));
        }
    }
    Ok(())
}

/// Weighted per-sector accumulators for one side.
///
/// `weight` is the *net* sector weight (long minus short) and `abs_weight` the
/// gross weight; the pair distinguishes "sector absent from this side"
/// (`abs_weight == 0`) from "sector present with offsetting positions"
/// (`abs_weight > 0`, `weight == 0`), which [`check_net_weight`] rejects.
#[derive(Clone, Copy, Default)]
struct SideAgg {
    weight: f64,
    abs_weight: f64,
    ret: f64,
    carry: f64,
    treasury: f64,
    spread: f64,
    selection: f64,
}

impl SideAgg {
    /// Sector rate: weighted contribution ÷ net sector weight.
    ///
    /// Returns `0.0` only when the sector is absent from this side, i.e. every
    /// contribution is likewise `0.0`. Sectors present with an exactly-zero
    /// net weight are rejected by [`check_net_weight`] before this is called,
    /// so the guard never silently discards a real contribution. The `.abs()`
    /// is load-bearing: net-short sectors (`weight < 0`) are legal and must
    /// take the division branch.
    fn rate(&self, contribution: f64) -> f64 {
        if self.weight.abs() > 0.0 {
            contribution / self.weight
        } else {
            0.0
        }
    }
}

/// Fail closed on a sector that is present on a side but nets to exactly zero
/// weight (a long/short pair, a CDS hedge against a cash bond in the same
/// bucket, a fully-hedged sector).
///
/// Such a sector still contributes `Σ_j w_j r_j ≠ 0` to the side total, but its
/// per-unit rate `contribution / weight` is undefined, so every per-sector
/// effect would be forced to zero while the contribution stayed in the side
/// return — breaking the telescoping identity while `active_return` still ties
/// out against performance data.
fn check_net_weight(sector: &str, agg: &SideAgg, side_name: &str) -> Result<()> {
    if agg.weight == 0.0 && agg.abs_weight > 0.0 {
        return Err(Error::invalid_input(format!(
            "{side_name} sector '{sector}' has offsetting positions netting to exactly \
             zero weight (gross weight {}); a zero-net-weight sector cannot be \
             attributed because its per-unit rate contribution / weight is undefined. \
             Split the offsetting positions into distinct sectors, or net them into a \
             single snapshot with non-zero weight.",
            agg.abs_weight
        )));
    }
    Ok(())
}

/// Accumulate one side into per-sector aggregates and side totals.
///
/// Returns `(side_return, side_components)` and fills `sectors` (union map,
/// first-seen order preserved).
fn aggregate_side(
    snapshots: &[FiPositionSnapshot],
    config: &FiAttributionConfig,
    sectors: &mut IndexMap<String, (SideAgg, SideAgg)>,
    is_portfolio: bool,
) -> Result<(f64, FiComponents)> {
    let (side, side_name) = if is_portfolio {
        ("portfolio", "Portfolio")
    } else {
        ("benchmark", "Benchmark")
    };
    let mut sum_w = NeumaierAccumulator::new();
    let mut sum_r = NeumaierAccumulator::new();
    let mut sum_carry = NeumaierAccumulator::new();
    let mut sum_tsy = NeumaierAccumulator::new();
    let mut sum_spr = NeumaierAccumulator::new();
    let mut sum_sel = NeumaierAccumulator::new();

    for s in snapshots {
        validate_snapshot(s, side)?;
        let (carry, treasury, spread) = position_components(s, config);
        let selection = s.total_return - carry - treasury - spread;

        sum_w.add(s.weight);
        sum_r.add(s.weight * s.total_return);
        sum_carry.add(s.weight * carry);
        sum_tsy.add(s.weight * treasury);
        sum_spr.add(s.weight * spread);
        sum_sel.add(s.weight * selection);

        let entry = sectors.entry(s.sector.clone()).or_default();
        let agg = if is_portfolio {
            &mut entry.0
        } else {
            &mut entry.1
        };
        agg.weight += s.weight;
        agg.abs_weight += s.weight.abs();
        agg.ret += s.weight * s.total_return;
        agg.carry += s.weight * carry;
        agg.treasury += s.weight * treasury;
        agg.spread += s.weight * spread;
        agg.selection += s.weight * selection;
    }

    let total_w = sum_w.total();
    if (total_w - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(Error::invalid_input(format!(
            "{side_name} weights must sum to 1.0 (got {total_w})"
        )));
    }

    let ret = sum_r.total();
    let carry = sum_carry.total();
    let treasury = sum_tsy.total();
    let spread = sum_spr.total();
    let selection = sum_sel.total();
    Ok((
        ret,
        FiComponents {
            carry,
            treasury,
            spread,
            selection,
            total: carry + treasury + spread + selection,
        },
    ))
}

/// Compute a single-period Campisi benchmark-relative attribution.
///
/// Decomposes each side's return into carry, treasury, spread and selection
/// (Campisi 2000), then buckets by sector and splits the active return into
/// allocation plus four active component effects using Brinson-Fachler sign
/// conventions (see the module docs for the exact formulas and the
/// reconciliation proof).
///
/// A sector missing from one side is treated with zero weight on that side,
/// so the decomposition stays complete. A sector that is *present* on a side
/// but whose positions net to exactly zero weight (a long/short pair, a CDS
/// hedge against a cash bond in the same bucket) has no defined per-unit rate
/// and is rejected — see the errors below.
///
/// # Arguments
///
/// * `portfolio` - Portfolio position/bucket snapshots; weights must sum to 1.
/// * `benchmark` - Benchmark snapshots; weights must sum to 1.
/// * `config` - Period length.
///
/// # Errors
///
/// * [`Error::InvalidInput`] if either side is empty, any value is
///   non-finite, weights don't sum to 1.0 (±1e-6), `period_years` is not
///   finite and positive, or a sector has offsetting positions netting to
///   exactly zero weight on either side.
///
/// The spread *level* is unconstrained: zero and negative spreads are accepted,
/// because the spread effect `−SD · Δs` never divides by it.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::fi_attribution::{
///     campisi_attribution, FiAttributionConfig, FiPositionSnapshot,
/// };
///
/// let bucket = |sector: &str, weight: f64, r: f64, y: f64, md: f64| FiPositionSnapshot {
///     sector: sector.into(),
///     weight,
///     total_return: r,
///     yield_annual: y,
///     modified_duration: md,
///     spread_duration: 0.0,
///     spread: 0.0,
///     delta_treasury_yield: -0.001,
///     delta_spread: 0.0,
/// };
/// let portfolio = vec![bucket("GOVT", 0.6, 0.016, 0.04, 5.0), bucket("CORP", 0.4, 0.012, 0.06, 4.0)];
/// let benchmark = vec![bucket("GOVT", 0.5, 0.015, 0.04, 5.5), bucket("CORP", 0.5, 0.011, 0.055, 4.5)];
/// let result = campisi_attribution(&portfolio, &benchmark, &FiAttributionConfig::new(0.25))?;
/// assert!(result.reconciliation_check(1e-10).is_reconciled);
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
pub fn campisi_attribution(
    portfolio: &[FiPositionSnapshot],
    benchmark: &[FiPositionSnapshot],
    config: &FiAttributionConfig,
) -> Result<FiAttributionResult> {
    if !config.period_years.is_finite() || config.period_years <= 0.0 {
        return Err(Error::invalid_input(format!(
            "Campisi config period_years must be finite and positive (got {})",
            config.period_years
        )));
    }
    if portfolio.is_empty() {
        return Err(Error::invalid_input(
            "Campisi attribution requires at least one portfolio snapshot",
        ));
    }
    if benchmark.is_empty() {
        return Err(Error::invalid_input(
            "Campisi attribution requires at least one benchmark snapshot",
        ));
    }

    // Union of sectors in first-seen order: portfolio first, then
    // benchmark-only sectors (deterministic IndexMap ordering).
    let mut sectors: IndexMap<String, (SideAgg, SideAgg)> = IndexMap::new();
    let (portfolio_return, portfolio_components) =
        aggregate_side(portfolio, config, &mut sectors, true)?;
    let (benchmark_return, benchmark_components) =
        aggregate_side(benchmark, config, &mut sectors, false)?;

    // A sector present on a side but netting to exactly zero weight has no
    // defined per-unit rate; attributing it would zero its five effects while
    // its contribution stayed in the side return. Fail closed instead.
    for (sector, (p, b)) in &sectors {
        check_net_weight(sector, p, "Portfolio")?;
        check_net_weight(sector, b, "Benchmark")?;
    }

    let mut total_allocation = NeumaierAccumulator::new();
    let mut total_active_carry = NeumaierAccumulator::new();
    let mut total_active_treasury = NeumaierAccumulator::new();
    let mut total_active_spread = NeumaierAccumulator::new();
    let mut total_selection = NeumaierAccumulator::new();
    let mut sector_effects = Vec::with_capacity(sectors.len());

    for (sector, (p, b)) in sectors {
        let r_p = p.rate(p.ret);
        let r_b = b.rate(b.ret);
        let allocation = (p.weight - b.weight) * (r_b - benchmark_return);
        let active_carry = p.weight * (p.rate(p.carry) - b.rate(b.carry));
        let active_treasury = p.weight * (p.rate(p.treasury) - b.rate(b.treasury));
        let active_spread = p.weight * (p.rate(p.spread) - b.rate(b.spread));
        let selection = p.weight * (p.rate(p.selection) - b.rate(b.selection));

        total_allocation.add(allocation);
        total_active_carry.add(active_carry);
        total_active_treasury.add(active_treasury);
        total_active_spread.add(active_spread);
        total_selection.add(selection);

        sector_effects.push(FiSectorEffect {
            sector,
            portfolio_weight: p.weight,
            benchmark_weight: b.weight,
            portfolio_return: r_p,
            benchmark_return: r_b,
            allocation,
            active_carry,
            active_treasury,
            active_spread,
            selection,
            total_active: allocation + active_carry + active_treasury + active_spread + selection,
        });
    }

    Ok(FiAttributionResult {
        sectors: sector_effects,
        portfolio_components,
        benchmark_components,
        portfolio_return,
        benchmark_return,
        active_return: portfolio_return - benchmark_return,
        total_allocation: total_allocation.total(),
        total_active_carry: total_active_carry.total(),
        total_active_treasury: total_active_treasury.total(),
        total_active_spread: total_active_spread.total(),
        total_selection: total_selection.total(),
    })
}

/// One attribution period's raw inputs for multi-period linking.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiPeriodInput {
    /// Portfolio snapshots for the period.
    pub portfolio: Vec<FiPositionSnapshot>,
    /// Benchmark snapshots for the period.
    pub benchmark: Vec<FiPositionSnapshot>,
}

/// Carino-linked per-sector FI effects summed across periods.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiLinkedSectorEffect {
    /// Sector label.
    pub sector: String,
    /// Linked allocation effect.
    pub allocation: f64,
    /// Linked active carry effect.
    pub active_carry: f64,
    /// Linked active treasury effect.
    pub active_treasury: f64,
    /// Linked active spread effect.
    pub active_spread: f64,
    /// Linked selection effect.
    pub selection: f64,
    /// Sum of the five linked effects.
    pub total_active: f64,
}

/// Multi-period Carino-linked Campisi attribution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiCarinoLinkedResult {
    /// Per-period single-period results, in chronological order.
    pub periods: Vec<FiAttributionResult>,
    /// Geometrically compounded portfolio return `∏(1 + r_p,t) − 1`.
    pub portfolio_return_compounded: f64,
    /// Geometrically compounded benchmark return.
    pub benchmark_return_compounded: f64,
    /// Per-sector linked effects; their grand total reconstructs the
    /// compounded active return exactly.
    pub linked_sectors: Vec<FiLinkedSectorEffect>,
    /// Sum of linked allocation effects.
    pub linked_allocation: f64,
    /// Sum of linked active carry effects.
    pub linked_active_carry: f64,
    /// Sum of linked active treasury effects.
    pub linked_active_treasury: f64,
    /// Sum of linked active spread effects.
    pub linked_active_spread: f64,
    /// Sum of linked selection effects.
    pub linked_selection: f64,
}

/// Apply Carino (1999) smoothing to per-period Campisi results so the five
/// arithmetic effects reconstruct the geometrically compounded active return.
///
/// Reuses the smoothing coefficient from [`crate::brinson`] (`k_t / K`
/// rescaling); this function only adapts it to the five-component FI
/// decomposition.
///
/// # Arguments
///
/// * `periods` - Chronologically ordered results with identical sector
///   ordering across periods.
///
/// # Errors
///
/// * [`Error::InvalidInput`] if `periods` is empty, sector ordering differs
///   across periods, any period return is non-finite, or a return is at or
///   below −100 % (Carino domain, see [`crate::brinson::carino_link`]).
pub fn campisi_carino_link(periods: &[FiAttributionResult]) -> Result<FiCarinoLinkedResult> {
    let Some(first) = periods.first() else {
        return Err(Error::invalid_input(
            "Campisi Carino linking requires at least one period",
        ));
    };
    let sector_names: Vec<String> = first.sectors.iter().map(|e| e.sector.clone()).collect();
    for (idx, p) in periods.iter().enumerate().skip(1) {
        let same_order = p.sectors.len() == sector_names.len()
            && p.sectors
                .iter()
                .zip(sector_names.iter())
                .all(|(e, n)| e.sector == *n);
        if !same_order {
            return Err(Error::invalid_input(format!(
                "Campisi Carino linking requires identical sector ordering across all periods \
                 (period {idx} differs from period 0)"
            )));
        }
    }

    let mut compounded_p = 1.0_f64;
    let mut compounded_b = 1.0_f64;
    for p in periods {
        if !p.portfolio_return.is_finite() || !p.benchmark_return.is_finite() {
            return Err(Error::invalid_input(format!(
                "Campisi Carino linking requires finite period returns \
                 (got portfolio_return = {}, benchmark_return = {})",
                p.portfolio_return, p.benchmark_return
            )));
        }
        compounded_p *= 1.0 + p.portfolio_return;
        compounded_b *= 1.0 + p.benchmark_return;
    }
    let r_p_total = compounded_p - 1.0;
    let r_b_total = compounded_b - 1.0;
    let big_k = carino_coefficient(r_p_total, r_b_total)?;

    const N_EFFECTS: usize = 5;
    let mut acc: Vec<[NeumaierAccumulator; N_EFFECTS]> =
        vec![[NeumaierAccumulator::new(); N_EFFECTS]; sector_names.len()];

    for period in periods {
        let k_t = carino_coefficient(period.portfolio_return, period.benchmark_return)?;
        let scale = k_t / big_k;
        for (sector_acc, e) in acc.iter_mut().zip(period.sectors.iter()) {
            sector_acc[0].add(scale * e.allocation);
            sector_acc[1].add(scale * e.active_carry);
            sector_acc[2].add(scale * e.active_treasury);
            sector_acc[3].add(scale * e.active_spread);
            sector_acc[4].add(scale * e.selection);
        }
    }

    let mut linked_sectors = Vec::with_capacity(sector_names.len());
    let mut totals = [NeumaierAccumulator::new(); N_EFFECTS];
    for (name, sector_acc) in sector_names.into_iter().zip(acc) {
        let [allocation, active_carry, active_treasury, active_spread, selection] =
            sector_acc.map(|value| value.total());
        for (total, value) in totals.iter_mut().zip([
            allocation,
            active_carry,
            active_treasury,
            active_spread,
            selection,
        ]) {
            total.add(value);
        }
        linked_sectors.push(FiLinkedSectorEffect {
            sector: name,
            allocation,
            active_carry,
            active_treasury,
            active_spread,
            selection,
            total_active: allocation + active_carry + active_treasury + active_spread + selection,
        });
    }

    let [linked_allocation, linked_active_carry, linked_active_treasury, linked_active_spread, linked_selection] =
        totals.map(|value| value.total());
    Ok(FiCarinoLinkedResult {
        periods: periods.to_vec(),
        portfolio_return_compounded: r_p_total,
        benchmark_return_compounded: r_b_total,
        linked_sectors,
        linked_allocation,
        linked_active_carry,
        linked_active_treasury,
        linked_active_spread,
        linked_selection,
    })
}

/// Compute per-period Campisi attributions and Carino-link them.
///
/// Canonical entry point for bindings that receive raw period snapshots.
///
/// # Arguments
///
/// * `periods` - Chronologically ordered period inputs. Every period must
///   produce the same sector set in the same first-seen order.
/// * `config` - Shared period length.
///
/// # Errors
///
/// Propagates any [`campisi_attribution`] or [`campisi_carino_link`] error.
pub fn campisi_carino_link_from_snapshots(
    periods: &[FiPeriodInput],
    config: &FiAttributionConfig,
) -> Result<FiCarinoLinkedResult> {
    let results = periods
        .iter()
        .map(|p| campisi_attribution(&p.portfolio, &p.benchmark, config))
        .collect::<Result<Vec<_>>>()?;
    campisi_carino_link(&results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn snap(
        sector: &str,
        weight: f64,
        total_return: f64,
        yield_annual: f64,
        modified_duration: f64,
        spread_duration: f64,
        spread: f64,
        delta_treasury_yield: f64,
        delta_spread: f64,
    ) -> FiPositionSnapshot {
        FiPositionSnapshot {
            sector: sector.to_string(),
            weight,
            total_return,
            yield_annual,
            modified_duration,
            spread_duration,
            spread,
            delta_treasury_yield,
            delta_spread,
        }
    }

    /// Hand-worked golden fixture: 2 sectors × 2 positions per side,
    /// quarterly period.
    fn golden_portfolio() -> Vec<FiPositionSnapshot> {
        vec![
            snap("GOVT", 0.30, 0.0155, 0.040, 5.0, 0.0, 0.0, -0.0010, 0.0),
            snap("GOVT", 0.20, 0.0190, 0.045, 8.0, 0.0, 0.0, -0.0010, 0.0),
            snap(
                "CORP", 0.30, 0.0120, 0.060, 4.0, 3.8, 0.0150, -0.0010, 0.0020,
            ),
            snap(
                "CORP", 0.20, 0.0118, 0.070, 6.0, 5.5, 0.0250, -0.0010, 0.0020,
            ),
        ]
    }

    fn golden_benchmark() -> Vec<FiPositionSnapshot> {
        vec![
            snap("GOVT", 0.45, 0.0155, 0.038, 6.0, 0.0, 0.0, -0.0010, 0.0),
            snap("GOVT", 0.15, 0.0195, 0.042, 9.0, 0.0, 0.0, -0.0010, 0.0),
            snap(
                "CORP", 0.25, 0.0090, 0.055, 5.0, 4.8, 0.0120, -0.0010, 0.0020,
            ),
            snap(
                "CORP", 0.15, 0.0100, 0.065, 7.0, 6.5, 0.0200, -0.0010, 0.0020,
            ),
        ]
    }

    fn assert_close(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "{label}: got {actual}, expected {expected}"
        );
    }

    #[test]
    fn campisi_single_period_matches_hand_worked_golden() {
        let config = FiAttributionConfig::new(0.25);
        let r = campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config)
            .expect("valid golden inputs");

        // Headline returns.
        assert_close(r.portfolio_return, 0.01441, "portfolio_return");
        assert_close(r.benchmark_return, 0.01365, "benchmark_return");
        assert_close(r.active_return, 0.00076, "active_return");

        // Absolute Campisi split, portfolio side.
        assert_close(r.portfolio_components.carry, 0.01325, "p carry");
        assert_close(r.portfolio_components.treasury, 0.0055, "p treasury");
        assert_close(r.portfolio_components.spread, -0.00448, "p spread");
        assert_close(r.portfolio_components.selection, 0.00014, "p selection");
        assert_close(r.portfolio_components.total, 0.01441, "p total");

        // Absolute Campisi split, benchmark side.
        assert_close(r.benchmark_components.carry, 0.011725, "b carry");
        assert_close(r.benchmark_components.treasury, 0.00635, "b treasury");
        assert_close(r.benchmark_components.spread, -0.00435, "b spread");
        assert_close(r.benchmark_components.selection, -0.000075, "b selection");

        // Sector effects, in portfolio-first-seen order.
        assert_eq!(r.sectors.len(), 2);
        let govt = &r.sectors[0];
        assert_eq!(govt.sector, "GOVT");
        assert_close(govt.portfolio_weight, 0.50, "govt w_p");
        assert_close(govt.benchmark_weight, 0.60, "govt w_b");
        assert_close(govt.portfolio_return, 0.0169, "govt r_p");
        assert_close(govt.benchmark_return, 0.0165, "govt r_b");
        assert_close(govt.allocation, -0.000285, "govt allocation");
        assert_close(govt.active_carry, 0.000375, "govt active_carry");
        assert_close(govt.active_treasury, -0.000275, "govt active_treasury");
        assert_close(govt.active_spread, 0.0, "govt active_spread");
        assert_close(govt.selection, 0.0001, "govt selection");
        assert_close(govt.total_active, -0.000085, "govt total_active");

        let corp = &r.sectors[1];
        assert_eq!(corp.sector, "CORP");
        assert_close(corp.allocation, -0.0004275, "corp allocation");
        assert_close(corp.active_carry, 0.00065625, "corp active_carry");
        assert_close(corp.active_treasury, -0.000475, "corp active_treasury");
        assert_close(corp.active_spread, 0.0009575, "corp active_spread");
        assert_close(corp.selection, 0.00013375, "corp selection");
        assert_close(corp.total_active, 0.000845, "corp total_active");

        // Totals.
        assert_close(r.total_allocation, -0.0007125, "total_allocation");
        assert_close(r.total_active_carry, 0.00103125, "total_active_carry");
        assert_close(r.total_active_treasury, -0.00075, "total_active_treasury");
        assert_close(r.total_active_spread, 0.0009575, "total_active_spread");
        assert_close(r.total_selection, 0.00023375, "total_selection");

        // Reconciliation by construction.
        let recon = r.reconciliation_check(1e-10);
        assert!(recon.is_reconciled, "residual {}", recon.total_residual);
    }

    /// The five effects must reconstruct active return exactly even when a
    /// sector exists on only one side (zero weight on the other).
    #[test]
    fn campisi_handles_one_sided_sector() {
        let config = FiAttributionConfig::new(0.25);
        let portfolio = vec![
            snap("CORE", 0.80, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0),
            snap(
                "EXTRA", 0.20, 0.0210, 0.070, 3.0, 4.0, 0.0300, -0.0010, -0.0010,
            ),
        ];
        let benchmark = vec![snap(
            "CORE", 1.0, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0,
        )];

        let r = campisi_attribution(&portfolio, &benchmark, &config).expect("valid inputs");
        let reconstructed = r.total_allocation
            + r.total_active_carry
            + r.total_active_treasury
            + r.total_active_spread
            + r.total_selection;
        assert!(
            (reconstructed - r.active_return).abs() < 1e-12,
            "components {reconstructed} must equal active {}",
            r.active_return
        );
        assert!(r.reconciliation_check(1e-10).is_reconciled);

        // The zero-weight `rate` guard exists exactly for this case: a sector
        // genuinely absent from a side. It must keep working after the
        // zero-net-weight fail-closed check was added.
        let extra = &r.sectors[1];
        assert_eq!(extra.sector, "EXTRA");
        assert_close(extra.portfolio_weight, 0.20, "extra w_p");
        assert_close(extra.benchmark_weight, 0.0, "extra w_b");
        assert_close(extra.benchmark_return, 0.0, "extra r_b");
        assert!(
            extra.active_carry.abs() > 0.0,
            "an absent benchmark sector must still produce real portfolio-side effects"
        );
    }

    /// A sector that is *present* on a side but whose positions net to exactly
    /// zero weight (a long/short pair, a CDS hedge against a cash bond in the
    /// same bucket) has no defined per-unit rate. Silently zeroing its five
    /// effects leaves its real contribution `Σ w_j r_j ≠ 0` in the side total,
    /// breaking the telescoping identity while `active_return` still ties out
    /// against performance data. It must fail closed instead.
    #[test]
    fn campisi_rejects_zero_net_weight_sector_on_portfolio_side() {
        let config = FiAttributionConfig::new(0.25);
        let portfolio = vec![
            snap("CORE", 1.00, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0),
            snap("HEDGE", 0.50, 0.0400, 0.060, 3.0, 0.0, 0.0, -0.0010, 0.0),
            snap("HEDGE", -0.50, 0.0100, 0.020, 1.0, 0.0, 0.0, -0.0010, 0.0),
        ];
        let benchmark = vec![snap(
            "CORE", 1.0, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0,
        )];

        let err = campisi_attribution(&portfolio, &benchmark, &config)
            .expect_err("zero-net-weight sector must be rejected");
        let message = err.to_string();
        assert!(
            message.contains("HEDGE"),
            "error must name the sector: {message}"
        );
        assert!(
            message.contains("Portfolio"),
            "error must name the offending side: {message}"
        );
        assert!(
            message.contains("zero"),
            "error must explain the zero-net-weight cause: {message}"
        );
    }

    /// Same failure on the benchmark side, where the reviewer measured a
    /// reported attribution with the *opposite sign* to the actual active
    /// return.
    #[test]
    fn campisi_rejects_zero_net_weight_sector_on_benchmark_side() {
        let config = FiAttributionConfig::new(0.25);
        let portfolio = vec![snap(
            "CORE", 1.0, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0,
        )];
        let benchmark = vec![
            snap("CORE", 1.00, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0),
            snap("HEDGE", 0.40, 0.0300, 0.055, 4.0, 0.0, 0.0, -0.0010, 0.0),
            snap("HEDGE", -0.40, 0.0050, 0.015, 1.0, 0.0, 0.0, -0.0010, 0.0),
        ];

        let err = campisi_attribution(&portfolio, &benchmark, &config)
            .expect_err("zero-net-weight benchmark sector must be rejected");
        let message = err.to_string();
        assert!(
            message.contains("HEDGE"),
            "error must name the sector: {message}"
        );
        assert!(
            message.contains("Benchmark"),
            "error must name the offending side: {message}"
        );
    }

    /// Negative net sector weights are legal (a net-short sector) and must be
    /// attributed by dividing through, not zeroed. Pins the `.abs()` in
    /// [`SideAgg::rate`]'s guard: dropping it silently zeroes the entire
    /// decomposition for every net-short sector.
    #[test]
    fn campisi_attributes_net_short_sector() {
        let config = FiAttributionConfig::new(0.25);
        let portfolio = vec![
            snap("CORE", 1.20, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0),
            snap("SHORT", -0.20, 0.0100, 0.030, 2.0, 0.0, 0.0, -0.0010, 0.0),
        ];
        let benchmark = vec![snap(
            "CORE", 1.0, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0,
        )];

        let r = campisi_attribution(&portfolio, &benchmark, &config).expect("net-short is legal");
        let short = &r.sectors[1];
        assert_eq!(short.sector, "SHORT");
        assert_close(short.portfolio_weight, -0.20, "short w_p");
        // r_p = (−0.2 × 0.0100) / −0.2 = 0.0100; carry rate = 0.030 × 0.25.
        assert_close(short.portfolio_return, 0.0100, "short r_p");
        assert_close(short.active_carry, -0.20 * 0.0075, "short active_carry");
        // −MD·Δy = −2.0 × −0.0010 = 0.0020, benchmark absent.
        assert_close(
            short.active_treasury,
            -0.20 * 0.0020,
            "short active_treasury",
        );
        assert!(r.reconciliation_check(1e-12).is_reconciled);
    }

    #[test]
    fn campisi_rejects_weights_not_summing_to_one() {
        let config = FiAttributionConfig::new(0.25);
        let mut portfolio = golden_portfolio();
        portfolio[0].weight = 0.10; // now sums to 0.80
        let err = campisi_attribution(&portfolio, &golden_benchmark(), &config)
            .expect_err("weights must sum to 1");
        assert!(err.to_string().contains("Portfolio weights"), "{err}");
    }

    /// Pins [`WEIGHT_TOLERANCE`] itself. The test above misses by 0.20, which
    /// still trips under an absurdly loose tolerance; these two cases bracket
    /// the documented ±1e-6 boundary so loosening or tightening it fails here.
    #[test]
    fn campisi_weight_tolerance_is_pinned_at_1e_minus_6() {
        let config = FiAttributionConfig::new(0.25);

        let mut just_over = golden_portfolio();
        just_over[0].weight += 2e-6; // sums to 1 + 2e-6 > tolerance
        let err = campisi_attribution(&just_over, &golden_benchmark(), &config)
            .expect_err("1 + 2e-6 must be rejected at a 1e-6 tolerance");
        assert!(err.to_string().contains("Portfolio weights"), "{err}");

        let mut just_under = golden_portfolio();
        just_under[0].weight += 5e-7; // sums to 1 + 5e-7 < tolerance
        assert!(
            campisi_attribution(&just_under, &golden_benchmark(), &config).is_ok(),
            "1 + 5e-7 must be accepted at a 1e-6 tolerance"
        );
    }

    #[test]
    fn campisi_rejects_non_finite_inputs() {
        let config = FiAttributionConfig::new(0.25);
        let mut portfolio = golden_portfolio();
        portfolio[2].delta_spread = f64::NAN;
        let err = campisi_attribution(&portfolio, &golden_benchmark(), &config)
            .expect_err("NaN must be rejected");
        assert!(err.to_string().contains("finite"), "{err}");
    }

    #[test]
    fn campisi_rejects_bad_period_years() {
        let mut config = FiAttributionConfig::new(0.0);
        let err = campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config)
            .expect_err("period_years must be positive");
        assert!(err.to_string().contains("period_years"), "{err}");
        config.period_years = f64::INFINITY;
        assert!(campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config).is_err());
    }

    #[test]
    fn campisi_rejects_empty_sides() {
        let config = FiAttributionConfig::new(0.25);
        assert!(campisi_attribution(&[], &golden_benchmark(), &config).is_err());
        assert!(campisi_attribution(&golden_portfolio(), &[], &config).is_err());
    }

    #[test]
    fn snapshot_serde_denies_unknown_fields_and_uses_stable_names() {
        let json = r#"{
            "sector": "CORP", "weight": 0.5, "total_return": 0.01,
            "yield_annual": 0.06, "modified_duration": 4.0,
            "spread_duration": 3.8, "spread": 0.015,
            "delta_treasury_yield": -0.001, "delta_spread": 0.002
        }"#;
        let snap: FiPositionSnapshot = serde_json::from_str(json).expect("stable names");
        assert_eq!(snap.sector, "CORP");

        let bad = r#"{
            "sector": "CORP", "weight": 0.5, "total_return": 0.01,
            "yield_annual": 0.06, "modified_duration": 4.0,
            "spread_duration": 3.8, "spread": 0.015,
            "delta_treasury_yield": -0.001, "delta_spread": 0.002,
            "surprise": 1.0
        }"#;
        assert!(serde_json::from_str::<FiPositionSnapshot>(bad).is_err());

        let config: FiAttributionConfig =
            serde_json::from_str(r#"{"period_years": 0.25}"#).expect("config parses");
        assert!((config.period_years - 0.25).abs() < 1e-15);

        // `period_years` is the config's only field and carries no serde
        // default: an empty object must fail rather than assume a period.
        assert!(serde_json::from_str::<FiAttributionConfig>("{}").is_err());

        // The removed `spread_mode` key must now fail closed as an unknown
        // field, so a stale caller is told rather than silently ignored.
        assert!(serde_json::from_str::<FiAttributionConfig>(
            r#"{"period_years": 0.25, "spread_mode": "dts"}"#
        )
        .is_err());
    }

    /// The spread effect is `−SD · Δs` and never touches the spread *level*,
    /// so the level is inert: zero and negative spreads must be accepted and
    /// must not perturb any effect. This replaces the old pair of DTS-mode
    /// tests (mode-equivalence, and the DTS rejection of a non-positive
    /// spread), which pinned a mode that no longer exists.
    #[test]
    fn campisi_spread_level_is_inert_and_may_be_zero_or_negative() {
        let config = FiAttributionConfig::new(0.25);
        let build = |p_spread: f64, b_spread: f64| {
            (
                vec![snap(
                    "CORP", 1.0, 0.0120, 0.060, 4.0, 3.8, p_spread, -0.0010, 0.0020,
                )],
                vec![snap(
                    "CORP", 1.0, 0.0090, 0.055, 5.0, 4.8, b_spread, -0.0010, 0.0020,
                )],
            )
        };

        // Baseline: ordinary positive spread levels.
        let (portfolio, benchmark) = build(0.0150, 0.0120);
        let base = campisi_attribution(&portfolio, &benchmark, &config).expect("positive spreads");
        // w_p (−SD_p Δs − (−SD_b Δs)) = 1.0 × (−3.8 + 4.8) × 0.0020.
        assert_close(base.total_active_spread, 0.0020, "baseline active_spread");

        // A zero spread on the portfolio side with a non-zero SD × Δs term is
        // exactly what the old DTS mode rejected. It is now a legal input.
        let (zero_p, zero_b) = build(0.0, 0.0120);
        let zero = campisi_attribution(&zero_p, &zero_b, &config).expect("zero spread is legal");

        // Negative spread levels are real (Bund asset swaps, negative OAS).
        let (neg_p, neg_b) = build(-0.0025, -0.0040);
        let negative =
            campisi_attribution(&neg_p, &neg_b, &config).expect("negative spread is legal");

        for (label, r) in [("zero", &zero), ("negative", &negative)] {
            assert_close(
                r.total_active_spread,
                base.total_active_spread,
                &format!("{label} active_spread must match the baseline"),
            );
            assert_close(
                r.total_selection,
                base.total_selection,
                &format!("{label} selection must match the baseline"),
            );
            assert!(r.reconciliation_check(1e-10).is_reconciled);
        }
    }

    /// Carino linking must rescale the five FI effects so their linked sum
    /// reconstructs the geometric compounded active return exactly; with two
    /// identical periods every linked component is its arithmetic two-period
    /// sum times the uniform scale `geometric_active / arithmetic_active`.
    #[test]
    fn campisi_carino_link_matches_compounded_active_return() {
        let config = FiAttributionConfig::new(0.25);
        let period = FiPeriodInput {
            portfolio: golden_portfolio(),
            benchmark: golden_benchmark(),
        };
        let linked = campisi_carino_link_from_snapshots(&[period.clone(), period], &config)
            .expect("carino link");

        // Compounded returns (hand-worked): 1.01441^2 − 1, 1.01365^2 − 1.
        let rp = 1.01441_f64.powi(2) - 1.0;
        let rb = 1.01365_f64.powi(2) - 1.0;
        assert!((linked.portfolio_return_compounded - rp).abs() < 1e-12);
        assert!((linked.benchmark_return_compounded - rb).abs() < 1e-12);

        let geometric_active = rp - rb;
        let reconstructed = linked.linked_allocation
            + linked.linked_active_carry
            + linked.linked_active_treasury
            + linked.linked_active_spread
            + linked.linked_selection;
        assert!(
            (reconstructed - geometric_active).abs() < 1e-10,
            "linked effects {reconstructed} must equal geometric active {geometric_active}"
        );

        // Smoothing must not be a no-op: arithmetic ≠ geometric here.
        let arithmetic_active = 2.0 * 0.00076;
        assert!((arithmetic_active - geometric_active).abs() > 1e-7);

        // Identical periods ⇒ identical k_t ⇒ one uniform scale factor.
        let scale = geometric_active / arithmetic_active;
        assert!(
            (linked.linked_active_spread - 2.0 * 0.0009575 * scale).abs() < 1e-12,
            "linked_active_spread must be uniformly Carino-scaled"
        );
        assert!((linked.linked_allocation - 2.0 * -0.0007125 * scale).abs() < 1e-12);

        // Sector ordering preserved.
        let names: Vec<&str> = linked
            .linked_sectors
            .iter()
            .map(|s| s.sector.as_str())
            .collect();
        assert_eq!(names, ["GOVT", "CORP"]);

        // Every per-sector linked effect, field by field, against the golden
        // single-period value × 2 periods × the uniform Carino scale. Without
        // these the grand totals stay correct even if the per-sector record
        // transposes two effects or drops one from `total_active`, because the
        // totals accumulate from the pre-record locals.
        let golden: [(&str, [f64; 5]); 2] = [
            ("GOVT", [-0.000285, 0.000375, -0.000275, 0.0, 0.0001]),
            (
                "CORP",
                [-0.0004275, 0.00065625, -0.000475, 0.0009575, 0.00013375],
            ),
        ];
        for (linked_sector, (name, effects)) in linked.linked_sectors.iter().zip(golden) {
            assert_eq!(linked_sector.sector, name);
            let [allocation, carry, treasury, spread, selection] =
                effects.map(|value| 2.0 * value * scale);
            assert_close(
                linked_sector.allocation,
                allocation,
                &format!("{name} linked allocation"),
            );
            assert_close(
                linked_sector.active_carry,
                carry,
                &format!("{name} linked active_carry"),
            );
            assert_close(
                linked_sector.active_treasury,
                treasury,
                &format!("{name} linked active_treasury"),
            );
            assert_close(
                linked_sector.active_spread,
                spread,
                &format!("{name} linked active_spread"),
            );
            assert_close(
                linked_sector.selection,
                selection,
                &format!("{name} linked selection"),
            );
            assert_close(
                linked_sector.total_active,
                allocation + carry + treasury + spread + selection,
                &format!("{name} linked total_active"),
            );
        }
    }

    #[test]
    fn campisi_carino_link_rejects_empty_and_inconsistent_periods() {
        assert!(campisi_carino_link(&[]).is_err());

        let config = FiAttributionConfig::new(0.25);
        let p1 = campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config)
            .expect("period 1");
        let mut p2 = p1.clone();
        p2.sectors[0].sector = "DIFFERENT".to_string();
        let err = campisi_carino_link(&[p1, p2]).expect_err("sector ordering must match");
        assert!(err.to_string().contains("sector ordering"), "{err}");
    }

    #[test]
    fn snapshot_from_position_metrics_reads_fi_metrics() {
        let mut metrics = indexmap::IndexMap::new();
        metrics.insert("ytm".to_string(), 0.060);
        metrics.insert("duration_mod".to_string(), 4.0);
        metrics.insert("spread_duration".to_string(), 3.8);
        metrics.insert("z_spread".to_string(), 0.0150);
        let position_metrics = crate::metrics::PositionMetrics {
            currency: finstack_quant_core::currency::Currency::USD,
            metrics,
        };

        let snap = snapshot_from_position_metrics(
            &position_metrics,
            "CORP",
            0.30,
            0.0120,
            -0.0010,
            0.0020,
            "z_spread",
        )
        .expect("all metrics present");

        assert_eq!(snap.sector, "CORP");
        assert!((snap.yield_annual - 0.060).abs() < 1e-15);
        assert!((snap.modified_duration - 4.0).abs() < 1e-15);
        assert!((snap.spread_duration - 3.8).abs() < 1e-15);
        assert!((snap.spread - 0.0150).abs() < 1e-15);
        assert!((snap.weight - 0.30).abs() < 1e-15);
        assert!((snap.total_return - 0.0120).abs() < 1e-15);
        // `delta_treasury_yield` and `delta_spread` are adjacent bare `f64`
        // parameters; without these two assertions a transposition in the
        // struct literal silently reassigns return between the treasury and
        // spread effects while the Campisi identity still reconciles.
        assert!((snap.delta_treasury_yield - (-0.0010)).abs() < 1e-15);
        assert!((snap.delta_spread - 0.0020).abs() < 1e-15);
    }

    #[test]
    fn snapshot_from_position_metrics_names_missing_metric() {
        let position_metrics = crate::metrics::PositionMetrics {
            currency: finstack_quant_core::currency::Currency::USD,
            metrics: indexmap::IndexMap::new(),
        };
        let err = snapshot_from_position_metrics(
            &position_metrics,
            "CORP",
            0.30,
            0.0120,
            -0.0010,
            0.0020,
            "z_spread",
        )
        .expect_err("missing metrics must be named");
        assert!(err.to_string().contains("ytm"), "{err}");
    }

    /// The empty-map case above cannot distinguish a message that interpolates
    /// the missing ID from one that hardcodes `"ytm"`, because `"ytm"` is the
    /// first metric checked. Supply every fixed metric and let only the
    /// caller-chosen spread metric be absent, so the message must name *that*
    /// ID.
    #[test]
    fn snapshot_from_position_metrics_names_missing_spread_metric() {
        let mut metrics = indexmap::IndexMap::new();
        metrics.insert("ytm".to_string(), 0.060);
        metrics.insert("duration_mod".to_string(), 4.0);
        metrics.insert("spread_duration".to_string(), 3.8);
        let position_metrics = crate::metrics::PositionMetrics {
            currency: finstack_quant_core::currency::Currency::USD,
            metrics,
        };
        let err = snapshot_from_position_metrics(
            &position_metrics,
            "CORP",
            0.30,
            0.0120,
            -0.0010,
            0.0020,
            "oas",
        )
        .expect_err("absent spread metric must be named");
        let message = err.to_string();
        assert!(
            message.contains("oas"),
            "error must name the absent spread metric, got: {message}"
        );
        assert!(
            !message.contains("ytm"),
            "error must not name a metric that is present, got: {message}"
        );
    }

    /// [`FiAttributionResult`] is an *input* to [`campisi_carino_link`] and to
    /// both bindings, so it must round-trip its own serialization exactly and
    /// reject unknown keys — a stale or misspelled field must not be silently
    /// dropped into a result that then "reconciles" on partial data.
    #[test]
    fn attribution_result_round_trips_and_denies_unknown_fields() {
        let config = FiAttributionConfig::new(0.25);
        let result = campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config)
            .expect("golden inputs");
        let json = serde_json::to_string(&result).expect("serialize result");

        // Round-trip: `deny_unknown_fields` must not reject our own output.
        let parsed: FiAttributionResult = serde_json::from_str(&json).expect("round-trip");
        assert!((parsed.active_return - result.active_return).abs() < 1e-15);
        assert_eq!(parsed.sectors.len(), result.sectors.len());
        // And it still links, i.e. the round-tripped value is fully usable.
        assert!(campisi_carino_link(&[parsed]).is_ok());

        // Top-level unknown key.
        let bogus = json.replacen('{', r#"{"bogus_field": 1.0,"#, 1);
        assert!(
            serde_json::from_str::<FiAttributionResult>(&bogus).is_err(),
            "unknown top-level field must be rejected"
        );

        // Nested unknown keys in the two inbound child types.
        let bogus_sector = json.replacen(r#"{"sector""#, r#"{"bogus_field": 1.0, "sector""#, 1);
        assert_ne!(bogus_sector, json, "fixture must actually mutate a sector");
        assert!(
            serde_json::from_str::<FiAttributionResult>(&bogus_sector).is_err(),
            "unknown field inside FiSectorEffect must be rejected"
        );
        let bogus_components = json.replacen(r#""carry""#, r#""bogus_field": 1.0, "carry""#, 1);
        assert_ne!(
            bogus_components, json,
            "fixture must actually mutate a components block"
        );
        assert!(
            serde_json::from_str::<FiAttributionResult>(&bogus_components).is_err(),
            "unknown field inside FiComponents must be rejected"
        );
    }

    /// [`FiAttributionResult::reconciliation_check`] is reachable from the
    /// bindings as JSON, so its report must serialize under stable names.
    #[test]
    fn reconciliation_report_serializes_under_stable_names() {
        let config = FiAttributionConfig::new(0.25);
        let result = campisi_attribution(&golden_portfolio(), &golden_benchmark(), &config)
            .expect("golden inputs");
        let json =
            serde_json::to_string(&result.reconciliation_check(1e-10)).expect("serialize report");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse report");
        assert_eq!(value["is_reconciled"], serde_json::json!(true));
        assert_eq!(value["tolerance"], serde_json::json!(1e-10));
        assert!(value["total_residual"].as_f64().expect("residual").abs() <= 1e-10);
    }
}
