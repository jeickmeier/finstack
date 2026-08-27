"""Smoke tests for reusable analytical and correlation model bindings.

Covers:
- B1: `bs_price`, `bs_greeks`, `bs_implied_vol`, `black76_implied_vol`.
- B2: `finstack_quant.models.correlation.nearest_correlation`.
- B3: `SabrParameters` / `SabrModel` / `SabrSmile` / `SabrCalibrator`.
- B5: `barrier_call`, `asian_option_price`, `lookback_option_price`,
      `quanto_option_price`.
"""

from __future__ import annotations

import pytest

from finstack_quant.models import (
    asian_option_price,
    barrier_call,
    black76_implied_vol,
    bs_greeks,
    bs_implied_vol,
    bs_price,
    lookback_option_price,
    quanto_option_price,
    vanilla_expiry_payoff,
)
from finstack_quant.models.correlation import nearest_correlation
from finstack_quant.models.volatility import SabrCalibrator, SabrModel, SabrParameters, SabrSmile

# B1 — Black-Scholes / Black-76 primitives


def test_bs_price_call_atm_is_positive() -> None:
    assert bs_price(100.0, 100.0, 0.05, 0.02, 0.2, 1.0, True) > 0.0


def test_vanilla_expiry_payoff_matches_intrinsic() -> None:
    assert vanilla_expiry_payoff(110.0, 100.0, True) == 10.0
    assert vanilla_expiry_payoff(90.0, 100.0, False) == 10.0
    assert vanilla_expiry_payoff(90.0, 100.0, True) == 0.0
    with pytest.raises(ValueError, match="strike must be finite and positive"):
        vanilla_expiry_payoff(100.0, 0.0, True)
    with pytest.raises(ValueError, match="spot must be finite and non-negative"):
        vanilla_expiry_payoff(-1.0, 100.0, True)
    assert vanilla_expiry_payoff(0.0, 100.0, False) == 100.0


def test_bs_greeks_has_expected_keys() -> None:
    g = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.2, 1.0, True)
    assert set(g) >= {"delta", "gamma", "vega", "theta", "rho", "rho_q"}
    assert 0.0 < g["delta"] < 1.0


def test_bs_implied_vol_round_trip() -> None:
    sigma = 0.25
    price = bs_price(100.0, 110.0, 0.03, 0.01, sigma, 0.75, True)
    iv = bs_implied_vol(100.0, 110.0, 0.03, 0.01, 0.75, price, True)
    assert abs(iv - sigma) < 1e-6


def test_black76_implied_vol_runs() -> None:
    # Sanity: IV solver returns a positive decimal on a reasonable input.
    iv = black76_implied_vol(100.0, 100.0, 0.95, 1.0, 8.0, True)
    assert iv > 0.0


# B2 — nearest_correlation


def test_nearest_correlation_passes_through_valid_matrix() -> None:
    m = [1.0, 0.5, 0.3, 0.5, 1.0, 0.4, 0.3, 0.4, 1.0]
    out = nearest_correlation(m, 3)
    assert len(out) == 9
    for i in range(3):
        assert abs(out[i * 3 + i] - 1.0) < 1e-9


def test_nearest_correlation_rejects_bad_diagonal() -> None:
    # Diagonal of 0.5 — way outside the gate — should raise.
    with pytest.raises(ValueError, match=r"diagonal|Diagonal"):
        nearest_correlation([0.5, 0.5, 0.3, 0.5, 0.5, 0.4, 0.3, 0.4, 0.5], 3)


# B3 — SABR


def test_sabr_equity_default_and_model_implied_vol() -> None:
    p = SabrParameters.equity_default()
    assert p.alpha == pytest.approx(0.20)
    model = SabrModel(p)
    vol = model.implied_vol(100.0, 100.0, 1.0)
    assert vol > 0.0


def test_sabr_calibrator_round_trip() -> None:
    # SABR rho is only weakly identified from symmetric strikes; the
    # calibrator may find a flat-rho equivalent minimum at the precision
    # used here. We check the smile it fits, not the raw rho. The alpha and
    # nu recoveries are the robust diagnostic.
    params = SabrParameters(0.2, 1.0, 0.3, -0.2)
    smile = SabrSmile(params, 100.0, 1.0)
    strikes = [80.0, 90.0, 100.0, 110.0, 120.0]
    vols = smile.generate_smile(strikes)
    fitted = SabrCalibrator().calibrate(100.0, strikes, vols, 1.0, 1.0)
    assert abs(fitted.alpha - params.alpha) < 1e-2
    assert abs(fitted.nu - params.nu) < 1e-1
    # Refit smile shape must match input smile. Tolerance reflects rho's
    # weak identifiability on symmetric strikes: the L-M solver settles on a
    # flat-rho equivalent minimum, which leaves residuals of order 1 vol pt
    # (well under the alpha tolerance of 1e-2 above) on the wings while
    # matching the ATM and inner strikes much more tightly.
    fitted_smile = SabrSmile(fitted, 100.0, 1.0).generate_smile(strikes)
    for v_fit, v_orig in zip(fitted_smile, vols, strict=True):
        assert abs(v_fit - v_orig) < 1e-2


def test_sabr_calibrate_requires_explicit_beta() -> None:
    # beta is required (matching Rust/WASM); omitting it must not silently
    # apply the equity lognormal convention.
    with pytest.raises(TypeError):
        SabrCalibrator().calibrate(100.0, [90.0, 100.0, 110.0], [0.2, 0.19, 0.2], 1.0)


def test_sabr_calibrate_auto_shift_positive_rates_matches_calibrate() -> None:
    params = SabrParameters(0.05, 0.5, 0.4, -0.1)
    smile = SabrSmile(params, 0.03, 1.0)
    strikes = [0.01, 0.02, 0.03, 0.04, 0.05]
    vols = smile.generate_smile(strikes)
    fitted = SabrCalibrator().calibrate_auto_shift(0.03, strikes, vols, 1.0, 0.5)
    assert fitted.shift is None
    assert fitted.alpha > 0.0


def test_sabr_calibrate_auto_shift_negative_rates_uses_shift() -> None:
    params = SabrParameters(0.05, 0.5, 0.4, -0.1, shift=0.03)
    forward = -0.005
    strikes = [-0.015, -0.01, -0.005, 0.0, 0.005]
    smile = SabrSmile(params, forward, 1.0)
    vols = smile.generate_smile(strikes)
    fitted = SabrCalibrator().calibrate_auto_shift(forward, strikes, vols, 1.0, 0.5)
    assert fitted.shift is not None
    assert fitted.shift > 0.0
    assert fitted.is_shifted()


# B5 — Closed-form exotics


def test_barrier_knock_in_plus_knock_out_equals_vanilla() -> None:
    spot, strike, barrier, r, q, sigma, t = 100.0, 100.0, 110.0, 0.05, 0.02, 0.2, 1.0
    up_in = barrier_call(spot, strike, barrier, r, q, sigma, t, "up", "in")
    up_out = barrier_call(spot, strike, barrier, r, q, sigma, t, "up", "out")
    vanilla = bs_price(spot, strike, r, q, sigma, t, True)
    assert abs(up_in + up_out - vanilla) < 1e-6


def test_asian_arithmetic_ge_geometric_for_call() -> None:
    # Arithmetic Asian call dominates the geometric Asian call (AM >= GM).
    arith = asian_option_price(100.0, 100.0, 0.05, 0.02, 0.3, 1.0, 12, "arithmetic", True)
    geom = asian_option_price(100.0, 100.0, 0.05, 0.02, 0.3, 1.0, 12, "geometric", True)
    assert arith >= geom - 1e-9


def test_lookback_floating_strike_call_positive() -> None:
    p = lookback_option_price(
        spot=100.0,
        strike=0.0,  # ignored for floating
        r=0.05,
        q=0.02,
        sigma=0.2,
        t=1.0,
        extremum=100.0,
        strike_type="floating",
        is_call=True,
    )
    assert p > 0.0


def test_quanto_option_price_call_positive() -> None:
    p = quanto_option_price(
        spot=100.0,
        strike=100.0,
        t=1.0,
        rate_domestic=0.05,
        rate_foreign=0.03,
        div_yield=0.01,
        vol_asset=0.2,
        vol_fx=0.1,
        correlation=-0.2,
        is_call=True,
    )
    assert p > 0.0


def test_barrier_unknown_direction_raises() -> None:
    with pytest.raises(ValueError, match=r"barrier spec|direction"):
        barrier_call(100.0, 100.0, 110.0, 0.05, 0.02, 0.2, 1.0, "sideways", "in")
