"""Behavioral tests for MVA Python bindings (margin.compute_mva et al.)."""

from __future__ import annotations

import datetime as dt

import pytest

from finstack_quant.core.market_data.curves import DiscountCurve
from finstack_quant.margin import (
    ImDecayProfile,
    ImProfile,
    MvaResult,
    SimmCalculator,
    SimmSensitivities,
    compute_mva,
    im_profile_from_simm,
)


def flat_discount_curve() -> DiscountCurve:
    knots = [(0.5 * i, 1.0) for i in range(9)]
    return DiscountCurve("USD-OIS", dt.date(2025, 1, 1), knots, interp="log_linear")


def test_decay_profile_factors() -> None:
    assert ImDecayProfile.constant().factor(5.0) == pytest.approx(1.0)
    lin = ImDecayProfile.linear_to_maturity(2.0)
    assert lin.factor(1.0) == pytest.approx(0.5)
    assert lin.factor(3.0) == pytest.approx(0.0)
    sq = ImDecayProfile.sqrt_time(2.0)
    assert sq.factor(1.0) == pytest.approx(0.5**0.5)


def test_compute_mva_flat_spread_arithmetic() -> None:
    # IM = 1e6 constant on [1, 2], 50bp flat, DF = 1, no survival:
    # MVA = 0.005 * 1e6 * 2 = 10_000 exactly (see Rust tests).
    profile = ImProfile([1.0, 2.0], [1_000_000.0, 1_000_000.0])
    result = compute_mva(profile, [(0.0, 50.0)], flat_discount_curve())
    assert isinstance(result, MvaResult)
    assert result.mva == pytest.approx(10_000.0, abs=1e-6)
    assert result.average_im == pytest.approx(1_000_000.0, abs=1e-6)


def test_im_profile_from_simm_scales_by_decay() -> None:
    sens = SimmSensitivities("USD")
    sens.add_ir_delta("USD", "5Y", 50_000.0)
    calc = SimmCalculator("v2_6")
    decay = ImDecayProfile.linear_to_maturity(4.0)
    profile = im_profile_from_simm(calc, sens, "USD", decay, [1.0, 2.0, 4.0])
    assert profile.times == [1.0, 2.0, 4.0]
    assert profile.im_values[0] > 0.0
    assert profile.im_values[1] == pytest.approx(profile.im_values[0] * 2.0 / 3.0)
    assert profile.im_values[2] == pytest.approx(0.0, abs=1e-9)


def test_im_profile_to_dataframe() -> None:
    pd = pytest.importorskip("pandas")
    profile = ImProfile([1.0, 2.0], [100.0, 50.0])
    df = profile.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["im"]
    assert list(df.index) == [1.0, 2.0]
    assert df["im"].tolist() == [100.0, 50.0]


def test_mva_result_to_dataframe_and_json_round_trip() -> None:
    pd = pytest.importorskip("pandas")
    profile = ImProfile([1.0, 2.0], [1_000_000.0, 500_000.0])
    result = compute_mva(profile, [(0.0, 50.0)], flat_discount_curve())
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["im"]
    back = MvaResult.from_json(result.to_json())
    assert back.mva == pytest.approx(result.mva)


def test_compute_mva_validation_errors() -> None:
    profile = ImProfile([1.0], [-5.0])
    with pytest.raises(ValueError, match="non-negative"):
        compute_mva(profile, [(0.0, 50.0)], flat_discount_curve())
    good = ImProfile([1.0], [5.0])
    with pytest.raises(ValueError, match="funding_spread_curve"):
        compute_mva(good, [], flat_discount_curve())


def test_compute_mva_keyword_arguments() -> None:
    # Guards against keyword/positional signature drift (text_signature must
    # match the #[pyo3(signature = ...)] parameter names exactly).
    profile = ImProfile(times=[1.0, 2.0], im_values=[1_000_000.0, 1_000_000.0])
    result = compute_mva(
        im_profile=profile,
        funding_spread_curve=[(0.0, 50.0)],
        discount_curve=flat_discount_curve(),
        survival_curve=None,
    )
    assert result.mva == pytest.approx(10_000.0, abs=1e-6)


def test_im_profile_from_simm_keyword_arguments() -> None:
    sens = SimmSensitivities("USD")
    sens.add_ir_delta("USD", "5Y", 50_000.0)
    calc = SimmCalculator("v2_6")
    decay = ImDecayProfile.linear_to_maturity(maturity_years=4.0)
    profile = im_profile_from_simm(
        calculator=calc,
        sensitivities=sens,
        currency="USD",
        decay=decay,
        time_grid=[1.0, 2.0, 4.0],
    )
    assert profile.times == [1.0, 2.0, 4.0]
