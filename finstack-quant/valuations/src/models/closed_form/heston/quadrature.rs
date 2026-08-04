use super::characteristic_fn::{heston_pj_characteristic_function, HestonCfStatus};
use super::strip_pricer::HESTON_STRIP_MAX_CORRUPT_FRACTION;
use super::{HestonFourierSettings, HestonParams};
use finstack_quant_core::math::gauss_legendre_integrate_composite;
use num_complex::Complex;
use std::f64::consts::PI;

fn gl_nodes_weights(order: usize) -> Option<(&'static [f64], &'static [f64])> {
    match order {
        2 => Some((
            &[-0.577_350_269_189_625_7, 0.577_350_269_189_625_7],
            &[1.0, 1.0],
        )),
        4 => Some((
            &[
                -0.861_136_311_594_052_6,
                -0.339_981_043_584_856_3,
                0.339_981_043_584_856_3,
                0.861_136_311_594_052_6,
            ],
            &[
                0.347_854_845_137_453_85,
                0.652_145_154_862_546_1,
                0.652_145_154_862_546_1,
                0.347_854_845_137_453_85,
            ],
        )),
        8 => Some((
            &[
                -0.960_289_856_497_536_3,
                -0.796_666_477_413_626_7,
                -0.525_532_409_916_329,
                -0.183_434_642_495_649_8,
                0.183_434_642_495_649_8,
                0.525_532_409_916_329,
                0.796_666_477_413_626_7,
                0.960_289_856_497_536_3,
            ],
            &[
                0.101_228_536_290_376_26,
                0.222_381_034_453_374_48,
                0.313_706_645_877_887_27,
                0.362_683_783_378_361_96,
                0.362_683_783_378_361_96,
                0.313_706_645_877_887_27,
                0.222_381_034_453_374_48,
                0.101_228_536_290_376_26,
            ],
        )),
        16 => Some((
            &[
                -0.989_400_934_991_649_9,
                -0.944_575_023_073_232_6,
                -0.865_631_202_387_831_8,
                -0.755_404_408_355_003,
                -0.617_876_244_402_643_8,
                -0.458_016_777_657_227_37,
                -0.281_603_550_779_258_9,
                -0.095_012_509_837_637_44,
                0.095_012_509_837_637_44,
                0.281_603_550_779_258_9,
                0.458_016_777_657_227_37,
                0.617_876_244_402_643_8,
                0.755_404_408_355_003,
                0.865_631_202_387_831_8,
                0.944_575_023_073_232_6,
                0.989_400_934_991_649_9,
            ],
            &[
                0.027_152_459_411_754_095,
                0.062_253_523_938_647_894,
                0.095_158_511_682_492_78,
                0.124_628_971_255_533_88,
                0.149_595_988_816_576_73,
                0.169_156_519_395_002_54,
                0.182_603_415_044_923_58,
                0.189_450_610_455_068_5,
                0.189_450_610_455_068_5,
                0.182_603_415_044_923_58,
                0.169_156_519_395_002_54,
                0.149_595_988_816_576_73,
                0.124_628_971_255_533_88,
                0.095_158_511_682_492_78,
                0.062_253_523_938_647_894,
                0.027_152_459_411_754_095,
            ],
        )),
        _ => None,
    }
}

pub(super) fn composite_gauss_legendre_grid(
    a: f64,
    b: f64,
    order: usize,
    panels: usize,
) -> Option<Vec<(f64, f64)>> {
    if panels == 0 || !(a.is_finite() && b.is_finite()) || b <= a {
        return None;
    }

    let (xs, ws) = gl_nodes_weights(order)?;
    let h = (b - a) / panels as f64;
    let mut grid = Vec::with_capacity(xs.len() * panels);

    for panel_idx in 0..panels {
        let panel_start = a + panel_idx as f64 * h;
        let panel_end = panel_start + h;
        let half = 0.5 * (panel_end - panel_start);
        let mid = panel_start + half;

        for (x, w) in xs.iter().zip(ws.iter()) {
            grid.push((mid + half * x, half * w));
        }
    }

    Some(grid)
}

/// Fraction of the upper integration range whose absolute integrand mass is
/// used to estimate the truncated Gil-Pelaez tail (audit item 4).
pub(super) const HESTON_TAIL_WINDOW_FRACTION: f64 = 0.1;

/// Diagnostics from a single Gil-Pelaez probability inversion.
///
/// Carries the information needed to detect the two silent failure modes the
/// audit flagged: characteristic-function corruption (item 5) and truncation
/// of the Fourier integral at a fixed `u_max` (item 4).
#[derive(Debug, Clone, Copy)]
pub(super) struct HestonPjDiagnostics {
    /// Probability clamped to `[0, 1]` — the value used for pricing.
    pub(super) probability: f64,
    /// Probability *before* the `[0, 1]` clamp. A value materially outside
    /// `[0, 1]` is direct evidence that the truncated integral lost or gained
    /// mass; the clamp would otherwise hide it.
    pub(super) raw_probability: f64,
    /// Estimated magnitude of the truncated tail beyond `u_max`, expressed on
    /// the probability scale. Computed from the absolute integrand mass in the
    /// last [`HESTON_TAIL_WINDOW_FRACTION`] of the integration range — if the
    /// integrand has genuinely decayed this is tiny; if `u_max` is too small
    /// for the maturity it stays large.
    pub(super) tail_estimate: f64,
    /// `true` when too many interior integration nodes had a non-finite /
    /// overflow-zeroed characteristic function (see
    /// [`HESTON_STRIP_MAX_CORRUPT_FRACTION`]); the integral is then unreliable
    /// and pricing must fall back to Black-Scholes — mirroring the strip pricer.
    pub(super) corrupted: bool,
}

/// Compute the Pj probability for Heston call pricing via Fourier inversion,
/// returning full diagnostics alongside the value.
///
/// P_j = 0.5 + (1/π) ∫_0^∞ Re[exp(-i*φ*ln(K)) * ψ_j(φ) / (i*φ)] dφ
///
/// The integral is evaluated on the explicit composite Gauss-Legendre grid so
/// that, in a single pass, the routine can also:
/// - count overflow-zeroed characteristic-function nodes (audit item 5), and
/// - estimate the truncated tail mass beyond `u_max` (audit item 4).
///
/// # Arguments
///
/// * `j` - Probability index (1 or 2)
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `time` - Time to maturity
/// * `params` - Heston model parameters
/// * `settings` - Integration settings
pub(super) fn heston_pj_with_diagnostics(
    j: u8,
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> HestonPjDiagnostics {
    let log_spot = spot.ln();
    let log_strike = strike.ln();
    let i = Complex::new(0.0, 1.0);

    // Build the same composite Gauss-Legendre grid the strip pricer uses, so we
    // can inspect per-node behaviour rather than treating the quadrature as a
    // black box.
    let grid =
        composite_gauss_legendre_grid(0.0, settings.u_max, settings.gl_order, settings.panels);
    let Some(grid) = grid else {
        // Degenerate settings: fall back to the library quadrature with no
        // node-level diagnostics available.
        let integrand = |phi: f64| {
            if phi.abs() < settings.phi_eps {
                return 0.0;
            }
            let (psi, _status) = heston_pj_characteristic_function(j, phi, time, log_spot, params);
            let exp_term = (-i * phi * log_strike).exp();
            (exp_term * psi / (i * phi)).re
        };
        let (integral, integration_failed) = match gauss_legendre_integrate_composite(
            integrand,
            0.0,
            settings.u_max,
            settings.gl_order,
            settings.panels,
        ) {
            Ok(v) => (v, false),
            Err(_) => (0.0, true),
        };
        let raw = 0.5 + integral / PI;
        return HestonPjDiagnostics {
            probability: raw.clamp(0.0, 1.0),
            raw_probability: raw,
            tail_estimate: f64::INFINITY,
            // If the fallback integrator also failed, surface corruption so the
            // caller falls back to Black-Scholes rather than silently using 0.5.
            corrupted: integration_failed,
        };
    };

    heston_pj_on_grid(j, spot, strike, time, params, settings, &grid)
}

/// Gil-Pelaez `Pj` probability and diagnostics evaluated on a *prebuilt*
/// composite Gauss-Legendre grid.
///
/// The grid depends only on `settings` (not on `j`, `spot`, `strike`, or
/// `time`), so [`heston_call_price_fourier_with_settings`] builds it once and
/// shares it across the `j = 1` and `j = 2` evaluations instead of rebuilding
/// the `gl_order * panels`-node grid twice per scalar price.
pub(super) fn heston_pj_on_grid(
    j: u8,
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
    grid: &[(f64, f64)],
) -> HestonPjDiagnostics {
    let log_spot = spot.ln();
    let log_strike = strike.ln();
    let i = Complex::new(0.0, 1.0);

    // The tail window starts at this φ; absolute integrand mass beyond it
    // estimates the error from truncating the integral at `u_max`.
    let tail_window_start = settings.u_max * (1.0 - HESTON_TAIL_WINDOW_FRACTION);

    let mut integral = 0.0;
    let mut tail_abs_mass = 0.0;
    let mut interior_nodes = 0_usize;
    let mut corrupted_nodes = 0_usize;
    let mut ok_nodes = 0_usize;

    for (phi, weight) in grid {
        // Handle singularity at φ=0.
        if phi.abs() < settings.phi_eps {
            continue;
        }
        interior_nodes += 1;

        let (psi, status) = heston_pj_characteristic_function(j, *phi, time, log_spot, params);
        // Only **overflow**-corrupted nodes count toward the corruption
        // fraction; legitimate underflow (|ψ| → 0 with well-formed inputs)
        // contributes a correct zero to the integral.
        if status == HestonCfStatus::Overflow {
            corrupted_nodes += 1;
        }
        if status == HestonCfStatus::Ok {
            ok_nodes += 1;
        }

        let exp_term = (-i * *phi * log_strike).exp();
        let value = (exp_term * psi / (i * *phi)).re;
        if value.is_finite() {
            integral += *weight * value;
            if *phi >= tail_window_start {
                tail_abs_mass += weight.abs() * value.abs();
            }
        }
    }

    // Corrupted when too many nodes overflowed, or when *no* node carried
    // information at all (every interior node zeroed): the integral then
    // degenerates to the 0.5 baseline. Partial underflow is legitimate.
    let corrupted = interior_nodes > 0
        && ((corrupted_nodes as f64) / (interior_nodes as f64) > HESTON_STRIP_MAX_CORRUPT_FRACTION
            || ok_nodes == 0);

    let raw_probability = 0.5 + integral / PI;
    HestonPjDiagnostics {
        probability: raw_probability.clamp(0.0, 1.0),
        raw_probability,
        tail_estimate: tail_abs_mass / PI,
        corrupted,
    }
}
