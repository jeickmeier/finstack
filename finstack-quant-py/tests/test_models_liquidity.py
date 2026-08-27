"""Liquidity binding behavior."""

import pytest

from finstack_quant.models.liquidity import almgren_chriss_impact, kyle_lambda


def test_almgren_chriss_impact_preserves_host_shape() -> None:
    """The Python projection exposes the five canonical ``AlmgrenChrissImpactView`` keys."""
    result = almgren_chriss_impact(
        position_size=10_000.0,
        avg_daily_volume=1_000_000.0,
        volatility=0.02,
        execution_horizon_days=1.0,
        permanent_impact_coef=0.0,
        temporary_impact_coef=0.01,
        reference_price=100.0,
    )

    assert set(result) == {
        "permanent_impact",
        "temporary_impact",
        "total_impact",
        "expected_cost_bp",
        "execution_risk",
    }
    assert result["total_impact"] == pytest.approx(result["permanent_impact"] + result["temporary_impact"])
    assert result["expected_cost_bp"] == pytest.approx(result["total_impact"] / (10_000.0 * 100.0) * 10_000.0)
    assert result["execution_risk"] >= 0.0


def test_kyle_lambda_uses_reference_price() -> None:
    """Kyle lambda is calibrated in price space."""
    assert kyle_lambda([100.0, 200.0], [0.01, -0.02], 50.0) == pytest.approx(0.005)
    assert kyle_lambda([100.0], [0.01], 0.0) is None


def test_liquidity_moved_without_portfolio_aliases() -> None:
    """Liquidity APIs exist only under the models-owned namespace."""
    from finstack_quant import portfolio

    assert not hasattr(portfolio, "almgren_chriss_impact")
    assert not hasattr(portfolio, "kyle_lambda")
