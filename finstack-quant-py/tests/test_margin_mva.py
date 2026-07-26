"""Behavioral tests for MVA Python bindings (margin.compute_mva et al.)."""

from __future__ import annotations

import datetime as dt
import json

import pytest

from finstack_quant.core.market_data.curves import DiscountCurve, HazardCurve
from finstack_quant.margin import (
    ExposureProfile,
    FundingConfig,
    ImDecayProfile,
    ImProfile,
    MvaResult,
    SimmCalculator,
    SimmSensitivities,
    XvaResult,
    compute_bilateral_xva,
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


def test_funding_config_carries_im_profile_for_mva() -> None:
    """``FundingConfig`` exposes the MVA inputs the Rust engine consumes."""
    profile = ImProfile([1.0, 2.0], [1_000_000.0, 1_000_000.0])
    config = FundingConfig(50.0, 30.0, profile, 20.0)

    assert config.funding_spread_bps == pytest.approx(50.0)
    assert config.effective_benefit_bps() == pytest.approx(30.0)
    assert config.margin_funding_spread_bps == pytest.approx(20.0)
    assert config.effective_margin_spread_bps() == pytest.approx(20.0)
    assert config.im_profile is not None
    assert config.im_profile.times == [1.0, 2.0]


def test_funding_config_margin_spread_defaults_to_funding_spread() -> None:
    config = FundingConfig(45.0)
    assert config.im_profile is None
    assert config.margin_funding_spread_bps is None
    assert config.effective_margin_spread_bps() == pytest.approx(45.0)


def test_xva_result_exposes_mva_and_total_xva() -> None:
    """``total_xva`` is the all-in CVA - DVA + FVA + MVA composition."""
    payload = json.dumps({
        "cva": 100.0,
        "dva": 30.0,
        "fva": 20.0,
        "mva": 15.0,
        "bilateral_cva": 70.0,
        "total_xva": 105.0,
        "epe_profile": [[1.0, 10.0]],
        "ene_profile": [[1.0, 2.0]],
        "pfe_profile": [[1.0, 10.0]],
        "max_pfe": 10.0,
        "effective_epe_profile": [[1.0, 10.0]],
        "effective_epe": 10.0,
    })
    result = XvaResult.from_json(payload)

    assert result.mva == pytest.approx(15.0)
    assert result.bilateral_cva == pytest.approx(result.cva - result.dva)
    assert result.total_xva == pytest.approx(result.cva - result.dva + result.fva + result.mva)


def test_xva_result_mva_fields_are_optional() -> None:
    """Payloads without the funding legs still load, with ``None`` fields."""
    payload = json.dumps({
        "cva": 100.0,
        "epe_profile": [[1.0, 10.0]],
        "ene_profile": [[1.0, 0.0]],
        "pfe_profile": [[1.0, 10.0]],
        "max_pfe": 10.0,
        "effective_epe_profile": [[1.0, 10.0]],
        "effective_epe": 10.0,
    })
    result = XvaResult.from_json(payload)

    assert result.mva is None
    assert result.total_xva is None


def flat_hazard_curve(lam: float) -> HazardCurve:
    return HazardCurve("HZ", dt.date(2025, 1, 1), [(0.0, lam), (30.0, lam)])


def uniform_exposure() -> ExposureProfile:
    return ExposureProfile([1.0, 2.0], [1e6, 1e6], [1e6, 1e6], [0.0, 0.0])


def test_compute_bilateral_xva_aggregates_mva_into_total() -> None:
    """MVA reaches ``total_xva``; zero hazard pins it to the Rust figure."""
    no_default = flat_hazard_curve(0.0)
    funding = FundingConfig(50.0, 30.0, ImProfile([1.0, 2.0], [1e6, 1e6]))

    result = compute_bilateral_xva(
        uniform_exposure(),
        no_default,
        no_default,
        flat_discount_curve(),
        0.40,
        0.40,
        funding,
    )

    # Zero hazard ⇒ no credit legs, and MVA is exactly the hand-checked 10_000
    # (flat 1e6 IM, 50bp, DF = 1, grid [1, 2]).
    assert result.cva == pytest.approx(0.0)
    assert result.dva == pytest.approx(0.0)
    assert result.mva == pytest.approx(10_000.0)
    assert result.fva > 0.0
    assert result.total_xva == pytest.approx(result.cva - result.dva + result.fva + result.mva)


def test_compute_bilateral_xva_without_funding_has_no_funding_legs() -> None:
    result = compute_bilateral_xva(
        uniform_exposure(),
        flat_hazard_curve(0.02),
        flat_hazard_curve(0.03),
        flat_discount_curve(),
        0.40,
        0.40,
    )

    assert result.fva is None
    assert result.mva is None
    assert result.cva > 0.0
    # bilateral_cva is credit-only, and with no funding legs it is the total.
    assert result.bilateral_cva == pytest.approx(result.cva - result.dva)
    assert result.total_xva == pytest.approx(result.bilateral_cva)


def test_compute_bilateral_xva_rejects_bad_recovery_rate() -> None:
    with pytest.raises(ValueError, match="recovery_rate"):
        compute_bilateral_xva(
            uniform_exposure(),
            flat_hazard_curve(0.02),
            flat_hazard_curve(0.02),
            flat_discount_curve(),
            1.5,
            0.40,
        )


def test_compute_bilateral_xva_accepts_keyword_arguments() -> None:
    result = compute_bilateral_xva(
        exposure_profile=uniform_exposure(),
        counterparty_hazard_curve=flat_hazard_curve(0.02),
        own_hazard_curve=flat_hazard_curve(0.03),
        discount_curve=flat_discount_curve(),
        counterparty_recovery_rate=0.40,
        own_recovery_rate=0.40,
        funding=None,
    )
    assert result.total_xva is not None
