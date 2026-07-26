"""Portfolio Campisi fixed-income attribution binding tests."""

from __future__ import annotations

import json

import pytest


def _snap(
    sector: str,
    w: float,
    r: float,
    y: float,
    md: float,
    sd: float,
    s: float,
    dy: float,
    ds: float,
) -> dict[str, float | str]:
    """Build one ``FiPositionSnapshot`` mapping with the canonical serde names."""
    return {
        "sector": sector,
        "weight": w,
        "total_return": r,
        "yield_annual": y,
        "modified_duration": md,
        "spread_duration": sd,
        "spread": s,
        "delta_treasury_yield": dy,
        "delta_spread": ds,
    }


def _portfolio() -> list[dict[str, float | str]]:
    """Hand-worked golden portfolio: two sectors, two positions each."""
    return [
        _snap("GOVT", 0.30, 0.0155, 0.040, 5.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("GOVT", 0.20, 0.0190, 0.045, 8.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("CORP", 0.30, 0.0120, 0.060, 4.0, 3.8, 0.0150, -0.0010, 0.0020),
        _snap("CORP", 0.20, 0.0118, 0.070, 6.0, 5.5, 0.0250, -0.0010, 0.0020),
    ]


def _benchmark() -> list[dict[str, float | str]]:
    """Hand-worked golden benchmark matching the Rust unit-test fixture."""
    return [
        _snap("GOVT", 0.45, 0.0155, 0.038, 6.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("GOVT", 0.15, 0.0195, 0.042, 9.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("CORP", 0.25, 0.0090, 0.055, 5.0, 4.8, 0.0120, -0.0010, 0.0020),
        _snap("CORP", 0.15, 0.0100, 0.065, 7.0, 6.5, 0.0200, -0.0010, 0.0020),
    ]


_CONFIG = {"period_years": 0.25, "spread_mode": "spread_duration"}


def test_campisi_attribution_matches_hand_worked_golden() -> None:
    """The binding reproduces the hand-worked golden decomposition."""
    from finstack_quant.portfolio import campisi_attribution

    result = json.loads(campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG)))

    assert result["portfolio_return"] == pytest.approx(0.01441, abs=1e-12)
    assert result["benchmark_return"] == pytest.approx(0.01365, abs=1e-12)
    assert result["active_return"] == pytest.approx(0.00076, abs=1e-12)
    assert result["total_allocation"] == pytest.approx(-0.0007125, abs=1e-12)
    assert result["total_active_carry"] == pytest.approx(0.00103125, abs=1e-12)
    assert result["total_active_treasury"] == pytest.approx(-0.00075, abs=1e-12)
    assert result["total_active_spread"] == pytest.approx(0.0009575, abs=1e-12)
    assert result["total_selection"] == pytest.approx(0.00023375, abs=1e-12)
    assert result["spread_mode"] == "spread_duration"

    # Absolute per-side Campisi split is surfaced too.
    assert result["portfolio_components"]["carry"] == pytest.approx(0.01325, abs=1e-12)
    assert result["portfolio_components"]["treasury"] == pytest.approx(0.0055, abs=1e-12)
    assert result["portfolio_components"]["spread"] == pytest.approx(-0.00448, abs=1e-12)
    assert result["portfolio_components"]["selection"] == pytest.approx(0.00014, abs=1e-12)
    assert result["benchmark_components"]["carry"] == pytest.approx(0.011725, abs=1e-12)

    assert [s["sector"] for s in result["sectors"]] == ["GOVT", "CORP"]
    # sectors is a flat records array — pandas-ready without helper code.
    assert set(result["sectors"][0]) >= {
        "sector",
        "portfolio_weight",
        "benchmark_weight",
        "portfolio_return",
        "benchmark_return",
        "allocation",
        "active_carry",
        "active_treasury",
        "active_spread",
        "selection",
        "total_active",
    }
    govt = result["sectors"][0]
    assert govt["allocation"] == pytest.approx(-0.000285, abs=1e-12)
    assert govt["active_carry"] == pytest.approx(0.000375, abs=1e-12)
    assert govt["active_treasury"] == pytest.approx(-0.000275, abs=1e-12)
    assert govt["active_spread"] == pytest.approx(0.0, abs=1e-12)
    assert govt["selection"] == pytest.approx(0.0001, abs=1e-12)


def test_campisi_attribution_accepts_keyword_arguments() -> None:
    """The advertised ``text_signature`` names are the real keyword names."""
    from finstack_quant.portfolio import campisi_attribution

    result = json.loads(
        campisi_attribution(
            portfolio_json=json.dumps(_portfolio()),
            benchmark_json=json.dumps(_benchmark()),
            config_json=json.dumps(_CONFIG),
        )
    )
    assert result["active_return"] == pytest.approx(0.00076, abs=1e-12)


def test_campisi_attribution_accepts_dts_spread_mode() -> None:
    """``"dts"`` is the other accepted ``spread_mode`` literal and is echoed back."""
    from finstack_quant.portfolio import campisi_attribution

    config = {"period_years": 0.25, "spread_mode": "dts"}
    result = json.loads(campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(config)))

    assert result["spread_mode"] == "dts"
    # Exact inputs ⇒ DTS and spread-duration conventions agree numerically.
    assert result["total_active_spread"] == pytest.approx(0.0009575, abs=1e-12)


def test_campisi_attribution_rejects_unknown_spread_mode() -> None:
    """Only the two documented ``spread_mode`` literals parse."""
    from finstack_quant.portfolio import campisi_attribution

    config = {"period_years": 0.25, "spread_mode": "SpreadDuration"}
    with pytest.raises(ValueError, match="config JSON"):
        campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(config))


def test_campisi_carino_link_reconstructs_compounded_active_return() -> None:
    """Linked effects reconstruct the geometric active return."""
    from finstack_quant.portfolio import campisi_carino_link

    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    result = json.loads(campisi_carino_link(json.dumps([period, period]), json.dumps(_CONFIG)))

    geometric_active = result["portfolio_return_compounded"] - result["benchmark_return_compounded"]
    reconstructed = (
        result["linked_allocation"]
        + result["linked_active_carry"]
        + result["linked_active_treasury"]
        + result["linked_active_spread"]
        + result["linked_selection"]
    )
    assert reconstructed == pytest.approx(geometric_active, abs=1e-10)
    # Carino smoothing must not be a no-op: arithmetic ≠ geometric here.
    arithmetic_active = 2.0 * 0.00076
    assert abs(arithmetic_active - geometric_active) > 1e-7
    scale = geometric_active / arithmetic_active
    assert result["linked_active_spread"] == pytest.approx(2.0 * 0.0009575 * scale, abs=1e-12)
    assert result["linked_allocation"] == pytest.approx(2.0 * -0.0007125 * scale, abs=1e-12)

    assert [s["sector"] for s in result["linked_sectors"]] == ["GOVT", "CORP"]
    assert len(result["periods"]) == 2
    assert result["periods"][0]["active_return"] == pytest.approx(0.00076, abs=1e-12)


def test_campisi_carino_link_accepts_keyword_arguments() -> None:
    """The advertised ``text_signature`` names are the real keyword names."""
    from finstack_quant.portfolio import campisi_carino_link

    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    result = json.loads(
        campisi_carino_link(
            periods_json=json.dumps([period, period]),
            config_json=json.dumps(_CONFIG),
        )
    )
    assert len(result["linked_sectors"]) == 2


def test_campisi_attribution_rejects_bad_json_and_bad_weights() -> None:
    """Malformed JSON raises ValueError; domain errors raise PortfolioError."""
    from finstack_quant.portfolio import PortfolioError, campisi_attribution

    with pytest.raises(ValueError, match="portfolio JSON"):
        campisi_attribution("not json", json.dumps(_benchmark()), json.dumps(_CONFIG))
    with pytest.raises(ValueError, match="benchmark JSON"):
        campisi_attribution(json.dumps(_portfolio()), "not json", json.dumps(_CONFIG))

    bad = _portfolio()
    bad[0]["weight"] = 0.10  # sums to 0.80
    with pytest.raises(PortfolioError, match="Portfolio weights"):
        campisi_attribution(json.dumps(bad), json.dumps(_benchmark()), json.dumps(_CONFIG))


def test_campisi_carino_link_rejects_bad_json_and_domain_errors() -> None:
    """The linked entry point maps parse and domain failures the same way."""
    from finstack_quant.portfolio import PortfolioError, campisi_carino_link

    with pytest.raises(ValueError, match="periods JSON"):
        campisi_carino_link("not json", json.dumps(_CONFIG))
    with pytest.raises(ValueError, match="config JSON"):
        campisi_carino_link(json.dumps([]), "not json")
    with pytest.raises(PortfolioError, match="at least one period"):
        campisi_carino_link(json.dumps([]), json.dumps(_CONFIG))
