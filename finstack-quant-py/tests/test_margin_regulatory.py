"""Behavioral tests for regulatory-capital Python bindings."""

from __future__ import annotations

import json
import pickle

import pytest

from finstack_quant.margin import (
    EadResult,
    FrtbSbaEngine,
    FrtbSbaResult,
    FrtbSensitivities,
    SaCcrEngine,
    SaCcrNettingSetConfig,
    SaCcrTrade,
    frtb_sba_charge,
    saccr_ead,
)


def linear_trade_payload() -> dict[str, object]:
    """Return a complete canonical linear-trade payload."""
    return {
        "trade_id": "IRS-1",
        "asset_class": "interest_rate",
        "notional": 1_000_000.0,
        "start_date": "2025-01-15",
        "end_date": "2030-01-15",
        "underlier": "USD-SOFR",
        "hedging_set": "USD-IR",
        "direction": 1.0,
        "supervisory_delta": 1.0,
        "mtm": 10_000.0,
        "is_option": False,
        "option_type": None,
    }


def test_sa_ccr_trade_requires_validated_canonical_json() -> None:
    payload = linear_trade_payload()
    trade = SaCcrTrade.from_json(json.dumps(payload))

    assert trade.trade_id == "IRS-1"
    assert trade.asset_class == "interest_rate"
    assert trade.notional == pytest.approx(1_000_000.0)

    with pytest.raises(TypeError, match=r"cannot create .*SaCcrTrade.* instances"):
        SaCcrTrade(
            "IRS-1",
            "interest_rate",
            1_000_000.0,
            2025,
            1,
            15,
            2030,
            1,
            15,
            "USD-SOFR",
            "USD-IR",
            1.0,
            10_000.0,
        )


def test_sa_ccr_trade_from_json_validates_regulatory_semantics() -> None:
    option_payload = linear_trade_payload()
    option_payload["is_option"] = True
    option_payload["option_type"] = "call_long"
    option_payload["supervisory_delta"] = 0.6
    option = SaCcrTrade.from_json(json.dumps(option_payload))
    assert json.loads(option.to_json())["option_type"] == "call_long"

    payload = linear_trade_payload()
    payload["supervisory_delta"] = 0.5
    with pytest.raises(ValueError, match="linear trade supervisory_delta must be"):
        SaCcrTrade.from_json(json.dumps(payload))

    payload = linear_trade_payload()
    payload["is_option"] = True
    payload["option_type"] = None
    with pytest.raises(ValueError, match="requires option_type"):
        SaCcrTrade.from_json(json.dumps(payload))

    payload = linear_trade_payload()
    payload["supervisory_factor"] = 0.05
    with pytest.raises(ValueError, match="unknown field"):
        SaCcrTrade.from_json(json.dumps(payload))


def test_sa_ccr_engine_accepts_only_active_configuration() -> None:
    with pytest.raises(TypeError, match="reporting_currency"):
        SaCcrEngine(reporting_currency="EUR")


def test_saccr_ead_returns_typed_result() -> None:
    """`saccr_ead` returns the full `EadResult`, not a lossy 3-tuple."""
    trade = SaCcrTrade.from_json(json.dumps(linear_trade_payload()))

    result = saccr_ead([trade], 2025, 1, 15)

    assert isinstance(result, EadResult)
    assert result.ead == pytest.approx(result.alpha * (result.rc + result.pfe))
    # Fields the old (rc, pfe, ead) tuple dropped entirely.
    assert result.alpha == pytest.approx(1.4)
    assert result.multiplier > 0.0
    assert result.maturity_factor > 0.0
    assert result.add_on_aggregate > 0.0
    assert result.add_on_by_asset_class["interest_rate"] == pytest.approx(result.add_on_aggregate)


def test_ead_result_wire_and_frame_surfaces() -> None:
    trade = SaCcrTrade.from_json(json.dumps(linear_trade_payload()))
    result = saccr_ead([trade], 2025, 1, 15)

    round_tripped = EadResult.from_json(result.to_json())
    assert round_tripped.ead == pytest.approx(result.ead)
    assert pickle.loads(pickle.dumps(result)).ead == pytest.approx(  # noqa: S301
        result.ead
    )
    assert "EadResult(" in repr(result)

    frame = result.to_dataframe()
    assert list(frame.columns) == [
        "ead",
        "rc",
        "pfe",
        "multiplier",
        "add_on_aggregate",
        "alpha",
        "maturity_factor",
    ]
    assert len(frame) == 1
    assert str(frame["ead"].dtype) == "float64"

    add_ons = result.to_add_on_dataframe()
    assert list(add_ons.columns) == ["asset_class", "add_on"]
    assert add_ons["asset_class"].tolist() == ["interest_rate"]
    assert str(add_ons["add_on"].dtype) == "float64"


def test_sa_ccr_engine_calculate_ead_returns_typed_result() -> None:
    config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)

    result = SaCcrEngine(alpha=1.5).calculate_ead(config, [])

    assert isinstance(result, EadResult)
    assert result.alpha == pytest.approx(1.5)
    assert result.ead == pytest.approx(0.0)
    assert result.add_on_by_asset_class == {}


def sample_sensitivities() -> FrtbSensitivities:
    sens = FrtbSensitivities("USD")
    sens.add_girr_delta("5Y", 25_000.0)
    sens.add_equity_delta("ACME", 1, 12_000.0)
    sens.add_rrao_position("EXOTIC-1", 5_000_000.0, True)
    return sens


def test_frtb_sba_charge_returns_typed_result() -> None:
    result = frtb_sba_charge(sample_sensitivities())

    assert isinstance(result, FrtbSbaResult)
    assert result.total > 0.0
    assert result.rrao > 0.0
    assert result.drc >= 0.0
    assert result.binding_scenario in {"low", "medium", "high"}
    assert set(result.scenario_charges) <= {"low", "medium", "high"}
    assert result.delta_by_risk_class["girr"] > 0.0
    assert result.delta_by_risk_class["equity"] > 0.0


def test_frtb_sba_result_wire_and_frame_surfaces() -> None:
    result = FrtbSbaEngine().calculate(sample_sensitivities())

    assert isinstance(result, FrtbSbaResult)
    round_tripped = FrtbSbaResult.from_json(result.to_json())
    assert round_tripped.total == pytest.approx(result.total)
    assert pickle.loads(pickle.dumps(result)).total == pytest.approx(  # noqa: S301
        result.total
    )
    assert "FrtbSbaResult(" in repr(result)

    frame = result.to_dataframe()
    assert list(frame.columns) == ["total", "drc", "rrao", "binding_scenario"]
    assert len(frame) == 1
    assert str(frame["total"].dtype) == "float64"

    breakdown = result.to_breakdown_dataframe()
    assert list(breakdown.columns) == ["component", "risk_class", "charge"]
    assert set(breakdown["component"]) <= {"delta", "vega", "curvature"}
    assert str(breakdown["charge"].dtype) == "float64"


def test_frtb_scenario_selection_still_honoured() -> None:
    only_low = frtb_sba_charge(sample_sensitivities(), correlation_scenario="low")

    assert only_low.binding_scenario == "low"
    assert set(only_low.scenario_charges) == {"low"}
