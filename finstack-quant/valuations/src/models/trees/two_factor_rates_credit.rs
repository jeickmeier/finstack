//! Two-factor binomial tree: short rate + credit hazard (intensity).
//!
//! Models the joint evolution of the risk-free short rate and the credit hazard
//! rate using correlated binomial moves. Both factors are **calibrated** to their
//! respective market curves (discount curve for rates, hazard curve for credit)
//! via independent Arrow-Debreu forward induction, analogous to Ho-Lee calibration.
//!
//! # Volatility regimes
//!
//! One lattice covers all four regimes; there is no separate rate-only or
//! hazard-only tree, and no second model key. Each factor is deterministic
//! when its volatility is zero (`None` and `0.0` are equivalent), and the
//! curves are repriced exactly either way:
//!
//! | `rate_vol` | `hazard_vol` | Behavior |
//! | --- | --- | --- |
//! | zero | zero | Deterministic rates and deterministic credit |
//! | nonzero | zero | Stochastic rates, deterministic credit |
//! | zero | nonzero | Deterministic rates, stochastic credit |
//! | nonzero | nonzero | Correlated stochastic rates and credit |
//!
//! Public configuration reaches these through
//! [`resolve_rates_credit_config`], which reads `hw1f_sigma`,
//! `hazard_volatility`, the two mean reversions, and
//! `rate_credit_correlation` from an instrument's `ModelConfig`.
//!
//! # Units
//!
//! Both volatilities are **absolute** (normal), on the same scale as the
//! factor itself: `rate_vol = 0.01` is 100 bp/yr of short rate, and
//! `hazard_vol = 0.02` is 2 percentage points of hazard per √year. A hazard
//! volatility is **not** a relative credit-spread volatility — a CDS-option
//! quote such as `0.35` is roughly an order of magnitude too large to use
//! directly. Convert with
//! [`models::credit::market_anchored`](crate::models::credit::market_anchored):
//! `σ_λ = σ_fractional · λ_ref`. Heavy
//! [`HazardFloorSaturation`] is the symptom of having skipped that step.
//!
//! # Lattice convention
//!
//! Each factor evolves on an **additive normal** binomial lattice: from a node
//! with value `x`, the up move is `x + σ√Δt` and the down move is `x - σ√Δt`,
//! where `σ` is the factor's annualized **normal** volatility. The node spacing
//! `σ√Δt` is therefore in **absolute rate units** (rate per year), not log-rate
//! units. The lattice recombines: row `k` is uniformly spaced by `2σ√Δt`.
//!
//! # Mean reversion — accuracy limits
//!
//! When a factor's mean-reversion speed `κ` is non-zero, the one-step transition
//! probability is moment-matched to a mean-reverting drift. The drift
//! `μ = −κ·(x − x_ref)` and the lattice spacing `σ√Δt` are **both in absolute
//! rate units**, so the moment-matched up-probability is dimensionally coherent:
//!
//! ```text
//! p_up = ½ + μ·√Δt / (2σ) = ½ − κ·(x − x_ref)·√Δt / (2σ)
//! ```
//!
//! Here `x_ref` is the factor's calibrated initial instantaneous rate `x₀`
//! (the level the input curve implies at `t = 0`). Because **calibration and
//! pricing use the identical per-node probability**, the forward Arrow-Debreu
//! recursion and the backward induction remain exact duals and the tree
//! **reprices the input curves exactly for any κ** — the Ho-Lee theta absorbs
//! the first-moment bias entirely.
//!
//! **However**, an additive binomial lattice has only ONE free parameter (`p`),
//! so it can match either the conditional mean **or** the conditional variance,
//! not both. This implementation matches the mean. The conditional variance of
//! one step is `σ²Δt · 4p(1−p)`, which collapses as `p` moves away from ½.
//! Consequence: **option-value accuracy degrades as κ grows**. Concretely, a
//! node near the reversion reference at `κ = 0.15` retains roughly 80 % of the
//! intended conditional variance; by `κ = 0.3` only ~55 % remains. This tree
//! prices callable bonds and term loans, so option-value accuracy matters.
//!
//! For accurate mean-reverting optionality, use [`HullWhiteTree`] (the
//! Hull-White trinomial tree in the same module), which does not have this
//! variance-collapse limitation.
//!
//! This implementation enforces `κ ≤ 0.15` for each factor via a
//! [`Error::Validation`] returned from `calibrate()`. See
//! [`KAPPA_MAX`] for the threshold and its justification.
//!
//! (The earlier implementation calibrated with `p = ½` but priced with a
//! mean-reversion-dependent probability, so the tree no longer repriced the
//! discount curve; it also mixed a *log-rate* drift with the *absolute-rate*
//! lattice spacing, which was dimensionally incoherent.)
//!
//! # Calibration
//!
//! `calibrate()` must be called before `price()`. The calibration ensures:
//! - Tree-implied zero-coupon bond prices match the discount curve at every step
//! - Tree-implied survival probabilities match the hazard curve at every step
//!
//! The hazard factor is additive-normal, so low nodes can go negative. A
//! negative hazard is not a credit state, so node hazards pass through a
//! non-negative transform — **in calibration and in backward induction
//! alike**. Sharing one transform is what keeps the forward Arrow-Debreu
//! recursion and the backward induction exact duals, so the survival curve the
//! valuator sees is the one that was calibrated. The floored step equation is
//! non-linear in the per-step shift, so that shift is bracketed and bisected
//! rather than read off in closed form. Where the floor binds, the realized
//! hazard dispersion is smaller than `hazard_vol` asks;
//! [`RatesCreditTree::hazard_floor_saturation`] reports how much.
//!
//! Correlation feasibility is settled at calibration time. Two Bernoulli
//! marginals only admit correlations inside their Fréchet bounds, and mean
//! reversion skews the corner nodes' marginals away from ½, so a large `|ρ|`
//! can be unattainable. `calibrate()` rejects such a request with the
//! offending node and the lattice-wide maximum; the same figure is available
//! up front from [`RatesCreditTree::max_feasible_correlation`]. Pricing
//! therefore never meets an infeasible node and never silently alters a
//! marginal to accommodate one.
//!
//! # OAS
//!
//! Option-adjusted spread is read from `initial_vars["oas"]` (basis points) and
//! applied as a parallel shift to calibrated short rates during backward induction.
//! This matches the `ShortRateTree` convention.
//!
//! # Node-dependent floating resets
//!
//! A floating coupon whose index fixes in the future re-fixes off the rate
//! node it meets, not off today's curve. [`RatesCreditTree::price_with_node_coupons`]
//! values that dependence with **two distinct operators**:
//!
//! 1. **Forward derivation** ([`RatesCreditTree::conditional_discount_factors`]):
//!    the one-period node forward comes from tree-conditional risk-free
//!    discount factors built from the **raw calibrated rate nodes** — no OAS,
//!    no survival, no positive floor. Node forwards are a property of the
//!    calibrated risk-free lattice; the OAS never moves them.
//! 2. **Pricing-measure folding**: the coupon's node-dependent *increment*
//!    over its deterministic projection is folded into continuation at the
//!    reset slice using exactly the discounting backward induction applies —
//!    raw rate **plus** OAS, floored-hazard survival, and the correlated
//!    joint transitions.
//!
//! The deterministic projection of every coupon stays booked in the
//! valuator's cashflow vectors unchanged; only the increment is
//! node-dependent, so the stochastic path collapses onto today's projected
//! cashflows as `rate_vol → 0` by construction. For an option-free FRN with
//! zero rate/credit correlation the folded increment prices to zero
//! *exactly* (the classic result that a floater's forward-set leg is worth
//! its curve projection; cf. Tuckman & Serrat, *Fixed Income Securities*,
//! ch. 2 on floaters pricing to par), which the tests assert on the lattice.
//!
//! [`HullWhiteTree`]: super::hull_white_tree::HullWhiteTree

use crate::cashflow::builder::rate_helpers::{calculate_floating_rate, FloatingRateParams};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::HazardCurve;
use finstack_quant_core::market_data::traits::Discounting;
use finstack_quant_core::HashMap;
use finstack_quant_core::{Error, Result};

use super::tree_framework::{CachedValues, NodeState, TreeModel, TreeValuator};

/// Maximum allowed mean-reversion speed (κ) for either factor.
///
/// Above this threshold the probability shift `p = ½ + μ√Δt/(2σ)` pushes `p`
/// far enough from ½ that the conditional variance `σ²Δt·4p(1−p)` becomes
/// materially understated relative to the target Hull-White variance. At
/// `κ = 0.15` the worst-case unclamped node sitting 2σ√Δt above the reversion
/// reference retains ≈ 80 % of the intended conditional variance — a tolerable
/// discretisation error given the other approximations in a binomial tree. By
/// `κ = 0.20` that figure drops to ≈ 65 %, and by `κ = 0.30` to ≈ 55 %;
/// option-value errors become material for callable bond / term-loan pricing.
///
/// Callers needing accurate mean-reverting optionality above this threshold
/// should use [`HullWhiteTree`], which uses a trinomial branching scheme that
/// preserves the conditional variance exactly.
///
/// [`HullWhiteTree`]: super::hull_white_tree::HullWhiteTree
pub const KAPPA_MAX: f64 = 0.15;

/// Configuration for rates + credit two-factor tree.
///
/// Mean-reversion speeds (`rate_mean_reversion` and `hazard_mean_reversion`)
/// are validated by [`RatesCreditTree::calibrate`] against [`KAPPA_MAX`].
/// Values above that threshold return a [`Error::Validation`] — use
/// [`HullWhiteTree`] instead for strong mean reversion.
///
/// [`HullWhiteTree`]: super::hull_white_tree::HullWhiteTree
#[derive(Debug, Clone)]
pub struct RatesCreditConfig {
    /// Number of time steps
    pub steps: usize,
    /// Short-rate volatility (annualized, normal / Ho-Lee convention)
    pub rate_vol: f64,
    /// Credit hazard volatility (annualized, normal convention)
    pub hazard_vol: f64,
    /// Instantaneous correlation between rate and hazard shocks
    pub correlation: f64,
    /// Mean reversion speed for the short rate (`κ_r`, annualized, `0.0` = no
    /// reversion). Must be ≤ [`KAPPA_MAX`]; the rate factor reverts toward the
    /// discount-curve-implied `t = 0` instantaneous rate.
    pub rate_mean_reversion: f64,
    /// Mean reversion speed for the hazard rate (`κ_h`, annualized, `0.0` = no
    /// reversion). Must be ≤ [`KAPPA_MAX`]; the hazard factor reverts toward
    /// the hazard-curve-implied `t = 0` instantaneous hazard.
    pub hazard_mean_reversion: f64,
}

impl Default for RatesCreditConfig {
    /// Deterministic in both factors.
    ///
    /// The default deliberately carries **zero** volatility on each factor:
    /// the lattice still reprices the discount and survival curves exactly,
    /// it just carries no diffusion. A stochastic default would make
    /// `..Default::default()` construction silently price optionality that
    /// the caller never asked for, which is exactly how the bond path
    /// inherited a `0.20` hazard vol it never declared. Callers that want a
    /// stochastic factor must say so explicitly.
    fn default() -> Self {
        Self {
            steps: 100,
            rate_vol: 0.0,
            hazard_vol: 0.0,
            correlation: 0.0,
            rate_mean_reversion: 0.0,
            hazard_mean_reversion: 0.0,
        }
    }
}

/// Resolve the fully explicit rates-credit lattice configuration from an
/// instrument's pricing overrides.
///
/// This is the single mapping from public configuration to
/// [`RatesCreditConfig`] for every callable instrument on the rates-credit
/// path. Bond and term-loan engines must both route through it so one
/// instrument description cannot mean two different lattices.
///
/// # Channel resolution
///
/// The rate factor reads only `hw1f_sigma` / `hw1f_mean_reversion`. The
/// legacy `market_quotes.implied_volatility` and `model_config.mean_reversion`
/// channels are rejected on this path when their canonical counterpart is
/// absent: silently reinterpreting an option implied vol as a short-rate σ
/// (or silently dropping it, flipping a stochastic configuration to a
/// deterministic one) would move prices with no diagnostic. When both are
/// set the `hw1f_*` value wins, so a single `ModelConfig` can describe both
/// the credit-risky and the risk-free routing of the same instrument.
///
/// # Arguments
///
/// * `overrides` - the instrument's pricing overrides (both `model_config`
///   and `market_quotes` are consulted)
/// * `steps` - resolved lattice step count
///
/// # Errors
///
/// Returns [`Error::Validation`] when a volatility or mean reversion is
/// non-finite or negative, a mean reversion exceeds [`KAPPA_MAX`], the
/// correlation is outside `[-1, 1]`, a non-zero correlation is inert
/// (either factor deterministic), a legacy vol/mean-reversion channel is
/// used without its canonical counterpart, or an unsupported
/// `hw1f_sigma_schedule` is present.
pub fn resolve_rates_credit_config(
    overrides: &crate::instruments::InstrumentPricingOverrides,
    steps: usize,
) -> Result<RatesCreditConfig> {
    let model = &overrides.model_config;
    let quotes = &overrides.market_quotes;

    if model.hw1f_sigma_schedule.is_some() {
        return Err(Error::Validation(
            "hw1f_sigma_schedule is not supported on the rates-credit callable \
             path: the two-factor lattice carries a single scalar short-rate \
             volatility. Set hw1f_sigma instead, or price without a credit curve \
             to use a term-structure Hull-White tree."
                .to_string(),
        ));
    }

    // Legacy-channel guards. `implied_volatility` is an option (Black/normal)
    // quote, not a short-rate sigma; `mean_reversion` predates the typed HW1F
    // pair. Accepting either here would silently change the regime.
    if model.hw1f_sigma.is_none() && quotes.implied_volatility.is_some() {
        return Err(Error::Validation(
            "the rates-credit callable path reads the short-rate volatility from \
             model_config.hw1f_sigma (annualised absolute short-rate sigma, \
             e.g. 0.01 = 100 bp/yr), not market_quotes.implied_volatility. Set \
             hw1f_sigma explicitly; leave it unset only if you intend \
             deterministic rates."
                .to_string(),
        ));
    }
    if model.hw1f_mean_reversion.is_none() && model.mean_reversion.is_some_and(|k| k != 0.0) {
        return Err(Error::Validation(
            "the rates-credit callable path reads the short-rate mean reversion \
             from model_config.hw1f_mean_reversion (annualised kappa), not \
             model_config.mean_reversion. Set hw1f_mean_reversion explicitly."
                .to_string(),
        ));
    }

    let rate_vol = model.hw1f_sigma.unwrap_or(0.0);
    let hazard_vol = model.hazard_volatility.unwrap_or(0.0);
    let rate_mean_reversion = model.hw1f_mean_reversion.unwrap_or(0.0);
    let hazard_mean_reversion = model.hazard_mean_reversion.unwrap_or(0.0);
    let correlation = model.rate_credit_correlation.unwrap_or(0.0);

    for (label, value) in [
        ("hw1f_sigma", rate_vol),
        ("hazard_volatility", hazard_vol),
        ("hw1f_mean_reversion", rate_mean_reversion),
        ("hazard_mean_reversion", hazard_mean_reversion),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(Error::Validation(format!(
                "{label} must be finite and non-negative on the rates-credit \
                 path, got {value}"
            )));
        }
    }
    for (label, kappa) in [
        ("hw1f_mean_reversion", rate_mean_reversion),
        ("hazard_mean_reversion", hazard_mean_reversion),
    ] {
        if kappa > KAPPA_MAX {
            return Err(Error::Validation(format!(
                "{label} = {kappa:.4} exceeds the rates-credit binomial-lattice \
                 limit (KAPPA_MAX = {KAPPA_MAX}). Above it the conditional \
                 variance of the factor collapses far enough to degrade callable \
                 option values. Reduce the speed, or price without a credit curve \
                 to use HullWhiteTree."
            )));
        }
    }
    if !correlation.is_finite() || !(-1.0..=1.0).contains(&correlation) {
        return Err(Error::Validation(format!(
            "rate_credit_correlation must be finite and in [-1, 1], got {correlation}"
        )));
    }
    if correlation != 0.0 && (rate_vol <= 0.0 || hazard_vol <= 0.0) {
        return Err(Error::Validation(format!(
            "rate_credit_correlation = {correlation} is inert: correlation only \
             has meaning when both factors diffuse, but hw1f_sigma = {rate_vol} \
             and hazard_volatility = {hazard_vol}. Set both volatilities \
             positive, or leave the correlation unset."
        )));
    }

    Ok(RatesCreditConfig {
        steps,
        rate_vol,
        hazard_vol,
        correlation,
        rate_mean_reversion,
        hazard_mean_reversion,
    })
}

/// Two-factor correlated binomial tree (short rate + hazard rate).
///
/// Both factors are calibrated to market curves via `calibrate()`. Calling
/// `price()` without prior calibration returns an error.
#[derive(Debug, Clone)]
pub struct RatesCreditTree {
    /// Rates-credit tree configuration
    pub config: RatesCreditConfig,
    /// Calibrated short rates: `rates[step][node_i]`.
    /// Populated by `calibrate()`.
    calibrated_rates: Vec<Vec<f64>>,
    /// Calibrated hazard rates: `hazards[step][node_j]`.
    /// Populated by `calibrate()`.
    calibrated_hazards: Vec<Vec<f64>>,
    /// Recovery rate from the hazard curve (populated by `calibrate()`).
    recovery_rate: f64,
    /// Mean-reversion reference level for the rate factor (the calibrated
    /// `t = 0` instantaneous rate `r₀`). Populated by `calibrate()`. Pricing
    /// reverts the rate factor toward this level, matching the level used
    /// during calibration so the tree reprices the discount curve.
    rate_ref: f64,
    /// Mean-reversion reference level for the hazard factor (the calibrated
    /// `t = 0` instantaneous hazard `h₀`). Populated by `calibrate()`.
    hazard_ref: f64,
    /// Hazard floor-saturation diagnostic from the most recent `calibrate()`.
    hazard_floor_saturation: HazardFloorSaturation,
}

/// How much of the calibrated hazard lattice sits on the zero floor.
///
/// The hazard factor is additive-normal, so a wide lattice drives low nodes
/// negative. A negative hazard is not a credit state, so those nodes are
/// floored at zero — consistently in calibration and in pricing. Where the
/// floor binds, the lattice cannot spread as far downward as the configured
/// volatility asks, so the **realized** hazard volatility is lower than the
/// configured `hazard_vol`, and the calibrated theta compensates on the
/// upside to keep repricing the survival curve exactly.
///
/// The survival curve is still reproduced; what degrades is the dispersion
/// that drives credit optionality. Heavy saturation means the configured
/// `hazard_vol` is too large for the hazard level, which usually indicates a
/// **relative** spread volatility was supplied where an **absolute** hazard
/// volatility belongs (see
/// [`models::credit::market_anchored`](crate::models::credit::market_anchored)).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HazardFloorSaturation {
    /// Largest share of step-conditional state-price mass sitting at the
    /// zero floor across all calibrated steps, in `[0, 1]`.
    pub max_mass_at_floor: f64,
    /// Step index at which `max_mass_at_floor` occurred.
    pub worst_step: usize,
    /// Number of steps where the target survival probability was
    /// unreachable because the whole row floored. Non-zero means the
    /// lattice could not reprice the survival curve at those steps.
    pub unreachable_steps: usize,
}

impl HazardFloorSaturation {
    /// Whether any node hazard was floored during calibration.
    pub fn is_saturated(&self) -> bool {
        self.max_mass_at_floor > 0.0
    }
}

/// A future floating-coupon reset valued at tree nodes.
///
/// Describes one coupon whose index rate fixes at `reset_step` (in the
/// future) and pays at `payment_step`. The instrument's **deterministic**
/// projection of this coupon stays booked exactly as before in the
/// valuator's step-indexed cashflow vector; the lattice adds only the
/// **node-dependent increment**
///
/// ```text
/// ΔC(i) = notional · accrual · timing_scale
///         · [ compose(base_index_rate + ΔF(i)) − compose(base_index_rate) ]
/// ΔF(i) = node_discount_forward(i) − base_discount_forward
/// ```
///
/// where `compose` is the existing floating-rate composition
/// ([`calculate_floating_rate`]: index floor/cap, gearing, spread, all-in
/// floor/cap) and `node_discount_forward(i)` is the simply-compounded
/// forward implied by the tree-conditional risk-free discount factor at
/// rate node `i` (see [`RatesCreditTree::conditional_discount_factors`]).
///
/// Anchoring the node index rate to the **emission's own**
/// `base_index_rate` implements the deterministic multi-curve basis
///
/// ```text
/// node_projection_forward
///   = node_discount_forward + (market_projection_forward − market_discount_forward)
/// ```
///
/// (additive deterministic basis in the sense of Mercurio (2009),
/// "Interest Rates and The Credit Crunch: New Formulas and Market
/// Models", §2), and makes the increment vanish identically as
/// `rate_vol → 0`, so the stochastic path collapses onto today's
/// deterministic projection by construction.
///
/// # Invariants
///
/// - `reset_step < payment_step ≤ steps`: reset and payment snap to
///   distinct tree slices; a coarser grid must be refined, not silently
///   collapsed.
/// - `accrual > 0`, all fields finite; `timing_scale > 0` carries the
///   `DF(payment_date)/DF(payment_slice)` timing correction the
///   deterministic booking applies via `value_at_step_time`.
#[derive(Debug, Clone)]
pub struct NodeCoupon {
    /// Tree slice at which the coupon's index rate fixes (accrual start).
    pub reset_step: usize,
    /// Tree slice at which the coupon pays (accrual end / payment date).
    pub payment_step: usize,
    /// Accrual fraction τ of the underlying period in the instrument's
    /// day count (the emitted flow's `accrual_factor`).
    pub accrual: f64,
    /// Outstanding principal the coupon accrues on.
    pub notional: f64,
    /// Index rate (pre-composition) the deterministic emission projected
    /// for this period (`CashFlowAccrual::projected_index_rate`).
    pub base_index_rate: f64,
    /// Market simply-compounded discount forward over the **snapped slice
    /// interval** `[t(reset_step), t(payment_step)]` with the same
    /// `accrual` denominator: `(DF(t_n)/DF(t_m) − 1) / accrual`.
    pub base_discount_forward: f64,
    /// Payment-timing correction `DF(payment_date)/DF(t(payment_step))`
    /// mirroring the deterministic booking's `value_at_step_time`.
    pub timing_scale: f64,
    /// Floating-rate composition parameters (index floor/cap, gearing,
    /// spread, all-in floor/cap) taken from the instrument's
    /// `FloatingRateSpec` — no second coupon specification.
    pub params: FloatingRateParams,
}

impl NodeCoupon {
    /// Validate the descriptor against the calibrated lattice geometry.
    fn validate(&self, steps: usize) -> Result<()> {
        if self.reset_step >= self.payment_step {
            return Err(Error::Validation(format!(
                "node coupon reset_step ({}) and payment_step ({}) must snap to \
                 distinct, ordered tree slices; the tree grid is too coarse for \
                 this coupon period. Increase tree_steps.",
                self.reset_step, self.payment_step
            )));
        }
        if self.payment_step > steps {
            return Err(Error::Validation(format!(
                "node coupon payment_step ({}) exceeds the lattice step count ({steps})",
                self.payment_step
            )));
        }
        for (label, value) in [
            ("accrual", self.accrual),
            ("notional", self.notional),
            ("base_index_rate", self.base_index_rate),
            ("base_discount_forward", self.base_discount_forward),
            ("timing_scale", self.timing_scale),
        ] {
            if !value.is_finite() {
                return Err(Error::Validation(format!(
                    "node coupon {label} must be finite, got {value}"
                )));
            }
        }
        if self.accrual <= 0.0 {
            return Err(Error::Validation(format!(
                "node coupon accrual must be positive, got {}",
                self.accrual
            )));
        }
        if self.timing_scale <= 0.0 {
            return Err(Error::Validation(format!(
                "node coupon timing_scale must be positive, got {}",
                self.timing_scale
            )));
        }
        Ok(())
    }
}

/// Per-factor lattice geometry and calibration target shared by the rate and
/// hazard passes of [`RatesCreditTree::calibrate`].
#[derive(Debug, Clone, Copy)]
struct FactorGrid {
    /// Number of time steps.
    steps: usize,
    /// Step size in years.
    dt: f64,
    /// Annualised absolute (normal) volatility of this factor.
    sigma: f64,
    /// Total horizon in years.
    time_to_maturity: f64,
    /// Whether node values are passed through the non-negative transform.
    /// Set for the hazard factor; the rate factor allows negative rates.
    floor_at_zero: bool,
}

impl RatesCreditTree {
    /// Create a new rates-credit tree with the given configuration.
    ///
    /// `calibrate()` must be called before `price()`.
    pub fn new(config: RatesCreditConfig) -> Self {
        Self {
            config,
            calibrated_rates: Vec::new(),
            calibrated_hazards: Vec::new(),
            recovery_rate: 0.0,
            rate_ref: 0.0,
            hazard_ref: 0.0,
            hazard_floor_saturation: HazardFloorSaturation::default(),
        }
    }

    /// Hazard floor-saturation diagnostic from the most recent `calibrate()`.
    ///
    /// See [`HazardFloorSaturation`] for how to read it.
    pub fn hazard_floor_saturation(&self) -> HazardFloorSaturation {
        self.hazard_floor_saturation
    }

    /// Node hazard actually used by calibration and pricing.
    ///
    /// The additive-normal factor can go negative on a wide lattice; a
    /// negative hazard is not a credit state. Both the forward Arrow-Debreu
    /// recursion and the backward induction pass every node hazard through
    /// this same transform, which is what makes them exact duals and lets the
    /// tree reproduce the survival curve as the valuator sees it.
    #[inline]
    fn effective_hazard(raw: f64) -> f64 {
        raw.max(0.0)
    }

    /// Calibrate both factors to market curves using Arrow-Debreu forward induction.
    ///
    /// - **Rate factor**: calibrated to the discount curve (Ho-Lee style theta adjustment)
    /// - **Hazard factor**: calibrated to the hazard curve's survival probabilities
    ///
    /// After calibration, `price()` uses the stored per-node rates and hazards.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for risk-free rate calibration
    /// * `hazard` - Hazard curve for credit intensity calibration
    /// * `time_to_maturity` - Total time horizon in years
    pub fn calibrate(
        &mut self,
        disc: &dyn Discounting,
        hazard: &HazardCurve,
        time_to_maturity: f64,
    ) -> Result<()> {
        let steps = self.config.steps;
        if steps == 0 || time_to_maturity <= 0.0 {
            return Err(Error::internal(
                "rates-credit tree calibration requires positive steps and time_to_maturity",
            ));
        }

        // Guard: mean-reversion speeds above KAPPA_MAX cause material
        // conditional-variance collapse on the fixed-geometry binomial lattice.
        // Discount-curve repricing is exact for any κ, but option values
        // (callable bonds, term loans) degrade as κ grows — negligible for
        // κ ≲ 0.10, material by κ ≈ 0.20+. Use HullWhiteTree for κ > KAPPA_MAX.
        let kappa_r = self.config.rate_mean_reversion;
        if kappa_r > KAPPA_MAX {
            return Err(Error::Validation(format!(
                "rate_mean_reversion = {kappa_r:.4} exceeds the binomial-lattice limit \
                 (KAPPA_MAX = {KAPPA_MAX}). At this speed the conditional variance of \
                 the rate factor collapses to a fraction of its intended value, which \
                 degrades option-value accuracy for callable bonds and term loans. \
                 Use HullWhiteTree for mean reversion above this threshold."
            )));
        }
        let kappa_h = self.config.hazard_mean_reversion;
        if kappa_h > KAPPA_MAX {
            return Err(Error::Validation(format!(
                "hazard_mean_reversion = {kappa_h:.4} exceeds the binomial-lattice limit \
                 (KAPPA_MAX = {KAPPA_MAX}). At this speed the conditional variance of \
                 the hazard factor collapses to a fraction of its intended value, which \
                 degrades option-value accuracy for callable bonds and term loans. \
                 Use HullWhiteTree for mean reversion above this threshold."
            )));
        }

        let dt = time_to_maturity / steps as f64;

        // Store recovery rate from hazard curve.
        self.recovery_rate = hazard.recovery_rate();

        // Reference (reversion) levels: the t=0 instantaneous rate / hazard
        // implied by each input curve. Both factors revert toward these levels
        // during pricing; calibration uses the same levels so the tree reprices
        // the input curves with mean reversion active.
        self.rate_ref = Self::initial_instantaneous(|t| disc.df(t), dt);
        self.hazard_ref = Self::initial_instantaneous(|t| hazard.sp(t), dt);

        // --- Rate factor calibration (Ho-Lee style) ---
        let rate_vol = self.config.rate_vol;
        let rate_kappa = self.config.rate_mean_reversion;
        let rate_ref = self.rate_ref;
        let (calibrated_rates, _) = self.calibrate_factor_ho_lee(
            FactorGrid {
                steps,
                dt,
                sigma: rate_vol,
                time_to_maturity,
                floor_at_zero: false,
            },
            |t| disc.df(t),
            |r| Self::mean_reverting_up_prob(r, rate_ref, rate_kappa, rate_vol, dt),
        )?;
        self.calibrated_rates = calibrated_rates;

        // --- Hazard factor calibration (same Ho-Lee approach targeting survival) ---
        let hazard_vol = self.config.hazard_vol;
        let hazard_kappa = self.config.hazard_mean_reversion;
        let hazard_ref = self.hazard_ref;
        let (calibrated_hazards, saturation) = self.calibrate_factor_ho_lee(
            FactorGrid {
                steps,
                dt,
                sigma: hazard_vol,
                time_to_maturity,
                floor_at_zero: true,
            },
            |t| hazard.sp(t),
            |h| Self::mean_reverting_up_prob(h, hazard_ref, hazard_kappa, hazard_vol, dt),
        )?;
        self.calibrated_hazards = calibrated_hazards;
        self.hazard_floor_saturation = saturation;

        // Correlation feasibility is fully determined once both factors are
        // calibrated: each node's marginal up-probability is fixed by its
        // level, mean reversion, volatility, and step size, and the OAS shift
        // in `price()` never touches transition probabilities. Checking here
        // means pricing can never meet an infeasible node.
        self.validate_correlation_feasibility(dt)?;

        Ok(())
    }

    /// Verify the configured correlation is attainable at every node pair.
    ///
    /// Two Bernoulli marginals admit a correlation only inside their Fréchet
    /// bounds. With both mean reversions zero every marginal is exactly ½ and
    /// the whole range `[-1, 1]` is attainable, so the scan is skipped. Mean
    /// reversion skews the corner nodes' marginals away from ½ and shrinks the
    /// attainable interval sharply: with both speeds at [`KAPPA_MAX`] over a
    /// five-year lattice the feasible `|ρ|` measures about `0.12`
    /// (σ_r = 0.012, σ_λ = 0.05, 40 steps). The exact bound depends on the
    /// volatilities and step count as well — the skew scales like
    /// `κ·(x − x_ref)·√Δt / (2σ)` — so callers should read it from
    /// [`Self::max_feasible_correlation`] rather than assume a fixed number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] naming the offending step and node pair,
    /// their marginal probabilities, and the largest `|ρ|` feasible across the
    /// whole lattice.
    fn validate_correlation_feasibility(&self, dt: f64) -> Result<()> {
        let rho = self.config.correlation;
        if rho == 0.0 {
            return Ok(());
        }
        // Without mean reversion every marginal is ½ and any |ρ| ≤ 1 is
        // attainable, so there is nothing to scan.
        if self.config.rate_mean_reversion == 0.0 && self.config.hazard_mean_reversion == 0.0 {
            return Ok(());
        }

        let (lattice_lo, lattice_hi, worst) = self.scan_correlation_feasibility(dt, Some(rho));
        let Some((step, i, j, p_r, p_h, node_lo, node_hi)) = worst else {
            return Ok(());
        };
        let max_feasible_abs = lattice_lo.abs().min(lattice_hi.abs()).max(0.0);
        Err(Error::Validation(format!(
            "rate_credit_correlation = {rho:.4} is not attainable on this lattice. \
             At step {step}, rate node {i} / hazard node {j}, the marginal \
             up-probabilities are p_r = {p_r:.4} and p_h = {p_h:.4}, whose Fréchet \
             bounds admit only ρ ∈ [{node_lo:.4}, {node_hi:.4}]. Across the whole \
             lattice the attainable interval is [{lattice_lo:.4}, {lattice_hi:.4}], \
             so the largest usable |ρ| is {max_feasible_abs:.4}. Mean reversion is \
             what narrows this: it skews the corner nodes' marginals away from ½. \
             Reduce |ρ|, or reduce the mean-reversion speeds \
             (rate κ = {kappa_r}, hazard κ = {kappa_h}).",
            kappa_r = self.config.rate_mean_reversion,
            kappa_h = self.config.hazard_mean_reversion,
        )))
    }

    /// Lattice-wide Fréchet-admissible correlation interval.
    ///
    /// Returns the intersection over every node pair of the per-node
    /// admissible interval, together with the first node (if any) at which
    /// `probe` falls outside it.
    #[allow(clippy::type_complexity)]
    fn scan_correlation_feasibility(
        &self,
        dt: f64,
        probe: Option<f64>,
    ) -> (f64, f64, Option<(usize, usize, usize, f64, f64, f64, f64)>) {
        let mut lattice_lo = -1.0_f64;
        let mut lattice_hi = 1.0_f64;
        let mut worst = None;

        for step in 0..self.config.steps {
            for (i, &r) in self.calibrated_rates[step].iter().enumerate() {
                let p_r = Self::mean_reverting_up_prob(
                    r,
                    self.rate_ref,
                    self.config.rate_mean_reversion,
                    self.config.rate_vol,
                    dt,
                );
                for (j, &h) in self.calibrated_hazards[step].iter().enumerate() {
                    let p_h = Self::mean_reverting_up_prob(
                        h,
                        self.hazard_ref,
                        self.config.hazard_mean_reversion,
                        self.config.hazard_vol,
                        dt,
                    );
                    // A degenerate marginal carries no correlation at all; the
                    // joint reduces to the independent product there.
                    let Some((node_lo, node_hi)) = Self::node_correlation_range(p_r, p_h) else {
                        continue;
                    };
                    lattice_lo = lattice_lo.max(node_lo);
                    lattice_hi = lattice_hi.min(node_hi);
                    if probe.is_some_and(|rho| rho < node_lo || rho > node_hi) {
                        worst.get_or_insert((step, i, j, p_r, p_h, node_lo, node_hi));
                    }
                }
            }
        }
        (lattice_lo, lattice_hi, worst)
    }

    /// Largest `|ρ|` this calibrated lattice can express.
    ///
    /// The Fréchet bounds of two Bernoulli marginals limit how much
    /// correlation a node can carry, and mean reversion skews the corner
    /// nodes' marginals away from ½. This reports the binding constraint
    /// across the whole lattice, so a caller can pick a workable correlation
    /// instead of discovering the limit through a calibration failure.
    ///
    /// # Arguments
    ///
    /// * `time_to_maturity` - total lattice horizon in years (defines the
    ///   step size the marginal probabilities are evaluated at)
    ///
    /// Returns `1.0` for an uncalibrated tree or one without mean reversion,
    /// where every correlation in `[-1, 1]` is attainable.
    pub fn max_feasible_correlation(&self, time_to_maturity: f64) -> f64 {
        if self.calibrated_rates.is_empty()
            || self.config.steps == 0
            || time_to_maturity <= 0.0
            || (self.config.rate_mean_reversion == 0.0 && self.config.hazard_mean_reversion == 0.0)
        {
            return 1.0;
        }
        let dt = time_to_maturity / self.config.steps as f64;
        let (lo, hi, _) = self.scan_correlation_feasibility(dt, None);
        lo.abs().min(hi.abs()).max(0.0)
    }

    /// Fréchet-admissible correlation interval for two Bernoulli marginals.
    ///
    /// With `±1` coding the joint law is pinned by the single joint-up
    /// probability `a = P(up, up)`, which must satisfy
    /// `max(0, p_r + p_h − 1) ≤ a ≤ min(p_r, p_h)`, and the realized
    /// correlation is `ρ = (a − p_r·p_h) / √(var_r·var_h)`. Mapping the bounds
    /// on `a` through that relation gives the attainable `ρ` interval.
    ///
    /// Returns `None` when either marginal is degenerate (`p = 0` or `1`):
    /// that factor does not move at the node, so no correlation is
    /// expressible and the joint is the independent product.
    #[inline]
    fn node_correlation_range(p_r: f64, p_h: f64) -> Option<(f64, f64)> {
        let var_r = p_r * (1.0 - p_r);
        let var_h = p_h * (1.0 - p_h);
        let denom = (var_r * var_h).sqrt();
        if denom.is_nan() || denom <= 0.0 {
            return None;
        }
        let a_lo = (p_r + p_h - 1.0).max(0.0);
        let a_hi = p_r.min(p_h);
        Some(((a_lo - p_r * p_h) / denom, (a_hi - p_r * p_h) / denom))
    }

    /// Return the recovery rate from the most recent `calibrate()` call.
    pub fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }

    /// Calibrated short rate at node `(step, node_i)`.
    ///
    /// Returns an error if the tree has not been calibrated or the indices are
    /// out of bounds. Node `0` is the lowest rate, node `step` the highest
    /// (additive Ho-Lee lattice).
    pub fn rate_at_node(&self, step: usize, node: usize) -> Result<f64> {
        self.calibrated_rates
            .get(step)
            .and_then(|row| row.get(node))
            .copied()
            .ok_or_else(|| {
                Error::internal(format!(
                    "rates-credit tree rate node out of bounds: step={step}, node={node}"
                ))
            })
    }

    /// Calibrated hazard rate at node `(step, node_j)`.
    ///
    /// Returns an error if the tree has not been calibrated or the indices are
    /// out of bounds.
    pub fn hazard_at_node(&self, step: usize, node: usize) -> Result<f64> {
        self.calibrated_hazards
            .get(step)
            .and_then(|row| row.get(node))
            .copied()
            .ok_or_else(|| {
                Error::internal(format!(
                    "rates-credit tree hazard node out of bounds: step={step}, node={node}"
                ))
            })
    }

    /// Initial instantaneous rate implied by a target curve: `−ln(target(Δt))/Δt`.
    ///
    /// Falls back to `0.03` if the target is non-positive (degenerate curve).
    fn initial_instantaneous(target_fn: impl Fn(f64) -> f64, dt: f64) -> f64 {
        let target_dt = target_fn(dt);
        if target_dt > 0.0 && dt > 0.0 {
            -target_dt.ln() / dt
        } else {
            0.03
        }
    }

    /// Moment-matched up-probability for a mean-reverting factor on the additive
    /// normal lattice.
    ///
    /// # Units
    ///
    /// Every quantity below is in **absolute rate units** (rate per year), so
    /// the formula is dimensionally coherent:
    /// - `x`, `x_ref`: rate level (per year)
    /// - drift `μ = −κ·(x − x_ref)`: rate per year
    /// - lattice up/down step `σ√Δt`: rate (per year, integrated over `√Δt`)
    ///
    /// The standard moment-matched binomial up-probability for a drift `μ` on a
    /// lattice with step `±σ√Δt` is
    ///
    /// ```text
    /// p_up = ½ + (μ·Δt) / (2·σ√Δt) = ½ + μ·√Δt / (2σ)
    /// ```
    ///
    /// which matches the conditional **mean** `E[Δx] = μ·Δt`. With `κ = 0`
    /// this reduces to `p_up = ½` (plain Ho-Lee).
    ///
    /// # Mean vs variance trade-off
    ///
    /// An additive binomial lattice has a single free parameter (`p`). This
    /// function uses it to match the conditional mean, leaving the conditional
    /// variance `σ²Δt · 4p(1−p)` understated once `p ≠ ½`. Discount-curve
    /// repricing is **exact for any κ** because calibration and pricing apply
    /// the **identical** clamped probability, so the forward Arrow-Debreu
    /// recursion and backward induction remain exact duals. However,
    /// **option-value accuracy degrades as κ grows** — a limitation that
    /// matters when pricing callable bonds and term loans. For κ beyond
    /// [`KAPPA_MAX`] the degradation becomes material; use [`HullWhiteTree`]
    /// instead.
    ///
    /// [`HullWhiteTree`]: super::hull_white_tree::HullWhiteTree
    #[inline]
    fn mean_reverting_up_prob(x: f64, x_ref: f64, kappa: f64, sigma: f64, dt: f64) -> f64 {
        if kappa <= 0.0 || dt <= 0.0 {
            return 0.5;
        }
        let sigma = sigma.max(1e-12);
        // Drift in absolute rate units (rate / year).
        let drift = -kappa * (x - x_ref);
        (0.5 + drift * dt.sqrt() / (2.0 * sigma)).clamp(0.0, 1.0)
    }

    /// Ho-Lee style calibration for a single factor.
    ///
    /// Builds a 1D binomial lattice with additive normal volatility (`sigma * sqrt(dt)`)
    /// and solves for a theta (drift) at each step so that the lattice-implied
    /// "discount factor" matches a target curve.
    ///
    /// - For the rate factor: `target_fn(t) = disc.df(t)` (discount factor)
    /// - For the hazard factor: `target_fn(t) = hazard.sp(t)` (survival probability)
    ///
    /// Both share the same mathematical structure: the product `exp(-x * dt)` over
    /// path nodes must match the target curve value at each maturity.
    ///
    /// `up_prob_fn` returns the up-transition probability for a node given its
    /// (post-theta) rate. It **must** be the identical function the pricing pass
    /// uses for backward induction: the forward Arrow-Debreu recursion below and
    /// the backward induction in `price()` are exact duals only when they share
    /// the same per-node probability, which is what makes the calibrated tree
    /// reprice the input curve when mean reversion is active.
    ///
    /// # Non-negative factors
    ///
    /// When `floor_at_zero` is set (the hazard factor), every node value is
    /// passed through [`Self::effective_hazard`] in **both** the state-price
    /// recursion and the theta solve, matching what `price()` applies. The
    /// floor makes the step equation non-linear in theta — `exp(-θΔt)` no
    /// longer factors out — so theta is bracketed and bisected instead of
    /// being read off in closed form. The rate factor is left unfloored:
    /// negative short rates are a legitimate market state, and the discounting
    /// in `price()` deliberately uses the raw calibrated rate.
    fn calibrate_factor_ho_lee(
        &self,
        grid: FactorGrid,
        target_fn: impl Fn(f64) -> f64,
        up_prob_fn: impl Fn(f64) -> f64,
    ) -> Result<(Vec<Vec<f64>>, HazardFloorSaturation)> {
        let FactorGrid {
            steps,
            dt,
            sigma,
            time_to_maturity,
            floor_at_zero,
        } = grid;
        let mut rates = vec![Vec::new(); steps + 1];

        // Initial rate: r0 = -ln(target(dt)) / dt
        let r0 = Self::initial_instantaneous(&target_fn, dt);
        rates[0] = vec![if floor_at_zero {
            Self::effective_hazard(r0)
        } else {
            r0
        }];

        // Arrow-Debreu state prices
        let mut state_prices = vec![1.0];
        let mut saturation = HazardFloorSaturation::default();

        let sqrt_dt = dt.sqrt();
        // Value actually used for discounting/survival at a node.
        let effective = |x: f64| -> f64 {
            if floor_at_zero {
                Self::effective_hazard(x)
            } else {
                x
            }
        };

        for step in 0..steps {
            let next_nodes = step + 2;
            let mut next_rates_base = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            // Propagate state prices and compute base rates (without theta).
            //
            // The transition probability uses the SAME mean-reversion-aware
            // function the pricing pass applies, evaluated on the (calibrated)
            // current-row rate. Forward induction here is the exact dual of the
            // backward induction in `price()` because both use this probability.
            for (i, &current_rate) in rates[step].iter().enumerate() {
                let q = state_prices[i];
                let df_i = (-effective(current_rate) * dt).exp();
                let p_up = up_prob_fn(current_rate);

                // Up move (to node i+1)
                let r_up_base = current_rate + sigma * sqrt_dt;
                if i + 1 < next_nodes {
                    next_rates_base[i + 1] = r_up_base;
                    next_state_prices[i + 1] += q * df_i * p_up;
                }

                // Down move (to node i)
                let r_down_base = current_rate - sigma * sqrt_dt;
                if i < next_nodes {
                    next_rates_base[i] = r_down_base;
                    next_state_prices[i] += q * df_i * (1.0 - p_up);
                }
            }

            // Solve for theta so the lattice reproduces the target curve at
            // the next maturity.
            let next_next_time = (step + 2) as f64 * dt;
            let theta = if next_next_time <= time_to_maturity + dt * 0.5 {
                let p_target = target_fn(next_next_time);
                if !floor_at_zero {
                    // Unfloored: exp(-θΔt) factors out, so theta is exact.
                    let mut p_model_base = 0.0;
                    for (j, &q_next) in next_state_prices.iter().enumerate() {
                        p_model_base += q_next * (-next_rates_base[j] * dt).exp();
                    }
                    if p_model_base > 0.0 && p_target > 0.0 {
                        -(p_target / p_model_base).ln() / dt
                    } else {
                        0.0
                    }
                } else {
                    Self::solve_floored_theta(
                        &next_state_prices,
                        &next_rates_base,
                        dt,
                        p_target,
                        step + 1,
                        &mut saturation,
                    )
                }
            } else {
                0.0
            };

            // Apply theta to get final calibrated rates
            let mut next_rates = vec![0.0; next_nodes];
            for j in 0..next_nodes {
                next_rates[j] = next_rates_base[j] + theta;
            }
            if floor_at_zero {
                Self::record_floor_saturation(
                    &next_rates,
                    &next_state_prices,
                    step + 1,
                    &mut saturation,
                );
            }
            rates[step + 1] = next_rates;
            state_prices = next_state_prices;
        }

        Ok((rates, saturation))
    }

    /// Solve the per-step hazard shift `θ` against the target survival
    /// probability under the non-negative transform.
    ///
    /// Finds `θ` with
    ///
    /// ```text
    /// Σ_j Q[j] · exp(−max(0, base_j + θ)·Δt) = target
    /// ```
    ///
    /// The left side is continuous and non-increasing in `θ`: it equals the
    /// total state-price mass `Σ_j Q[j]` once `θ` is low enough to floor the
    /// whole row, and tends to `0` as `θ` grows. A root therefore exists
    /// exactly when `0 < target < Σ_j Q[j]`. When the target exceeds the
    /// floored ceiling the survival curve is unreachable at this step: the
    /// shift is driven to the all-floored limit and the step is counted in
    /// `saturation.unreachable_steps` rather than failing, so a caller can
    /// still price while seeing that the configured hazard volatility is too
    /// large for the hazard level.
    fn solve_floored_theta(
        state_prices: &[f64],
        base: &[f64],
        dt: f64,
        target: f64,
        step: usize,
        saturation: &mut HazardFloorSaturation,
    ) -> f64 {
        let survival_at = |theta: f64| -> f64 {
            state_prices
                .iter()
                .zip(base.iter())
                .map(|(q, b)| q * (-Self::effective_hazard(b + theta) * dt).exp())
                .sum::<f64>()
        };

        let total_mass: f64 = state_prices.iter().sum();
        if target.is_nan() || target <= 0.0 || total_mass <= 0.0 {
            return 0.0;
        }
        // All-floored limit: θ low enough that *every* node hits zero hazard,
        // which is the highest survival the row can produce. That requires
        // θ ≤ −max_j(base_j) — the largest base node is the last one to floor.
        let max_base = base.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let theta_all_floored = -max_base;
        if target >= total_mass {
            // Unreachable: even zero hazard everywhere survives less than the
            // target asks (the target itself already exceeds the mass carried
            // into this step).
            saturation.unreachable_steps += 1;
            saturation.worst_step = step;
            saturation.max_mass_at_floor = 1.0_f64.max(saturation.max_mass_at_floor);
            return theta_all_floored;
        }

        // Expand upward from the all-floored point until survival drops below
        // the target, then bisect. Survival is monotone non-increasing in θ.
        let (mut lo, mut hi) = (theta_all_floored, theta_all_floored.max(0.0) + 1.0);
        let mut guard = 0;
        while survival_at(hi) > target && guard < 200 {
            hi = theta_all_floored + (hi - theta_all_floored) * 2.0;
            guard += 1;
        }
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if survival_at(mid) > target {
                lo = mid;
            } else {
                hi = mid;
            }
            if (hi - lo).abs() <= 1e-14 * (1.0 + hi.abs()) {
                break;
            }
        }
        0.5 * (lo + hi)
    }

    /// Record how much state-price mass sits on the zero hazard floor at a
    /// calibrated step.
    fn record_floor_saturation(
        rates: &[f64],
        state_prices: &[f64],
        step: usize,
        saturation: &mut HazardFloorSaturation,
    ) {
        let total: f64 = state_prices.iter().sum();
        if total <= 0.0 {
            return;
        }
        let floored: f64 = rates
            .iter()
            .zip(state_prices.iter())
            .filter(|(rate, _)| **rate <= 0.0)
            .map(|(_, q)| *q)
            .sum();
        let share = (floored / total).clamp(0.0, 1.0);
        if share > saturation.max_mass_at_floor {
            saturation.max_mass_at_floor = share;
            saturation.worst_step = step;
        }
    }

    /// Couple the two marginal up-probabilities into joint cell probabilities.
    ///
    /// The joint law of two Bernoullis has exactly one degree of freedom once
    /// the marginals are fixed, so it is built from the single joint-up
    /// probability
    ///
    /// ```text
    /// a = P(up, up) = p_r·p_h + ρ·√(var_r·var_h)
    /// ```
    ///
    /// and the remaining three cells follow by **subtraction**:
    ///
    /// ```text
    /// p_ud = p_r − a,  p_du = p_h − a,  p_dd = 1 − p_r − p_h + a
    /// ```
    ///
    /// Deriving them this way makes both marginals exact by construction —
    /// `p_uu + p_ud = p_r` and `p_uu + p_du = p_h` hold identically — which is
    /// what keeps the forward Arrow-Debreu recursion and the backward
    /// induction exact duals, so the calibrated tree still reprices both input
    /// curves. The previous formulation clamped all four cells independently
    /// and renormalised, which silently perturbed the marginals (and hence the
    /// curve repricing) whenever a cell hit the clamp.
    ///
    /// `a` is confined to the Fréchet interval
    /// `[max(0, p_r + p_h − 1), min(p_r, p_h)]` purely as a numerical guard:
    /// [`Self::validate_correlation_feasibility`] has already proved at
    /// calibration time that the configured `ρ` is attainable at every node,
    /// so the clamp is unreachable in a calibrated tree and is asserted as
    /// such in debug builds.
    #[inline]
    fn joint_probabilities(&self, p_r: f64, p_h: f64) -> (f64, f64, f64, f64) {
        let var_r = p_r * (1.0 - p_r);
        let var_h = p_h * (1.0 - p_h);
        let a_target = p_r * p_h + self.config.correlation * (var_r * var_h).sqrt();

        let a_lo = (p_r + p_h - 1.0).max(0.0);
        let a_hi = p_r.min(p_h);
        debug_assert!(
            a_target >= a_lo - 1e-9 && a_target <= a_hi + 1e-9,
            "calibration must have rejected an infeasible correlation before \
             pricing: p_r={p_r}, p_h={p_h}, a={a_target}, bounds=[{a_lo}, {a_hi}]"
        );
        let a = a_target.clamp(a_lo, a_hi);

        // Marginals are exact by construction.
        (a, p_r - a, p_h - a, 1.0 - p_r - p_h + a)
    }

    /// Tree-conditional risk-free discount factors `P(t_n → t_m | rate node)`.
    ///
    /// For each rate node `i` at `reset_step`, returns the conditional
    /// zero-coupon price to `payment_step` implied by backward induction on
    /// the **rate-marginal** lattice using the **raw calibrated rates** —
    /// no OAS shift, no survival, no positive floor. The rate marginal is
    /// Markov on its own binomial lattice because the joint transition
    /// probabilities are built from the marginals by Fréchet coupling
    /// (`p_uu + p_ud = p_r` identically), so conditioning on the rate node
    /// alone is exact.
    ///
    /// This is the **forward-derivation operator** for node-dependent
    /// floating coupons: the simply-compounded node forward over the
    /// schedule accrual fraction τ is `(1/P − 1)/τ`. It is deliberately a
    /// *different* operator from the pricing-measure folding in
    /// [`Self::price_with_node_coupons`] (which applies OAS, survival, and
    /// the correlated joint transitions): node forwards are a property of
    /// the calibrated risk-free lattice and must not move with the OAS.
    ///
    /// # Arguments
    ///
    /// * `reset_step` - slice at which the forward is observed
    /// * `payment_step` - slice at which the notional would be repaid
    /// * `time_to_maturity` - total lattice horizon in years (defines `Δt`)
    ///
    /// # Returns
    ///
    /// `P[i]` for `i in 0..=reset_step`, each in `(0, ∞)`; higher rate
    /// nodes produce smaller discount factors.
    ///
    /// # Errors
    ///
    /// Returns an error when the tree is uncalibrated, the steps are not
    /// strictly ordered within the lattice, or `time_to_maturity` is not
    /// positive.
    pub fn conditional_discount_factors(
        &self,
        reset_step: usize,
        payment_step: usize,
        time_to_maturity: f64,
    ) -> Result<Vec<f64>> {
        if self.calibrated_rates.is_empty() {
            return Err(Error::internal(
                "rates-credit tree must be calibrated before conditional discounting",
            ));
        }
        let steps = self.config.steps;
        if time_to_maturity <= 0.0 || steps == 0 {
            return Err(Error::internal(
                "conditional discounting requires positive steps and time_to_maturity",
            ));
        }
        if reset_step >= payment_step || payment_step > steps {
            return Err(Error::Validation(format!(
                "conditional discounting requires reset_step < payment_step <= steps, \
                 got reset_step={reset_step}, payment_step={payment_step}, steps={steps}"
            )));
        }
        let dt = time_to_maturity / steps as f64;

        // Backward induction on the rate marginal: value 1 at payment_step,
        // discount with the raw calibrated node rate, transition with the
        // same mean-reversion-aware probability pricing uses.
        let mut values = vec![1.0; payment_step + 1];
        for k in (reset_step..payment_step).rev() {
            let mut next = vec![0.0; k + 1];
            for (i, slot) in next.iter_mut().enumerate() {
                let r = self.calibrated_rates[k][i];
                let p_up = Self::mean_reverting_up_prob(
                    r,
                    self.rate_ref,
                    self.config.rate_mean_reversion,
                    self.config.rate_vol,
                    dt,
                );
                let df = (-r * dt).exp();
                *slot = df * (p_up * values[i + 1] + (1.0 - p_up) * values[i]);
            }
            values = next;
        }
        Ok(values)
    }

    /// Pricing-measure fold of one node coupon's increment onto its reset
    /// slice.
    ///
    /// Returns the amount to add to the **continuation value** at each
    /// joint node `(i, j)` of `coupon.reset_step` (flattened with stride
    /// `max_nodes = steps + 1`). The unit claim — 1 paid at
    /// `payment_step` contingent on survival — is rolled back with exactly
    /// the operator the main backward induction applies to a deterministic
    /// cashflow at that slice: per-step discounting at the raw calibrated
    /// rate **plus the active OAS**, per-step survival at the floored node
    /// hazard, and the correlated joint transitions. The valuator applies
    /// the reset slice's own survival weighting when it wraps the
    /// continuation, so this fold deliberately stops one survival factor
    /// short at the reset slice; a coupon paying at the terminal slice
    /// seeds at `1` because `value_at_maturity` carries no survival
    /// weighting.
    ///
    /// The increment amount per rate node is `ΔC(i)` as documented on
    /// [`NodeCoupon`]; multiplying the unit fold by `ΔC(i)` is exact
    /// because the amount is fixed at the reset node.
    fn folded_increment_values(
        &self,
        coupon: &NodeCoupon,
        time_to_maturity: f64,
        oas_decimal: f64,
    ) -> Result<Vec<f64>> {
        let steps = self.config.steps;
        let dt = time_to_maturity / steps as f64;
        let max_nodes = steps + 1;
        let n = coupon.reset_step;
        let m = coupon.payment_step;

        // Node-dependent increment amounts from the forward-derivation
        // operator (raw rates, no OAS, no survival).
        let p_rf = self.conditional_discount_factors(n, m, time_to_maturity)?;
        let base_composed = calculate_floating_rate(coupon.base_index_rate, &coupon.params);
        let mut delta_amounts = vec![0.0; n + 1];
        for (i, slot) in delta_amounts.iter_mut().enumerate() {
            let node_forward = (1.0 / p_rf[i] - 1.0) / coupon.accrual;
            let delta_f = node_forward - coupon.base_discount_forward;
            let bumped = calculate_floating_rate(coupon.base_index_rate + delta_f, &coupon.params);
            *slot =
                coupon.notional * coupon.accrual * coupon.timing_scale * (bumped - base_composed);
        }

        // Unit-claim roll-back on the joint lattice with the pricing
        // operator. `w` holds the claim value at the slice currently being
        // consumed, in the same "post-valuator" form the main induction's
        // value function carries.
        let mut w = vec![0.0; max_nodes * max_nodes];
        let mut w_next = vec![0.0; max_nodes * max_nodes];
        for i in 0..=m {
            for j in 0..=m {
                let seed = if m == steps {
                    // value_at_maturity applies no survival weighting.
                    1.0
                } else {
                    let h = Self::effective_hazard(self.calibrated_hazards[m][j]);
                    (-h * dt).exp()
                };
                w[i * max_nodes + j] = seed;
            }
        }
        for k in (n + 1..m).rev() {
            for i in 0..=k {
                let r = self.calibrated_rates[k][i];
                let p_r = Self::mean_reverting_up_prob(
                    r,
                    self.rate_ref,
                    self.config.rate_mean_reversion,
                    self.config.rate_vol,
                    dt,
                );
                let df = (-(r + oas_decimal) * dt).exp();
                for j in 0..=k {
                    let h = self.calibrated_hazards[k][j];
                    let p_h = Self::mean_reverting_up_prob(
                        h,
                        self.hazard_ref,
                        self.config.hazard_mean_reversion,
                        self.config.hazard_vol,
                        dt,
                    );
                    let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_r, p_h);
                    let expectation = p_uu * w[(i + 1) * max_nodes + (j + 1)]
                        + p_ud * w[(i + 1) * max_nodes + j]
                        + p_du * w[i * max_nodes + (j + 1)]
                        + p_dd * w[i * max_nodes + j];
                    let p_surv = (-Self::effective_hazard(h) * dt).exp();
                    w_next[i * max_nodes + j] = p_surv * df * expectation;
                }
            }
            std::mem::swap(&mut w, &mut w_next);
        }

        // Assemble at the reset slice: continuation-form (discount + joint
        // expectation, no survival — the valuator applies it), scaled by the
        // node's increment amount.
        let mut folded = vec![0.0; max_nodes * max_nodes];
        for (i, &delta) in delta_amounts.iter().enumerate() {
            let r = self.calibrated_rates[n][i];
            let p_r = Self::mean_reverting_up_prob(
                r,
                self.rate_ref,
                self.config.rate_mean_reversion,
                self.config.rate_vol,
                dt,
            );
            let df = (-(r + oas_decimal) * dt).exp();
            for j in 0..=n {
                let h = self.calibrated_hazards[n][j];
                let p_h = Self::mean_reverting_up_prob(
                    h,
                    self.hazard_ref,
                    self.config.hazard_mean_reversion,
                    self.config.hazard_vol,
                    dt,
                );
                let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_r, p_h);
                let expectation = p_uu * w[(i + 1) * max_nodes + (j + 1)]
                    + p_ud * w[(i + 1) * max_nodes + j]
                    + p_du * w[i * max_nodes + (j + 1)]
                    + p_dd * w[i * max_nodes + j];
                folded[i * max_nodes + j] = delta * df * expectation;
            }
        }
        Ok(folded)
    }
}

impl RatesCreditTree {
    /// Price with node-dependent floating-coupon increments folded into
    /// continuation at their reset slices.
    ///
    /// This is the full backward induction of [`TreeModel::price`] plus, at
    /// each [`NodeCoupon::reset_step`], the coupon's node-dependent
    /// increment added to the continuation value **before** the valuator's
    /// exercise decision and survival weighting. Adding it to continuation
    /// gives the correct exercise economics: a redemption at the reset
    /// slice forfeits the not-yet-accrued coupon, while a redemption at
    /// the payment slice still pays it (matching the engine's
    /// coupon-paid-regardless convention). Because the folded unit claim
    /// carries no exercise decisions between reset and payment, callers
    /// must ensure no exercise step lies strictly inside a node coupon's
    /// `(reset_step, payment_step)` — the instrument engines validate
    /// this before pricing.
    ///
    /// Two distinct operators are involved, per the callable-integration
    /// design:
    ///
    /// 1. **Forward derivation** — raw calibrated rates, no OAS, no
    ///    survival ([`Self::conditional_discount_factors`]); sets the
    ///    coupon amount at each rate node.
    /// 2. **Pricing-measure folding** — the same per-node discounting the
    ///    main induction applies (raw rate + OAS, floored-hazard survival,
    ///    correlated joint transitions); moves that amount from payment to
    ///    reset.
    ///
    /// With an empty `node_coupons` slice this is exactly
    /// [`TreeModel::price`], which delegates here.
    ///
    /// # Arguments
    ///
    /// * `initial_vars` - initial state variables; `"oas"` (basis points)
    ///   is applied as a parallel shift to the calibrated short rates
    /// * `time_to_maturity` - total lattice horizon in years
    /// * `market_context` - market data passed through to the valuator
    /// * `valuator` - instrument value function driven by the induction
    /// * `node_coupons` - future floating-coupon increments to fold at
    ///   their reset slices
    ///
    /// # Errors
    ///
    /// Returns an error when the tree is uncalibrated, the horizon is not
    /// positive, a descriptor violates its invariants (see
    /// [`NodeCoupon`]), or the valuator fails.
    pub fn price_with_node_coupons<V: TreeValuator>(
        &self,
        initial_vars: HashMap<&'static str, f64>,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
        node_coupons: &[NodeCoupon],
    ) -> Result<f64> {
        if self.calibrated_rates.is_empty() || self.calibrated_hazards.is_empty() {
            return Err(Error::internal(
                "rates-credit tree must be calibrated before pricing",
            ));
        }
        if self.config.steps == 0 || time_to_maturity <= 0.0 {
            return Err(Error::internal(
                "rates-credit tree pricing requires positive steps and time_to_maturity",
            ));
        }

        let steps = self.config.steps;
        let dt = time_to_maturity / steps as f64;

        // OAS from initial variables (bp units, same convention as ShortRateTree)
        let oas_decimal = initial_vars.get("oas").copied().unwrap_or(0.0) / 10_000.0;

        // Fold every node coupon's increment onto its reset slice up front;
        // the claims are independent of the instrument value function, so
        // they precompute cleanly. The fold depends on the active OAS, so
        // an OAS solve re-folds on every objective evaluation by design.
        let mut folded_by_step: Vec<Option<Vec<f64>>> = vec![None; steps];
        for coupon in node_coupons {
            coupon.validate(steps)?;
            let folded = self.folded_increment_values(coupon, time_to_maturity, oas_decimal)?;
            match &mut folded_by_step[coupon.reset_step] {
                Some(existing) => {
                    for (slot, add) in existing.iter_mut().zip(folded.iter()) {
                        *slot += add;
                    }
                }
                slot @ None => *slot = Some(folded),
            }
        }

        // Pre-allocate flat double buffers for backward induction (zero
        // allocations in the loop). Row-major `[i * max_nodes + j]` storage is
        // cache-friendlier than a `Vec<Vec<f64>>` (no per-row pointer chase).
        let max_nodes = steps + 1;
        let mut curr_values: Vec<f64> = vec![0.0; max_nodes * max_nodes];
        let mut next_values: Vec<f64> = vec![0.0; max_nodes * max_nodes];

        // No valuator used with this tree reads node coordinates from
        // `state.vars`; they consume the cached `interest_rate`/`hazard_rate`
        // fields and `state.step`. Build each `NodeState` via `with_cached`
        // (supplying the per-node values directly) and skip the per-node
        // `HashMap` writes entirely — `initial_vars` passes through unchanged.

        // Initialize terminal values
        for i in 0..=steps {
            let r_t = self.calibrated_rates[steps][i];
            for j in 0..=steps {
                let h_t = self.calibrated_hazards[steps][j];
                let cached = CachedValues {
                    interest_rate: Some(r_t.max(1e-8)),
                    hazard_rate: Some(Self::effective_hazard(h_t)),
                    ..CachedValues::default()
                };
                let state = NodeState::with_cached(
                    steps,
                    time_to_maturity,
                    &initial_vars,
                    market_context,
                    cached,
                );
                curr_values[i * max_nodes + j] = valuator.value_at_maturity(&state)?;
            }
        }

        // Backward induction with double-buffering
        for k in (0..steps).rev() {
            for i in 0..=k {
                let r_t = self.calibrated_rates[k][i];

                // Rate transition probability with mean reversion. This is the
                // SAME function used during calibration; using an identical
                // per-node probability is what makes the tree reprice the
                // discount curve when mean reversion is non-zero.
                let p_r = Self::mean_reverting_up_prob(
                    r_t,
                    self.rate_ref,
                    self.config.rate_mean_reversion,
                    self.config.rate_vol,
                    dt,
                );

                for j in 0..=k {
                    let h_t = self.calibrated_hazards[k][j];

                    // Hazard transition probability with mean reversion (same
                    // function and reference level used during calibration).
                    let p_h = Self::mean_reverting_up_prob(
                        h_t,
                        self.hazard_ref,
                        self.config.hazard_mean_reversion,
                        self.config.hazard_vol,
                        dt,
                    );

                    // Joint probabilities
                    let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_r, p_h);

                    // Continuation from four children at step k+1
                    let v_uu = curr_values[(i + 1) * max_nodes + (j + 1)];
                    let v_ud = curr_values[(i + 1) * max_nodes + j];
                    let v_du = curr_values[i * max_nodes + (j + 1)];
                    let v_dd = curr_values[i * max_nodes + j];

                    // Risk-free discounting with calibrated rate + OAS.
                    //
                    // The rate is NOT floored here: `calibrate_factor_ho_lee`
                    // discounts with the raw (un-floored) calibrated rate, so
                    // backward induction must do the same or the tree will not
                    // reprice the discount curve once a wide lattice produces
                    // negative node rates. The `1e-8` floor is still applied
                    // below to the INTEREST_RATE / HAZARD_RATE *state
                    // variables*, which shields valuators that cannot accept
                    // non-positive rates.
                    let df = (-(r_t + oas_decimal) * dt).exp();
                    let mut cont = df * (p_uu * v_uu + p_ud * v_ud + p_du * v_du + p_dd * v_dd);

                    // Node-dependent floating-coupon increments fold into
                    // continuation at their reset slice, ahead of the
                    // valuator's exercise decision and survival weighting.
                    if let Some(folded) = folded_by_step.get(k).and_then(|f| f.as_ref()) {
                        cont += folded[i * max_nodes + j];
                    }

                    // `r_t`/`h_t` are floored only for the cached *state*
                    // variables (shields valuators that reject non-positive
                    // rates); the discounting above intentionally uses the raw
                    // calibrated rate.
                    let cached = CachedValues {
                        interest_rate: Some(r_t.max(1e-8)),
                        hazard_rate: Some(Self::effective_hazard(h_t)),
                        df: Some(df),
                        ..CachedValues::default()
                    };
                    let state = NodeState::with_cached(
                        k,
                        k as f64 * dt,
                        &initial_vars,
                        market_context,
                        cached,
                    );
                    next_values[i * max_nodes + j] = valuator.value_at_node(&state, cont, dt)?;
                }
            }
            // Swap buffers (O(1) pointer swap, no data copy)
            std::mem::swap(&mut curr_values, &mut next_values);
        }

        Ok(curr_values[0])
    }
}

impl TreeModel for RatesCreditTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: HashMap<&'static str, f64>,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        self.price_with_node_coupons(
            initial_vars,
            time_to_maturity,
            market_context,
            valuator,
            &[],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::InstrumentPricingOverrides;
    use finstack_quant_core::market_data::context::MarketContext;
    use finstack_quant_core::market_data::term_structures::DiscountCurve;
    use finstack_quant_core::math::interp::InterpStyle;

    // Resolver: public configuration to explicit lattice inputs

    /// Build overrides with the rates-credit fields set.
    fn overrides_with(
        hw1f_sigma: Option<f64>,
        hazard_volatility: Option<f64>,
        correlation: Option<f64>,
    ) -> InstrumentPricingOverrides {
        let mut o = InstrumentPricingOverrides::default();
        o.model_config.hw1f_sigma = hw1f_sigma;
        o.model_config.hazard_volatility = hazard_volatility;
        o.model_config.rate_credit_correlation = correlation;
        o
    }

    /// All four regimes in the plan's matrix resolve to the intended lattice
    /// inputs, and omitted values mean deterministic rather than defaulted.
    #[test]
    fn resolver_maps_all_four_volatility_regimes() {
        let cases = [
            ("zero/zero", None, None, 0.0, 0.0),
            ("nonzero/zero", Some(0.012), None, 0.012, 0.0),
            ("zero/nonzero", None, Some(0.02), 0.0, 0.02),
            ("nonzero/nonzero", Some(0.012), Some(0.02), 0.012, 0.02),
        ];
        for (label, sigma, hazard_vol, want_rate, want_hazard) in cases {
            let cfg = resolve_rates_credit_config(&overrides_with(sigma, hazard_vol, None), 64)
                .unwrap_or_else(|e| panic!("{label} must resolve: {e}"));
            assert_eq!(cfg.steps, 64, "{label}: steps");
            assert!(
                (cfg.rate_vol - want_rate).abs() < 1e-15,
                "{label}: rate_vol = {}, want {want_rate}",
                cfg.rate_vol
            );
            assert!(
                (cfg.hazard_vol - want_hazard).abs() < 1e-15,
                "{label}: hazard_vol = {}, want {want_hazard}",
                cfg.hazard_vol
            );
            assert_eq!(cfg.correlation, 0.0, "{label}: correlation");
        }

        // Explicit zero and omitted must be indistinguishable.
        let omitted = resolve_rates_credit_config(&overrides_with(None, None, None), 32).unwrap();
        let explicit_zero =
            resolve_rates_credit_config(&overrides_with(Some(0.0), Some(0.0), Some(0.0)), 32)
                .unwrap();
        assert_eq!(omitted.rate_vol, explicit_zero.rate_vol);
        assert_eq!(omitted.hazard_vol, explicit_zero.hazard_vol);
        assert_eq!(omitted.correlation, explicit_zero.correlation);
    }

    /// The default config is deterministic in both factors, so
    /// `..Default::default()` construction can never silently price
    /// optionality the caller did not request.
    #[test]
    fn default_config_is_deterministic_in_both_factors() {
        let cfg = RatesCreditConfig::default();
        assert_eq!(cfg.rate_vol, 0.0);
        assert_eq!(cfg.hazard_vol, 0.0);
        assert_eq!(cfg.correlation, 0.0);
        assert_eq!(cfg.rate_mean_reversion, 0.0);
        assert_eq!(cfg.hazard_mean_reversion, 0.0);
    }

    /// Invalid, non-finite, out-of-range, and inert inputs are rejected with
    /// messages naming the offending field.
    #[test]
    fn resolver_rejects_invalid_and_inert_inputs() {
        let negative_sigma = overrides_with(Some(-0.01), None, None);
        assert!(resolve_rates_credit_config(&negative_sigma, 32)
            .expect_err("negative sigma")
            .to_string()
            .contains("hw1f_sigma"));

        let nan_hazard = overrides_with(Some(0.01), Some(f64::NAN), None);
        assert!(resolve_rates_credit_config(&nan_hazard, 32)
            .expect_err("NaN hazard vol")
            .to_string()
            .contains("hazard_volatility"));

        let rho_out_of_range = overrides_with(Some(0.01), Some(0.02), Some(1.5));
        assert!(resolve_rates_credit_config(&rho_out_of_range, 32)
            .expect_err("rho > 1")
            .to_string()
            .contains("rate_credit_correlation"));

        // Inert correlation: one factor deterministic.
        for (label, sigma, hazard_vol) in [
            ("deterministic rates", None, Some(0.02)),
            ("deterministic credit", Some(0.01), None),
        ] {
            let inert = overrides_with(sigma, hazard_vol, Some(0.5));
            let err = resolve_rates_credit_config(&inert, 32)
                .expect_err("inert correlation must be rejected")
                .to_string();
            assert!(err.contains("inert"), "{label}: unexpected error {err}");
        }

        // Mean reversion above the lattice limit.
        let mut steep = overrides_with(Some(0.01), Some(0.02), None);
        steep.model_config.hazard_mean_reversion = Some(KAPPA_MAX + 0.01);
        let err = resolve_rates_credit_config(&steep, 32)
            .expect_err("kappa above KAPPA_MAX")
            .to_string();
        assert!(
            err.contains("hazard_mean_reversion") && err.contains("KAPPA_MAX"),
            "{err}"
        );

        // Unsupported term-structure sigma.
        let mut scheduled = overrides_with(Some(0.01), None, None);
        scheduled.model_config.hw1f_sigma_schedule = Some(
            finstack_quant_core::math::piecewise::PiecewiseConstantCurve::new(
                vec![0.0, 5.0],
                vec![0.01, 0.012],
            )
            .expect("piecewise curve"),
        );
        assert!(resolve_rates_credit_config(&scheduled, 32)
            .expect_err("sigma schedule")
            .to_string()
            .contains("hw1f_sigma_schedule"));
    }

    /// The legacy vol/mean-reversion channels fail loudly on this path rather
    /// than being reinterpreted or silently dropped.
    #[test]
    fn resolver_rejects_legacy_channels_without_canonical_counterpart() {
        let mut legacy_vol = InstrumentPricingOverrides::default();
        legacy_vol.market_quotes.implied_volatility = Some(0.01);
        let err = resolve_rates_credit_config(&legacy_vol, 32)
            .expect_err("implied_volatility without hw1f_sigma")
            .to_string();
        assert!(
            err.contains("hw1f_sigma") && err.contains("implied_volatility"),
            "error must name both fields: {err}"
        );

        let mut legacy_kappa = InstrumentPricingOverrides::default();
        legacy_kappa.model_config.mean_reversion = Some(0.03);
        let err = resolve_rates_credit_config(&legacy_kappa, 32)
            .expect_err("mean_reversion without hw1f_mean_reversion")
            .to_string();
        assert!(
            err.contains("hw1f_mean_reversion") && err.contains("mean_reversion"),
            "error must name both fields: {err}"
        );

        // A zero legacy mean reversion is not a regime selection, so it passes.
        let mut zero_legacy_kappa = InstrumentPricingOverrides::default();
        zero_legacy_kappa.model_config.mean_reversion = Some(0.0);
        resolve_rates_credit_config(&zero_legacy_kappa, 32)
            .expect("zero legacy mean reversion is not a regime selection");
    }

    // Lattice safety: marginals, correlation feasibility, floor saturation

    /// The Fréchet construction preserves both marginals exactly and realizes
    /// the requested correlation, including at skewed marginals where the old
    /// clamp-and-renormalise formulation silently distorted them.
    #[test]
    fn joint_probabilities_preserve_marginals_at_skewed_nodes() {
        for &(p_r, p_h) in &[
            (0.5, 0.5),
            (0.2, 0.8),
            (0.12, 0.5),
            (0.875, 0.125),
            (0.35, 0.4),
        ] {
            let (lo, hi) = RatesCreditTree::node_correlation_range(p_r, p_h)
                .expect("non-degenerate marginals");
            // Sample inside the feasible interval, including both endpoints.
            for &rho in &[lo, lo * 0.5, 0.0, hi * 0.5, hi] {
                let tree = RatesCreditTree::new(RatesCreditConfig {
                    rate_vol: 0.01,
                    hazard_vol: 0.20,
                    correlation: rho,
                    ..RatesCreditConfig::default()
                });
                let (p_uu, p_ud, p_du, p_dd) = tree.joint_probabilities(p_r, p_h);

                for (label, cell) in [("uu", p_uu), ("ud", p_ud), ("du", p_du), ("dd", p_dd)] {
                    assert!(
                        cell >= -1e-15,
                        "p_{label} negative at p_r={p_r}, p_h={p_h}, rho={rho}: {cell}"
                    );
                }
                let sum = p_uu + p_ud + p_du + p_dd;
                assert!(
                    (sum - 1.0).abs() < 1e-14,
                    "probabilities must sum to 1: {sum}"
                );
                assert!(
                    (p_uu + p_ud - p_r).abs() < 1e-14,
                    "rate marginal distorted at p_r={p_r}, p_h={p_h}, rho={rho}: {}",
                    p_uu + p_ud
                );
                assert!(
                    (p_uu + p_du - p_h).abs() < 1e-14,
                    "hazard marginal distorted at p_r={p_r}, p_h={p_h}, rho={rho}: {}",
                    p_uu + p_du
                );

                // Realized correlation with ±1 coding.
                let var_r = p_r * (1.0 - p_r);
                let var_h = p_h * (1.0 - p_h);
                let realized = (p_uu - p_r * p_h) / (var_r * var_h).sqrt();
                assert!(
                    (realized - rho).abs() < 1e-12,
                    "realized correlation {realized} != requested {rho} at \
                     p_r={p_r}, p_h={p_h}"
                );
            }
        }
    }

    /// Without mean reversion every marginal is ½, so the whole `[-1, 1]`
    /// range stays feasible and calibration accepts extreme correlations.
    #[test]
    fn correlation_is_unconstrained_without_mean_reversion() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        for &rho in &[-1.0, -0.99, 0.0, 0.99, 1.0] {
            let mut tree = RatesCreditTree::new(RatesCreditConfig {
                steps: 30,
                rate_vol: 0.012,
                hazard_vol: 0.05,
                correlation: rho,
                ..RatesCreditConfig::default()
            });
            tree.calibrate(&disc, &haz, 5.0)
                .unwrap_or_else(|e| panic!("rho={rho} must calibrate without mean reversion: {e}"));
        }
    }

    /// An infeasible correlation fails at `calibrate()` — before any pricing —
    /// and the message carries the offending node, its marginals, and the
    /// largest usable |ρ|. A request at that bound then calibrates and prices.
    #[test]
    fn infeasible_correlation_fails_at_calibration_with_bound() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let ttm = 5.0;

        let strong_reversion = |rho: f64| RatesCreditConfig {
            steps: 40,
            rate_vol: 0.012,
            hazard_vol: 0.05,
            correlation: rho,
            rate_mean_reversion: KAPPA_MAX,
            hazard_mean_reversion: KAPPA_MAX,
        };

        let mut tree = RatesCreditTree::new(strong_reversion(0.9));
        let err = tree
            .calibrate(&disc, &haz, ttm)
            .expect_err("rho = 0.9 must be infeasible under strong mean reversion");
        let msg = err.to_string();
        for needle in [
            "rate_credit_correlation",
            "not attainable",
            "step",
            "largest usable",
            "Fréchet",
        ] {
            assert!(msg.contains(needle), "message must mention {needle}: {msg}");
        }

        // The same bound is available programmatically, so a caller can pick a
        // workable correlation without scraping the message. A tree calibrated
        // at zero correlation exposes it; a request inside it then prices.
        let mut probe = RatesCreditTree::new(strong_reversion(0.0));
        probe.calibrate(&disc, &haz, ttm).expect("probe calibrates");
        let bound = probe.max_feasible_correlation(ttm);
        assert!(
            (0.0..1.0).contains(&bound),
            "reported bound {bound} must be a proper fraction"
        );
        assert!(
            msg.contains(&format!("{bound:.4}")),
            "the error must quote the same bound the accessor reports \
             ({bound:.4}): {msg}"
        );

        let mut feasible = RatesCreditTree::new(strong_reversion(bound * 0.95));
        feasible
            .calibrate(&disc, &haz, ttm)
            .expect("a correlation inside the reported bound must calibrate");
        feasible
            .price(
                HashMap::<&'static str, f64>::default(),
                ttm,
                &MarketContext::new(),
                &DummyValuator,
            )
            .expect("and must price");
    }

    /// The floor-saturation diagnostic is silent when the hazard lattice never
    /// floors and positive when it does — the signal that a *relative* spread
    /// vol was supplied where an *absolute* hazard vol belongs.
    #[test]
    fn hazard_floor_saturation_reports_binding_floor() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let ttm = 5.0;

        // Hazard ~2-3.5% with a small absolute vol: the lattice stays positive.
        let mut calm = RatesCreditTree::new(RatesCreditConfig {
            steps: 40,
            hazard_vol: 0.001,
            ..RatesCreditConfig::default()
        });
        calm.calibrate(&disc, &haz, ttm).expect("calibrate calm");
        let calm_saturation = calm.hazard_floor_saturation();
        assert!(
            !calm_saturation.is_saturated(),
            "a small absolute hazard vol must not floor: {calm_saturation:?}"
        );
        assert_eq!(calm_saturation.unreachable_steps, 0);

        // 0.20 absolute hazard vol against a ~3% hazard is roughly a 600%
        // relative vol: the floor must bind hard and say so.
        let mut violent = RatesCreditTree::new(RatesCreditConfig {
            steps: 40,
            hazard_vol: 0.20,
            ..RatesCreditConfig::default()
        });
        violent
            .calibrate(&disc, &haz, ttm)
            .expect("calibrate violent");
        let violent_saturation = violent.hazard_floor_saturation();
        assert!(
            violent_saturation.is_saturated() && violent_saturation.max_mass_at_floor > 0.25,
            "an absurd absolute hazard vol must report heavy floor saturation: \
             {violent_saturation:?}"
        );
    }

    /// Deterministic limits: the zero-vol lattice is the deterministic
    /// backward induction, and both vols tending to zero converge to it.
    /// There is no separate rate-only or hazard-only tree.
    #[test]
    fn zero_volatility_factors_are_deterministic_limits() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let ttm = 5.0;
        let ctx = MarketContext::new();

        let price_at = |rate_vol: f64, hazard_vol: f64| -> f64 {
            let mut tree = RatesCreditTree::new(RatesCreditConfig {
                steps: 40,
                rate_vol,
                hazard_vol,
                ..RatesCreditConfig::default()
            });
            tree.calibrate(&disc, &haz, ttm).expect("calibrate");
            tree.price(
                HashMap::<&'static str, f64>::default(),
                ttm,
                &ctx,
                &DummyValuator,
            )
            .expect("price")
        };

        // zero/zero reprices the discount curve (DummyValuator pays 1 and
        // passes continuation through, so the tree price is the ZCB price).
        let deterministic = price_at(0.0, 0.0);
        let market_df = disc.df(ttm);
        assert!(
            (deterministic - market_df).abs() * 10_000.0 < 1.0,
            "zero/zero must reprice the discount curve: tree={deterministic:.8}, \
             market={market_df:.8}"
        );

        // Each single-factor regime reprices the same curve.
        for (label, rate_vol, hazard_vol) in
            [("nonzero/zero", 0.01, 0.0), ("zero/nonzero", 0.0, 0.02)]
        {
            let price = price_at(rate_vol, hazard_vol);
            assert!(
                price.is_finite() && (price - market_df).abs() * 10_000.0 < 1.0,
                "{label} must reprice the discount curve: {price:.8} vs {market_df:.8}"
            );
        }

        // Both vols tending to zero converge to the zero/zero result.
        let mut previous = f64::INFINITY;
        for scale in [1e-2, 1e-3, 1e-4, 1e-5] {
            let gap = (price_at(0.01 * scale, 0.02 * scale) - deterministic).abs();
            assert!(
                gap <= previous + 1e-12,
                "convergence must be monotone as vols shrink: gap={gap}, previous={previous}"
            );
            previous = gap;
        }
        assert!(
            previous < 1e-9,
            "the vanishing-vol limit must reach the deterministic price, gap={previous}"
        );
    }

    /// With both channels populated the canonical `hw1f_*` values win, so one
    /// `ModelConfig` can describe the credit-risky and risk-free routings of
    /// the same instrument.
    #[test]
    fn canonical_fields_win_when_both_channels_are_set() {
        let mut both = InstrumentPricingOverrides::default();
        both.market_quotes.implied_volatility = Some(0.35);
        both.model_config.mean_reversion = Some(0.09);
        both.model_config.hw1f_sigma = Some(0.011);
        both.model_config.hw1f_mean_reversion = Some(0.05);

        let cfg = resolve_rates_credit_config(&both, 48).expect("both channels resolve");
        assert!((cfg.rate_vol - 0.011).abs() < 1e-15);
        assert!((cfg.rate_mean_reversion - 0.05).abs() < 1e-15);
    }

    /// The joint transition probabilities must realize the configured
    /// correlation exactly in the moderate regime (no clamping active).
    /// With balanced marginals `p_r = p_h = 0.5` and ±1 coding of the
    /// up/down moves, E[X] = E[Y] = 0 and Var[X] = Var[Y] = 1, so
    /// realized ρ = p_uu + p_dd − p_ud − p_du, and no cell can clamp for
    /// any |ρ| ≤ 1 (each cell is 0.25·(1 ± ρ) ∈ [0, 0.5]).
    ///
    /// Note the deliberate limitation this test does NOT cover: at skewed
    /// marginals a large |ρ| can exceed the Fréchet bound for two
    /// Bernoullis; the cell clamp + renormalisation then reduces the
    /// realized correlation (and shifts the marginals) with no diagnostic.
    /// See the doc comment on `joint_probabilities`.
    #[test]
    fn joint_probabilities_realize_configured_correlation_when_unclamped() {
        for &target_rho in &[-0.9, -0.5, 0.0, 0.5, 0.9] {
            let tree = RatesCreditTree::new(RatesCreditConfig {
                rate_vol: 0.01,
                hazard_vol: 0.20,
                correlation: target_rho,
                ..RatesCreditConfig::default()
            });
            let (p_uu, p_ud, p_du, p_dd) = tree.joint_probabilities(0.5, 0.5);

            let sum = p_uu + p_ud + p_du + p_dd;
            assert!((sum - 1.0).abs() < 1e-14, "probs must sum to 1, got {sum}");
            // Marginals preserved.
            assert!(
                (p_uu + p_ud - 0.5).abs() < 1e-14 && (p_uu + p_du - 0.5).abs() < 1e-14,
                "marginals distorted at rho={target_rho}: pr={}, ph={}",
                p_uu + p_ud,
                p_uu + p_du
            );
            // Realized correlation matches the configured value exactly.
            let realized = p_uu + p_dd - p_ud - p_du;
            assert!(
                (realized - target_rho).abs() < 1e-14,
                "realized correlation {realized} != configured {target_rho}"
            );
        }
    }

    struct DummyValuator;

    impl TreeValuator for DummyValuator {
        fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
            Ok(1.0)
        }
        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: f64,
            _dt: f64,
        ) -> Result<f64> {
            Ok(continuation_value)
        }
    }

    fn test_base_date() -> finstack_quant_core::dates::Date {
        finstack_quant_core::dates::Date::from_calendar_date(2025, time::Month::January, 1)
            .expect("valid date")
    }

    fn sloped_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(test_base_date())
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (2.0, 0.91),
                (3.0, 0.86),
                (5.0, 0.78),
                (10.0, 0.60),
            ])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("curve should build")
    }

    fn test_hazard_curve() -> HazardCurve {
        use finstack_quant_core::market_data::term_structures::ParInterp;
        HazardCurve::builder("TEST-HAZ")
            .base_date(test_base_date())
            .recovery_rate(0.4)
            .knots([(0.0, 0.02), (2.0, 0.025), (5.0, 0.03), (10.0, 0.035)])
            .par_interp(ParInterp::Linear)
            .build()
            .expect("hazard curve should build")
    }

    fn near_zero_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(test_base_date())
            .knots([
                (0.0, 1.0),
                (1.0, (-0.000001_f64).exp()),
                (2.0, (-0.000002_f64).exp()),
                (5.0, (-0.000005_f64).exp()),
            ])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("curve should build")
    }

    fn near_zero_hazard_curve() -> HazardCurve {
        use finstack_quant_core::market_data::term_structures::ParInterp;
        HazardCurve::builder("LOW-HAZ")
            .base_date(test_base_date())
            .recovery_rate(0.4)
            .knots([(0.0, 1e-8), (2.0, 1e-8), (5.0, 1e-8)])
            .par_interp(ParInterp::Linear)
            .build()
            .expect("hazard curve should build")
    }

    /// Valuator that pays step-indexed cashflows under the same survival
    /// convention the bond/term-loan valuators use (no recovery, no
    /// exercise). Used to probe the node-coupon fold in isolation.
    struct SurvivalCashflowValuator {
        cashflows: Vec<f64>,
    }

    impl TreeValuator for SurvivalCashflowValuator {
        fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
            Ok(self.cashflows.get(state.step).copied().unwrap_or(0.0))
        }
        fn value_at_node(
            &self,
            state: &NodeState,
            continuation_value: f64,
            dt: f64,
        ) -> Result<f64> {
            let alive = self.cashflows.get(state.step).copied().unwrap_or(0.0) + continuation_value;
            if let Some(hazard) = state.hazard_rate {
                let p_surv = (-hazard.max(0.0) * dt).exp();
                Ok(p_surv * alive)
            } else {
                Ok(alive)
            }
        }
    }

    /// With zero rate volatility every rate node collapses onto the
    /// calibrated deterministic path, so the tree-conditional discount
    /// factor must equal the market forward discount factor over the same
    /// slice interval.
    #[test]
    fn conditional_discount_factors_match_curve_at_zero_rate_vol() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 40;
        let ttm = 5.0;
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.0,
            hazard_vol: 0.0,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibration");

        let dt = ttm / steps as f64;
        let (n, m) = (16usize, 20usize);
        let market_fwd_df = disc.df(m as f64 * dt) / disc.df(n as f64 * dt);
        let conditional = tree
            .conditional_discount_factors(n, m, ttm)
            .expect("conditional discounting");
        assert_eq!(conditional.len(), n + 1);
        for (i, p) in conditional.iter().enumerate() {
            assert!(
                (p - market_fwd_df).abs() < 1e-9,
                "node {i}: conditional DF {p} must equal market forward DF {market_fwd_df} \
                 when the rate factor is deterministic"
            );
        }
    }

    /// Higher rate nodes must produce smaller conditional discount factors,
    /// and the derivation must ignore the OAS entirely (it takes no OAS
    /// input — asserted here by construction through the API shape).
    #[test]
    fn conditional_discount_factors_decrease_in_rate_node() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 40;
        let ttm = 5.0;
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.015,
            hazard_vol: 0.0,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibration");

        let conditional = tree
            .conditional_discount_factors(16, 24, ttm)
            .expect("conditional discounting");
        for pair in conditional.windows(2) {
            assert!(
                pair[1] < pair[0],
                "conditional DF must strictly decrease in the rate node: {conditional:?}"
            );
        }
        assert!(conditional.iter().all(|p| *p > 0.0 && p.is_finite()));
    }

    /// The razor identity behind the whole floating-reset design: for an
    /// increment defined off the slice-interval market forward with identity
    /// composition, the fold prices to **zero** when rates and credit are
    /// uncorrelated — for any rate vol, hazard vol, and OAS. The increment
    /// `N·((1/P − 1) − f·τ)` satisfies
    /// `E[D·(1/P − 1 − f·τ)·P] = (DF(n) − DF(m)) − f·τ·DF(m) = 0`, and with
    /// `ρ = 0` the survival and OAS factors multiply out node-independently.
    /// A non-zero correlation breaks the factorization and must move the
    /// value — that is precisely the coupon/discount-survival covariance the
    /// milestone exists to capture.
    #[test]
    fn pure_forward_increment_folds_to_zero_without_correlation() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 40;
        let ttm = 5.0;
        let dt = ttm / steps as f64;
        let (n, m) = (16usize, 20usize);
        let tau = (m - n) as f64 * dt;
        let slice_fwd = (disc.df(n as f64 * dt) / disc.df(m as f64 * dt) - 1.0) / tau;

        let coupon = NodeCoupon {
            reset_step: n,
            payment_step: m,
            accrual: tau,
            notional: 1_000_000.0,
            base_index_rate: slice_fwd,
            base_discount_forward: slice_fwd,
            timing_scale: 1.0,
            params: FloatingRateParams::default(),
        };
        let valuator = SurvivalCashflowValuator {
            cashflows: vec![0.0; steps + 1],
        };

        let price_with_rho = |rho: f64| -> f64 {
            let mut tree = RatesCreditTree::new(RatesCreditConfig {
                steps,
                rate_vol: 0.015,
                hazard_vol: 0.01,
                correlation: rho,
                ..Default::default()
            });
            tree.calibrate(&disc, &haz, ttm).expect("calibration");
            let mut vars = HashMap::<&'static str, f64>::default();
            vars.insert("oas", 175.0);
            tree.price_with_node_coupons(
                vars,
                ttm,
                &MarketContext::new(),
                &valuator,
                std::slice::from_ref(&coupon),
            )
            .expect("pricing")
        };

        let uncorrelated = price_with_rho(0.0);
        assert!(
            uncorrelated.abs() < 1e-2,
            "pure forward increment must fold to ~zero at rho = 0 (got {uncorrelated})"
        );

        // Positive rate/credit correlation puts the high-coupon states on
        // the low-survival paths, so the coupon-specific covariance is
        // strictly negative — this is the isolated sign of the channel the
        // floating-reset milestone exists to capture. (At the instrument
        // level the total correlation response also carries the opposing
        // risky-discount covariance `E[S·D]` on every booked cashflow,
        // which exists for fixed bonds too.)
        let correlated = price_with_rho(0.5);
        assert!(
            correlated < -1.0,
            "positive correlation must make the folded increment strictly \
             negative (high coupons on low-survival paths), got {correlated}"
        );
    }

    /// The node-coupon path with an empty descriptor slice is exactly the
    /// plain pricing path.
    #[test]
    fn empty_node_coupons_match_plain_price() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 30;
        let ttm = 5.0;
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.012,
            hazard_vol: 0.015,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibration");

        let mut cashflows = vec![0.0; steps + 1];
        cashflows[steps] = 1_000_000.0;
        let valuator = SurvivalCashflowValuator { cashflows };

        let plain = tree
            .price(
                HashMap::<&'static str, f64>::default(),
                ttm,
                &MarketContext::new(),
                &valuator,
            )
            .expect("plain price");
        let with_empty = tree
            .price_with_node_coupons(
                HashMap::<&'static str, f64>::default(),
                ttm,
                &MarketContext::new(),
                &valuator,
                &[],
            )
            .expect("empty-coupon price");
        assert_eq!(plain, with_empty);
    }

    /// Descriptor invariants are enforced before any folding happens.
    #[test]
    fn node_coupon_descriptor_validation_rejects_bad_geometry() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 20;
        let ttm = 5.0;
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.01,
            hazard_vol: 0.0,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibration");
        let valuator = SurvivalCashflowValuator {
            cashflows: vec![0.0; steps + 1],
        };

        let base = NodeCoupon {
            reset_step: 4,
            payment_step: 8,
            accrual: 1.0,
            notional: 100.0,
            base_index_rate: 0.03,
            base_discount_forward: 0.03,
            timing_scale: 1.0,
            params: FloatingRateParams::default(),
        };

        let collapsed = NodeCoupon {
            payment_step: 4,
            ..base.clone()
        };
        let err = tree
            .price_with_node_coupons(
                HashMap::default(),
                ttm,
                &MarketContext::new(),
                &valuator,
                std::slice::from_ref(&collapsed),
            )
            .expect_err("collapsed reset/payment must be rejected");
        assert!(
            err.to_string().contains("too coarse"),
            "unexpected error: {err}"
        );

        let beyond = NodeCoupon {
            payment_step: steps + 1,
            ..base.clone()
        };
        assert!(tree
            .price_with_node_coupons(
                HashMap::default(),
                ttm,
                &MarketContext::new(),
                &valuator,
                std::slice::from_ref(&beyond),
            )
            .is_err());

        let bad_accrual = NodeCoupon {
            accrual: 0.0,
            ..base
        };
        assert!(tree
            .price_with_node_coupons(
                HashMap::default(),
                ttm,
                &MarketContext::new(),
                &valuator,
                std::slice::from_ref(&bad_accrual),
            )
            .is_err());
    }

    #[test]
    fn rates_credit_calibrated_prices_positive() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        // Both factors stochastic: stated explicitly now that the default is
        // deterministic.
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps: 40,
            rate_vol: 0.01,
            hazard_vol: 0.20,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, 5.0).expect("calibration");

        let ctx = MarketContext::new();
        let vars = HashMap::<&'static str, f64>::default();
        let val = DummyValuator;
        let price = tree.price(vars, 5.0, &ctx, &val).expect("should succeed");
        assert!(price.is_finite() && price > 0.0);
    }

    #[test]
    fn uncalibrated_tree_returns_error() {
        let tree = RatesCreditTree::new(RatesCreditConfig::default());
        let ctx = MarketContext::new();
        let vars = HashMap::<&'static str, f64>::default();
        let val = DummyValuator;
        let result = tree.price(vars, 1.0, &ctx, &val);
        assert!(result.is_err(), "price() without calibrate() must fail");
    }

    /// Verify that tree-implied ZCB prices at each step match `disc.df(t)` within 1e-6.
    ///
    /// The DummyValuator passes continuation through unchanged and pays 1.0 at
    /// maturity, so tree price = ZCB price ≈ disc.df(T) for any number of steps.
    #[test]
    fn calibration_quality_zcb_repricing() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 60;
        let ttm = 5.0;

        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.01,
            hazard_vol: 0.0, // no hazard vol → pure rate test
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibrate");

        let ctx = MarketContext::new();
        let vars = HashMap::<&'static str, f64>::default();
        let val = DummyValuator;
        let tree_price = tree.price(vars, ttm, &ctx, &val).expect("price");
        let market_df = disc.df(ttm);

        let error_bp = (tree_price - market_df).abs() * 10_000.0;
        assert!(
            error_bp < 1.0, // within 1 bp
            "ZCB repricing error = {:.4} bp (tree={:.8}, market={:.8})",
            error_bp,
            tree_price,
            market_df
        );
    }

    /// Verify that calibrated hazard rates reproduce the hazard curve's survival
    /// probabilities at each step, using Arrow-Debreu forward induction on the
    /// 1D hazard lattice.
    #[test]
    fn calibration_quality_survival_matching() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 50;
        let ttm = 5.0;
        let dt = ttm / steps as f64;

        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            hazard_vol: 0.20,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibrate");

        // Forward-propagate Arrow-Debreu state prices through the calibrated
        // hazard lattice to compute model survival probability at each step.
        // The non-negative transform is applied here exactly as calibration
        // and `price()` apply it: a negative additive-normal node is not a
        // credit state, and all three passes must agree or the tree stops
        // reproducing the survival curve the valuator actually sees.
        let mut state_prices = vec![1.0_f64]; // Q_h[0] = 1.0

        for k in 0..steps {
            let next_nodes = k + 2;
            let mut next_sp = vec![0.0_f64; next_nodes];
            for j in 0..=k {
                let h_j = RatesCreditTree::effective_hazard(tree.calibrated_hazards[k][j]);
                let surv_df = (-h_j * dt).exp();
                let q = state_prices[j];
                // Up move to j+1, down move to j — p = 0.5 each (no mean reversion)
                if j + 1 < next_nodes {
                    next_sp[j + 1] += q * surv_df * 0.5;
                }
                next_sp[j] += q * surv_df * 0.5;
            }
            state_prices = next_sp;

            // Model survival probability at step k+1 = sum of state prices
            let model_sp: f64 = state_prices.iter().sum();
            let t = (k + 1) as f64 * dt;
            let market_sp = haz.sp(t);

            let error = (model_sp - market_sp).abs();
            assert!(
                error < 1e-6,
                "Survival mismatch at step {} (t={:.3}): model={:.8}, market={:.8}, err={:.2e}",
                k + 1,
                t,
                model_sp,
                market_sp,
                error
            );
        }
    }

    #[test]
    fn near_zero_rates_with_mean_reversion_price_finitely() {
        let disc = near_zero_discount_curve();
        let haz = near_zero_hazard_curve();
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps: 20,
            rate_vol: 0.20,
            hazard_vol: 0.20,
            rate_mean_reversion: 0.001,
            hazard_mean_reversion: 0.001,
            ..Default::default()
        });

        tree.calibrate(&disc, &haz, 2.0).expect("calibration");

        let price = tree
            .price(
                HashMap::<&'static str, f64>::default(),
                2.0,
                &MarketContext::new(),
                &DummyValuator,
            )
            .expect("pricing should succeed");

        assert!(price.is_finite() && price > 0.0, "price={price}");
    }

    /// The tree must reprice the input discount curve **with mean reversion
    /// active**. The `DummyValuator` pays 1.0 at maturity and passes
    /// continuation through unchanged, so the tree price equals the implied
    /// ZCB price, which must match `disc.df(T)`.
    ///
    /// On the parent (`e7dd696da`) this test fails by hundreds of bp: the
    /// calibration assumed `p = 0.5` while pricing used a different
    /// mean-reversion-dependent probability, so the tree no longer repriced
    /// the curve once `rate_mean_reversion != 0`.
    ///
    /// κ values are capped at `KAPPA_MAX` (= 0.15) because above that threshold
    /// `calibrate()` returns a `Validation` error. The fix is still demonstrated
    /// by these values — the parent was off by > 1000 bp even at κ = 0.05.
    #[test]
    fn calibration_reprices_disc_curve_with_rate_mean_reversion() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let ttm = 5.0;

        for &kappa in &[0.05_f64, 0.10, 0.15] {
            let mut tree = RatesCreditTree::new(RatesCreditConfig {
                steps: 60,
                rate_vol: 0.012,
                hazard_vol: 0.0, // isolate the rate factor
                rate_mean_reversion: kappa,
                ..Default::default()
            });
            tree.calibrate(&disc, &haz, ttm).expect("calibrate");

            let ctx = MarketContext::new();
            let price = tree
                .price(
                    HashMap::<&'static str, f64>::default(),
                    ttm,
                    &ctx,
                    &DummyValuator,
                )
                .expect("price");
            let market_df = disc.df(ttm);
            let error_bp = (price - market_df).abs() * 10_000.0;
            assert!(
                error_bp < 1.0,
                "kappa={kappa}: ZCB repricing error {error_bp:.4} bp \
                 (tree={price:.8}, market={market_df:.8})",
            );
        }
    }

    /// The hazard factor must likewise reprice the survival curve when its own
    /// mean reversion is active.
    #[test]
    fn calibration_reprices_survival_with_hazard_mean_reversion() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 60;
        let ttm = 5.0;
        let dt = ttm / steps as f64;

        // κ values capped at KAPPA_MAX (0.15); above that calibrate() returns
        // a Validation error (see `mean_reversion_above_kappa_max_returns_validation_error`).
        for &kappa in &[0.05_f64, 0.10, 0.15] {
            let mut tree = RatesCreditTree::new(RatesCreditConfig {
                steps,
                hazard_vol: 0.20,
                hazard_mean_reversion: kappa,
                ..Default::default()
            });
            tree.calibrate(&disc, &haz, ttm).expect("calibrate");

            // Forward-propagate Arrow-Debreu state prices through the
            // calibrated hazard lattice using the SAME mean-reversion
            // probability the calibration applied. The summed state prices at
            // step k must equal the market survival probability at t_k.
            let mut state_prices = vec![1.0_f64];
            for k in 0..steps {
                let next_nodes = k + 2;
                let mut next_sp = vec![0.0_f64; next_nodes];
                for j in 0..=k {
                    // The raw additive-normal level drives the lattice
                    // dynamics (it is what recombines with uniform spacing, so
                    // the mean-reversion drift is measured on it); the
                    // non-negative transform applies only where the level is
                    // read as a credit intensity. Calibration and `price()`
                    // make exactly this split.
                    let raw_j = tree.hazard_at_node(k, j).expect("node");
                    let surv_df = (-RatesCreditTree::effective_hazard(raw_j) * dt).exp();
                    let q = state_prices[j];
                    let p_up = RatesCreditTree::mean_reverting_up_prob(
                        raw_j,
                        tree.hazard_ref,
                        kappa,
                        0.20,
                        dt,
                    );
                    if j + 1 < next_nodes {
                        next_sp[j + 1] += q * surv_df * p_up;
                    }
                    next_sp[j] += q * surv_df * (1.0 - p_up);
                }
                state_prices = next_sp;

                let model_sp: f64 = state_prices.iter().sum();
                let t = (k + 1) as f64 * dt;
                let market_sp = haz.sp(t);
                let error = (model_sp - market_sp).abs();
                assert!(
                    error < 1e-6,
                    "kappa={kappa}: survival mismatch at step {} (t={t:.3}): \
                     model={model_sp:.8}, market={market_sp:.8}, err={error:.2e}",
                    k + 1,
                );
            }
        }
    }

    /// At **zero** mean reversion the two-factor tree's rate factor must
    /// reproduce a standalone Ho-Lee `ShortRateTree` node-for-node: both use
    /// the identical additive lattice (`r ± σ√Δt`, `p = ½`, theta calibration).
    #[test]
    fn rate_factor_matches_short_rate_tree_at_zero_mean_reversion() {
        use super::super::short_rate_tree::{ShortRateTree, ShortRateTreeConfig};
        use finstack_quant_core::types::CurveId;

        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 50;
        let ttm = 5.0;
        let vol = 0.011;

        let mut two_factor = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: vol,
            hazard_vol: 0.0,
            rate_mean_reversion: 0.0,
            ..Default::default()
        });
        two_factor
            .calibrate(&disc, &haz, ttm)
            .expect("calibrate 2F");

        let mut short_rate = ShortRateTree::new(ShortRateTreeConfig::ho_lee(steps, vol));
        short_rate
            .calibrate(&CurveId::new("USD-OIS"), &disc, ttm)
            .expect("calibrate SR");

        // Compare every calibrated rate node. The terminal row (step == steps)
        // is geometry-only and excluded from both trees' discounting, so the
        // comparison covers the discounting rows 0..steps.
        for step in 0..steps {
            for node in 0..=step {
                let r_2f = two_factor.rate_at_node(step, node).expect("2F node");
                let r_sr = short_rate.rate_at_node(step, node).expect("SR node");
                assert!(
                    (r_2f - r_sr).abs() < 1e-10,
                    "rate mismatch at (step={step}, node={node}): \
                     two_factor={r_2f:.12}, short_rate={r_sr:.12}",
                );
            }
        }
    }

    /// Calibration must reject mean-reversion speeds above `KAPPA_MAX` with a
    /// `Validation` error that names the offending value, the threshold, and
    /// the variance-degradation reason. Values at or below the threshold pass.
    #[test]
    fn mean_reversion_above_kappa_max_returns_validation_error() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();

        // Exactly at the threshold: must succeed.
        let mut tree_at_limit = RatesCreditTree::new(RatesCreditConfig {
            steps: 20,
            rate_vol: 0.01,
            hazard_vol: 0.20,
            rate_mean_reversion: KAPPA_MAX,
            ..Default::default()
        });
        tree_at_limit
            .calibrate(&disc, &haz, 5.0)
            .expect("kappa == KAPPA_MAX must succeed");

        // Just above the threshold: must fail with Validation.
        let over_rate = KAPPA_MAX + 0.01;
        let mut tree_over_rate = RatesCreditTree::new(RatesCreditConfig {
            steps: 20,
            rate_vol: 0.01,
            hazard_vol: 0.20,
            rate_mean_reversion: over_rate,
            ..Default::default()
        });
        match tree_over_rate.calibrate(&disc, &haz, 5.0) {
            Err(Error::Validation(msg)) => {
                assert!(
                    msg.contains("rate_mean_reversion"),
                    "error message must name the field; got: {msg}"
                );
                assert!(
                    msg.contains("HullWhiteTree"),
                    "error message must point to HullWhiteTree; got: {msg}"
                );
            }
            other => panic!("expected Validation error for rate κ={over_rate}, got: {other:?}"),
        }

        // Hazard factor guard: just above the threshold.
        let over_hazard = KAPPA_MAX + 0.01;
        let mut tree_over_hazard = RatesCreditTree::new(RatesCreditConfig {
            steps: 20,
            rate_vol: 0.01,
            hazard_vol: 0.20,
            hazard_mean_reversion: over_hazard,
            ..Default::default()
        });
        match tree_over_hazard.calibrate(&disc, &haz, 5.0) {
            Err(Error::Validation(msg)) => {
                assert!(
                    msg.contains("hazard_mean_reversion"),
                    "error message must name the field; got: {msg}"
                );
                assert!(
                    msg.contains("HullWhiteTree"),
                    "error message must point to HullWhiteTree; got: {msg}"
                );
            }
            other => panic!("expected Validation error for hazard κ={over_hazard}, got: {other:?}"),
        }
    }

    /// The moment-matched up-probability is dimensionally coherent and reduces
    /// to `½` at zero mean reversion / at the reference level.
    #[test]
    fn mean_reverting_up_prob_is_coherent() {
        let sigma = 0.01_f64;
        let dt = 0.05_f64;
        let kappa = 0.10_f64;
        let r_ref = 0.03_f64;

        // At the reference level the drift is zero -> p = 1/2.
        let p_at_ref = RatesCreditTree::mean_reverting_up_prob(r_ref, r_ref, kappa, sigma, dt);
        assert!((p_at_ref - 0.5).abs() < 1e-15, "p at ref should be 1/2");

        // Zero mean reversion -> p = 1/2 everywhere.
        let p_no_mr = RatesCreditTree::mean_reverting_up_prob(0.07, r_ref, 0.0, sigma, dt);
        assert!((p_no_mr - 0.5).abs() < 1e-15, "p without MR should be 1/2");

        // Above the reference level the drift is negative -> p < 1/2 (pull
        // down); below -> p > 1/2 (pull up). Symmetric about 1/2.
        let p_high = RatesCreditTree::mean_reverting_up_prob(r_ref + 0.02, r_ref, kappa, sigma, dt);
        let p_low = RatesCreditTree::mean_reverting_up_prob(r_ref - 0.02, r_ref, kappa, sigma, dt);
        assert!(
            p_high < 0.5 && p_low > 0.5,
            "mean reversion must pull toward ref"
        );
        assert!(
            ((p_high - 0.5) + (p_low - 0.5)).abs() < 1e-15,
            "probability must be symmetric about the reference level",
        );

        // Magnitude check: p = 1/2 + mu*sqrt(dt)/(2*sigma), mu = -kappa*(r-r_ref),
        // all in absolute rate units. With the inputs above:
        //   mu = -0.10 * 0.02 = -0.002 (rate/yr)
        //   p  = 0.5 + (-0.002)*sqrt(0.05)/(2*0.01) = 0.5 - 0.0223607...
        let expected = 0.5 + (-kappa * 0.02) * dt.sqrt() / (2.0 * sigma);
        assert!(
            (p_high - expected).abs() < 1e-14,
            "p_high={p_high}, expected={expected}"
        );

        // Always in [0, 1], even for an extreme node.
        let p_extreme =
            RatesCreditTree::mean_reverting_up_prob(r_ref + 100.0, r_ref, kappa, sigma, dt);
        assert!(
            (0.0..=1.0).contains(&p_extreme),
            "probability must stay in [0,1]"
        );
    }
}
