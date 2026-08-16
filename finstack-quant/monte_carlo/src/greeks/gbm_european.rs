//! Canonical GBM European finite-difference Greek entry points for host bindings.
//!
//! These free functions own the `TimeGrid` + engine + GBM process + vanilla
//! payoff composition that Python exposes as `finite_diff_delta` /
//! `finite_diff_gamma` (and the CRN variants), so the pipeline — including
//! registry-backed defaults — is defined once in Rust.

use crate::discretization::exact::ExactGbm;
use crate::engine::{McEngine, McEngineConfig};
use crate::greeks::finite_diff::{
    finite_diff_delta, finite_diff_delta_crn, finite_diff_gamma, finite_diff_gamma_crn,
};
use crate::payoff::vanilla::{EuropeanCall, EuropeanPut};
use crate::process::gbm::GbmProcess;
use crate::registry;
use crate::rng::philox::PhiloxRng;
use crate::time_grid::TimeGrid;
use finstack_quant_core::cashflow::flat_discount_factor;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::Result;
use std::str::FromStr;

/// Inputs for the GBM European finite-difference Greek convenience functions.
#[derive(Debug, Clone)]
pub struct GbmEuropeanFdSpec {
    /// Spot level at time `0`.
    pub spot: f64,
    /// Exercise price in the same units as `spot`.
    pub strike: f64,
    /// Continuously compounded risk-free rate (decimal, annualized).
    pub rate: f64,
    /// Continuous dividend yield (decimal, annualized).
    pub dividend_yield: f64,
    /// Annualized GBM volatility (decimal).
    pub volatility: f64,
    /// Time to expiry in years; also the uniform-grid horizon.
    pub expiry: f64,
    /// Simulated paths; `None` uses the registry binding default.
    pub num_paths: Option<usize>,
    /// RNG seed; `None` uses the registry binding default.
    pub seed: Option<u64>,
    /// Time steps per path; `None` uses the registry binding default.
    pub num_steps: Option<usize>,
    /// Relative spot bump used as an MC finite-difference shock; `None` uses
    /// the registry binding default (a 1% of spot MC shock, not a local
    /// closed-form step).
    pub bump_size: Option<f64>,
    /// `"call"` or `"put"`. Required; there is no default option type.
    pub option_type: String,
    /// Currency stamped on simulated payoffs; `None` uses the registry default.
    pub currency: Option<Currency>,
}

enum FdKind {
    Delta,
    DeltaCrn,
    Gamma,
    GammaCrn,
}

/// Finite-difference delta for a vanilla European option under GBM.
///
/// Both this function and [`finite_diff_delta_crn_gbm`] reuse common random
/// numbers via a splittable RNG. This function reports a **conservative
/// independence-bound** stderr; [`finite_diff_delta_crn_gbm`] reports the
/// tighter paired CRN stderr.
///
/// `option_type` must be `"call"` or `"put"`; it is not defaulted.
///
/// # Arguments
///
/// * `spec` - Spot, strike, GBM parameters, required `option_type`, and
///   optional registry overrides. `bump_size` is a relative MC shock
///   (registry default `0.01` = 1% of spot).
///
/// # Errors
///
/// Returns an error if registry defaults cannot be loaded, `option_type` is
/// not `"call"` or `"put"`, GBM or bump inputs fail validation, or either
/// pricing run fails.
pub fn finite_diff_delta_gbm(spec: GbmEuropeanFdSpec) -> Result<(f64, f64)> {
    run_gbm_fd(spec, FdKind::Delta)
}

/// Finite-difference delta with paired common-random-number stderr.
///
/// Same CRN-priced central difference as [`finite_diff_delta_gbm`]; only the
/// reported stderr estimator differs (paired pathwise differences instead of
/// the independence bound).
///
/// # Arguments
///
/// * `spec` - Spot, strike, GBM parameters, required `option_type`, and
///   optional registry overrides.
///
/// # Errors
///
/// Same failure modes as [`finite_diff_delta_gbm`].
pub fn finite_diff_delta_crn_gbm(spec: GbmEuropeanFdSpec) -> Result<(f64, f64)> {
    run_gbm_fd(spec, FdKind::DeltaCrn)
}

/// Finite-difference gamma for a vanilla European option under GBM.
///
/// Both this function and [`finite_diff_gamma_crn_gbm`] reuse common random
/// numbers. This function reports a conservative independence-bound stderr.
///
/// # Arguments
///
/// * `spec` - Spot, strike, GBM parameters, required `option_type`, and
///   optional registry overrides.
///
/// # Errors
///
/// Same failure modes as [`finite_diff_delta_gbm`].
pub fn finite_diff_gamma_gbm(spec: GbmEuropeanFdSpec) -> Result<(f64, f64)> {
    run_gbm_fd(spec, FdKind::Gamma)
}

/// Finite-difference gamma with paired common-random-number stderr.
///
/// Same CRN-priced second difference as [`finite_diff_gamma_gbm`]; only the
/// reported stderr estimator differs.
///
/// # Arguments
///
/// * `spec` - Spot, strike, GBM parameters, required `option_type`, and
///   optional registry overrides.
///
/// # Errors
///
/// Same failure modes as [`finite_diff_delta_gbm`].
pub fn finite_diff_gamma_crn_gbm(spec: GbmEuropeanFdSpec) -> Result<(f64, f64)> {
    run_gbm_fd(spec, FdKind::GammaCrn)
}

fn parse_registry_currency(code: &str) -> Result<Currency> {
    Currency::from_str(code).map_err(|err| {
        finstack_quant_core::Error::Validation(format!(
            "invalid registry default currency '{code}': {err}"
        ))
    })
}

fn parse_option_type(name: &str) -> Result<bool> {
    match name {
        "call" => Ok(true),
        "put" => Ok(false),
        _ => Err(finstack_quant_core::Error::Validation(format!(
            "unknown option_type '{name}'; expected 'call' or 'put'"
        ))),
    }
}

fn run_gbm_fd(spec: GbmEuropeanFdSpec, kind: FdKind) -> Result<(f64, f64)> {
    let defaults = &registry::embedded_defaults()?.convenience.greeks;
    let num_paths = spec.num_paths.unwrap_or(defaults.num_paths);
    let seed = spec.seed.unwrap_or(defaults.seed);
    let num_steps = spec.num_steps.unwrap_or(defaults.num_steps);
    let bump_size = spec.bump_size.unwrap_or(defaults.bump_size);
    let is_call = parse_option_type(&spec.option_type)?;
    let currency = match spec.currency {
        Some(currency) => currency,
        None => {
            parse_registry_currency(&registry::embedded_defaults()?.convenience.default_currency)?
        }
    };

    #[cfg(target_arch = "wasm32")]
    let use_parallel = false;
    #[cfg(not(target_arch = "wasm32"))]
    let use_parallel = defaults.use_parallel;

    let time_grid = TimeGrid::uniform(spec.expiry, num_steps)?;
    let engine = McEngine::new(
        McEngineConfig::new(num_paths, time_grid)
            .parallel(use_parallel)
            .chunk_size(defaults.chunk_size)
            .antithetic(defaults.antithetic),
    );
    let rng = PhiloxRng::new(seed);
    let gbm = GbmProcess::with_params(spec.rate, spec.dividend_yield, spec.volatility)?;
    let disc = ExactGbm::new();
    let discount_factor = flat_discount_factor(spec.rate, spec.expiry)?;

    if is_call {
        let payoff = EuropeanCall::new(spec.strike, 1.0, num_steps);
        match kind {
            FdKind::Delta => finite_diff_delta(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::DeltaCrn => finite_diff_delta_crn(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::Gamma => finite_diff_gamma(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::GammaCrn => finite_diff_gamma_crn(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
        }
    } else {
        let payoff = EuropeanPut::new(spec.strike, 1.0, num_steps);
        match kind {
            FdKind::Delta => finite_diff_delta(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::DeltaCrn => finite_diff_delta_crn(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::Gamma => finite_diff_gamma(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
            FdKind::GammaCrn => finite_diff_gamma_crn(
                &engine,
                &rng,
                &gbm,
                &disc,
                spec.spot,
                &payoff,
                currency,
                discount_factor,
                bump_size,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atm_spec() -> GbmEuropeanFdSpec {
        GbmEuropeanFdSpec {
            spot: 100.0,
            strike: 100.0,
            rate: 0.05,
            dividend_yield: 0.0,
            volatility: 0.2,
            expiry: 1.0,
            num_paths: Some(2_000),
            seed: Some(42),
            num_steps: Some(12),
            bump_size: None,
            option_type: "call".to_string(),
            currency: None,
        }
    }

    #[test]
    fn finite_diff_delta_gbm_atm_call_is_between_zero_and_one() {
        let (delta, stderr) = finite_diff_delta_gbm(atm_spec()).expect("delta should succeed");
        assert!(delta > 0.0 && delta < 1.0, "delta={delta}");
        assert!(stderr.is_finite() && stderr >= 0.0);
    }

    fn bs_spot_delta_gamma(
        spot: f64,
        strike: f64,
        rate: f64,
        dividend_yield: f64,
        vol: f64,
        expiry: f64,
        is_call: bool,
    ) -> (f64, f64) {
        let forward = spot * ((rate - dividend_yield) * expiry).exp();
        let df_q = (-dividend_yield * expiry).exp();
        let forward_delta = if is_call {
            finstack_quant_core::math::volatility::black_delta_call(forward, strike, vol, expiry)
        } else {
            finstack_quant_core::math::volatility::black_delta_put(forward, strike, vol, expiry)
        };
        let forward_gamma =
            finstack_quant_core::math::volatility::black_gamma(forward, strike, vol, expiry);
        (
            df_q * forward_delta,
            df_q * forward_gamma * (forward / spot),
        )
    }

    fn crn_spec(spot: f64, strike: f64, option_type: &str) -> GbmEuropeanFdSpec {
        GbmEuropeanFdSpec {
            spot,
            strike,
            rate: 0.05,
            dividend_yield: 0.0,
            volatility: 0.2,
            expiry: 1.0,
            num_paths: Some(8_000),
            seed: Some(7),
            num_steps: Some(1),
            bump_size: Some(0.01),
            option_type: option_type.to_string(),
            currency: None,
        }
    }

    #[test]
    fn finite_diff_delta_crn_gbm_matches_black_scholes_atm_and_25d() {
        let (atm_spot, atm_strike) = (100.0, 100.0);
        let (delta, stderr) =
            finite_diff_delta_crn_gbm(crn_spec(atm_spot, atm_strike, "call")).expect("atm delta");
        let (bs_delta, _) = bs_spot_delta_gamma(atm_spot, atm_strike, 0.05, 0.0, 0.2, 1.0, true);
        let tol = (4.0 * stderr).max(0.03);
        assert!(
            (delta - bs_delta).abs() < tol,
            "ATM call delta {delta} vs BS {bs_delta} (stderr={stderr}, tol={tol})"
        );

        let otm_strike = 120.0;
        let (delta_25, stderr_25) =
            finite_diff_delta_crn_gbm(crn_spec(atm_spot, otm_strike, "call")).expect("otm delta");
        let (bs_delta_25, _) = bs_spot_delta_gamma(atm_spot, otm_strike, 0.05, 0.0, 0.2, 1.0, true);
        let tol_25 = (4.0 * stderr_25).max(0.03);
        assert!(
            (delta_25 - bs_delta_25).abs() < tol_25,
            "OTM call delta {delta_25} vs BS {bs_delta_25} (stderr={stderr_25}, tol={tol_25})"
        );
    }

    #[test]
    fn finite_diff_gamma_crn_gbm_matches_black_scholes_atm() {
        let spec = crn_spec(100.0, 100.0, "call");
        let (gamma, stderr) = finite_diff_gamma_crn_gbm(spec).expect("atm gamma");
        let (_, bs_gamma) = bs_spot_delta_gamma(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, true);
        let tol = (4.0 * stderr).max(0.01);
        assert!(
            (gamma - bs_gamma).abs() < tol,
            "ATM call gamma {gamma} vs BS {bs_gamma} (stderr={stderr}, tol={tol})"
        );
    }

    #[test]
    fn finite_diff_delta_gbm_rejects_unknown_option_type() {
        let mut spec = atm_spec();
        spec.option_type = "straddle".to_string();
        let err = finite_diff_delta_gbm(spec).expect_err("unknown type");
        assert!(
            err.to_string().contains("unknown option_type"),
            "unexpected error: {err}"
        );
    }
}
