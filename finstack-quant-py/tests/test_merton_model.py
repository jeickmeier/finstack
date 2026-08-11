"""Tests for Merton structural credit Python bindings."""

from __future__ import annotations

import datetime

import pytest

from finstack_quant.core.currency import Currency
from finstack_quant.core.money import Money
from finstack_quant.core.types import Rate
from finstack_quant.valuations.instruments import (
    BarrierCrossing,
    Bond,
    MertonMcConfig,
    PikMode,
    PikSchedule,
)
from finstack_quant.valuations.models.credit import (
    AssetDynamics,
    BarrierType,
    MertonModel,
)


def test_from_equity_recovers_known_values() -> None:
    known = MertonModel(100.0, 0.20, 80.0, 0.05)
    equity, equity_vol = known.try_implied_equity(1.0)
    calibrated = MertonModel.from_equity(equity, equity_vol, 80.0, 0.05, 0.0, 1.0)
    assert abs(calibrated.asset_value - 100.0) < 0.5
    assert abs(calibrated.asset_vol - 0.20) < 0.05


def test_try_implied_equity_rejects_non_positive_horizon() -> None:
    model = MertonModel(100.0, 0.20, 80.0, 0.05)
    with pytest.raises(ValueError):
        model.try_implied_equity(0.0)


def test_from_equity_rejects_invalid_inputs() -> None:
    with pytest.raises(ValueError):
        MertonModel.from_equity(0.0, 0.30, 80.0, 0.05, 0.0, 1.0)
    with pytest.raises(ValueError):
        MertonModel.from_equity(25.0, -0.30, 80.0, 0.05, 0.0, 1.0)


def test_from_cds_spread_and_from_target_pd_smoke() -> None:
    from_cds = MertonModel.from_cds_spread(200.0, 0.40, 80.0, 0.05, 5.0, 100.0, 0.0)
    assert from_cds.asset_value == 100.0
    assert from_cds.asset_vol > 0.0
    assert 0.0 < from_cds.default_probability(5.0) < 1.0

    from_pd = MertonModel.from_target_pd(100.0, 0.25, 0.05, 0.05, 1.0)
    assert abs(from_pd.default_probability(1.0) - 0.05) < 1e-4


def test_new_with_dynamics_first_passage() -> None:
    model = MertonModel.new_with_dynamics(
        100.0,
        0.25,
        80.0,
        0.05,
        0.0,
        BarrierType.first_passage(0.0),
        AssetDynamics.geometric_brownian(),
    )
    assert isinstance(model.barrier_type, BarrierType)
    assert isinstance(model.dynamics, AssetDynamics)
    terminal = MertonModel(100.0, 0.25, 80.0, 0.05)
    assert model.default_probability(1.0) >= terminal.default_probability(1.0)


def test_to_hazard_curve_returns_hazard_curve() -> None:
    model = MertonModel(100.0, 0.25, 80.0, 0.05)
    curve = model.to_hazard_curve(
        "ACME-HZD",
        datetime.date(2024, 1, 15),
        [1.0, 3.0, 5.0],
        0.40,
    )
    assert curve.id == "ACME-HZD"


def test_simulate_paths_deterministic_with_seed() -> None:
    model = MertonModel(100.0, 0.20, 80.0, 0.05)
    a = model.simulate_paths(10, 60, 5.0, 42, antithetic=False)
    b = model.simulate_paths(10, 60, 5.0, 42, antithetic=False)
    assert a.num_paths == 10
    assert a.num_steps == 60
    assert a.path(0) == b.path(0)
    assert a.get(0, 0) == pytest.approx(100.0)


def test_credit_grades_and_accessors() -> None:
    cg = MertonModel.credit_grades(50.0, 0.30, 40.0, 0.05, 0.10, 0.40)
    assert cg.asset_value > 0.0
    assert cg.debt_barrier > 0.0
    assert 0.0 <= cg.default_probability(1.0) <= 1.0


def test_merton_mc_bond_price_smoke() -> None:
    merton = MertonModel(100.0, 0.25, 60.0, 0.04)
    config = (
        MertonMcConfig(merton)
        .num_paths(64)
        .seed(7)
        .pik_schedule(PikSchedule.uniform(PikMode.pik()))
        .barrier_crossing(BarrierCrossing.discrete())
    )
    bond = Bond.fixed(
        "PIK-1",
        Money(100.0, Currency("USD")),
        Rate(0.08),
        datetime.date(2024, 1, 15),
        datetime.date(2029, 1, 15),
        "USD-OIS",
    )
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 15))
    assert result.num_paths == 64
    assert result.clean_price_pct > 0.0
    assert 0.0 <= result.path_statistics.default_rate <= 1.0
