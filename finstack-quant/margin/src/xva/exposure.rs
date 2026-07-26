//! Exposure simulation engine for XVA calculations.
//!
//! Computes exposure profiles (EPE, ENE, PFE) for a portfolio of instruments
//! by re-valuing them at future time points.
//!
//! # Methodology
//!
//! This module implements a **deterministic exposure** approach:
//! at each future time point, instruments are re-valued under the current
//! market data (curves rolled forward deterministically). This is a simplified
//! but conservative approach suitable for:
//!
//! - Initial XVA framework validation
//! - Portfolios with linear instruments (bonds, swaps)
//! - Regulatory SA-CCR style calculations
//!
//! For a full production implementation, Monte Carlo simulation of risk factors
//! would replace the deterministic forward roll. The API is designed to be
//! extended without breaking changes.
//!
//! # Exposure Definitions
//!
//! ```text
//! V(t)   = portfolio mark-to-market at time t
//! EPE(t) = E[max(V(t), 0)]     — Expected Positive Exposure
//! ENE(t) = E[max(-V(t), 0)]    — Expected Negative Exposure
//! PFE(t) = quantile(V(t), α)   — Potential Future Exposure at level α
//! ```
//!
//! # References
//!
//! - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
//! - BCBS 279 SA-CCR: `docs/REFERENCES.md#bcbs-279-saccr`

use std::sync::Arc;

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{Date, CALENDAR_DAYS_PER_YEAR};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::math::neumaier_sum;
use finstack_quant_core::money::fx::FxConversionPolicy;
use finstack_quant_core::money::fx::FxQuery;
use finstack_quant_core::money::Money;

use super::mva::PathImModel;
use super::traits::{PathValuer, Valuable};
use finstack_quant_monte_carlo::rng::philox::PhiloxRng;
use finstack_quant_monte_carlo::{
    state_keys, Discretization, PathState, RandomStream, StochasticProcess,
};

use super::netting::{apply_collateral_mpor, apply_variation_margin_mpor};
use super::types::{
    CsaTerms, ExposureProfile, StochasticExposureConfig, StochasticExposureProfile, XvaConfig,
    XvaNettingSet,
};

/// Map a year fraction to a whole-day offset using ACT/365F-style scaling.
///
/// Uses **half-up** rounding to the nearest calendar day. This avoids IEEE
/// "ties to even" surprises from [`f64::round`] (e.g. 182.5 days rounding to 182).
#[inline]
fn years_to_days_act_365f(years: f64) -> i64 {
    let raw = years * CALENDAR_DAYS_PER_YEAR;
    if !raw.is_finite() {
        return 0;
    }
    if raw >= 0.0 {
        (raw + 0.5).floor() as i64
    } else {
        (raw - 0.5).ceil() as i64
    }
}

fn resolve_reporting_currency(
    instruments: &[Arc<dyn Valuable>],
    market: &MarketContext,
    as_of: Date,
    netting_set: &XvaNettingSet,
) -> finstack_quant_core::Result<Currency> {
    if let Some(currency) = netting_set.reporting_currency {
        return Ok(currency);
    }

    let mut observed: Option<Currency> = None;
    for inst in instruments {
        let currency = inst.value(market, as_of)?.currency();
        match observed {
            None => observed = Some(currency),
            Some(existing) if existing == currency => {}
            Some(_) => {
                return Err(finstack_quant_core::Error::Validation(
                    "XVA exposure requires an explicit reporting currency for mixed-currency portfolios"
                        .to_string(),
                ))
            }
        }
    }

    observed.ok_or_else(|| {
        finstack_quant_core::Error::Validation(
            "XVA exposure requires at least one instrument to infer reporting currency".to_string(),
        )
    })
}

fn convert_to_reporting(
    value: Money,
    reporting_currency: Currency,
    market: &MarketContext,
    on: Date,
) -> finstack_quant_core::Result<f64> {
    if value.currency() == reporting_currency {
        return Ok(value.amount());
    }

    let fx = market.fx().ok_or_else(|| {
        finstack_quant_core::Error::Validation(format!(
            "XVA exposure requires FX data to convert {} into reporting currency {}",
            value.currency(),
            reporting_currency
        ))
    })?;
    let rate = fx
        .rate(FxQuery::with_policy(
            value.currency(),
            reporting_currency,
            on,
            FxConversionPolicy::CashflowDate,
        ))?
        .rate;
    Ok(value.amount() * rate)
}

/// Net (close-out netted) portfolio value in the reporting currency at `on`.
fn net_portfolio_value(
    instruments: &[Arc<dyn Valuable>],
    market: &MarketContext,
    on: Date,
    reporting_currency: Currency,
    horizon_years: f64,
) -> finstack_quant_core::Result<f64> {
    let mut values = Vec::with_capacity(instruments.len());
    for inst in instruments {
        let value = inst
            .value(market, on)
            .and_then(|value| convert_to_reporting(value, reporting_currency, market, on))
            .map_err(|error| {
                finstack_quant_core::Error::Validation(format!(
                    "XVA valuation failed for instrument '{}' at horizon {horizon_years} years: {error}",
                    inst.id()
                ))
            })?;
        values.push(value);
    }
    Ok(neumaier_sum(values.iter().copied()))
}

/// Linear interpolation of a piecewise-linear path over `times`, anchored at
/// `(0, v0)` and clamped to `[0, times.last()]`.
///
/// `times` must be strictly increasing and positive (guaranteed by
/// `XvaConfig::validate`); `values.len() == times.len()`.
fn interpolate_lagged(times: &[f64], values: &[f64], v0: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return v0;
    }
    let mut prev_t = 0.0;
    let mut prev_v = v0;
    for (&ti, &vi) in times.iter().zip(values.iter()) {
        if t <= ti {
            let w = (t - prev_t) / (ti - prev_t);
            return prev_v + w * (vi - prev_v);
        }
        prev_t = ti;
        prev_v = vi;
    }
    prev_v
}

/// Linear-interpolated quantile of `samples` (sorted in place).
fn interpolate_quantile(samples: &mut [f64], quantile: f64) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if samples.len() == 1 {
        return samples[0];
    }

    let scaled = quantile * (samples.len() - 1) as f64;
    let lo = scaled.floor() as usize;
    let hi = scaled.ceil() as usize;
    if lo == hi {
        samples[lo]
    } else {
        let w = scaled - lo as f64;
        samples[lo] * (1.0 - w) + samples[hi] * w
    }
}

/// Compute the exposure profile for a portfolio of instruments.
///
/// For each time point in the configuration's time grid, this function:
/// 1. Rolls the market data forward to `as_of + t` (deterministic roll)
/// 2. Re-values each instrument at the future date
/// 3. Applies close-out netting across the netting set
/// 4. Applies CSA collateral terms (if present)
/// 5. Records EPE and ENE values
///
/// # Arguments
///
/// * `instruments` - Portfolio of instruments in this netting set
/// * `market` - Current market data context
/// * `as_of` - Valuation date (T+0)
/// * `config` - XVA configuration (time grid, recovery, etc.)
/// * `netting_set` - Netting set specification with optional CSA
///
/// # Returns
///
/// An [`ExposureProfile`] containing MtM, EPE, and ENE at each time point.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration validation fails
/// - More than 50% of time grid points fail (market roll or valuation)
///
/// # Warnings
///
/// Time points where market data cannot be rolled forward are recorded
/// as zero exposure with a log warning. Instruments that fail to value
/// at a given horizon are treated as zero value (matured/settled).
///
/// # Limitations
///
/// - Uses deterministic (single-scenario) exposure; no Monte Carlo
/// - PFE equals EPE in this simplified model
/// - Margin period of risk (MPOR) *is* modeled when `netting_set.csa` is
///   present and `csa.mpor_days > 0`: collateral held at time `t` is lagged
///   to the (interpolated) net portfolio value at `t − MPOR`, following
///   Andersen, Pykhtin & Sokol (2017), so a rising exposure path produces
///   larger (correctly gap-risk-inclusive) collateralized EPE than the
///   `MPOR = 0` case. See [`super::netting::apply_collateral_mpor`].
/// - Curve roll uses constant-curves assumption (no carry/theta)
///
/// # References
///
/// - Andersen, L., Pykhtin, M., & Sokol, A. (2017). "Rethinking the margin
///   period of risk." *Journal of Credit Risk*, 13(1), 1-45.
/// - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
/// - BCBS 279 SA-CCR: `docs/REFERENCES.md#bcbs-279-saccr`
#[tracing::instrument(skip(instruments, market), fields(grid_points = config.time_grid.len()))]
pub fn compute_exposure_profile(
    instruments: &[Arc<dyn Valuable>],
    market: &MarketContext,
    as_of: Date,
    config: &XvaConfig,
    netting_set: &XvaNettingSet,
) -> finstack_quant_core::Result<ExposureProfile> {
    config.validate()?;
    let reporting_currency = resolve_reporting_currency(instruments, market, as_of, netting_set)?;

    let n = config.time_grid.len();
    let mut times = Vec::with_capacity(n);
    let mut mtm_values = Vec::with_capacity(n);

    for &t in &config.time_grid {
        // Convert years to days using ACT/365F convention
        let days = years_to_days_act_365f(t);
        let future_date = as_of + time::Duration::days(days);

        // Roll market data forward (constant-curves assumption).
        let rolled_market = market.roll_forward(days).map_err(|error| {
            finstack_quant_core::Error::Validation(format!(
                "XVA market roll failed at horizon {t} years: {error}"
            ))
        })?;

        let net_value = net_portfolio_value(
            instruments,
            &rolled_market,
            future_date,
            reporting_currency,
            t,
        )?;
        times.push(t);
        mtm_values.push(net_value);
    }

    let (epe, ene) = if let Some(ref csa) = netting_set.csa {
        // MPOR gap risk: collateral held at t was called against the exposure
        // at t − δ. Anchor the lag interpolation at today's net value.
        let mpor_lag_years = f64::from(csa.mpor_days) / CALENDAR_DAYS_PER_YEAR;
        let net_value_0 = net_portfolio_value(instruments, market, as_of, reporting_currency, 0.0)?;
        let mut epe = Vec::with_capacity(n);
        let mut ene = Vec::with_capacity(n);
        for (i, &t) in times.iter().enumerate() {
            let net_now = mtm_values[i];
            let net_lag = interpolate_lagged(&times, &mtm_values, net_value_0, t - mpor_lag_years);
            epe.push(apply_collateral_mpor(
                net_now.max(0.0),
                net_lag.max(0.0),
                csa,
            ));
            // `independent_amount` is collateral posted by the counterparty:
            // it reduces EPE, but not the bank-posted side (ENE/DVA).
            ene.push(apply_variation_margin_mpor(
                (-net_now).max(0.0),
                (-net_lag).max(0.0),
                csa,
            ));
        }
        (epe, ene)
    } else {
        (
            mtm_values.iter().map(|v| v.max(0.0)).collect(),
            mtm_values.iter().map(|v| (-v).max(0.0)).collect(),
        )
    };

    Ok(ExposureProfile {
        times,
        mtm_values,
        epe,
        ene,
        diagnostics: None,
    })
}

/// Time-major pathwise simulation output shared by the stochastic engines.
struct SimulatedExposurePaths {
    /// Pathwise MtM, indexed `[step_idx * num_paths + path_idx]`.
    mtms: Vec<f64>,
    /// Portfolio MtM at `t = 0` (all paths share the initial state); used as
    /// the anchor for MPOR lag interpolation.
    initial_mtm: f64,
    /// Optional pathwise IM, same indexing as `mtms`.
    im: Option<Vec<f64>>,
}

/// Simulate factor paths and evaluate MtM (and optionally IM) at each grid point.
///
/// RNG discipline is identical to the original engine: one Philox substream
/// per path (`substream(path_idx + 1)`), shocks drawn in step order, so results
/// are deterministic for a fixed seed and independent of aggregation.
#[expect(
    clippy::too_many_arguments,
    reason = "internal helper shared by both public stochastic entry points; \
              splitting args into a config struct would only move the surface, not shrink it"
)]
fn simulate_exposure_paths<P, D>(
    process: &P,
    discretization: &D,
    initial_state: &[f64],
    time_grid: &[f64],
    num_paths: usize,
    seed: u64,
    valuation_fn: &dyn Fn(&PathState, f64) -> finstack_quant_core::Result<f64>,
    im_model: Option<&dyn PathImModel>,
) -> finstack_quant_core::Result<SimulatedExposurePaths>
where
    P: StochasticProcess,
    D: Discretization<P>,
{
    if initial_state.len() != process.dim() {
        return Err(finstack_quant_core::Error::Validation(format!(
            "Stochastic exposure: initial_state length {} must match process dim {}",
            initial_state.len(),
            process.dim()
        )));
    }

    let time_count = time_grid.len();
    let total_cells = time_count.checked_mul(num_paths).ok_or_else(|| {
        finstack_quant_core::Error::Validation(format!(
            "Stochastic exposure: time_count ({time_count}) * num_paths ({num_paths}) overflows usize"
        ))
    })?;
    let mut mtms = vec![0.0f64; total_cells];
    let mut im = im_model.map(|_| vec![0.0f64; total_cells]);

    // t = 0 anchor valuation (consumes no randomness).
    let mut initial_ps = PathState::new(0, 0.0);
    initial_ps.set(state_keys::TIME, 0.0);
    initial_ps.set(state_keys::STEP, 0.0);
    process.populate_path_state(initial_state, &mut initial_ps);
    let initial_mtm = valuation_fn(&initial_ps, 0.0)?;

    let base_rng = PhiloxRng::new(seed);
    let mut state_vector = vec![0.0; process.dim()];
    let mut shocks = vec![0.0; process.num_factors()];
    let mut work = vec![0.0; discretization.work_size(process)];

    for path_idx in 0..num_paths {
        let mut rng = base_rng.substream((path_idx + 1) as u64);
        state_vector.copy_from_slice(initial_state);
        let mut prev_t = 0.0;

        for (step_idx, &t) in time_grid.iter().enumerate() {
            let dt = t - prev_t;
            rng.fill_std_normals(&mut shocks);
            discretization.step(process, prev_t, dt, &mut state_vector, &shocks, &mut work);

            let mut path_state = PathState::new(step_idx + 1, t);
            path_state.set(state_keys::TIME, t);
            path_state.set(state_keys::STEP, (step_idx + 1) as f64);
            process.populate_path_state(&state_vector, &mut path_state);

            let cell = step_idx * num_paths + path_idx;
            mtms[cell] = valuation_fn(&path_state, t)?;
            if let (Some(im_buf), Some(model)) = (im.as_mut(), im_model) {
                im_buf[cell] = model.im_on_path(&path_state, t)?;
            }
            prev_t = t;
        }
    }

    Ok(SimulatedExposurePaths {
        mtms,
        initial_mtm,
        im,
    })
}

/// Interpolate one path's MtM at time `t` (linear, anchored at `(0, v0)`,
/// clamped to the grid span). Time-major indexing as in `SimulatedExposurePaths`.
fn interpolate_path_value(
    mtms: &[f64],
    num_paths: usize,
    path_idx: usize,
    times: &[f64],
    v0: f64,
    t: f64,
) -> f64 {
    if t <= 0.0 {
        return v0;
    }
    let mut prev_t = 0.0;
    let mut prev_v = v0;
    for (step_idx, &ti) in times.iter().enumerate() {
        let vi = mtms[step_idx * num_paths + path_idx];
        if t <= ti {
            let w = (t - prev_t) / (ti - prev_t);
            return prev_v + w * (vi - prev_v);
        }
        prev_t = ti;
        prev_v = vi;
    }
    prev_v
}

/// Shared netting/collateral/quantile aggregation for the stochastic engines.
///
/// Close-out netting is `max(V_p(t), 0)` per path (the valuer/callback returns
/// the netted portfolio MtM). When `csa` is present, MPOR-lagged collateral is
/// applied per path before averaging: collateral at `t` reflects the path's
/// exposure at `t − mpor_days/365` (linear interpolation, `t = 0` anchor).
fn aggregate_stochastic_profile(
    time_grid: &[f64],
    sim: &SimulatedExposurePaths,
    num_paths: usize,
    pfe_quantile: f64,
    csa: Option<&CsaTerms>,
) -> finstack_quant_core::Result<StochasticExposureProfile> {
    let time_count = time_grid.len();
    let mut mtm_values = Vec::with_capacity(time_count);
    let mut epe = Vec::with_capacity(time_count);
    let mut ene = Vec::with_capacity(time_count);
    let mut pfe_profile = Vec::with_capacity(time_count);
    let mut positive_buf = Vec::with_capacity(num_paths);
    let path_count_f = num_paths as f64;
    let mpor_lag_years = csa.map_or(0.0, |c| f64::from(c.mpor_days) / CALENDAR_DAYS_PER_YEAR);

    for (step_idx, &t) in time_grid.iter().enumerate() {
        positive_buf.clear();
        let mut sum_mtm = 0.0;
        let mut sum_pos = 0.0;
        let mut sum_neg = 0.0;
        for path_idx in 0..num_paths {
            let mtm = sim.mtms[step_idx * num_paths + path_idx];
            sum_mtm += mtm;
            let (pos, neg) = match csa {
                Some(csa) => {
                    let lag_mtm = interpolate_path_value(
                        &sim.mtms,
                        num_paths,
                        path_idx,
                        time_grid,
                        sim.initial_mtm,
                        t - mpor_lag_years,
                    );
                    (
                        apply_collateral_mpor(mtm.max(0.0), lag_mtm.max(0.0), csa),
                        apply_variation_margin_mpor((-mtm).max(0.0), (-lag_mtm).max(0.0), csa),
                    )
                }
                None => (mtm.max(0.0), (-mtm).max(0.0)),
            };
            sum_pos += pos;
            sum_neg += neg;
            positive_buf.push(pos);
        }
        mtm_values.push(sum_mtm / path_count_f);
        epe.push(sum_pos / path_count_f);
        ene.push(sum_neg / path_count_f);
        pfe_profile.push(interpolate_quantile(&mut positive_buf, pfe_quantile));
    }

    let im_profile = sim.im.as_ref().map(|im| {
        (0..time_count)
            .map(|step_idx| {
                let row = &im[step_idx * num_paths..(step_idx + 1) * num_paths];
                row.iter().sum::<f64>() / path_count_f
            })
            .collect::<Vec<f64>>()
    });

    let profile = ExposureProfile {
        times: time_grid.to_vec(),
        mtm_values,
        epe,
        ene,
        diagnostics: None,
    };
    profile.validate()?;

    let stochastic_profile = StochasticExposureProfile {
        profile,
        pfe_profile,
        path_count: num_paths,
        pfe_quantile,
        im_profile,
    };
    stochastic_profile.validate()?;
    Ok(stochastic_profile)
}

/// Compute a stochastic exposure profile using the Monte Carlo primitives.
///
/// This engine simulates factor paths and revalues the portfolio through a
/// pathwise callback at each time bucket. It keeps the current deterministic
/// exposure API intact while providing a reusable route to genuine exposure
/// distributions and quantile-based PFE.
///
/// # Arguments
///
/// * `process` - Stochastic process that evolves the factor state
/// * `discretization` - Time-stepping scheme used to advance `process`
/// * `initial_state` - Initial factor state vector; length must equal `process.dim()`
/// * `xva_config` - Exposure time grid expressed as year fractions
/// * `stochastic_config` - Monte Carlo path count, RNG seed, and PFE quantile
/// * `valuation_fn` - Callback that converts a simulated [`PathState`] into a
///   signed portfolio MtM in reporting-currency units
///
/// # Returns
///
/// A [`StochasticExposureProfile`] containing path-average MtM/EPE/ENE and a
/// quantile-based positive-exposure profile.
///
/// # Errors
///
/// Returns an error if:
/// - `xva_config` or `stochastic_config` fails validation
/// - `initial_state` has the wrong dimension
/// - `valuation_fn` fails for any simulated path/time step
/// - the aggregated profile fails internal validation
///
/// # Example
///
/// ```ignore
/// use finstack_quant_margin::xva::types::{StochasticExposureConfig, XvaConfig};
/// use finstack_quant_margin::xva::types::{StochasticExposureConfig, XvaConfig};
///
/// #
/// # fn example<P, D>(process: &P, discretization: &D) -> finstack_quant_core::Result<()>
/// # where
/// #     P: finstack_quant_monte_carlo::StochasticProcess,
/// #     D: finstack_quant_monte_carlo::Discretization<P>,
/// # {
/// let xva_config = XvaConfig {
///     time_grid: vec![0.25, 0.5, 1.0],
///     ..XvaConfig::default()
/// };
/// let mc_config = StochasticExposureConfig::default();
/// let initial_state = vec![0.0; process.dim()];
///
/// let profile = compute_stochastic_exposure_profile(
///     process,
///     discretization,
///     &initial_state,
///     &xva_config,
///     &mc_config,
///     |_path_state| Ok(0.0),
/// )?;
/// # let _ = profile;
/// # Ok(())
/// # }
/// ```
///
/// # Limitations
///
/// - Collateral and netting must be represented inside `valuation_fn` or in the
///   factor-to-value mapping around it; this helper only simulates pathwise MtM.
/// - Time points are taken directly from `xva_config.time_grid` and are assumed
///   to be year fractions.
///
/// # References
///
/// - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
/// - BCBS 279 SA-CCR: `docs/REFERENCES.md#bcbs-279-saccr`
pub fn compute_stochastic_exposure_profile<P, D, V>(
    process: &P,
    discretization: &D,
    initial_state: &[f64],
    xva_config: &XvaConfig,
    stochastic_config: &StochasticExposureConfig,
    valuation_fn: V,
) -> finstack_quant_core::Result<StochasticExposureProfile>
where
    P: StochasticProcess,
    D: Discretization<P>,
    V: Fn(&PathState) -> finstack_quant_core::Result<f64>,
{
    xva_config.validate()?;
    stochastic_config.validate()?;
    let wrapped = |state: &PathState, _t: f64| valuation_fn(state);
    let sim = simulate_exposure_paths(
        process,
        discretization,
        initial_state,
        &xva_config.time_grid,
        stochastic_config.num_paths,
        stochastic_config.seed,
        &wrapped,
        None,
    )?;
    aggregate_stochastic_profile(
        &xva_config.time_grid,
        &sim,
        stochastic_config.num_paths,
        stochastic_config.pfe_quantile,
        None,
    )
}

/// Compute a stochastic exposure profile by repricing the actual portfolio on
/// simulated paths through a [`PathValuer`].
///
/// This is the path-consistent counterpart of
/// [`compute_stochastic_exposure_profile`]: instead of a generic valuation
/// callback it takes the margin-side [`PathValuer`] bridge (implemented by the
/// valuations crate or the caller), and it applies the netting set's CSA —
/// including MPOR-lagged collateral (gap risk) — and quantile PFE inside the
/// engine, per path:
///
/// ```text
/// E_p(t)    = max(V_p(t), 0)                                  (close-out netting)
/// C_p(t)    = max(E_p(t − δ) − (threshold + MTA), 0)          (MPOR-lagged collateral)
/// EPE(t)    = mean_p max(E_p(t) − C_p(t) − IA, 0)
/// PFE_α(t)  = quantile_α over p of the collateralized exposure
/// ```
///
/// with `δ = csa.mpor_days / 365` and per-path linear interpolation of the
/// lagged value (anchored at the `t = 0` portfolio value). Without a CSA the
/// raw netted exposures are aggregated.
///
/// When `im_model` is provided, per-path IM is evaluated at every grid point
/// and its mean is returned in `StochasticExposureProfile::im_profile`
/// (phase-2 MVA input; see [`crate::xva::mva::compute_mva`]).
///
/// # Errors
///
/// Returns an error if configuration validation fails, the initial state has
/// the wrong dimension, or the valuer / IM model fails on any path.
///
/// # Determinism
///
/// Fixed `stochastic_config.seed` gives bit-identical results (one Philox
/// substream per path, independent of aggregation order).
///
/// # Limitations
///
/// - **MPOR gap-risk discretization**: the lagged value at `t − δ` is linearly
///   interpolated between adjacent `time_grid` points (`interpolate_path_value`),
///   not simulated directly. For a Brownian-driven factor with grid spacing
///   `Δt`, this makes the modeled gap increment `(δ/Δt)·(V(t) − V(t−Δt))`, whose
///   standard deviation is `σ·δ/√Δt = σ√δ·√(δ/Δt)` rather than the true `σ√δ`
///   over the MPOR window. With `δ = 10/365` years and a typical `Δt = 0.5`
///   year grid, `√(δ/Δt) ≈ 0.23`, i.e. gap risk is understated by roughly
///   `4.3×`. Gap risk is only accurate when the grid spacing is comparable to
///   the MPOR; callers who need accurate MPOR gap risk should add secondary
///   valuation points at `t − δ` to `time_grid`. This is exactly the
///   discretization effect Andersen, Pykhtin & Sokol (2017) warn about.
/// - **One-sided MPOR window**: `EPE` and `ENE` are aggregated from separately
///   floored `max(V, 0)` / `max(-V, 0)` legs (see `aggregate_stochastic_profile`),
///   so when `V` crosses zero inside the MPOR window, outstanding posted VM on
///   the side that goes to zero is floored at zero rather than added to
///   exposure on the other side — consistent with the locked D4 cap semantics
///   (see [`super::netting::apply_collateral_mpor`] /
///   [`super::netting::apply_variation_margin_mpor`]).
///
/// # References
///
/// - Green, A. (2015). *XVA*. Wiley. Chapters 3, 10.
/// - Andersen, L., Pykhtin, M., & Sokol, A. (2017). "Rethinking the margin
///   period of risk." *Journal of Credit Risk*, 13(1).
/// - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
#[expect(
    clippy::too_many_arguments,
    reason = "public path-consistent entry point mirrors the deterministic engine's argument \
              shape plus the valuer/netting-set/IM-model additions; a config struct would just \
              relocate the surface"
)]
pub fn compute_stochastic_exposure_with_valuer<P, D>(
    process: &P,
    discretization: &D,
    initial_state: &[f64],
    valuer: &dyn PathValuer,
    xva_config: &XvaConfig,
    stochastic_config: &StochasticExposureConfig,
    netting_set: &XvaNettingSet,
    im_model: Option<&dyn PathImModel>,
) -> finstack_quant_core::Result<StochasticExposureProfile>
where
    P: StochasticProcess,
    D: Discretization<P>,
{
    xva_config.validate()?;
    stochastic_config.validate()?;
    let wrapped = |state: &PathState, t: f64| valuer.value_on_path(state, t);
    let sim = simulate_exposure_paths(
        process,
        discretization,
        initial_state,
        &xva_config.time_grid,
        stochastic_config.num_paths,
        stochastic_config.seed,
        &wrapped,
        im_model,
    )?;
    aggregate_stochastic_profile(
        &xva_config.time_grid,
        &sim,
        stochastic_config.num_paths,
        stochastic_config.pfe_quantile,
        netting_set.csa.as_ref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xva::cva::compute_cva;
    use crate::xva::netting::{apply_collateral, apply_netting};
    use crate::xva::types::CsaTerms;
    use finstack_quant_core::currency::Currency;
    use finstack_quant_core::dates::Date;
    use finstack_quant_core::market_data::context::MarketContext;
    use finstack_quant_core::market_data::term_structures::{DiscountCurve, HazardCurve};
    use finstack_quant_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_quant_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    // Note: Full integration tests require constructing instrument and market mocks.
    // These unit tests verify the exposure profile logic with synthetic data.

    #[derive(Clone, Debug)]
    struct StaticInstrument {
        id: String,
        pv: f64,
    }

    impl StaticInstrument {
        fn new(id: &str, pv: f64) -> Self {
            Self {
                id: id.to_string(),
                pv,
            }
        }
    }

    impl Valuable for StaticInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn value(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> finstack_quant_core::Result<Money> {
            Ok(Money::new(self.pv, Currency::USD))
        }
    }

    #[derive(Clone, Debug)]
    struct MultiCurrencyStaticInstrument {
        id: String,
        pv: Money,
    }

    impl MultiCurrencyStaticInstrument {
        fn new(id: &str, amount: f64, currency: Currency) -> Self {
            Self {
                id: id.to_string(),
                pv: Money::new(amount, currency),
            }
        }
    }

    impl Valuable for MultiCurrencyStaticInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn value(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> finstack_quant_core::Result<Money> {
            Ok(self.pv)
        }
    }

    #[test]
    fn exposure_profile_basic_structure() {
        let config = XvaConfig {
            time_grid: vec![0.25, 0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        config.validate().expect("Config should be valid");
        assert_eq!(config.time_grid.len(), 3);
    }

    #[test]
    fn years_to_days_act_365f_half_up_midpoint() {
        // 0.5 × 365 = 182.5 days → nearest whole day is 183 (half-up), not 182 (IEEE tie-to-even).
        assert_eq!(years_to_days_act_365f(0.5), 183);
    }

    #[test]
    fn exposure_profile_net_mtm_stable_summation() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![
            Arc::new(StaticInstrument::new("BIG", 1e16)),
            Arc::new(StaticInstrument::new("ONE", 1.0)),
            Arc::new(StaticInstrument::new("BIGNEG", -1e16)),
        ];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let netting_set = XvaNettingSet {
            id: "NS-STABLE-SUM".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");

        assert!(
            (profile.mtm_values[0] - 1.0).abs() < 1e-10,
            "expected net MtM ≈ 1, got {}",
            profile.mtm_values[0]
        );
    }

    #[test]
    fn exposure_profile_supports_valuable_trait_objects() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> =
            vec![Arc::new(StaticInstrument::new("USD-PV", 100.0))];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let netting_set = XvaNettingSet {
            id: "NS-VALUABLE".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");

        assert_eq!(profile.mtm_values, vec![100.0]);
        assert_eq!(profile.epe, vec![100.0]);
        assert_eq!(profile.ene, vec![0.0]);
    }

    #[test]
    fn exposure_profile_epe_non_negative() {
        // EPE by construction is max(V, 0) which is always >= 0
        let profile = ExposureProfile {
            times: vec![0.25, 0.5, 1.0],
            mtm_values: vec![100.0, -50.0, 25.0],
            epe: vec![100.0, 0.0, 25.0],
            ene: vec![0.0, 50.0, 0.0],
            diagnostics: None,
        };
        for &e in &profile.epe {
            assert!(e >= 0.0, "EPE must be non-negative, got {e}");
        }
    }

    #[test]
    fn exposure_profile_ene_non_negative() {
        let profile = ExposureProfile {
            times: vec![0.25, 0.5],
            mtm_values: vec![100.0, -50.0],
            epe: vec![100.0, 0.0],
            ene: vec![0.0, 50.0],
            diagnostics: None,
        };
        for &e in &profile.ene {
            assert!(e >= 0.0, "ENE must be non-negative, got {e}");
        }
    }

    // ── Integration tests: synthetic profiles through CVA pipeline ──

    /// Helper: build a flat hazard rate curve.
    fn flat_hazard_curve(lambda: f64) -> HazardCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        HazardCurve::builder("COUNTERPARTY")
            .base_date(base)
            .knots([(0.0, lambda), (30.0, lambda)])
            .build()
            .expect("HazardCurve should build")
    }

    /// Helper: build a flat discount curve.
    fn flat_discount_curve(rate: f64) -> DiscountCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let knots: Vec<(f64, f64)> = (0..=60)
            .map(|i| {
                let t = i as f64 * 0.5;
                (t, (-rate * t).exp())
            })
            .collect();
        DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots(knots)
            .interp(finstack_quant_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve should build")
    }

    #[test]
    fn collateral_reduces_cva_vs_uncollateralized() {
        // A CSA with zero threshold should reduce CVA compared to uncollateralized
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();

        // Uncollateralized profile
        let uncollat_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| 1_000_000.0).collect(),
            epe: times.iter().map(|_| 1_000_000.0).collect(),
            ene: times.iter().map(|_| 0.0).collect(),
            diagnostics: None,
        };

        // Collateralized profile: apply CSA to reduce EPE
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 500.0,
            mpor_days: 10,
            independent_amount: 0.0,
        };
        let collat_epe: Vec<f64> = times
            .iter()
            .map(|_| apply_collateral(1_000_000.0, &csa))
            .collect();
        let collat_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| 1_000_000.0).collect(),
            epe: collat_epe,
            ene: times.iter().map(|_| 0.0).collect(),
            diagnostics: None,
        };

        let cva_uncollat = compute_cva(&uncollat_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;
        let cva_collat = compute_cva(&collat_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;

        assert!(
            cva_collat < cva_uncollat,
            "Collateralized CVA ({cva_collat:.2}) should be less than uncollateralized ({cva_uncollat:.2})"
        );
    }

    #[test]
    fn netting_reduces_cva_vs_gross() {
        // Netting offsetting trades should produce lower CVA
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();

        // Gross: treat each trade individually (sum of positive exposures)
        let trade_a: f64 = 1_000_000.0;
        let trade_b: f64 = -800_000.0;
        let gross_epe: Vec<f64> = times.iter().map(|_| trade_a.max(0.0)).collect();
        let gross_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| trade_a).collect(),
            epe: gross_epe,
            ene: times.iter().map(|_| 0.0).collect(),
            diagnostics: None,
        };

        // Netted: use netting to compute net exposure
        let net_epe: Vec<f64> = times
            .iter()
            .map(|_| apply_netting(&[trade_a, trade_b]))
            .collect();
        let net_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| trade_a + trade_b).collect(),
            epe: net_epe,
            ene: times
                .iter()
                .map(|_| (-(trade_a + trade_b)).max(0.0))
                .collect(),
            diagnostics: None,
        };

        let cva_gross = compute_cva(&gross_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;
        let cva_net = compute_cva(&net_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;

        assert!(
            cva_net < cva_gross,
            "Netted CVA ({cva_net:.2}) should be less than gross CVA ({cva_gross:.2})"
        );
    }

    #[test]
    fn zero_value_portfolio_gives_zero_cva() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=10).map(|i| i as f64).collect();

        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: vec![0.0; times.len()],
            epe: vec![0.0; times.len()],
            ene: vec![0.0; times.len()],
            diagnostics: None,
        };

        let result = compute_cva(&profile, &hazard, &discount, 0.40)
            .expect("CVA should compute for zero portfolio");
        assert!(
            result.cva.abs() < 1e-12,
            "CVA for zero-value portfolio should be zero, got {}",
            result.cva
        );
    }

    #[test]
    fn single_instrument_profile() {
        // Single instrument with declining exposure (e.g., amortizing swap)
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=10).map(|i| i as f64).collect();

        let epe: Vec<f64> = times
            .iter()
            .map(|&t| 1_000_000.0 * (1.0 - t / 10.0))
            .collect();
        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: epe.clone(),
            epe,
            ene: vec![0.0; times.len()],
            diagnostics: None,
        };

        let result = compute_cva(&profile, &hazard, &discount, 0.40)
            .expect("CVA should compute for declining profile");

        assert!(result.cva > 0.0, "CVA should be positive");

        // Effective EPE profile should be non-decreasing
        for i in 1..result.effective_epe_profile.len() {
            assert!(
                result.effective_epe_profile[i].1 >= result.effective_epe_profile[i - 1].1 - 1e-12,
                "Effective EPE profile must be non-decreasing"
            );
        }

        // Validate the profile
        profile.validate().expect("Profile should be valid");
    }

    #[test]
    fn exposure_profile_validates_after_construction() {
        let times = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let profile = ExposureProfile {
            times,
            mtm_values: vec![100.0, -50.0, 25.0, 75.0, -10.0],
            epe: vec![100.0, 0.0, 25.0, 75.0, 0.0],
            ene: vec![0.0, 50.0, 0.0, 0.0, 10.0],
            diagnostics: None,
        };
        profile
            .validate()
            .expect("Manually constructed valid profile should pass validation");
    }

    #[test]
    fn collateral_reduces_ene_for_negative_net_mtm() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> =
            vec![Arc::new(StaticInstrument::new("NEGATIVE-PV", -1_000_000.0))];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 500.0,
            mpor_days: 10,
            independent_amount: 0.0,
        };
        let netting_set = XvaNettingSet {
            id: "CSA-NEG".into(),
            counterparty_id: "CP".into(),
            csa: Some(csa.clone()),
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");

        let expected_ene = apply_collateral(1_000_000.0, &csa);
        assert!(
            (profile.ene[0] - expected_ene).abs() < 1e-12,
            "CSA should reduce negative exposure symmetrically: got {}, expected {}",
            profile.ene[0],
            expected_ene
        );
    }

    /// Instrument whose value grows linearly with the valuation date:
    /// V(date) = slope_per_year × years(as ACT/365F days) since 2025-01-01.
    #[derive(Clone, Debug)]
    struct GrowingInstrument {
        id: String,
        slope_per_year: f64,
    }

    impl Valuable for GrowingInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn value(
            &self,
            _market: &MarketContext,
            as_of: Date,
        ) -> finstack_quant_core::Result<Money> {
            let base = Date::from_calendar_date(2025, Month::January, 1)
                .map_err(|e| finstack_quant_core::Error::Validation(e.to_string()))?;
            let days = (as_of - base).whole_days() as f64;
            Ok(Money::new(
                self.slope_per_year * days / 365.0,
                Currency::USD,
            ))
        }
    }

    #[test]
    fn mpor_lag_produces_gap_risk_on_growing_exposure() {
        // Zero-threshold/zero-MTA/zero-IA CSA with 10-day MPOR.
        //
        // Exposure grid (ACT/365F day rounding, half-up — see
        // years_to_days_act_365f):
        //   t1 = 0.25 → 91 days  → E1 = 1000 × 91/365  = 249.31506849315068
        //   t2 = 0.50 → 183 days → E2 = 1000 × 183/365 = 501.36986301369863
        //   E(0) anchor = 0 (portfolio is worth 0 at as_of)
        //   δ = 10/365 = 0.0273972602739726 years
        //
        // Exposure is piecewise linear on {0, t1, t2}, so the interpolated
        // lagged values give closed forms:
        //   epe(t1) = E1 − E1·(t1 − δ)/t1        = E1·δ/t1        = E1 × (8/73)
        //           = 27.322199 (≈ 27.32219929)
        //   epe(t2) = E2 − [E1 + (E2−E1)·(t2−δ−t1)/(t2−t1)]
        //           = (E2 − E1)·δ/(t2 − t1)      = (E2−E1) × (8/73)
        //           = 27.622443 (≈ 27.62244325)
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![Arc::new(GrowingInstrument {
            id: "GROW".into(),
            slope_per_year: 1000.0,
        })];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25, 0.5],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 0.0,
            mpor_days: 10,
            independent_amount: 0.0,
        };
        let netting_set = XvaNettingSet {
            id: "NS-MPOR".into(),
            counterparty_id: "CP".into(),
            csa: Some(csa),
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");

        let e1 = 1000.0 * 91.0 / 365.0;
        let e2 = 1000.0 * 183.0 / 365.0;
        let lag = 10.0 / 365.0;
        let expected1 = e1 * lag / 0.25;
        let expected2 = (e2 - e1) * lag / 0.25;
        assert!(
            (profile.epe[0] - expected1).abs() < 1e-9,
            "epe[0]={} expected {expected1}",
            profile.epe[0]
        );
        assert!(
            (profile.epe[1] - expected2).abs() < 1e-9,
            "epe[1]={} expected {expected2}",
            profile.epe[1]
        );
        assert!(profile.ene.iter().all(|&v| v.abs() < 1e-12));
    }

    #[test]
    fn mpor_lag_produces_gap_risk_on_declining_exposure() {
        // Mirror of `mpor_lag_produces_gap_risk_on_growing_exposure`, but with
        // a DECLINING net portfolio value (negative slope), which drives the
        // ENE/DVA side of `compute_exposure_profile` instead of EPE. The only
        // other ENE assertion among these tests is the trivial "all ≈ 0" in
        // the growing-exposure test above; this test hand-computes nonzero
        // expected ENE values to actually exercise
        // `apply_variation_margin_mpor`'s lag interpolation.
        //
        // Zero-threshold/zero-MTA/zero-IA CSA with 10-day MPOR.
        //
        // Exposure grid (ACT/365F day rounding, half-up):
        //   t1 = 0.25 → 91 days  → V1 = -1000 × 91/365  = -249.31506849315068
        //   t2 = 0.50 → 183 days → V2 = -1000 × 183/365 = -501.36986301369863
        //   V(0) anchor = 0
        //   δ = 10/365 = 0.0273972602739726 years
        //
        // -V(t) is piecewise linear on {0, t1, t2} and mirrors the growing
        // case exactly (same magnitudes, opposite sign), so the same closed
        // forms apply to ENE:
        //   ene(t1) = |V1|·δ/t1          = 27.322199 (≈ 27.32219929)
        //   ene(t2) = (|V2|−|V1|)·δ/0.25 = 27.622443 (≈ 27.62244325)
        // and EPE is ≈ 0 throughout (portfolio value never goes positive).
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![Arc::new(GrowingInstrument {
            id: "DECLINE".into(),
            slope_per_year: -1000.0,
        })];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25, 0.5],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 0.0,
            mpor_days: 10,
            independent_amount: 0.0,
        };
        let netting_set = XvaNettingSet {
            id: "NS-MPOR-DECLINE".into(),
            counterparty_id: "CP".into(),
            csa: Some(csa),
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");

        let e1 = 1000.0 * 91.0 / 365.0;
        let e2 = 1000.0 * 183.0 / 365.0;
        let lag = 10.0 / 365.0;
        let expected1 = e1 * lag / 0.25;
        let expected2 = (e2 - e1) * lag / 0.25;
        assert!(
            (profile.ene[0] - expected1).abs() < 1e-9,
            "ene[0]={} expected {expected1}",
            profile.ene[0]
        );
        assert!(
            (profile.ene[1] - expected2).abs() < 1e-9,
            "ene[1]={} expected {expected2}",
            profile.ene[1]
        );
        assert!(profile.epe.iter().all(|&v| v.abs() < 1e-12));
    }

    #[test]
    fn mpor_zero_days_matches_classic_collateral_on_growing_exposure() {
        // Same setup with mpor_days = 0: the lagged exposure equals the current
        // exposure, so a zero-threshold CSA fully collateralizes (EPE = 0),
        // matching apply_collateral semantics.
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![Arc::new(GrowingInstrument {
            id: "GROW".into(),
            slope_per_year: 1000.0,
        })];
        let market = MarketContext::new();
        let config = XvaConfig {
            time_grid: vec![0.25, 0.5],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 0.0,
            mpor_days: 0,
            independent_amount: 0.0,
        };
        let netting_set = XvaNettingSet {
            id: "NS-MPOR-0".into(),
            counterparty_id: "CP".into(),
            csa: Some(csa),
            reporting_currency: None,
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("profile should compute");
        assert!(profile.epe.iter().all(|&v| v.abs() < 1e-12));
    }

    #[test]
    fn mixed_currency_profile_requires_explicit_reporting_currency() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![
            Arc::new(MultiCurrencyStaticInstrument::new(
                "USD-PV",
                100.0,
                Currency::USD,
            )),
            Arc::new(MultiCurrencyStaticInstrument::new(
                "EUR-PV",
                100.0,
                Currency::EUR,
            )),
        ];

        let provider = {
            let p = SimpleFxProvider::new();
            p.set_quote(Currency::EUR, Currency::USD, 2.0)
                .expect("valid rate");
            p
        };
        let fx = FxMatrix::new(Arc::new(provider));

        let market = MarketContext::new().insert_fx(fx);
        let config = XvaConfig {
            time_grid: vec![0.25],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let netting_set = XvaNettingSet {
            id: "MIXED-CCY".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: None,
        };

        let err = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect_err(
            "mixed-currency portfolios must not aggregate without an explicit reporting currency",
        );
        assert!(
            err.to_string().contains("reporting currency"),
            "expected reporting currency validation error, got: {err}"
        );
    }

    #[test]
    fn mixed_currency_profile_converts_into_reporting_currency() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let instruments: Vec<Arc<dyn Valuable>> = vec![
            Arc::new(MultiCurrencyStaticInstrument::new(
                "USD-PV",
                100.0,
                Currency::USD,
            )),
            Arc::new(MultiCurrencyStaticInstrument::new(
                "EUR-PV",
                100.0,
                Currency::EUR,
            )),
        ];

        let provider = {
            let p = SimpleFxProvider::new();
            p.set_quote(Currency::EUR, Currency::USD, 2.0)
                .expect("valid rate");
            p
        };
        let fx = FxMatrix::new(Arc::new(provider));

        let market = MarketContext::new().insert_fx(fx);
        let config = XvaConfig {
            time_grid: vec![0.25],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let netting_set = XvaNettingSet {
            id: "MIXED-CCY".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: Some(Currency::USD),
        };

        let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)
            .expect("mixed-currency profile should compute with explicit reporting currency");
        assert!((profile.mtm_values[0] - 300.0).abs() < 1e-12);
        assert!((profile.epe[0] - 300.0).abs() < 1e-12);
    }

    #[test]
    fn stochastic_exposure_profile_uses_quantile_based_pfe() {
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        let process = GbmProcess::with_params(0.0, 0.0, 0.25).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 1_024,
            seed: 7,
            pfe_quantile: 0.975,
        };

        let profile = compute_stochastic_exposure_profile(
            &process,
            &discretization,
            &[100.0],
            &xva_config,
            &stochastic,
            |state| Ok(state.spot().unwrap_or(0.0) - 100.0),
        )
        .expect("stochastic profile should compute");

        assert_eq!(profile.profile.times.len(), 2);
        assert_eq!(profile.pfe_profile.len(), 2);
        assert!(
            profile.pfe_profile[0] > profile.profile.epe[0],
            "PFE should exceed EPE for a non-degenerate positive-tail distribution"
        );
    }

    #[test]
    fn stochastic_exposure_profile_collapses_to_deterministic_when_paths_are_identical() {
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        let process = GbmProcess::with_params(0.0, 0.0, 0.0).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![0.25, 0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 128,
            seed: 11,
            pfe_quantile: 0.975,
        };

        let profile = compute_stochastic_exposure_profile(
            &process,
            &discretization,
            &[110.0],
            &xva_config,
            &stochastic,
            |state| Ok(state.spot().unwrap_or(0.0) - 100.0),
        )
        .expect("stochastic profile should compute");

        assert!(profile
            .profile
            .epe
            .iter()
            .zip(profile.pfe_profile.iter())
            .all(|(epe, pfe)| (*epe - *pfe).abs() < 1e-12));
        assert!(profile
            .profile
            .mtm_values
            .iter()
            .all(|mtm| (*mtm - 10.0).abs() < 1e-12));
    }

    #[test]
    fn stochastic_engine_is_deterministic_across_runs() {
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        let process = GbmProcess::with_params(0.0, 0.0, 0.25).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 512,
            seed: 7,
            pfe_quantile: 0.975,
        };
        let run = || {
            compute_stochastic_exposure_profile(
                &process,
                &discretization,
                &[100.0],
                &xva_config,
                &stochastic,
                |state| Ok(state.spot().unwrap_or(0.0) - 100.0),
            )
            .expect("profile should compute")
        };
        let a = run();
        let b = run();
        assert_eq!(a.profile.epe, b.profile.epe);
        assert_eq!(a.pfe_profile, b.pfe_profile);
    }

    #[test]
    fn valuer_engine_with_zero_mpor_full_csa_kills_exposure() {
        // threshold = mta = ia = 0 and mpor_days = 0: collateral equals current
        // exposure ⇒ EPE ≡ 0 on every path.
        use crate::xva::traits::PathValuer;
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        struct ForwardValuer;
        impl PathValuer for ForwardValuer {
            fn value_on_path(
                &self,
                state: &PathState,
                _t: f64,
            ) -> finstack_quant_core::Result<f64> {
                state
                    .spot()
                    .map(|s| s - 100.0)
                    .ok_or_else(|| finstack_quant_core::Error::Validation("missing spot".into()))
            }
        }

        let process = GbmProcess::with_params(0.0, 0.0, 0.25).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 256,
            seed: 11,
            pfe_quantile: 0.975,
        };
        let netting_set = XvaNettingSet {
            id: "NS-CSA-0".into(),
            counterparty_id: "CP".into(),
            csa: Some(CsaTerms {
                threshold: 0.0,
                mta: 0.0,
                mpor_days: 0,
                independent_amount: 0.0,
            }),
            reporting_currency: None,
        };

        let profile = compute_stochastic_exposure_with_valuer(
            &process,
            &discretization,
            &[100.0],
            &ForwardValuer,
            &xva_config,
            &stochastic,
            &netting_set,
            None,
        )
        .expect("profile should compute");
        assert!(profile.profile.epe.iter().all(|&v| v.abs() < 1e-12));
        assert!(profile.pfe_profile.iter().all(|&v| v.abs() < 1e-12));
        // Raw MtM mean is untouched by collateral.
        assert!(profile.profile.mtm_values[1].abs() < 1.0);
    }

    #[test]
    fn valuer_engine_mpor_gap_risk_is_positive_but_below_uncollateralized() {
        use crate::xva::traits::PathValuer;
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        struct ForwardValuer;
        impl PathValuer for ForwardValuer {
            fn value_on_path(
                &self,
                state: &PathState,
                _t: f64,
            ) -> finstack_quant_core::Result<f64> {
                state
                    .spot()
                    .map(|s| s - 100.0)
                    .ok_or_else(|| finstack_quant_core::Error::Validation("missing spot".into()))
            }
        }

        let process = GbmProcess::with_params(0.0, 0.0, 0.25).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![0.5, 1.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 4_096,
            seed: 11,
            pfe_quantile: 0.975,
        };
        let make_ns = |mpor_days: u32, csa: bool| XvaNettingSet {
            id: format!("NS-{mpor_days}"),
            counterparty_id: "CP".into(),
            csa: csa.then_some(CsaTerms {
                threshold: 0.0,
                mta: 0.0,
                mpor_days,
                independent_amount: 0.0,
            }),
            reporting_currency: None,
        };

        let run = |ns: &XvaNettingSet| {
            compute_stochastic_exposure_with_valuer(
                &process,
                &discretization,
                &[100.0],
                &ForwardValuer,
                &xva_config,
                &stochastic,
                ns,
                None,
            )
            .expect("profile should compute")
        };

        let uncollat = run(&make_ns(0, false));
        let mpor10 = run(&make_ns(10, true));

        for i in 0..2 {
            assert!(
                mpor10.profile.epe[i] > 0.0,
                "MPOR gap risk must leave positive residual EPE at step {i}"
            );
            assert!(
                mpor10.profile.epe[i] < uncollat.profile.epe[i],
                "collateralized EPE must be below uncollateralized at step {i}"
            );
        }
    }

    #[test]
    fn valuer_engine_carries_scaled_simm_decay_im() {
        use crate::xva::mva::{ImDecayProfile, ScaledSimmDecayIm};
        use crate::xva::traits::PathValuer;
        use crate::xva::types::StochasticExposureConfig;
        use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};

        struct ForwardValuer;
        impl PathValuer for ForwardValuer {
            fn value_on_path(
                &self,
                state: &PathState,
                _t: f64,
            ) -> finstack_quant_core::Result<f64> {
                state
                    .spot()
                    .map(|s| s - 100.0)
                    .ok_or_else(|| finstack_quant_core::Error::Validation("missing spot".into()))
            }
        }

        let process = GbmProcess::with_params(0.0, 0.0, 0.25).expect("valid GBM params");
        let discretization = ExactGbm::new();
        let xva_config = XvaConfig {
            time_grid: vec![1.0, 2.0],
            recovery_rate: 0.40,
            own_recovery_rate: None,
            funding: None,
        };
        let stochastic = StochasticExposureConfig {
            num_paths: 64,
            seed: 3,
            pfe_quantile: 0.975,
        };
        let netting_set = XvaNettingSet {
            id: "NS-IM".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: None,
        };
        let im_model = ScaledSimmDecayIm::new(
            1_000_000.0,
            ImDecayProfile::LinearToMaturity {
                maturity_years: 4.0,
            },
        )
        .expect("valid IM model");

        let profile = compute_stochastic_exposure_with_valuer(
            &process,
            &discretization,
            &[100.0],
            &ForwardValuer,
            &xva_config,
            &stochastic,
            &netting_set,
            Some(&im_model),
        )
        .expect("profile should compute");

        // Deterministic IM model ⇒ mean per-path IM equals the decay exactly:
        // IM(1) = 1e6 × 0.75, IM(2) = 1e6 × 0.5.
        let im = profile.im_profile.as_ref().expect("IM profile present");
        assert!((im[0] - 750_000.0).abs() < 1e-6);
        assert!((im[1] - 500_000.0).abs() < 1e-6);

        // And it round-trips into compute_mva input.
        let im_profile = profile.to_im_profile().expect("convertible");
        im_profile.validate().expect("valid");
    }

    /// `im_profile` is additive: a payload serialized before this field
    /// existed (no `im_profile` key at all) must still deserialize cleanly,
    /// with `im_profile` defaulting to `None`.
    #[test]
    fn stochastic_exposure_profile_im_profile_is_wire_additive() {
        let pre_change_json = r#"{
            "profile": {
                "times": [0.5, 1.0],
                "mtm_values": [1.0, 2.0],
                "epe": [1.0, 2.0],
                "ene": [0.0, 0.0]
            },
            "pfe_profile": [1.5, 2.5],
            "path_count": 100,
            "pfe_quantile": 0.975
        }"#;
        let profile: StochasticExposureProfile =
            serde_json::from_str(pre_change_json).expect("pre-change payload must still parse");
        assert!(profile.im_profile.is_none());
        profile
            .validate()
            .expect("profile without IM must validate");

        // And a profile carrying IM round-trips through JSON unchanged.
        let with_im = StochasticExposureProfile {
            im_profile: Some(vec![10.0, 20.0]),
            ..profile
        };
        let json = serde_json::to_string(&with_im).expect("serialize");
        let back: StochasticExposureProfile =
            serde_json::from_str(&json).expect("deserialize round-trip");
        assert_eq!(back.im_profile, Some(vec![10.0, 20.0]));
    }
}
