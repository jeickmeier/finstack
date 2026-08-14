use super::*;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::scalars::MarketScalar;

#[test]
fn from_market_strict_errors_when_any_scalar_is_missing() {
    let market = MarketContext::new();
    let err = HestonParams::from_market_strict(&market, 0.05, 0.02)
        .expect_err("strict resolver must reject missing HESTON_KAPPA");
    let msg = err.to_string();
    assert!(
        msg.contains("HESTON_KAPPA"),
        "error must name the missing scalar, got: {msg}"
    );
}

#[test]
fn from_market_strict_errors_when_only_some_scalars_present() {
    let market = MarketContext::new()
        .insert_price("HESTON_KAPPA", MarketScalar::Unitless(1.5))
        .insert_price("HESTON_THETA", MarketScalar::Unitless(0.06))
        .insert_price("HESTON_SIGMA_V", MarketScalar::Unitless(0.4))
        .insert_price("HESTON_RHO", MarketScalar::Unitless(-0.5));
    let err = HestonParams::from_market_strict(&market, 0.0, 0.0)
        .expect_err("strict resolver must reject missing HESTON_V0");
    let msg = err.to_string();
    assert!(
        msg.contains("HESTON_V0"),
        "error must name the missing scalar, got: {msg}"
    );
}

#[test]
fn from_market_strict_succeeds_when_full_config_present() {
    let market = MarketContext::new()
        .insert_price("HESTON_KAPPA", MarketScalar::Unitless(1.5))
        .insert_price("HESTON_THETA", MarketScalar::Unitless(0.06))
        .insert_price("HESTON_SIGMA_V", MarketScalar::Unitless(0.4))
        .insert_price("HESTON_RHO", MarketScalar::Unitless(-0.5))
        .insert_price("HESTON_V0", MarketScalar::Unitless(0.05));
    let params =
        HestonParams::from_market_strict(&market, 0.03, 0.01).expect("strict ok with full set");
    assert_eq!(params.kappa, 1.5);
    assert_eq!(params.theta, 0.06);
    assert_eq!(params.sigma_v, 0.4);
    assert_eq!(params.rho, -0.5);
    assert_eq!(params.v0, 0.05);
}

/// Test that ψ_j(0) ≈ 1 for both probability characteristic functions.
#[test]
fn test_pj_char_function_at_zero() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let log_spot = 100.0_f64.ln();

    // At φ=0, ψ_j(0) should equal 1 (or very close)
    for j in [1u8, 2u8] {
        let (psi, _status) = heston_pj_characteristic_function(j, 1e-10, 1.0, log_spot, &params);
        assert!(
            (psi.re - 1.0).abs() < 0.01,
            "ψ_{}(0) real part should be ~1, got {}",
            j,
            psi.re
        );
        assert!(
            psi.im.abs() < 0.01,
            "ψ_{}(0) imag part should be ~0, got {}",
            j,
            psi.im
        );
    }
}

/// Test that P1 and P2 are within valid probability range [0, 1].
#[test]
fn test_probabilities_in_valid_range() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let settings = HestonFourierSettings::default();

    // Test various moneyness levels
    for strike in [80.0, 100.0, 120.0] {
        let p1 = heston_pj_with_diagnostics(1, 100.0, strike, 1.0, &params, &settings).probability;
        let p2 = heston_pj_with_diagnostics(2, 100.0, strike, 1.0, &params, &settings).probability;

        assert!(
            (0.0..=1.0).contains(&p1),
            "P1 should be in [0,1], got {} for K={}",
            p1,
            strike
        );
        assert!(
            (0.0..=1.0).contains(&p2),
            "P2 should be in [0,1], got {} for K={}",
            p2,
            strike
        );

        // P1 >= P2 for calls (P1 is stock measure, P2 is money measure)
        assert!(
            p1 >= p2 - 1e-6,
            "P1 should be >= P2, got P1={}, P2={} for K={}",
            p1,
            p2,
            strike
        );
    }
}

/// Test that call price is positive and reasonable.
#[test]
fn test_heston_call_positive() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params, None);

    assert!(price > 0.0, "Call price should be positive, got {}", price);
    assert!(
        price < 100.0,
        "Call price should be less than spot, got {}",
        price
    );
}

/// Test put-call parity holds.
#[test]
fn test_heston_put_call_parity() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    let call = heston_call_price_fourier(100.0, 100.0, 1.0, &params, None);
    let put = heston_put_price_fourier(100.0, 100.0, 1.0, &params, None);

    // Put-call parity: C - P = S*exp(-qT) - K*exp(-rT)
    let lhs = call - put;
    let rhs = 100.0 * (-0.02_f64 * 1.0).exp() - 100.0 * (-0.05_f64 * 1.0).exp();

    assert!(
        (lhs - rhs).abs() < 0.01,
        "Put-call parity failed: C-P={} vs S*exp(-qT)-K*exp(-rT)={}",
        lhs,
        rhs
    );
}

/// Test convergence to Black-Scholes as vol-of-vol → 0.
#[test]
fn test_black_scholes_limit() {
    let vol = 0.2;
    let variance = vol * vol;

    // Heston with very small sigma_v should match Black-Scholes
    let params = HestonParams::new(
        0.05,     // r
        0.0,      // q
        2.0,      // kappa (doesn't matter when sigma_v=0)
        variance, // theta = v0 for consistency
        1e-12,    // sigma_v ≈ 0
        0.0,      // rho
        variance, // v0
    )
    .expect("valid");

    let heston_price = heston_call_price_fourier(100.0, 100.0, 1.0, &params, None);
    let bs_price = black_scholes_call(100.0, 100.0, 1.0, 0.05, 0.0, vol);

    assert!(
        (heston_price - bs_price).abs() < 0.01,
        "Heston should converge to BS: Heston={}, BS={}",
        heston_price,
        bs_price
    );
}

/// Test against the volatility/heston.rs implementation.
///
/// Cross-validates this closed-form implementation against the canonical
/// Heston pricer in `finstack_quant_core::math::volatility::heston`.
///
/// The two carry independent implementations of the same Gil-Pelaez /
/// "Little Heston Trap" formulation with different quadrature (composite
/// Gauss-Legendre here, Kahl-Jackel-truncated in core), so this is the pin
/// that would catch either drifting away from the other.
#[test]
fn test_cross_validation_with_core_heston() {
    // Test parameters
    let spot = 100.0;
    let strike = 100.0;
    let time = 0.5;
    let r = 0.05;
    let q = 0.02;
    let v0 = 0.04;
    let kappa = 2.0;
    let theta = 0.04;
    let sigma_v = 0.3;
    let rho = -0.7;

    // Our implementation
    let params = HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0).expect("valid");
    let our_price = heston_call_price_fourier(spot, strike, time, &params, None);

    // Canonical core implementation
    let core_params = finstack_quant_core::math::volatility::heston::HestonParams::new(
        v0, kappa, theta, sigma_v, rho,
    )
    .expect("valid Heston params");
    let vol_price = core_params.price_european(spot, strike, r, q, time, true);

    // These are two implementations of the *same* Gil-Pelaez / Little-Trap
    // formulation, so the only admissible difference is quadrature error.
    // Measured agreement on this parameter set is 8.4e-9 absolute
    // (0.0000 bp); ATM 1y, ITM 2y and a high-vol-of-vol case all sit at or
    // below 0.005 bp. The tolerance is therefore 0.01 bp -- still ~1000x the
    // observed gap, but tight enough that any real algebraic drift fails
    // immediately. (The previous 10 bp bound was ~6 orders of magnitude
    // looser than actual agreement and would not have caught drift.)
    let diff_bp = (our_price - vol_price).abs() * 10_000.0 / our_price.max(1e-12);
    assert!(
        diff_bp < 0.01,
        "Heston implementations diverged by {:.6} bp at canonical params \
             (closed_form={:.9}, core={:.9}); these share a formulation, so any \
             visible gap is quadrature drift.",
        diff_bp,
        our_price,
        vol_price
    );
}

/// Deep-OTM short-dated is where the two quadrature schemes actually diverge.
///
/// Measured 2026-08-03 at S=100, K=120, T=0.25: closed_form 0.029983851 vs
/// core 0.030009480 -- 2.6e-5 absolute, but 8.5 bp *relative* because the
/// price is tiny. Both schemes integrate the same characteristic function, so
/// this is under-convergence in the tail of at least one of them, not a
/// modelling difference.
///
/// Pinned here so the wing behaviour cannot silently get worse. When the
/// shared characteristic function is extracted and a single converged
/// quadrature is adopted, this bound should drop to the 0.01 bp used above.
#[test]
fn test_cross_validation_deep_otm_wing_divergence_is_bounded() {
    let (spot, strike, time, r, q) = (100.0, 120.0, 0.25, 0.02, 0.0);
    let (v0, kappa, theta, sigma_v, rho) = (0.05, 3.0, 0.05, 0.5, -0.8);

    let params = HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0).expect("valid");
    let our_price = heston_call_price_fourier(spot, strike, time, &params, None);

    let core_params = finstack_quant_core::math::volatility::heston::HestonParams::new(
        v0, kappa, theta, sigma_v, rho,
    )
    .expect("valid Heston params");
    let core_price = core_params.price_european(spot, strike, r, q, time, true);

    let abs_diff = (our_price - core_price).abs();
    let diff_bp = abs_diff * 10_000.0 / our_price.max(1e-12);

    assert!(
        abs_diff < 1e-4,
        "deep-OTM absolute divergence grew: {abs_diff:.3e} \
         (closed_form={our_price:.9}, core={core_price:.9})"
    );
    assert!(
        diff_bp < 15.0,
        "deep-OTM relative divergence grew to {diff_bp:.4} bp \
         (closed_form={our_price:.9}, core={core_price:.9})"
    );
}

/// Test a known reference case with reasonable parameters.
///
/// Uses typical equity option parameters and validates the price
/// is within an expected range based on Black-Scholes bounds.
#[test]
fn test_reference_typical_params() {
    let params = HestonParams::new(
        0.05, // r
        0.0,  // q
        2.0,  // kappa
        0.04, // theta
        0.3,  // sigma_v
        -0.5, // rho
        0.04, // v0
    )
    .expect("valid");

    let price = heston_call_price_fourier(100.0, 100.0, 0.5, &params, None);

    // With v0=0.04 (20% vol) and T=0.5, ATM call should be roughly 5-8
    // BS with 20% vol gives ~5.87 for these params
    assert!(
        price > 4.0 && price < 10.0,
        "Heston price {} should be in reasonable range for these parameters",
        price
    );
}

/// Test another reference case: ATM option with typical equity parameters.
///
/// Parameters: S=100, K=100, T=1, r=0.05, q=0.02
/// v0=0.04, kappa=2.0, theta=0.04, sigma=0.3, rho=-0.7
#[test]
fn test_reference_typical_equity() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    let call = heston_call_price_fourier(100.0, 100.0, 1.0, &params, None);
    let put = heston_put_price_fourier(100.0, 100.0, 1.0, &params, None);

    // With v0=0.04 (20% vol), ATM call should be roughly 8-10
    assert!(
        call > 5.0 && call < 15.0,
        "ATM call price {} should be reasonable",
        call
    );
    assert!(
        put > 3.0 && put < 12.0,
        "ATM put price {} should be reasonable",
        put
    );
}

/// Test OTM and ITM options have correct ordering.
#[test]
fn test_moneyness_ordering() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    let call_itm = heston_call_price_fourier(100.0, 90.0, 1.0, &params, None);
    let call_atm = heston_call_price_fourier(100.0, 100.0, 1.0, &params, None);
    let call_otm = heston_call_price_fourier(100.0, 110.0, 1.0, &params, None);

    // ITM > ATM > OTM for calls
    assert!(
        call_itm > call_atm,
        "ITM call {} should be > ATM call {}",
        call_itm,
        call_atm
    );
    assert!(
        call_atm > call_otm,
        "ATM call {} should be > OTM call {}",
        call_atm,
        call_otm
    );
}

/// Test expired option returns intrinsic value.
#[test]
fn test_expired_option() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    // ITM call
    let call_itm = heston_call_price_fourier(100.0, 90.0, 0.0, &params, None);
    assert!(
        (call_itm - 10.0).abs() < 1e-10,
        "Expired ITM call should be intrinsic: {}",
        call_itm
    );

    // OTM call
    let call_otm = heston_call_price_fourier(100.0, 110.0, 0.0, &params, None);
    assert!(
        call_otm.abs() < 1e-10,
        "Expired OTM call should be 0: {}",
        call_otm
    );

    // ITM put
    let put_itm = heston_put_price_fourier(100.0, 110.0, 0.0, &params, None);
    assert!(
        (put_itm - 10.0).abs() < 1e-10,
        "Expired ITM put should be intrinsic: {}",
        put_itm
    );
}

/// Test with extreme parameters to ensure stability.
#[test]
fn test_stability_extreme_params() {
    // High vol-of-vol
    let params_high_vov = HestonParams::new(0.05, 0.0, 5.0, 0.09, 1.0, -0.9, 0.09).expect("valid");
    let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params_high_vov, None);
    assert!(
        price.is_finite() && price >= 0.0,
        "Should handle high vol-of-vol"
    );

    // Very short maturity
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let price_short = heston_call_price_fourier(100.0, 100.0, 0.01, &params, None);
    assert!(
        price_short.is_finite() && price_short >= 0.0,
        "Should handle short maturity"
    );

    // Deep OTM
    let price_deep_otm = heston_call_price_fourier(100.0, 200.0, 1.0, &params, None);
    assert!(
        price_deep_otm.is_finite() && price_deep_otm >= 0.0,
        "Should handle deep OTM"
    );

    // Deep ITM
    let price_deep_itm = heston_call_price_fourier(100.0, 50.0, 1.0, &params, None);
    assert!(
        price_deep_itm.is_finite() && price_deep_itm > 40.0,
        "Should handle deep ITM"
    );
}

/// Test improved accuracy for very short-dated options.
#[test]
fn test_short_maturity_adaptive() {
    let params = HestonParams::new(0.05, 0.0, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    // Very short maturity: T = 1 week
    let time = 7.0 / 365.0;
    let price = heston_call_price_fourier(100.0, 100.0, time, &params, None);

    // Should be close to BS with vol = sqrt(v0) = 0.2
    let bs = black_scholes_call(100.0, 100.0, time, 0.05, 0.0, 0.2);

    // With short maturity and moderate vol-of-vol, Heston ≈ BS
    assert!(
        (price - bs).abs() < 0.5,
        "Short-dated Heston={:.4} should be close to BS={:.4}",
        price,
        bs
    );
    assert!(price > 0.0, "Price must be positive");
}

/// Test that adaptive settings produce valid results across maturities.
#[test]
fn test_adaptive_settings_consistency() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    for &time in &[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0] {
        let price = heston_call_price_fourier(100.0, 100.0, time, &params, None);
        assert!(
            price.is_finite() && price >= 0.0,
            "Price must be finite and non-negative for T={}: got {}",
            time,
            price
        );

        // Put-call parity must hold
        let put = heston_put_price_fourier(100.0, 100.0, time, &params, None);
        let parity = price - put - (100.0 * (-0.02 * time).exp() - 100.0 * (-0.05 * time).exp());
        assert!(
            parity.abs() < 0.1,
            "Put-call parity violated for T={}: residual={}",
            time,
            parity
        );
    }
}

/// Test multi-strike pricing matches the existing single-strike API.
#[test]
fn test_heston_call_strip_matches_single_strike_prices() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];

    let strip_prices = heston_call_prices_fourier(100.0, &strikes, 0.5, &params, None);

    assert_eq!(strip_prices.len(), strikes.len());
    for (idx, &strike) in strikes.iter().enumerate() {
        let single_price = heston_call_price_fourier(100.0, strike, 0.5, &params, None);
        assert!(
            (strip_prices[idx] - single_price).abs() < 1e-12,
            "strip price {} should match single-strike price {} for K={}",
            strip_prices[idx],
            single_price,
            strike
        );
    }
}

/// Test multi-strike put pricing matches the existing single-strike API.
#[test]
fn test_heston_put_strip_matches_single_strike_prices() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];

    let strip_prices = heston_put_prices_fourier(100.0, &strikes, 0.5, &params, None);

    assert_eq!(strip_prices.len(), strikes.len());
    for (idx, &strike) in strikes.iter().enumerate() {
        let single_price = heston_put_price_fourier(100.0, strike, 0.5, &params, None);
        assert!(
            (strip_prices[idx] - single_price).abs() < 1e-12,
            "strip put price {} should match single-strike put price {} for K={}",
            strip_prices[idx],
            single_price,
            strike
        );
    }
}

/// Test multi-strike pricing preserves expected call ordering across a strip.
#[test]
fn test_heston_call_strip_monotonic_in_strike() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let strikes: Vec<f64> = (75..=124).map(f64::from).collect();

    let strip_prices = heston_call_prices_fourier(100.0, &strikes, 1.0, &params, None);

    assert_eq!(strip_prices.len(), strikes.len());
    for window in strip_prices.windows(2) {
        assert!(
            window[0] >= window[1],
            "call strip should be non-increasing in strike: {:?}",
            window
        );
    }
}

/// Test strip pricing remains positive and respects put-call parity.
#[test]
fn test_heston_call_strip_consistency_across_many_strikes() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let spot: f64 = 100.0;
    let time: f64 = 1.0;
    let strikes: Vec<f64> = (75..=124).map(f64::from).collect();

    let strip_prices = heston_call_prices_fourier(spot, &strikes, time, &params, None);

    for (&strike, &call) in strikes.iter().zip(strip_prices.iter()) {
        assert!(
            call.is_finite() && call >= 0.0,
            "call strip price should be finite and non-negative"
        );

        let put = heston_put_price_fourier(spot, strike, time, &params, None);
        let parity =
            call - put - (spot * (-params.q * time).exp() - strike * (-params.r * time).exp());
        assert!(
            parity.abs() < 1e-10,
            "put-call parity should hold across strip for K={strike}: residual={parity}"
        );
    }
}

#[test]
fn test_validation_rejects_invalid_params() {
    assert!(HestonParams::new(0.05, 0.02, -1.0, 0.04, 0.3, -0.7, 0.04).is_err());
    assert!(HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, 1.1, 0.04).is_err());
    assert!(HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.0).is_err());
}

/// W-02: unsupported `gl_order` must be rejected at the construction
/// boundary rather than silently degrading the pricer to the per-strike path.
#[test]
fn fourier_settings_rejects_unsupported_gl_order() {
    let err = HestonFourierSettings::new(100.0, 100, 10, 1e-8)
        .expect_err("gl_order=10 has no Gauss-Legendre table and must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("gl_order"),
        "error should mention gl_order, got: {msg}"
    );

    // Supported orders construct successfully.
    for &order in &SUPPORTED_GL_ORDERS {
        assert!(
            HestonFourierSettings::new(100.0, 100, order, 1e-8).is_ok(),
            "gl_order={order} should be accepted"
        );
    }
}

/// W-02: `validate` rejects degenerate `panels` / `u_max` too.
#[test]
fn fourier_settings_rejects_degenerate_grid() {
    assert!(HestonFourierSettings::new(100.0, 0, 16, 1e-8).is_err());
    assert!(HestonFourierSettings::new(0.0, 100, 16, 1e-8).is_err());
    assert!(HestonFourierSettings::new(f64::NAN, 100, 16, 1e-8).is_err());
    // The default settings must always be valid.
    assert!(HestonFourierSettings::default().validate().is_ok());
}

/// W-03: with extreme parameters that overflow the characteristic function
/// on a large fraction of grid nodes, the strip pricer must degrade to a
/// Black-Scholes fallback (like the scalar Fourier path) rather than return
/// a plausible-but-wrong finite number from a mass-losing integral.
#[test]
fn strip_pricer_falls_back_to_bs_on_corrupted_nodes() {
    // Extreme κ/θ/σᵥ with positive correlation and long maturity drives the
    // characteristic-function exponent past its real-part overflow limit on
    // the bulk of the integration grid, so `heston_pj_characteristic_function`
    // returns `Complex::ZERO` for those nodes.
    let params = HestonParams::new(0.05, 0.0, 10.0, 100.0, 90.0, 0.99, 90.0).expect("valid");
    let settings = HestonFourierSettings::default();
    let spot = 100.0;
    let strike = 100.0;
    let time = 30.0;

    let pricer = HestonStripPricer::new(spot, time, &params, &settings).expect("grid constructs");
    assert!(
        pricer.integrand_corrupted,
        "extreme params should corrupt a large fraction of integration nodes"
    );

    let strip_price = pricer.price_call(strike);
    let bs = black_scholes_call(
        spot,
        strike,
        time,
        params.r,
        params.q,
        params.deterministic_avg_variance(time).sqrt(),
    );

    // The strip price must equal the BS fallback exactly (same code path),
    // not a finite-but-wrong value from the corrupted Fourier integral.
    // Before the W-03 fix the strip path had no mass-loss fallback: the
    // corrupted Gil-Pelaez integral lost most of its mass and produced a
    // plausible-but-wrong call price with no diagnostic.
    assert!(
        (strip_price - bs).abs() < 1e-9,
        "corrupted strip pricer should return the BS fallback {bs}, got {strip_price}"
    );
    assert!(strip_price.is_finite(), "fallback price must be finite");
}

/// W-03: a well-behaved parameter set must NOT trip the corruption fallback
/// — the strip price must still match the per-strike Fourier price.
#[test]
fn strip_pricer_no_false_corruption_on_normal_params() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let settings = HestonFourierSettings::default();
    let pricer = HestonStripPricer::new(100.0, 1.0, &params, &settings).expect("constructs");
    assert!(
        !pricer.integrand_corrupted,
        "benign parameters must not be flagged as corrupted"
    );
    let strip = pricer.price_call(100.0);
    let scalar = heston_call_price_fourier(100.0, 100.0, 1.0, &params, Some(&settings));
    assert!(
        (strip - scalar).abs() < 1e-9,
        "uncorrupted strip price {strip} should match scalar path {scalar}"
    );
}

#[test]
fn test_characteristic_function_handles_extreme_inputs() {
    let params = HestonParams::new(0.05, 0.0, 0.1, 0.04, 1.0, 0.9, 0.04).expect("valid");
    let (psi, _status) = heston_pj_characteristic_function(1, 0.0, 1.0, 100.0_f64.ln(), &params);
    assert!(
        psi.is_finite(),
        "characteristic function should stay finite"
    );
}

/// M-heston-cf: legitimate CF **underflow** must not be conflated with
/// overflow corruption. A long-dated, high-κθ surface (T=15y, κθ=0.27,
/// σᵥ=0.2) underflows |ψ| to zero over much of the grid with perfectly
/// well-formed inputs; the corruption diagnostic must stay clear and the
/// Fourier price must NOT collapse to the Black-Scholes fallback.
#[test]
fn long_dated_high_kappa_theta_does_not_fall_back_to_bs() {
    // κθ = 3.0 × 0.09 = 0.27.
    let params = HestonParams::new(0.03, 0.0, 3.0, 0.09, 0.2, -0.6, 0.04).expect("valid");
    let spot = 100.0;
    let strike = 100.0;
    let time = 15.0;
    let settings = HestonFourierSettings::for_maturity_with_variance(time, params.v0);

    // Neither Gil-Pelaez probability may be flagged corrupted.
    for j in [1u8, 2u8] {
        let diag = heston_pj_with_diagnostics(j, spot, strike, time, &params, &settings);
        assert!(
            !diag.corrupted,
            "P{j} flagged corrupted on a legitimately-underflowing long-dated \
                 surface (underflow misclassified as overflow)"
        );
    }

    // And the price must be a genuine Heston Fourier price, not the BS
    // fallback at either v0 or v_bar(T).
    let price = heston_call_price_fourier(spot, strike, time, &params, Some(&settings));
    let bs_v0 = black_scholes_call(spot, strike, time, params.r, params.q, params.v0.sqrt());
    let bs_vbar = black_scholes_call(
        spot,
        strike,
        time,
        params.r,
        params.q,
        params.deterministic_avg_variance(time).sqrt(),
    );
    assert!(price.is_finite() && price > 0.0);
    assert!(
        (price - bs_v0).abs() > 1e-6 && (price - bs_vbar).abs() > 1e-6,
        "long-dated high-κθ Heston price ({price}) must differ from the BS \
             fallbacks (v0: {bs_v0}, v_bar: {bs_vbar}) — it should be a real \
             Fourier price, not a fallback"
    );
}

/// M-heston-cf: the BS fallback must use the deterministic average
/// variance v̄(T), not v₀. With v0=0.01, θ=0.09, κ=2, T=1 the two differ
/// by a factor ~5.5 in variance; forcing the σᵥ→0 BS branch must
/// reproduce BS at √v̄(T).
#[test]
fn bs_fallback_uses_deterministic_avg_variance_not_v0() {
    let params = HestonParams::new(0.05, 0.0, 2.0, 0.09, 1e-12, -0.5, 0.01).expect("valid");
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;

    // Closed-form check of v_bar itself.
    let kt: f64 = 2.0;
    let expected_vbar = 0.09 + (0.01 - 0.09) * (1.0 - (-kt).exp()) / kt;
    let vbar = params.deterministic_avg_variance(time);
    assert!(
        (vbar - expected_vbar).abs() < 1e-14,
        "v_bar(T) mismatch: got {vbar}, expected {expected_vbar}"
    );

    // σᵥ < 1e-10 forces the BS branch; it must price at √v̄, not √v₀.
    let price = heston_call_price_fourier(spot, strike, time, &params, None);
    let bs_vbar = black_scholes_call(spot, strike, time, params.r, params.q, vbar.sqrt());
    let bs_v0 = black_scholes_call(spot, strike, time, params.r, params.q, params.v0.sqrt());
    assert!(
        (price - bs_vbar).abs() < 1e-12,
        "BS fallback must use v_bar: got {price}, expected {bs_vbar}"
    );
    assert!(
        (price - bs_v0).abs() > 1e-3,
        "BS fallback must NOT use v0 ({bs_v0}); got {price}"
    );
}

/// κ→0 Taylor branch of the deterministic average variance: v̄(t) → v₀ +
/// (v₀−θ) corrections vanish smoothly, matching the closed form just
/// above the switch point.
#[test]
fn deterministic_avg_variance_taylor_branch_is_continuous() {
    let t = 1.0;
    let mk = |kappa: f64| HestonParams {
        r: 0.0,
        q: 0.0,
        kappa,
        theta: 0.09,
        sigma_v: 0.3,
        rho: -0.5,
        v0: 0.01,
    };
    // Just below and just above the 1e-6 κt switch. The two κ values
    // genuinely differ, so allow the O(κt·(v0−θ)) ≈ 1e-8 physical gap.
    let below = mk(0.9e-6).deterministic_avg_variance(t);
    let above = mk(1.1e-6).deterministic_avg_variance(t);
    assert!(
        (below - above).abs() < 5e-8,
        "Taylor/closed-form branches must agree at the switch: {below} vs {above}"
    );
    // κt → 0 limit is v0.
    assert!((below - 0.01).abs() < 1e-6);
}

/// Audit item 6: `From<monte_carlo::HestonParams>` bypassed
/// `HestonParams::new` validation entirely.
///
/// Failure mode locked in: the Monte Carlo `HestonParams` accepts
/// `ρ ∈ [-1, 1]` (inclusive), but the closed-form Fourier pricer requires
/// `ρ ∈ (-1, 1)` (exclusive). A `ρ = ±1` Monte Carlo parameter set must NOT
/// convert into a closed-form `HestonParams` silently — the conversion is
/// now a `TryFrom` that re-runs the full validation.
#[test]
fn try_from_monte_carlo_params_revalidates_correlation_bound() {
    // ρ = 1.0 is valid for the MC process but invalid for the closed-form
    // Fourier pricer; the boundary value must be rejected on conversion.
    let mc_rho_one = finstack_quant_monte_carlo::process::heston::HestonParams::new(
        0.05, 0.02, 2.0, 0.04, 0.3, 1.0, 0.04,
    )
    .expect("rho=1 is accepted by the Monte Carlo constructor");
    let converted: Result<HestonParams, _> = HestonParams::try_from(mc_rho_one);
    assert!(
        converted.is_err(),
        "rho=1 MC params must fail conversion to closed-form HestonParams"
    );

    let mc_rho_neg_one = finstack_quant_monte_carlo::process::heston::HestonParams::new(
        0.05, 0.02, 2.0, 0.04, 0.3, -1.0, 0.04,
    )
    .expect("rho=-1 is accepted by the Monte Carlo constructor");
    assert!(
        HestonParams::try_from(mc_rho_neg_one).is_err(),
        "rho=-1 MC params must fail conversion to closed-form HestonParams"
    );

    // A well-formed MC parameter set still converts successfully.
    let mc_ok = finstack_quant_monte_carlo::process::heston::HestonParams::new(
        0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04,
    )
    .expect("valid MC params");
    let cf = HestonParams::try_from(mc_ok).expect("valid MC params must convert");
    assert_eq!(cf.rho, -0.7);
    assert_eq!(cf.kappa, 2.0);
}

/// The scalar and strip Fourier paths both fall back to Black-Scholes when
/// characteristic-function nodes overflow rather than returning a price
/// computed from incomplete probability mass.
#[test]
fn scalar_fourier_falls_back_to_bs_on_corrupted_nodes() {
    // Same extreme parameter set the strip-pricer corruption test uses:
    // huge κ/θ/σᵥ + ρ≈1 + long maturity overflow the char-function
    // exponent on the bulk of the integration grid.
    let params = HestonParams::new(0.05, 0.0, 10.0, 100.0, 90.0, 0.99, 90.0).expect("valid");
    let settings = HestonFourierSettings::default();
    let spot = 100.0;
    let strike = 100.0;
    let time = 30.0;

    let scalar_price = heston_call_price_fourier(spot, strike, time, &params, Some(&settings));
    let bs = black_scholes_call(
        spot,
        strike,
        time,
        params.r,
        params.q,
        params.deterministic_avg_variance(time).sqrt(),
    );

    // The corrupted scalar path must return the BS fallback exactly, the
    // same way the strip pricer does — not a finite-but-wrong number from
    // a mass-losing Gil-Pelaez integral.
    assert!(
        (scalar_price - bs).abs() < 1e-9,
        "corrupted scalar Fourier pricer should return the BS fallback {bs}, \
             got {scalar_price}"
    );
    assert!(scalar_price.is_finite());
}

/// Audit item 5: a benign parameter set must NOT trip the scalar corruption
/// fallback — the scalar Fourier price must still match the strip price.
#[test]
fn scalar_fourier_no_false_corruption_on_normal_params() {
    let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");
    let settings = HestonFourierSettings::default();
    let scalar = heston_call_price_fourier(100.0, 100.0, 1.0, &params, Some(&settings));
    let strip = HestonStripPricer::new(100.0, 1.0, &params, &settings)
        .expect("constructs")
        .price_call(100.0);
    assert!(
        (scalar - strip).abs() < 1e-9,
        "benign params: scalar {scalar} should match strip {strip}, no false fallback"
    );
}

/// Audit item 4: the Gil-Pelaez probability integral was truncated at a
/// fixed `u_max` with no residual-tail check, and the `[0, 1]` clamp hid the
/// resulting truncation error.
///
/// Failure mode locked in: `heston_pj_with_diagnostics` exposes the
/// pre-clamp probability and an estimated truncation-tail mass. For a
/// short-dated option (rapidly oscillating, slowly decaying integrand) the
/// diagnostic must remain finite and the tail-mass estimate must be
/// available so a caller can detect mis-truncation instead of silently
/// trusting a clamped value.
#[test]
fn gil_pelaez_exposes_truncation_tail_diagnostic() {
    let params = HestonParams::new(0.05, 0.0, 2.0, 0.04, 0.3, -0.7, 0.04).expect("valid");

    // Short maturity with a deliberately too-small u_max: the integrand has
    // not decayed by u_max, so the truncation tail is non-negligible.
    let coarse = HestonFourierSettings::new(8.0, 40, 16, 1e-8).expect("valid settings");
    let diag = heston_pj_with_diagnostics(1, 100.0, 100.0, 0.02, &params, &coarse);
    assert!(
        diag.probability.is_finite() && diag.raw_probability.is_finite(),
        "diagnostic probabilities must be finite"
    );
    assert!(
        diag.tail_estimate.is_finite() && diag.tail_estimate >= 0.0,
        "tail-mass estimate must be a finite non-negative number, got {}",
        diag.tail_estimate
    );
    // The clamped probability is always a valid probability.
    assert!((0.0..=1.0).contains(&diag.probability));
    // A coarse/short-dated truncation must register a non-trivial tail so
    // the mis-truncation is observable rather than hidden by the clamp.
    assert!(
        diag.tail_estimate > 1e-6,
        "coarse u_max on a short-dated option must flag a non-negligible \
             truncation tail, got {}",
        diag.tail_estimate
    );

    // With a well-resolved grid the tail estimate must be small (the
    // integrand has genuinely decayed) — no false positive.
    let fine = HestonFourierSettings::for_maturity(1.0);
    let diag_fine = heston_pj_with_diagnostics(1, 100.0, 100.0, 1.0, &params, &fine);
    assert!(
        diag_fine.tail_estimate < 1e-3,
        "well-resolved integral must have a small truncation tail, got {}",
        diag_fine.tail_estimate
    );
}

/// Low-variance regimes must widen `u_max` so the truncated Gil-Pelaez
/// integral keeps its tail mass.
///
/// With v0 = 0.0016 (4% vol) at T = 2y, `v0·T` is far below the regime the
/// maturity buckets assume; the variance-aware settings must price within
/// tolerance of a brute-force high-`u_max` reference, and must not leave a
/// non-negligible truncation tail.
#[test]
fn low_variance_settings_match_high_umax_reference() {
    let v0 = 0.0016; // 4% vol
    let params = HestonParams::new(0.02, 0.0, 1.5, 0.0016, 0.2, -0.5, v0).expect("valid");
    let (spot, strike, time) = (100.0, 105.0, 2.0);

    let settings = HestonFourierSettings::for_maturity_with_variance(time, v0);
    assert!(
        settings.u_max > HestonFourierSettings::for_maturity(time).u_max,
        "low v0 must widen u_max beyond the maturity-bucket default"
    );

    let reference = HestonFourierSettings::new(2000.0, 2000, 16, 1e-8).expect("valid");
    let price = heston_call_price_fourier(spot, strike, time, &params, Some(&settings));
    let ref_price = heston_call_price_fourier(spot, strike, time, &params, Some(&reference));

    assert!(
        (price - ref_price).abs() < 1e-6 * spot,
        "variance-aware settings ({price:.8}) must match high-u_max reference \
             ({ref_price:.8})"
    );

    // The widened grid must also leave a negligible truncation tail.
    let diag = heston_pj_with_diagnostics(1, spot, strike, time, &params, &settings);
    assert!(
        diag.tail_estimate < HESTON_TAIL_DIAGNOSTIC_THRESHOLD,
        "variance-aware settings must not trip the tail diagnostic, got {}",
        diag.tail_estimate
    );
}
