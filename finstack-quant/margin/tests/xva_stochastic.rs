//! End-to-end test of the path-consistent exposure bridge: a hand-written toy
//! `PathValuer` (forward contract, `V = S_t − K`) driven through
//! `compute_stochastic_exposure_with_valuer`, checked against the closed-form
//! lognormal expectation, then fed into MVA via a scaled-SIMM IM model.
//!
//! Analytic reference (K = S₀ = 100, σ = 0.2, r = q = 0, t = 1):
//!   EPE(1) = S₀(2Φ(σ√t/2) − 1) = 100 × (2×0.539827837 − 1) = 7.96556746
//!   E[V(1)] = 0
//!   PFE₀.₉₇₅(1) = 100·exp(−0.02 + 0.2×1.959964) − 100 ≈ 45.06

use finstack_quant_core::math::special_functions::norm_cdf;
use finstack_quant_margin::xva::exposure::compute_stochastic_exposure_with_valuer;
use finstack_quant_margin::xva::mva::{compute_mva, ImDecayProfile, ScaledSimmDecayIm};
use finstack_quant_margin::xva::traits::PathValuer;
use finstack_quant_margin::xva::types::{
    CsaTerms, StochasticExposureConfig, XvaConfig, XvaNettingSet,
};
use finstack_quant_monte_carlo::prelude::{ExactGbm, GbmProcess};
use finstack_quant_monte_carlo::PathState;

/// Toy path valuer: a forward contract on the simulated spot, `V = S_t − K`.
struct ForwardValuer {
    strike: f64,
}

impl PathValuer for ForwardValuer {
    fn value_on_path(&self, state: &PathState, _t: f64) -> finstack_quant_core::Result<f64> {
        state
            .spot()
            .map(|s| s - self.strike)
            .ok_or_else(|| finstack_quant_core::Error::Validation("missing spot".into()))
    }
}

fn xva_config() -> XvaConfig {
    XvaConfig {
        time_grid: vec![0.5, 1.0],
        recovery_rate: 0.40,
        own_recovery_rate: None,
        funding: None,
    }
}

fn uncollateralized_ns() -> XvaNettingSet {
    XvaNettingSet {
        id: "NS-E2E".into(),
        counterparty_id: "CP".into(),
        csa: None,
        reporting_currency: None,
    }
}

#[test]
fn toy_forward_valuer_matches_analytic_epe_and_pfe() {
    let s0 = 100.0;
    let sigma = 0.2;
    let process = GbmProcess::with_params(0.0, 0.0, sigma).expect("valid GBM params");
    let discretization = ExactGbm::new();
    let stochastic = StochasticExposureConfig {
        num_paths: 65_536,
        seed: 42,
        pfe_quantile: 0.975,
    };
    let valuer = ForwardValuer { strike: s0 };

    let profile = compute_stochastic_exposure_with_valuer(
        &process,
        &discretization,
        &[s0],
        &valuer,
        &xva_config(),
        &stochastic,
        &uncollateralized_ns(),
        None,
    )
    .expect("profile should compute");

    // EPE(1) analytic: S₀(2Φ(σ/2) − 1) with σ√t = 0.2 at t = 1.
    //
    // Blind spot: at this ATM/driftless parametrization (K = S₀, r = q = 0),
    // put-call parity makes E[(S−K)⁺] = E[(K−S)⁺] exactly, so this EPE
    // assertion ALONE cannot distinguish a correct EPE/ENE aggregation from
    // one with the two swapped — a mutation test confirmed that swapping the
    // EPE/ENE formula in `aggregate_stochastic_profile` is caught only by the
    // PFE assertion below and by `toy_forward_valuer_zero_vol_collapses_to_intrinsic`'s
    // `ene ≈ 0` check. Do not remove those two checks in the name of
    // "simplifying" this test — they are the only things actually
    // discriminating EPE from ENE here.
    let epe_analytic = s0 * (2.0 * norm_cdf(sigma / 2.0) - 1.0); // = 7.96556746
    let epe_mc = profile.profile.epe[1];
    assert!(
        (epe_mc - epe_analytic).abs() / epe_analytic < 0.025,
        "EPE(1) MC {epe_mc} vs analytic {epe_analytic}"
    );

    // Martingale: E[V(1)] = 0 (abs tolerance ~6 s.e. of the mean).
    assert!(
        profile.profile.mtm_values[1].abs() < 0.5,
        "mean MtM should be ~0, got {}",
        profile.profile.mtm_values[1]
    );

    // PFE(1) analytic: lognormal 97.5% quantile of (S₁ − K)⁺.
    //
    // Tolerance: empirically SE(PFE) ≈ 0.303 at 65,536 paths (~0.67%
    // relative to the ≈45.06 analytic value), so a 3% relative band is
    // ~4.5 standard errors — comparable validation weight to the EPE band
    // above (~3.9 SE). Do not widen this band to paper over an unrelated
    // failure; if it flakes, the seed or path count changed, or there is a
    // real regression.
    let z = 1.959_963_985;
    let pfe_analytic = s0 * (-0.5 * sigma * sigma + sigma * z).exp() - s0; // ≈ 45.06
    let pfe_mc = profile.pfe_profile[1];
    assert!(
        (pfe_mc - pfe_analytic).abs() / pfe_analytic < 0.03,
        "PFE(1) MC {pfe_mc} vs analytic {pfe_analytic}"
    );

    // Ordering sanity: PFE ≫ EPE > 0; EPE grows with horizon (√t risk).
    assert!(profile.pfe_profile[1] > profile.profile.epe[1]);
    assert!(profile.profile.epe[1] > profile.profile.epe[0]);
}

#[test]
fn toy_forward_valuer_zero_vol_collapses_to_intrinsic() {
    // σ = 0, S₀ = 110, K = 100: every path is S_t ≡ 110 ⇒ exposure exactly 10.
    let process = GbmProcess::with_params(0.0, 0.0, 0.0).expect("valid GBM params");
    let discretization = ExactGbm::new();
    let stochastic = StochasticExposureConfig {
        num_paths: 128,
        seed: 7,
        pfe_quantile: 0.975,
    };
    let valuer = ForwardValuer { strike: 100.0 };

    let profile = compute_stochastic_exposure_with_valuer(
        &process,
        &discretization,
        &[110.0],
        &valuer,
        &xva_config(),
        &stochastic,
        &uncollateralized_ns(),
        None,
    )
    .expect("profile should compute");

    for i in 0..2 {
        assert!((profile.profile.mtm_values[i] - 10.0).abs() < 1e-12);
        assert!((profile.profile.epe[i] - 10.0).abs() < 1e-12);
        assert!((profile.pfe_profile[i] - 10.0).abs() < 1e-12);
        assert!(profile.profile.ene[i].abs() < 1e-12);
    }
}

#[test]
fn toy_forward_valuer_mpor_csa_and_path_im_to_mva() {
    // Full pipeline: valuer → MPOR-collateralized exposure + per-path IM →
    // ImProfile → compute_mva, with hand-checkable MVA arithmetic.
    use finstack_quant_core::dates::Date;
    use finstack_quant_core::market_data::term_structures::DiscountCurve;
    use time::Month;

    let process = GbmProcess::with_params(0.0, 0.0, 0.2).expect("valid GBM params");
    let discretization = ExactGbm::new();
    let stochastic = StochasticExposureConfig {
        num_paths: 4_096,
        seed: 42,
        pfe_quantile: 0.975,
    };
    let valuer = ForwardValuer { strike: 100.0 };
    let netting_set = XvaNettingSet {
        id: "NS-E2E-CSA".into(),
        counterparty_id: "CP".into(),
        csa: Some(CsaTerms {
            threshold: 0.0,
            mta: 0.0,
            mpor_days: 10,
            independent_amount: 0.0,
        }),
        reporting_currency: None,
    };
    // Constant IM model: IM(t) = 1_000_000 on every path.
    let im_model =
        ScaledSimmDecayIm::new(1_000_000.0, ImDecayProfile::Constant).expect("valid IM model");

    let config = XvaConfig {
        time_grid: vec![1.0, 2.0],
        recovery_rate: 0.40,
        own_recovery_rate: None,
        funding: None,
    };
    let profile = compute_stochastic_exposure_with_valuer(
        &process,
        &discretization,
        &[100.0],
        &valuer,
        &config,
        &stochastic,
        &netting_set,
        Some(&im_model),
    )
    .expect("profile should compute");

    // Gap risk: collateralized EPE positive but far below the uncollateralized
    // analytic EPE(1) = 7.9656.
    assert!(profile.profile.epe[0] > 0.0);
    assert!(profile.profile.epe[0] < 7.9656);

    // IM profile is exact (deterministic model): [1e6, 1e6].
    let im_profile = profile.to_im_profile().expect("IM profile present");
    assert!((im_profile.im_values[0] - 1_000_000.0).abs() < 1e-6);
    assert!((im_profile.im_values[1] - 1_000_000.0).abs() < 1e-6);

    // MVA at flat 50bp, DF = 1, no survival:
    //   grid [1, 2] ⇒ MVA = 0.005 × 1e6 × (1 + 1) = 10_000 exactly.
    //
    // Coverage limit: the IM profile here is perfectly flat (ScaledSimmDecayIm
    // with ImDecayProfile::Constant), so every quadrature rule that integrates
    // a constant exactly (trapezoid, midpoint, Simpson, ...) reproduces the
    // same 10_000. This test validates units/discounting/wiring end-to-end but
    // cannot discriminate `compute_mva`'s trapezoidal rule from another
    // reasonable one; `mva_linear_decay_profile` and
    // `mva_interpolates_spread_curve` in `xva/mva.rs` are what actually pin
    // the trapezoid convention on non-constant inputs.
    let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots((0..=8).map(|i| (i as f64 * 0.5, 1.0)).collect::<Vec<_>>())
        .interp(finstack_quant_core::math::interp::InterpStyle::LogLinear)
        .build()
        .expect("DiscountCurve should build");
    let mva =
        compute_mva(&im_profile, &[(0.0, 50.0)], &discount, None).expect("MVA should compute");
    assert!(
        (mva.mva - 10_000.0).abs() < 1e-6,
        "end-to-end MVA {} != 10_000",
        mva.mva
    );
}
