//! Cross-implementation parity between this crate's `SABRModel` and the
//! canonical Hagan/Obloj expansion in `finstack_quant_core`.
//!
//! Both implement Hagan et al. (2002) with the Obloj (2008) correction, and
//! before this test nothing compared them — so either could have drifted
//! silently. Measured 2026-08-03: they agree to printed precision for every
//! β > 0 case, and differ by ~34 bp of implied vol in the β = 0 (normal)
//! branch, which is pinned separately below rather than hidden.
//!
//! Note the constructors take ρ and ν in **opposite order**
//! (`SABRParameters::new(alpha, beta, nu, rho)` vs
//! `SabrParams::new(alpha, beta, rho, nu)`), so the mapping here is
//! deliberate, not a typo.

use finstack_quant_core::math::volatility::sabr::SabrParams as CoreSabr;
use finstack_quant_models::volatility::sabr::{SABRModel, SABRParameters};

const FWD: f64 = 0.03;
const EXPIRIES: [f64; 3] = [0.25, 1.0, 5.0];
const MONEYNESS: [f64; 5] = [0.5, 0.8, 1.0, 1.25, 2.0];

/// Worst relative gap between the two lognormal expansions over a grid.
fn worst_lognormal_gap(alpha: f64, beta: f64, rho: f64, nu: f64) -> f64 {
    let model =
        SABRModel::new(SABRParameters::new(alpha, beta, nu, rho).expect("valid valuations params"));
    let core = CoreSabr::new(alpha, beta, rho, nu).expect("valid core params");
    let mut worst: f64 = 0.0;
    for t in EXPIRIES {
        for m in MONEYNESS {
            let k = FWD * m;
            let (Ok(v), Ok(c)) = (
                model.implied_volatility(FWD, k, t),
                core.implied_vol_lognormal(FWD, k, t),
            ) else {
                continue;
            };
            worst = worst.max((v - c).abs() / v.max(1e-12));
        }
    }
    worst
}

/// For β > 0 the two expansions must agree to floating-point noise.
#[test]
fn sabr_lognormal_branch_matches_core_expansion() {
    for (name, alpha, beta, rho, nu) in [
        ("beta=0.5 typical", 0.30, 0.5, -0.3, 0.4),
        ("beta=1.0 lognormal", 0.20, 1.0, -0.2, 0.3),
        ("high nu skew", 0.25, 0.7, -0.6, 0.9),
    ] {
        let gap = worst_lognormal_gap(alpha, beta, rho, nu);
        assert!(
            gap < 1e-9,
            "{name}: valuations and core SABR diverged by {:.4} bp; these share \
             the Hagan(2002)+Obloj(2008) expansion, so any visible gap is drift",
            gap * 10_000.0
        );
    }
}

/// β = 0 is the one branch where the two disagree.
///
/// The valuations model returns a **normal** vol here (tagged
/// `SabrVolType::Normal`), so it is compared against `implied_vol_normal`.
/// Measured worst gap 33.98 bp at t=1, K/F=2. Both claim the same reference,
/// so one of the two normal-SABR forms is wrong; pinned so it cannot widen
/// while that is decided.
#[test]
fn sabr_normal_branch_divergence_from_core_is_bounded() {
    let (alpha, beta, rho, nu) = (0.010, 0.0, 0.0, 0.3);
    let model =
        SABRModel::new(SABRParameters::new(alpha, beta, nu, rho).expect("valid valuations params"));
    let core = CoreSabr::new(alpha, beta, rho, nu).expect("valid core params");

    let mut worst: f64 = 0.0;
    for t in EXPIRIES {
        for m in MONEYNESS {
            let k = FWD * m;
            let (Ok(v), Ok(c)) = (
                model.implied_volatility(FWD, k, t),
                core.implied_vol_normal(FWD, k, t),
            ) else {
                continue;
            };
            worst = worst.max((v - c).abs() / v.max(1e-12));
        }
    }

    assert!(
        worst < 40e-4,
        "beta=0 SABR divergence widened to {:.4} bp (was 33.98 bp when pinned)",
        worst * 10_000.0
    );
    assert!(
        worst > 1e-6,
        "beta=0 SABR branches now agree to {:.4} bp -- if the normal-SABR \
         formulas were reconciled, delete this test and fold beta=0 into \
         sabr_lognormal_branch_matches_core_expansion",
        worst * 10_000.0
    );
}
