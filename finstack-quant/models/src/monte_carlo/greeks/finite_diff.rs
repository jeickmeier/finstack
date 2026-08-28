//! Finite differences with Common Random Numbers (CRN).
//!
//! Computes Greeks by bump-and-revalue using the same random numbers for base
//! and bumped scenarios. This reduces variance significantly compared with an
//! independent re-run.
//!
//! # CRN invariant
//!
//! CRN here relies on a **splittable, counter-based RNG** whose `split(i)`
//! output depends only on the seed, never on how much of the stream has been
//! consumed. Philox (the default [`crate::monte_carlo::rng::philox::PhiloxRng`]) satisfies
//! this; Sobol explicitly does not. These helpers therefore guard at runtime
//! via [`RandomStream::supports_splitting`] to prevent silent CRN breakage.
//!
//! # Reported standard errors are conservative
//!
//! The `stderr` returned by [`finite_diff_delta`] and [`finite_diff_gamma`]
//! combines the per-run MC standard errors **as if the bumped and base runs
//! were statistically independent**:
//!
//! ```text
//! se(Δ̂) ≈ √(se_up² + se_down²) / (2h)
//! se(Γ̂) ≈ √(se_up² + 4·se_base² + se_down²) / h²
//! ```
//!
//! CRN introduces strong positive correlation between the paired estimators,
//! so the *true* variance of the difference is almost always smaller — often
//! by one to two orders of magnitude for smooth payoffs. The quantity we
//! report is therefore an **upper bound** on the CRN stderr, not the CRN
//! stderr itself. A tight CRN stderr requires per-path pairing of the bumped
//! and base path values, which is not exposed through the current
//! [`McEngine::price`] API. Treat these numbers as safe for sizing error
//! budgets but not as an accurate diagnostic of the finite-difference noise.

use super::super::engine::McEngine;
use crate::monte_carlo::engine::build_correlation_factor;
use crate::monte_carlo::traits::Payoff;
use crate::monte_carlo::traits::{Discretization, RandomStream, StochasticProcess};
use crate::monte_carlo::OnlineStats;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::Result;

const MIN_SPOT_FOR_BUMP: f64 = 1.0e-12;
const MIN_BUMP_AMOUNT: f64 = 1.0e-8;

/// Guard that the supplied RNG supports deterministic splitting, which is a
/// prerequisite for CRN across bump-and-revalue calls.
fn require_splittable_rng<R: RandomStream>(rng: &R, routine: &str) -> Result<()> {
    if !rng.supports_splitting() {
        return Err(finstack_quant_core::Error::Validation(format!(
            "{routine} requires an RNG that supports deterministic splitting (e.g. PhiloxRng); \
             the supplied generator reports supports_splitting() = false. Without stream \
             splitting the bumped and base valuations consume different random numbers and CRN \
             variance reduction is lost."
        )));
    }
    Ok(())
}

/// Validate and construct a symmetric central-difference stencil.
///
/// `bump_size` is a positive relative spot bump. The absolute bump retains the
/// historical `1e-8` numerical floor, but the lower state is never clamped:
/// callers receive a validation error when a symmetric stencil would cross the
/// supported positive-spot boundary.
fn validate_central_bump(initial_spot: f64, bump_size: f64) -> Result<(f64, f64, f64, f64)> {
    if !initial_spot.is_finite() || initial_spot <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "finite-difference initial_spot must be finite and positive, got {initial_spot}"
        )));
    }
    if !bump_size.is_finite() || bump_size <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "finite-difference bump_size must be finite and positive, got {bump_size}"
        )));
    }

    let h = (initial_spot.abs() * bump_size).max(MIN_BUMP_AMOUNT);
    if !h.is_finite() {
        return Err(finstack_quant_core::Error::Validation(format!(
            "finite-difference bump produced a non-finite stencil: initial_spot={initial_spot}, bump_size={bump_size}, h={h}"
        )));
    }
    let down = initial_spot - h;
    let up = initial_spot + h;
    if !up.is_finite() {
        return Err(finstack_quant_core::Error::Validation(format!(
            "finite-difference bump produced a non-finite upper state: initial_spot={initial_spot}, bump_size={bump_size}, h={h}"
        )));
    }
    if down < MIN_SPOT_FOR_BUMP {
        return Err(finstack_quant_core::Error::Validation(format!(
            "finite-difference central stencil crosses the minimum supported spot {MIN_SPOT_FOR_BUMP:e}: initial_spot={initial_spot}, bump_size={bump_size}, down={down}"
        )));
    }

    Ok((down, initial_spot, up, h))
}

/// Compute delta using central finite differences with CRN.
///
/// ```text
/// Δ ≈ (V(S₀+h) − V(S₀−h)) / (2h)
/// ```
///
/// Both valuations reuse `rng` by reference. This works only when the RNG is
/// splittable (e.g. [`crate::monte_carlo::rng::philox::PhiloxRng`]) so that `rng.split(i)`
/// produces identical per-path streams across calls regardless of how much of
/// the parent stream has been consumed.
///
/// # Arguments
///
/// * `engine` - MC engine (borrowed, used for the two price runs)
/// * `rng` - Random number generator; must support splitting
/// * `process` - Stochastic process
/// * `disc` - Discretization scheme
/// * `initial_spot` - Finite positive initial spot price (S₀). The down-bumped
///   state must remain at least `1e-12`.
/// * `payoff` - Payoff specification
/// * `currency` - Currency tag assigned to each simulated payoff and resulting
///   price estimate.
/// * `discount_factor` - Discount factor from the payoff horizon to valuation;
///   it must use the same pricing convention as `payoff`.
/// * `bump_size` - Finite positive relative bump (e.g. `0.01` for 1 %). The
///   absolute bump is `max(|S₀| * bump_size, 1e-8)`.
///
/// # Returns
///
/// `(delta, stderr)` — the central-difference estimator and its standard
/// error under the assumption of independence between the up and down runs
/// (conservative; CRN makes the true stderr smaller but computing it exactly
/// would require per-path pairing outside this helper).
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when `initial_spot` or
/// `bump_size` is non-finite or non-positive, the symmetric down-bump is below
/// `1e-12`, the supplied RNG does not support splitting, or either
/// `engine.price` call fails.
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_delta<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P> + Clone,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_delta")?;
    let (initial_down, _initial_base, initial_up, h) =
        validate_central_bump(initial_spot, bump_size)?;

    let initial_up = vec![initial_up];
    let result_up = engine.price(
        rng,
        process,
        disc,
        &initial_up,
        payoff,
        currency,
        discount_factor,
    )?;

    let initial_down = vec![initial_down];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    let delta = (result_up.mean.amount() - result_down.mean.amount()) / (2.0 * h);
    // Conservative stderr under the independence assumption. Propagating a
    // tighter CRN stderr would require per-path bookkeeping not exposed
    // through the current engine API.
    let se_up = result_up.stderr;
    let se_down = result_down.stderr;
    let stderr = (se_up * se_up + se_down * se_down).sqrt() / (2.0 * h);

    Ok((delta, stderr))
}

/// Compute gamma using a second central difference with CRN.
///
/// ```text
/// Γ ≈ (V(S₀+h) − 2·V(S₀) + V(S₀−h)) / h²
/// ```
///
/// # Returns
///
/// `(gamma, stderr)` — the second-difference estimator and a conservative
/// standard error under the assumption of independent MC stderrs at the
/// three grid points. Both values are in the pricing currency per squared
/// initial-spot unit; the reported error is deliberately conservative because
/// the common random numbers induce positive dependence between valuations.
///
/// The absolute bump is `max(|S₀| * bump_size, 1e-8)`. The symmetric stencil
/// is rejected rather than clamped if its lower state would fall below
/// `1e-12`.
///
/// # Arguments
///
/// * `engine` - Monte Carlo engine used for the base, up-bumped, and
///   down-bumped valuation runs.
/// * `rng` - Splittable random stream; matching child streams provide common
///   random numbers across all stencil points.
/// * `process` - Stochastic process that evolves the bumped initial state.
/// * `disc` - Cloneable discretization scheme compatible with `process`.
/// * `initial_spot` - Finite positive base underlying spot level in the
///   payoff's price units. The down-bumped state must remain at least `1e-12`.
/// * `payoff` - Payoff evaluated on each simulated path.
/// * `currency` - Currency tag assigned to each simulated payoff and price.
/// * `discount_factor` - Discount factor from payoff horizon to valuation.
/// * `bump_size` - Finite positive relative spot bump, such as `0.01` for one
///   percent. The absolute bump is `max(|S₀| * bump_size, 1e-8)`.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when `initial_spot` or
/// `bump_size` is non-finite or non-positive, the symmetric down-bump is below
/// `1e-12`, or `rng` cannot split deterministically. Propagates errors from any
/// of the three [`McEngine::price`] runs.
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_gamma<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P> + Clone,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_gamma")?;
    let (initial_down, initial_base, initial_up, h) =
        validate_central_bump(initial_spot, bump_size)?;

    let initial_base = vec![initial_base];
    let result_base = engine.price(
        rng,
        process,
        disc,
        &initial_base,
        payoff,
        currency,
        discount_factor,
    )?;

    let initial_up = vec![initial_up];
    let result_up = engine.price(
        rng,
        process,
        disc,
        &initial_up,
        payoff,
        currency,
        discount_factor,
    )?;

    let initial_down = vec![initial_down];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    let gamma = (result_up.mean.amount() - 2.0 * result_base.mean.amount()
        + result_down.mean.amount())
        / (h * h);
    let se_up = result_up.stderr;
    let se_base = result_base.stderr;
    let se_down = result_down.stderr;
    // Variance of (V_up − 2V_base + V_down): 1·V_up + 4·V_base + 1·V_down under
    // independence. CRN makes this pessimistic but not wrong.
    let stderr = (se_up * se_up + 4.0 * se_base * se_base + se_down * se_down).sqrt() / (h * h);

    Ok((gamma, stderr))
}

// CRN-paired finite differences (true CRN stderr)

/// Run a paired CRN finite-difference loop and return per-path payoff
/// differences for each of `n_states` initial-state perturbations.
///
/// Each path uses an independent splittable substream keyed on `path_id`. The
/// substream is re-cloned for each perturbation so all `n_states` variants
/// consume identical shock sequences — this is what makes the per-path
/// difference a tight CRN estimator.
///
/// Returns a `Vec<Vec<f64>>` of length `n_states` where each inner vector has
/// length `engine.config.num_paths` and contains discounted payoff amounts
/// (currency stripped via `MoneyEstimate`-style conversion).
#[allow(clippy::too_many_arguments)]
fn paired_per_path_payoffs<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_states: &[Vec<f64>],
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
) -> Result<Vec<Vec<f64>>>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P> + Clone,
    F: Payoff,
{
    let n_states = initial_states.len();
    debug_assert!(
        n_states >= 1,
        "paired_per_path_payoffs requires at least one initial state"
    );

    let cfg = engine.config();
    let dim = process.dim();
    let num_factors = process.num_factors();
    let work_size = disc.work_size(process);

    // Validate state shapes once.
    for (i, s) in initial_states.iter().enumerate() {
        if s.len() != dim {
            return Err(finstack_quant_core::Error::Validation(format!(
                "paired finite-diff initial_states[{i}].len() = {} does not match process dim = {}",
                s.len(),
                dim
            )));
        }
    }

    let correlation = build_correlation_factor(process, disc)?;
    let mut payoff_local = payoff.clone();
    let mut state = vec![0.0; dim];
    let mut z = vec![0.0; num_factors];
    let mut z_raw = vec![
        0.0;
        if correlation.is_some() {
            num_factors
        } else {
            0
        }
    ];
    let mut work = vec![0.0; work_size];

    let mut per_state_values: Vec<Vec<f64>> = (0..n_states)
        .map(|_| Vec::with_capacity(cfg.num_paths))
        .collect();

    for path_id in 0..cfg.num_paths {
        let base_split = rng.split(path_id as u64).ok_or_else(|| {
            finstack_quant_core::Error::Validation(
                "RandomStream reports splitting support but split() returned None for paired \
                 finite-diff Greek"
                    .to_string(),
            )
        })?;

        for (state_idx, s0) in initial_states.iter().enumerate() {
            let mut path_rng = base_split.clone();
            payoff_local.reset();
            payoff_local.on_path_start(&mut path_rng);
            let v = engine.simulate_path(
                &mut path_rng,
                process,
                disc,
                s0,
                &mut payoff_local,
                &mut state,
                &mut z,
                &mut z_raw,
                &mut work,
                correlation.as_ref(),
                currency,
            )?;
            let discounted = v * discount_factor;
            if !discounted.is_finite() {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "non-finite paired finite-diff payoff on path {path_id}, state index \
                     {state_idx}: payoff={v}, discount_factor={discount_factor}"
                )));
            }
            per_state_values[state_idx].push(discounted);
        }
    }

    Ok(per_state_values)
}

/// Compute delta with **true CRN stderr** by per-path pairing.
///
/// Like [`finite_diff_delta`] but reports the proper paired standard error
/// `stderr({(V_up_i − V_down_i) / 2h})`, which exploits the strong positive
/// correlation introduced by common random numbers and is typically one to two
/// orders of magnitude tighter than the conservative independence bound.
///
/// Always runs serially (paired stderr requires deterministic per-path order).
/// The pricer's `use_parallel` flag is honored only by [`finite_diff_delta`].
///
/// # Arguments
///
/// * `engine` - Monte Carlo engine that supplies deterministic serial path
///   valuation for the paired estimator.
/// * `rng` - Splittable random stream providing common random numbers for the
///   up and down paths.
/// * `process` - Stochastic process that evolves the bumped initial state.
/// * `disc` - Cloneable discretization scheme compatible with `process`.
/// * `initial_spot` - Finite positive base underlying spot level in the
///   payoff's price units. The down-bumped state must remain at least `1e-12`.
/// * `payoff` - Payoff evaluated path by path at the simulation horizon.
/// * `currency` - Currency tag assigned to path payoff amounts.
/// * `discount_factor` - Discount factor from payoff horizon to valuation.
/// * `bump_size` - Finite positive relative spot bump, such as `0.01` for one
///   percent. The absolute bump is `max(|S₀| * bump_size, 1e-8)`.
///
/// # Returns
///
/// `(delta, stderr)` where `stderr` is the **paired** standard error.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when `initial_spot` or
/// `bump_size` is non-finite or non-positive, the symmetric down-bump is below
/// `1e-12`, the RNG is not splittable, configuration is invalid, or any path
/// simulation fails (e.g., non-finite payoff).
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_delta_crn<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P> + Clone,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_delta_crn")?;
    let (initial_down, _initial_base, initial_up, h) =
        validate_central_bump(initial_spot, bump_size)?;

    let initial_states = vec![vec![initial_up], vec![initial_down]];
    let per_state = paired_per_path_payoffs(
        engine,
        rng,
        process,
        disc,
        &initial_states,
        payoff,
        currency,
        discount_factor,
    )?;

    let v_up = &per_state[0];
    let v_down = &per_state[1];
    let mut stats = OnlineStats::new();
    for i in 0..v_up.len() {
        stats.update((v_up[i] - v_down[i]) / (2.0 * h));
    }
    Ok((stats.mean(), stats.stderr()))
}

/// Compute gamma with **true CRN stderr** by per-path pairing.
///
/// Like [`finite_diff_gamma`] but reports the paired standard error of the
/// per-path second-difference estimator
/// `(V_up_i − 2 V_base_i + V_down_i) / h²`, which is typically one to two
/// orders of magnitude tighter than the independence bound.
///
/// # Arguments
///
/// * `engine` - Monte Carlo engine that supplies deterministic serial path
///   valuation for the paired estimator.
/// * `rng` - Splittable random stream providing common random numbers for all
///   three stencil valuations.
/// * `process` - Stochastic process that evolves the bumped initial state.
/// * `disc` - Cloneable discretization scheme compatible with `process`.
/// * `initial_spot` - Finite positive base underlying spot level in the
///   payoff's price units. The down-bumped state must remain at least `1e-12`.
/// * `payoff` - Payoff evaluated path by path at the simulation horizon.
/// * `currency` - Currency tag assigned to path payoff amounts.
/// * `discount_factor` - Discount factor from payoff horizon to valuation.
/// * `bump_size` - Finite positive relative spot bump, such as `0.01` for one
///   percent. The absolute bump is `max(|S₀| * bump_size, 1e-8)`.
///
/// # Returns
///
/// `(gamma, stderr)` where `stderr` is the **paired** standard error.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] for the invalid central
/// stencils documented by [`finite_diff_delta_crn`], or when path simulation
/// fails.
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_gamma_crn<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P> + Clone,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_gamma_crn")?;
    let (initial_down, initial_base, initial_up, h) =
        validate_central_bump(initial_spot, bump_size)?;

    let initial_states = vec![vec![initial_up], vec![initial_base], vec![initial_down]];
    let per_state = paired_per_path_payoffs(
        engine,
        rng,
        process,
        disc,
        &initial_states,
        payoff,
        currency,
        discount_factor,
    )?;

    let v_up = &per_state[0];
    let v_base = &per_state[1];
    let v_down = &per_state[2];
    let mut stats = OnlineStats::new();
    for i in 0..v_up.len() {
        stats.update((v_up[i] - 2.0 * v_base[i] + v_down[i]) / (h * h));
    }
    Ok((stats.mean(), stats.stderr()))
}

#[cfg(test)]
mod tests {
    use super::super::super::engine::McEngineConfig;
    use super::*;
    use crate::closed_form::black_scholes_spot_call;
    use crate::monte_carlo::discretization::exact::ExactGbm;
    use crate::monte_carlo::payoff::vanilla::EuropeanCall;
    use crate::monte_carlo::process::gbm::{GbmParams, GbmProcess};
    use crate::monte_carlo::rng::philox::PhiloxRng;
    use crate::monte_carlo::TimeGrid;
    use finstack_quant_core::math::{norm_cdf, norm_pdf};

    const SPOT: f64 = 100.0;
    const STRIKE: f64 = 100.0;
    const RATE: f64 = 0.05;
    const DIVIDEND_YIELD: f64 = 0.0;
    const VOLATILITY: f64 = 0.2;
    const EXPIRY: f64 = 1.0;

    fn test_engine(num_paths: usize) -> McEngine {
        let time_grid = TimeGrid::uniform(EXPIRY, 1).expect("valid time grid");
        McEngine::new(McEngineConfig {
            num_paths,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: None,
            path_capture: crate::monte_carlo::engine::PathCaptureConfig::default(),
            antithetic: false,
        })
    }

    fn bs_call(spot: f64) -> f64 {
        black_scholes_spot_call(spot, STRIKE, RATE, DIVIDEND_YIELD, VOLATILITY, EXPIRY)
    }

    fn deterministic_stencil(bump_size: f64) -> (f64, f64) {
        let (down, base, up, h) =
            validate_central_bump(SPOT, bump_size).expect("valid central stencil");
        let up_value = bs_call(up);
        let base_value = bs_call(base);
        let down_value = bs_call(down);
        (
            (up_value - down_value) / (2.0 * h),
            (up_value - 2.0 * base_value + down_value) / (h * h),
        )
    }

    fn analytic_bs_greeks() -> (f64, f64) {
        let root_t = EXPIRY.sqrt();
        let d1 = ((SPOT / STRIKE).ln()
            + (RATE - DIVIDEND_YIELD + 0.5 * VOLATILITY * VOLATILITY) * EXPIRY)
            / (VOLATILITY * root_t);
        let carry_discount = (-DIVIDEND_YIELD * EXPIRY).exp();
        (
            carry_discount * norm_cdf(d1),
            carry_discount * norm_pdf(d1) / (SPOT * VOLATILITY * root_t),
        )
    }

    fn assert_within_reported_error(label: &str, estimate: f64, reference: f64, stderr: f64) {
        assert!(
            stderr.is_finite() && stderr >= 0.0,
            "{label} reported invalid stderr {stderr}"
        );
        let tolerance = 5.0 * stderr + 1e-10 * reference.abs().max(1.0);
        assert!(
            (estimate - reference).abs() <= tolerance,
            "{label} estimate {estimate} differs from reference {reference} by {}, tolerance {tolerance}, stderr {stderr}",
            (estimate - reference).abs()
        );
    }

    fn assert_validation_contains(result: Result<(f64, f64)>, expected: &str) {
        match result {
            Err(finstack_quant_core::Error::Validation(message)) => assert!(
                message.contains(expected),
                "validation message {message:?} did not contain {expected:?}"
            ),
            other => panic!("expected validation error containing {expected:?}, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_central_stencil_converges_to_analytic_greeks() {
        let (analytic_delta, analytic_gamma) = analytic_bs_greeks();
        let mut previous_delta_error = f64::INFINITY;
        let mut previous_gamma_error = f64::INFINITY;

        for bump_size in [0.02, 0.01, 0.005] {
            let (delta, gamma) = deterministic_stencil(bump_size);
            let delta_error = (delta - analytic_delta).abs();
            let gamma_error = (gamma - analytic_gamma).abs();
            assert!(
                delta_error < previous_delta_error,
                "delta error did not decrease at bump {bump_size}: {delta_error} >= {previous_delta_error}"
            );
            assert!(
                gamma_error < previous_gamma_error,
                "gamma error did not decrease at bump {bump_size}: {gamma_error} >= {previous_gamma_error}"
            );
            previous_delta_error = delta_error;
            previous_gamma_error = gamma_error;
        }

        assert!(
            previous_delta_error < 2.5e-5,
            "delta error {previous_delta_error}"
        );
        assert!(
            previous_gamma_error < 7e-7,
            "gamma error {previous_gamma_error}"
        );
    }

    #[test]
    fn monte_carlo_estimators_match_the_black_scholes_stencil() {
        let engine = test_engine(20_000);
        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(
            GbmParams::new(RATE, DIVIDEND_YIELD, VOLATILITY).expect("valid GBM parameters"),
        );
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(STRIKE, EXPIRY, 1);
        let discount_factor = (-RATE * EXPIRY).exp();
        let bump_size = 0.01;
        let (reference_delta, reference_gamma) = deterministic_stencil(bump_size);

        let (delta_independent, delta_independent_stderr) = finite_diff_delta(
            &engine,
            &rng,
            &gbm,
            &disc,
            SPOT,
            &call,
            Currency::USD,
            discount_factor,
            bump_size,
        )
        .expect("independence-bound delta");
        let (delta_crn, delta_crn_stderr) = finite_diff_delta_crn(
            &engine,
            &rng,
            &gbm,
            &disc,
            SPOT,
            &call,
            Currency::USD,
            discount_factor,
            bump_size,
        )
        .expect("paired delta");
        let (gamma_independent, gamma_independent_stderr) = finite_diff_gamma(
            &engine,
            &rng,
            &gbm,
            &disc,
            SPOT,
            &call,
            Currency::USD,
            discount_factor,
            bump_size,
        )
        .expect("independence-bound gamma");
        let (gamma_crn, gamma_crn_stderr) = finite_diff_gamma_crn(
            &engine,
            &rng,
            &gbm,
            &disc,
            SPOT,
            &call,
            Currency::USD,
            discount_factor,
            bump_size,
        )
        .expect("paired gamma");

        assert_within_reported_error("paired delta", delta_crn, reference_delta, delta_crn_stderr);
        assert_within_reported_error(
            "independence-bound delta versus paired delta",
            delta_independent,
            delta_crn,
            delta_independent_stderr,
        );
        assert_within_reported_error("paired gamma", gamma_crn, reference_gamma, gamma_crn_stderr);
        assert_within_reported_error(
            "independence-bound gamma versus paired gamma",
            gamma_independent,
            gamma_crn,
            gamma_independent_stderr,
        );
        assert!(
            delta_crn_stderr < delta_independent_stderr,
            "paired delta stderr {delta_crn_stderr} should be below independence bound {delta_independent_stderr}"
        );
        assert!(
            gamma_crn_stderr < gamma_independent_stderr,
            "paired gamma stderr {gamma_crn_stderr} should be below independence bound {gamma_independent_stderr}"
        );
        assert!(delta_crn > 0.0);
        assert!(gamma_crn > 0.0);
    }

    #[test]
    fn all_estimators_reject_invalid_or_asymmetric_stencils() {
        let engine = test_engine(8);
        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(
            GbmParams::new(RATE, DIVIDEND_YIELD, VOLATILITY).expect("valid GBM parameters"),
        );
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(STRIKE, EXPIRY, 1);
        let discount_factor = (-RATE * EXPIRY).exp();
        let cases = [
            (0.0, 0.01, "initial_spot"),
            (-1.0, 0.01, "initial_spot"),
            (f64::NAN, 0.01, "initial_spot"),
            (f64::INFINITY, 0.01, "initial_spot"),
            (SPOT, 0.0, "bump_size"),
            (SPOT, -0.01, "bump_size"),
            (SPOT, f64::NAN, "bump_size"),
            (SPOT, f64::INFINITY, "bump_size"),
            (f64::MAX, 2.0, "non-finite stencil"),
            (1e-8, 2.0, "central stencil"),
        ];

        for (spot, bump_size, expected) in cases {
            assert_validation_contains(
                finite_diff_delta(
                    &engine,
                    &rng,
                    &gbm,
                    &disc,
                    spot,
                    &call,
                    Currency::USD,
                    discount_factor,
                    bump_size,
                ),
                expected,
            );
            assert_validation_contains(
                finite_diff_delta_crn(
                    &engine,
                    &rng,
                    &gbm,
                    &disc,
                    spot,
                    &call,
                    Currency::USD,
                    discount_factor,
                    bump_size,
                ),
                expected,
            );
            assert_validation_contains(
                finite_diff_gamma(
                    &engine,
                    &rng,
                    &gbm,
                    &disc,
                    spot,
                    &call,
                    Currency::USD,
                    discount_factor,
                    bump_size,
                ),
                expected,
            );
            assert_validation_contains(
                finite_diff_gamma_crn(
                    &engine,
                    &rng,
                    &gbm,
                    &disc,
                    spot,
                    &call,
                    Currency::USD,
                    discount_factor,
                    bump_size,
                ),
                expected,
            );
        }
    }
}
