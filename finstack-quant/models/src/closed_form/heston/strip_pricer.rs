use super::characteristic_fn::{heston_pj_characteristic_function, HestonCfStatus};
use super::params::HESTON_TAIL_DIAGNOSTIC_THRESHOLD;
use super::quadrature::{composite_gauss_legendre_grid, HESTON_TAIL_WINDOW_FRACTION};
use super::{HestonFourierSettings, HestonParams};
use finstack_quant_core::{Error, Result};
use num_complex::Complex;
use std::f64::consts::PI;
use tracing::warn;

/// Cached Heston Fourier data for pricing multiple strikes with shared parameters.
///
/// The characteristic function portion of the Gil-Pelaez integrand is independent
/// of strike, so it can be precomputed once on the composite Gauss-Legendre grid
/// and reused across a strike strip.
#[derive(Debug, Clone)]
pub struct HestonStripPricer {
    spot: f64,
    time: f64,
    params: HestonParams,
    /// Composite quadrature grid as `(phi, weight)` pairs.
    grid: Vec<(f64, f64)>,
    /// Start of the tail window `u_max * (1 - HESTON_TAIL_WINDOW_FRACTION)`,
    /// precomputed from `grid` so `probability` does not rescan it per call.
    tail_window_start: f64,
    /// Cached `psi_1(phi) / (i * phi)` values on the grid.
    psi1_over_iphi: Vec<Complex<f64>>,
    /// Cached `psi_2(phi) / (i * phi)` values on the grid.
    psi2_over_iphi: Vec<Complex<f64>>,
    /// `true` when too many grid nodes had a non-finite or overflow-zeroed
    /// characteristic function, making the cached integral unreliable.
    pub(super) integrand_corrupted: bool,
}

/// Maximum fraction of integration nodes that may be non-finite or zeroed
/// before the cached strip integral is rejected.
///
/// A Heston characteristic function that overflows at a node makes
/// [`heston_pj_characteristic_function`] return a zeroed value with
/// [`HestonCfStatus::Overflow`]. A few such nodes (typically in the tail,
/// where the integrand is already tiny) are harmless, but when a large
/// fraction of nodes are corrupted the Gil-Pelaez integral silently loses
/// mass and yields a plausible-but-wrong probability. Legitimate
/// [`HestonCfStatus::Underflow`] nodes (well-formed inputs, |ψ| → 0) are
/// *not* corruption and do not count toward this threshold.
pub(super) const HESTON_STRIP_MAX_CORRUPT_FRACTION: f64 = 0.05;

impl HestonStripPricer {
    /// Build a strip pricer with characteristic-function values cached on the
    /// composite Gauss-Legendre integration grid.
    #[must_use]
    pub fn new(
        spot: f64,
        time: f64,
        params: &HestonParams,
        settings: &HestonFourierSettings,
    ) -> Option<Self> {
        let grid =
            composite_gauss_legendre_grid(0.0, settings.u_max, settings.gl_order, settings.panels)?;
        let i = Complex::new(0.0, 1.0);
        let log_spot = spot.ln();
        let mut psi1_over_iphi = Vec::with_capacity(grid.len());
        let mut psi2_over_iphi = Vec::with_capacity(grid.len());

        // Count interior nodes (φ away from the singularity) and how many of
        // them returned an **overflow**-corrupted characteristic function.
        // Legitimate underflow (well-formed inputs, |ψ| → 0 in the decayed
        // tail) contributes exactly zero to the integral and must not count
        // toward corruption — long-dated / high-κθ surfaces underflow over
        // much of the grid without any loss of pricing accuracy.
        let mut interior_nodes = 0_usize;
        let mut corrupted_nodes = 0_usize;
        let mut ok_nodes = 0_usize;

        for (phi, _) in &grid {
            if phi.abs() < settings.phi_eps {
                psi1_over_iphi.push(Complex::new(0.0, 0.0));
                psi2_over_iphi.push(Complex::new(0.0, 0.0));
                continue;
            }

            interior_nodes += 1;
            let denom = i * *phi;
            let (psi1, status1) =
                heston_pj_characteristic_function(1, *phi, time, log_spot, params);
            let (psi2, status2) =
                heston_pj_characteristic_function(2, *phi, time, log_spot, params);

            if status1 == HestonCfStatus::Overflow || status2 == HestonCfStatus::Overflow {
                corrupted_nodes += 1;
            }
            if status1 == HestonCfStatus::Ok && status2 == HestonCfStatus::Ok {
                ok_nodes += 1;
            }

            psi1_over_iphi.push(psi1 / denom);
            psi2_over_iphi.push(psi2 / denom);
        }

        // Corrupted when too many nodes overflowed, or when *no* node carried
        // information at all (every interior node zeroed): the Gil-Pelaez
        // integral then degenerates to the 0.5 baseline and the resulting
        // price is plausible-but-wrong. Partial underflow (decayed tail) is
        // legitimate and does not count.
        let integrand_corrupted = interior_nodes > 0
            && ((corrupted_nodes as f64) / (interior_nodes as f64)
                > HESTON_STRIP_MAX_CORRUPT_FRACTION
                || ok_nodes == 0);

        // `u_max` (largest grid abscissa) and the tail-window start are fixed
        // once the grid is built, so compute them here rather than rescanning
        // the grid on every `probability` call.
        let u_max = grid.iter().map(|(phi, _)| *phi).fold(0.0_f64, f64::max);
        let tail_window_start = u_max * (1.0 - HESTON_TAIL_WINDOW_FRACTION);

        Some(Self {
            spot,
            time,
            params: *params,
            grid,
            tail_window_start,
            psi1_over_iphi,
            psi2_over_iphi,
            integrand_corrupted,
        })
    }

    /// Evaluate one Gil-Pelaez probability on the cached grid.
    ///
    /// Returns `(clamped_probability, raw_probability, tail_estimate)`. The raw
    /// (pre-clamp) probability and the truncated-tail estimate let the caller
    /// detect `u_max` truncation error that the `[0, 1]` clamp would otherwise
    /// hide (audit item 4). The tail estimate is the absolute integrand mass in
    /// the last [`HESTON_TAIL_WINDOW_FRACTION`] of the integration range,
    /// divided by π.
    fn probability(&self, log_strike: f64, cached_values: &[Complex<f64>]) -> (f64, f64, f64) {
        let i = Complex::new(0.0, 1.0);
        let mut integral = 0.0;
        let mut tail_abs_mass = 0.0;

        // `u_max` (largest grid abscissa) and the tail-window start were
        // precomputed at construction.
        let tail_window_start = self.tail_window_start;

        for ((phi, weight), cached) in self.grid.iter().zip(cached_values.iter()) {
            let exp_term = (-i * *phi * log_strike).exp();
            let value = (exp_term * *cached).re;
            if value.is_finite() {
                integral += *weight * value;
                if *phi >= tail_window_start {
                    tail_abs_mass += weight.abs() * value.abs();
                }
            }
        }

        let raw = 0.5 + integral / PI;
        (raw.clamp(0.0, 1.0), raw, tail_abs_mass / PI)
    }

    /// Price a single European call using the cached strip pricer.
    ///
    /// Returns a structured convergence error when characteristic-function
    /// corruption or a non-finite integral makes the Heston result unreliable.
    pub fn price_call(&self, strike: f64) -> Result<f64> {
        if self.integrand_corrupted {
            return Err(Error::Calibration {
                category: "heston_fourier".to_string(),
                message: format!(
                    "Heston strip integration is corrupted for spot={}, strike={}, time={}",
                    self.spot, strike, self.time
                ),
            });
        }

        let log_strike = strike.ln();
        let (p1, raw_p1, tail_p1) = self.probability(log_strike, &self.psi1_over_iphi);
        let (p2, raw_p2, tail_p2) = self.probability(log_strike, &self.psi2_over_iphi);

        // Audit item 4: surface a diagnostic when the truncated-tail estimate or
        // a pre-clamp probability excursion shows the integral was mis-truncated
        // at `u_max`, instead of silently relying on the `[0, 1]` clamp.
        let tail = tail_p1.max(tail_p2);
        let raw_excursion = (raw_p1 - raw_p1.clamp(0.0, 1.0))
            .abs()
            .max((raw_p2 - raw_p2.clamp(0.0, 1.0)).abs());
        if tail > HESTON_TAIL_DIAGNOSTIC_THRESHOLD
            || raw_excursion > HESTON_TAIL_DIAGNOSTIC_THRESHOLD
        {
            warn!(
                spot = self.spot,
                strike,
                time = self.time,
                tail_estimate = tail,
                raw_probability_excursion = raw_excursion,
                "Heston strip Gil-Pelaez integral truncated at u_max with a \
                 non-negligible residual tail; the price may be mis-truncated — \
                 consider a larger u_max"
            );
        }

        let call_price = self.spot * (-self.params.q * self.time).exp() * p1
            - strike * (-self.params.r * self.time).exp() * p2;

        if !call_price.is_finite() {
            return Err(Error::Calibration {
                category: "heston_fourier".to_string(),
                message: format!(
                    "Heston strip integration produced a non-finite price for \
                     spot={}, strike={}, time={}",
                    self.spot, strike, self.time
                ),
            });
        }

        Ok(call_price.max(0.0))
    }

    /// Price a strip of European calls using the cached strip pricer.
    pub fn price_calls(&self, strikes: &[f64]) -> Result<Vec<f64>> {
        strikes
            .iter()
            .map(|&strike| self.price_call(strike))
            .collect()
    }
}
