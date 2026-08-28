"""Behavioral coverage for models-owned credit-scoring bindings."""

import pytest

from finstack_quant.models.credit import pd, scoring


def test_altman_score_does_not_publish_uncalibrated_pd() -> None:
    args = (0.10, 0.20, 0.15, 1.50, 1.80)

    score, zone = scoring.altman_z_score(*args)
    assert score > 2.99
    assert zone == "safe"


def test_pit_cycle_sign_matches_documented_convention() -> None:
    ttc_pd = 0.02
    downturn = pd.ttc_to_pit(ttc_pd, 0.12, -1.0)
    benign = pd.ttc_to_pit(ttc_pd, 0.12, 1.0)
    assert downturn > ttc_pd > benign


def test_ohlson_indicators_must_be_exactly_binary() -> None:
    args = (8.0, 0.4, 0.2, 0.5, 0.0, 0.1, 0.3, 0.0, 0.1)
    scoring.ohlson_o_score(*args)

    with pytest.raises(ValueError, match="exactly 0 or 1"):
        scoring.ohlson_o_score(*args[:4], 0.5, *args[5:])
    with pytest.raises(ValueError, match="exactly 0 or 1"):
        scoring.ohlson_o_score(*args[:7], 2.0, args[8])
