"""Behavioral tests for regulatory-capital Python bindings."""

from __future__ import annotations

import pytest

from finstack_quant.margin import SaCcrEngine, SaCcrNettingSetConfig


def test_sa_ccr_engine_accepts_only_active_configuration() -> None:
    config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)

    result = SaCcrEngine(alpha=1.5).calculate_ead(config, [])

    assert result["alpha"] == pytest.approx(1.5)
    assert result["ead"] == pytest.approx(0.0)

    with pytest.raises(TypeError, match="reporting_currency"):
        SaCcrEngine(reporting_currency="EUR")
