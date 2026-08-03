"""Behavioral tests for regulatory-capital Python bindings."""

from __future__ import annotations

import json

import pytest

from finstack_quant.margin import SaCcrEngine, SaCcrNettingSetConfig, SaCcrTrade


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
    config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)

    result = SaCcrEngine(alpha=1.5).calculate_ead(config, [])

    assert result["alpha"] == pytest.approx(1.5)
    assert result["ead"] == pytest.approx(0.0)

    with pytest.raises(TypeError, match="reporting_currency"):
        SaCcrEngine(reporting_currency="EUR")
