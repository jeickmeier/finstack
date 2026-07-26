//! Campisi-style benchmark-relative fixed-income attribution.
//!
//! # Single-period decomposition
//!
//! Each position's period return is decomposed (Campisi 2000) into:
//!
//! ```text
//! carry_j     = yield_annual_j × Δt                    // income effect
//! treasury_j  = −MD_j × Δy_tsy,j                       // duration / curve effect
//! spread_j    = −SD_j × Δs_j                           // SpreadDuration mode
//!             = −(SD_j × s_j) × (Δs_j / s_j)           // Dts mode (Ben Dor et al. 2007)
//! selection_j = r_j − carry_j − treasury_j − spread_j  // residual
//! ```
//!
//! With exact inputs the two spread conventions coincide per position; the
//! mode governs fail-closed validation (Dts requires a positive spread
//! whenever `SD × Δs ≠ 0`) and is stamped into the result so downstream
//! consumers know which convention produced the numbers.
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
//! * Ben Dor, A., Dynkin, L., Hyman, J., Houweling, P., van Leeuwen, E., &
//!   Penninga, O. (2007). "DTS (Duration Times Spread)." *Journal of
//!   Portfolio Management*, 33(2), 77–100.

use crate::brinson::carino_coefficient;
use crate::error::{Error, Result};
use finstack_quant_core::math::summation::NeumaierAccumulator;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Tolerance for the requirement that weights sum to 1.0 on each side.
const WEIGHT_TOLERANCE: f64 = 1e-6;

/// Convention used for the spread component of the Campisi decomposition.
///
/// Both conventions produce identical numbers when `spread` and
/// `delta_spread` are exact (`−SD·Δs ≡ −(SD·s)·(Δs/s)`); the mode selects the
/// documented convention, is stamped into [`FiAttributionResult`], and in
/// [`SpreadChangeMode::Dts`] enforces a positive spread whenever the spread
/// term is non-zero (Ben Dor et al. 2007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadChangeMode {
    /// Spread effect `−spread_duration × delta_spread` (absolute change).
    SpreadDuration,
    /// Spread effect `−DTS × (delta_spread / spread)` with
    /// `DTS = spread_duration × spread` (relative change).
    Dts,
}

/// Configuration for [`campisi_attribution`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiAttributionConfig {
    /// Length of the attribution period in years (e.g. `0.25` for a
    /// quarter). Scales `yield_annual` into the period carry.
    pub period_years: f64,
    /// Spread-effect convention.
    pub spread_mode: SpreadChangeMode,
}

impl FiAttributionConfig {
    /// Create a config with the given period length and the default
    /// [`SpreadChangeMode::SpreadDuration`] convention.
    ///
    /// # Arguments
    ///
    /// * `period_years` - Attribution period length in years; must be finite
    ///   and positive.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_portfolio::fi_attribution::{FiAttributionConfig, SpreadChangeMode};
    ///
    /// let config = FiAttributionConfig::new(0.25);
    /// assert_eq!(config.period_years, 0.25);
    /// assert_eq!(config.spread_mode, SpreadChangeMode::SpreadDuration);
    /// ```
    pub fn new(period_years: f64) -> Self {
        Self {
            period_years,
            spread_mode: SpreadChangeMode::SpreadDuration,
        }
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
pub struct FiComponents {
    /// Income effect `Σ w · y · Δt`.
    pub carry: f64,
    /// Treasury/duration effect `Σ w · (−MD · Δy)`.
    pub treasury: f64,
    /// Spread effect (per [`SpreadChangeMode`]).
    pub spread: f64,
    /// Residual selection `Σ w · (r − explained)`.
    pub selection: f64,
    /// Sum of the four components — equals the side's total return.
    pub total: f64,
}

/// Per-sector benchmark-relative effects.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Spread convention that produced this result.
    pub spread_mode: SpreadChangeMode,
}

/// Report from reconciling the five effect totals against the active return,
/// mirroring [`crate::attribution`] reconciliation conventions.
#[derive(Clone, Debug)]
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
fn position_components(
    s: &FiPositionSnapshot,
    config: &FiAttributionConfig,
) -> Result<(f64, f64, f64)> {
    let carry = s.yield_annual * config.period_years;
    let treasury = -s.modified_duration * s.delta_treasury_yield;
    let spread = match config.spread_mode {
        SpreadChangeMode::SpreadDuration => -s.spread_duration * s.delta_spread,
        SpreadChangeMode::Dts => {
            if s.spread > 0.0 {
                let dts = s.spread_duration * s.spread;
                -dts * (s.delta_spread / s.spread)
            } else if (s.spread_duration * s.delta_spread).abs() > 0.0 {
                return Err(Error::invalid_input(format!(
                    "DTS spread mode requires a positive spread for sector '{}' \
                     (got spread = {}, spread_duration = {}, delta_spread = {})",
                    s.sector, s.spread, s.spread_duration, s.delta_spread
                )));
            } else {
                0.0
            }
        }
    };
    Ok((carry, treasury, spread))
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
#[derive(Clone, Copy, Default)]
struct SideAgg {
    weight: f64,
    ret: f64,
    carry: f64,
    treasury: f64,
    spread: f64,
    selection: f64,
}

/// Sector rates: weighted contribution ÷ sector weight (0 if empty side).
impl SideAgg {
    fn rate(contribution: f64, weight: f64) -> f64 {
        if weight.abs() > 0.0 {
            contribution / weight
        } else {
            0.0
        }
    }
}

/// Accumulate one side into per-sector aggregates and side totals.
///
/// Returns `(side_return, side_components)` and fills `sectors` (union map,
/// first-seen order preserved).
fn aggregate_side(
    snapshots: &[FiPositionSnapshot],
    config: &FiAttributionConfig,
    side: &str,
    sectors: &mut IndexMap<String, (SideAgg, SideAgg)>,
    is_portfolio: bool,
) -> Result<(f64, FiComponents)> {
    let mut sum_w = NeumaierAccumulator::new();
    let mut sum_r = NeumaierAccumulator::new();
    let mut sum_carry = NeumaierAccumulator::new();
    let mut sum_tsy = NeumaierAccumulator::new();
    let mut sum_spr = NeumaierAccumulator::new();
    let mut sum_sel = NeumaierAccumulator::new();

    for s in snapshots {
        validate_snapshot(s, side)?;
        let (carry, treasury, spread) = position_components(s, config)?;
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
        agg.ret += s.weight * s.total_return;
        agg.carry += s.weight * carry;
        agg.treasury += s.weight * treasury;
        agg.spread += s.weight * spread;
        agg.selection += s.weight * selection;
    }

    let total_w = sum_w.total();
    if (total_w - 1.0).abs() > WEIGHT_TOLERANCE {
        let side_name = if is_portfolio {
            "Portfolio"
        } else {
            "Benchmark"
        };
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
/// (Campisi 2000; Ben Dor et al. 2007 for the DTS spread convention), then
/// buckets by sector and splits the active return into allocation plus four
/// active component effects using Brinson-Fachler sign conventions (see the
/// module docs for the exact formulas and the reconciliation proof).
///
/// A sector missing from one side is treated with zero weight on that side,
/// so the decomposition stays complete.
///
/// # Arguments
///
/// * `portfolio` - Portfolio position/bucket snapshots; weights must sum to 1.
/// * `benchmark` - Benchmark snapshots; weights must sum to 1.
/// * `config` - Period length and spread convention.
///
/// # Errors
///
/// * [`Error::InvalidInput`] if either side is empty, any value is
///   non-finite, weights don't sum to 1.0 (±1e-6), `period_years` is not
///   finite and positive, or (Dts mode) a snapshot has non-positive spread
///   with a non-zero spread term.
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
        aggregate_side(portfolio, config, "portfolio", &mut sectors, true)?;
    let (benchmark_return, benchmark_components) =
        aggregate_side(benchmark, config, "benchmark", &mut sectors, false)?;

    let mut total_allocation = NeumaierAccumulator::new();
    let mut total_active_carry = NeumaierAccumulator::new();
    let mut total_active_treasury = NeumaierAccumulator::new();
    let mut total_active_spread = NeumaierAccumulator::new();
    let mut total_selection = NeumaierAccumulator::new();
    let mut sector_effects = Vec::with_capacity(sectors.len());

    for (sector, (p, b)) in sectors {
        let r_p = SideAgg::rate(p.ret, p.weight);
        let r_b = SideAgg::rate(b.ret, b.weight);
        let allocation = (p.weight - b.weight) * (r_b - benchmark_return);
        let active_carry =
            p.weight * (SideAgg::rate(p.carry, p.weight) - SideAgg::rate(b.carry, b.weight));
        let active_treasury =
            p.weight * (SideAgg::rate(p.treasury, p.weight) - SideAgg::rate(b.treasury, b.weight));
        let active_spread =
            p.weight * (SideAgg::rate(p.spread, p.weight) - SideAgg::rate(b.spread, b.weight));
        let selection = p.weight
            * (SideAgg::rate(p.selection, p.weight) - SideAgg::rate(b.selection, b.weight));

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
        spread_mode: config.spread_mode,
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
///   ordering and identical [`SpreadChangeMode`] across periods.
///
/// # Errors
///
/// * [`Error::InvalidInput`] if `periods` is empty, sector ordering or spread
///   mode differs across periods, any period return is non-finite, or a
///   return is at or below −100 % (Carino domain, see
///   [`crate::brinson::carino_link`]).
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
        if p.spread_mode != first.spread_mode {
            return Err(Error::invalid_input(format!(
                "Campisi Carino linking requires a consistent spread mode \
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
        let values: Vec<f64> = sector_acc.iter().map(|a| a.total()).collect();
        let mut effect = FiLinkedSectorEffect {
            sector: name,
            allocation: 0.0,
            active_carry: 0.0,
            active_treasury: 0.0,
            active_spread: 0.0,
            selection: 0.0,
            total_active: 0.0,
        };
        if let [alloc, carry, tsy, spr, sel] = values.as_slice() {
            effect.allocation = *alloc;
            effect.active_carry = *carry;
            effect.active_treasury = *tsy;
            effect.active_spread = *spr;
            effect.selection = *sel;
            effect.total_active = alloc + carry + tsy + spr + sel;
            for (acc, v) in totals.iter_mut().zip(values.iter()) {
                acc.add(*v);
            }
        }
        linked_sectors.push(effect);
    }

    let mut totals_iter = totals.iter().map(|a| a.total());
    Ok(FiCarinoLinkedResult {
        periods: periods.to_vec(),
        portfolio_return_compounded: r_p_total,
        benchmark_return_compounded: r_b_total,
        linked_sectors,
        linked_allocation: totals_iter.next().unwrap_or(0.0),
        linked_active_carry: totals_iter.next().unwrap_or(0.0),
        linked_active_treasury: totals_iter.next().unwrap_or(0.0),
        linked_active_spread: totals_iter.next().unwrap_or(0.0),
        linked_selection: totals_iter.next().unwrap_or(0.0),
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
/// * `config` - Shared period length and spread convention.
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
    /// quarterly period. Every expected value below is derived line-by-line
    /// in docs/superpowers/plans/2026-07-24-fi-benchmark-attribution.md.
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
            serde_json::from_str(r#"{"period_years": 0.25, "spread_mode": "spread_duration"}"#)
                .expect("config parses");
        assert!(matches!(
            config.spread_mode,
            SpreadChangeMode::SpreadDuration
        ));
    }

    /// With exact `spread` and `delta_spread` inputs, the DTS convention
    /// −DTS·(Δs/s) is algebraically identical to −SD·Δs (Ben Dor et al.
    /// 2007), so on all-positive-spread data both modes must agree to
    /// floating-point round-off.
    #[test]
    fn dts_mode_matches_spread_duration_mode_on_positive_spreads() {
        let portfolio = vec![
            snap(
                "CORP", 0.60, 0.0120, 0.060, 4.0, 3.8, 0.0150, -0.0010, 0.0020,
            ),
            snap("HY", 0.40, 0.0118, 0.070, 6.0, 5.5, 0.0250, -0.0010, 0.0020),
        ];
        let benchmark = vec![
            snap(
                "CORP", 0.50, 0.0090, 0.055, 5.0, 4.8, 0.0120, -0.0010, 0.0020,
            ),
            snap("HY", 0.50, 0.0100, 0.065, 7.0, 6.5, 0.0200, -0.0010, 0.0020),
        ];

        let sd_config = FiAttributionConfig::new(0.25);
        let mut dts_config = FiAttributionConfig::new(0.25);
        dts_config.spread_mode = SpreadChangeMode::Dts;

        let sd = campisi_attribution(&portfolio, &benchmark, &sd_config).expect("sd mode");
        let dts = campisi_attribution(&portfolio, &benchmark, &dts_config).expect("dts mode");

        assert!((sd.total_active_spread - dts.total_active_spread).abs() < 1e-14);
        assert!((sd.total_selection - dts.total_selection).abs() < 1e-14);
        assert!(matches!(dts.spread_mode, SpreadChangeMode::Dts));
        assert!(matches!(sd.spread_mode, SpreadChangeMode::SpreadDuration));
        assert!(dts.reconciliation_check(1e-10).is_reconciled);
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

    /// DTS mode must fail closed when a snapshot carries a non-zero spread
    /// term but a non-positive spread — silently substituting zero would
    /// misclassify spread P&L as selection.
    #[test]
    fn dts_mode_rejects_nonpositive_spread_with_nonzero_spread_term() {
        let mut config = FiAttributionConfig::new(0.25);
        config.spread_mode = SpreadChangeMode::Dts;
        let portfolio = vec![snap("CORP", 1.0, 0.01, 0.05, 4.0, 3.8, 0.0, -0.001, 0.002)];
        let benchmark = vec![snap(
            "CORP", 1.0, 0.01, 0.05, 4.0, 3.8, 0.0100, -0.001, 0.002,
        )];

        let err = campisi_attribution(&portfolio, &benchmark, &config)
            .expect_err("zero spread with non-zero SD×Δs must fail in DTS mode");
        assert!(err.to_string().contains("DTS"), "{err}");

        // Treasuries (SD = 0, Δs = 0) remain fine in DTS mode.
        let portfolio_ok = vec![snap("GOVT", 1.0, 0.01, 0.04, 5.0, 0.0, 0.0, -0.001, 0.0)];
        let benchmark_ok = vec![snap("GOVT", 1.0, 0.01, 0.04, 5.5, 0.0, 0.0, -0.001, 0.0)];
        assert!(campisi_attribution(&portfolio_ok, &benchmark_ok, &config).is_ok());
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
}
