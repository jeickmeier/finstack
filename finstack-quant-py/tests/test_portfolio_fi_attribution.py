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


def test_campisi_carino_link_from_snapshots_reconstructs_compounded_active_return() -> None:
    """Linked effects reconstruct the geometric active return."""
    from finstack_quant.portfolio import campisi_carino_link_from_snapshots

    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    result = json.loads(campisi_carino_link_from_snapshots(json.dumps([period, period]), json.dumps(_CONFIG)))

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

    # ``period_years`` moves exactly two of the five linked totals: carry (it
    # scales ``yield_annual``) and its residual counterpart, selection. Both
    # are pinned to explicit values so a wrong ``period_years`` reaching the
    # Rust call cannot pass silently. Two identical quarters at the golden
    # fixture give the uniform Carino scale k_t/K = 1.01403 exactly.
    assert result["linked_active_carry"] == pytest.approx(0.002091436875, abs=1e-12)
    assert result["linked_selection"] == pytest.approx(0.000474059025, abs=1e-12)
    # Restating the same pins against the per-period totals: 2 x quarterly
    # effect x Carino scale.
    assert result["linked_active_carry"] == pytest.approx(2.0 * 0.00103125 * scale, abs=1e-12)
    assert result["linked_selection"] == pytest.approx(2.0 * 0.00023375 * scale, abs=1e-12)

    assert [s["sector"] for s in result["linked_sectors"]] == ["GOVT", "CORP"]
    assert len(result["periods"]) == 2
    assert result["periods"][0]["active_return"] == pytest.approx(0.00076, abs=1e-12)
    assert result["periods"][0]["spread_mode"] == "spread_duration"
    assert result["periods"][0]["total_active_carry"] == pytest.approx(0.00103125, abs=1e-12)


def test_campisi_carino_link_from_snapshots_accepts_keyword_arguments() -> None:
    """The advertised ``text_signature`` names are the real keyword names."""
    from finstack_quant.portfolio import campisi_carino_link_from_snapshots

    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    result = json.loads(
        campisi_carino_link_from_snapshots(
            periods_json=json.dumps([period, period]),
            config_json=json.dumps(_CONFIG),
        )
    )
    assert len(result["linked_sectors"]) == 2


def test_campisi_carino_link_from_snapshots_propagates_dts_mode() -> None:
    """``spread_mode`` from ``config_json`` actually reaches every period."""
    from finstack_quant.portfolio import PortfolioError, campisi_carino_link_from_snapshots

    dts_config = {"period_years": 0.25, "spread_mode": "dts"}
    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    result = json.loads(campisi_carino_link_from_snapshots(json.dumps([period, period]), json.dumps(dts_config)))
    assert [p["spread_mode"] for p in result["periods"]] == ["dts", "dts"]

    # The two modes are numerically identical on positive spreads, so the echo
    # alone could be cosmetic. DTS additionally fails closed on a zero spread
    # with a non-zero spread term — a case ``spread_duration`` prices fine.
    # Only a genuinely propagated mode can produce this divergence.
    zero_spread_portfolio = [_snap("CORP", 1.00, 0.0120, 0.060, 4.0, 3.8, 0.0, -0.0010, 0.0020)]
    zero_spread_benchmark = [_snap("CORP", 1.00, 0.0090, 0.055, 5.0, 4.8, 0.0120, -0.0010, 0.0020)]
    zero_spread_period = {"portfolio": zero_spread_portfolio, "benchmark": zero_spread_benchmark}
    periods_json = json.dumps([zero_spread_period, zero_spread_period])

    ok = json.loads(campisi_carino_link_from_snapshots(periods_json, json.dumps(_CONFIG)))
    assert ok["periods"][0]["spread_mode"] == "spread_duration"
    assert ok["periods"][0]["total_active_spread"] == pytest.approx(0.0020, abs=1e-12)

    with pytest.raises(PortfolioError, match="DTS spread mode requires a positive spread"):
        campisi_carino_link_from_snapshots(periods_json, json.dumps(dts_config))


def test_campisi_carino_link_links_precomputed_results_of_mixed_lengths() -> None:
    """Results-based linking accepts periods of different lengths (act/365)."""
    from finstack_quant.portfolio import campisi_attribution, campisi_carino_link

    # Real calendar months on act/365: January and February 2025. A single
    # shared ``period_years`` cannot express this; only the results-based
    # entry point can.
    january = json.loads(
        campisi_attribution(
            json.dumps(_portfolio()),
            json.dumps(_benchmark()),
            json.dumps({"period_years": 31.0 / 365.0, "spread_mode": "spread_duration"}),
        )
    )
    february = json.loads(
        campisi_attribution(
            json.dumps(_portfolio()),
            json.dumps(_benchmark()),
            json.dumps({"period_years": 28.0 / 365.0, "spread_mode": "spread_duration"}),
        )
    )

    # The two periods really are different lengths: carry differs, and the
    # residual selection absorbs the difference (total return is an input).
    assert january["total_active_carry"] != february["total_active_carry"]
    assert january["total_active_carry"] == pytest.approx(0.004125 * (31.0 / 365.0), abs=1e-15)
    assert february["total_active_carry"] == pytest.approx(0.004125 * (28.0 / 365.0), abs=1e-15)

    result = json.loads(campisi_carino_link(json.dumps([january, february])))

    geometric_active = result["portfolio_return_compounded"] - result["benchmark_return_compounded"]
    reconstructed = (
        result["linked_allocation"]
        + result["linked_active_carry"]
        + result["linked_active_treasury"]
        + result["linked_active_spread"]
        + result["linked_selection"]
    )
    assert reconstructed == pytest.approx(geometric_active, abs=1e-12)

    # Compounding is over the same two identical total returns as the
    # equal-length case; only the carry/selection split differs.
    assert result["portfolio_return_compounded"] == pytest.approx(1.01441**2 - 1.0, abs=1e-12)
    assert result["benchmark_return_compounded"] == pytest.approx(1.01365**2 - 1.0, abs=1e-12)

    scale = geometric_active / (2.0 * 0.00076)
    expected_carry = (january["total_active_carry"] + february["total_active_carry"]) * scale
    expected_selection = (january["total_selection"] + february["total_selection"]) * scale
    assert result["linked_active_carry"] == pytest.approx(expected_carry, abs=1e-14)
    assert result["linked_selection"] == pytest.approx(expected_selection, abs=1e-14)

    assert [s["sector"] for s in result["linked_sectors"]] == ["GOVT", "CORP"]
    assert len(result["periods"]) == 2


def test_campisi_carino_link_accepts_keyword_arguments() -> None:
    """The advertised ``text_signature`` name is the real keyword name."""
    from finstack_quant.portfolio import campisi_attribution, campisi_carino_link

    period = json.loads(campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG)))
    result = json.loads(campisi_carino_link(periods_json=json.dumps([period, period])))
    assert len(result["linked_sectors"]) == 2
    assert result["linked_active_carry"] == pytest.approx(0.002091436875, abs=1e-12)


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


def test_campisi_attribution_rejects_zero_net_weight_sector() -> None:
    """A sector netting to exactly zero weight has no defined per-unit rate.

    Long/short pairs and CDS-vs-cash hedges inside one bucket net to zero. Their
    real contribution stays in the side return while every per-sector effect
    would be forced to zero, so the decomposition must fail closed with the
    offending sector named rather than silently reporting a broken split.
    """
    from finstack_quant.portfolio import PortfolioError, campisi_attribution

    core = _snap("CORE", 1.00, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0)
    hedged_portfolio = [
        core,
        _snap("HEDGE", 0.50, 0.0400, 0.060, 3.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("HEDGE", -0.50, 0.0100, 0.020, 1.0, 0.0, 0.0, -0.0010, 0.0),
    ]
    clean_benchmark = [_snap("CORE", 1.00, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0)]

    with pytest.raises(PortfolioError, match="HEDGE") as portfolio_side:
        campisi_attribution(json.dumps(hedged_portfolio), json.dumps(clean_benchmark), json.dumps(_CONFIG))
    assert "Portfolio" in str(portfolio_side.value)

    hedged_benchmark = [
        _snap("CORE", 1.00, 0.0140, 0.044, 5.5, 0.0, 0.0, -0.0010, 0.0),
        _snap("HEDGE", 0.40, 0.0300, 0.055, 4.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("HEDGE", -0.40, 0.0050, 0.015, 1.0, 0.0, 0.0, -0.0010, 0.0),
    ]
    with pytest.raises(PortfolioError, match="HEDGE") as benchmark_side:
        campisi_attribution(json.dumps([core]), json.dumps(hedged_benchmark), json.dumps(_CONFIG))
    assert "Benchmark" in str(benchmark_side.value)

    # A sector genuinely absent from one side stays legal — that is the
    # legitimate zero-weight case and must not regress.
    one_sided = [
        _snap("CORE", 0.80, 0.0150, 0.048, 5.0, 0.0, 0.0, -0.0010, 0.0),
        _snap("EXTRA", 0.20, 0.0210, 0.070, 3.0, 4.0, 0.0300, -0.0010, -0.0010),
    ]
    result = json.loads(campisi_attribution(json.dumps(one_sided), json.dumps(clean_benchmark), json.dumps(_CONFIG)))
    assert [s["sector"] for s in result["sectors"]] == ["CORE", "EXTRA"]
    assert result["sectors"][1]["benchmark_weight"] == pytest.approx(0.0, abs=1e-15)
    reconstructed = (
        result["total_allocation"]
        + result["total_active_carry"]
        + result["total_active_treasury"]
        + result["total_active_spread"]
        + result["total_selection"]
    )
    assert reconstructed == pytest.approx(result["active_return"], abs=1e-12)


def test_campisi_carino_link_from_snapshots_rejects_bad_json_and_domain_errors() -> None:
    """The snapshot-based linked entry point maps parse and domain failures."""
    from finstack_quant.portfolio import PortfolioError, campisi_carino_link_from_snapshots

    with pytest.raises(ValueError, match="periods JSON"):
        campisi_carino_link_from_snapshots("not json", json.dumps(_CONFIG))
    with pytest.raises(ValueError, match="config JSON"):
        campisi_carino_link_from_snapshots(json.dumps([]), "not json")
    with pytest.raises(PortfolioError, match="at least one period"):
        campisi_carino_link_from_snapshots(json.dumps([]), json.dumps(_CONFIG))


def test_campisi_carino_link_rejects_bad_json_and_domain_errors() -> None:
    """The results-based linked entry point maps parse and domain failures."""
    from finstack_quant.portfolio import PortfolioError, campisi_attribution, campisi_carino_link

    with pytest.raises(ValueError, match="period results JSON"):
        campisi_carino_link("not json")
    # A period *input* is not a period *result*: the schemas are distinct.
    with pytest.raises(ValueError, match="period results JSON"):
        campisi_carino_link(json.dumps([{"portfolio": _portfolio(), "benchmark": _benchmark()}]))
    with pytest.raises(PortfolioError, match="at least one period"):
        campisi_carino_link(json.dumps([]))

    period = json.loads(campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG)))
    renamed = json.loads(json.dumps(period))
    renamed["sectors"][0]["sector"] = "DIFFERENT"
    with pytest.raises(PortfolioError, match="sector ordering"):
        campisi_carino_link(json.dumps([period, renamed]))

    dts = json.loads(
        campisi_attribution(
            json.dumps(_portfolio()),
            json.dumps(_benchmark()),
            json.dumps({"period_years": 0.25, "spread_mode": "dts"}),
        )
    )
    with pytest.raises(PortfolioError, match="spread mode"):
        campisi_carino_link(json.dumps([period, dts]))


def test_campisi_result_denies_unknown_fields_on_every_input_path() -> None:
    """A stale or misspelled key must fail closed, not be silently dropped.

    ``FiAttributionResult`` is an *input* type: :func:`campisi_carino_link` and
    :func:`campisi_reconciliation_check` both deserialize it. Silently ignoring
    an unknown key would let a hand-assembled or stale payload flow through and
    still "reconcile", because linking reads the period returns rather than the
    supplied ``active_return``.
    """
    from finstack_quant.portfolio import (
        campisi_attribution,
        campisi_carino_link,
        campisi_reconciliation_check,
    )

    result = json.loads(campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG)))

    # Our own output round-trips: deny_unknown_fields must not reject it.
    campisi_carino_link(json.dumps([result]))
    campisi_reconciliation_check(json.dumps(result), 1e-10)

    top_level = {**result, "bogus_field": 1.0}
    with pytest.raises(ValueError, match="period results JSON"):
        campisi_carino_link(json.dumps([top_level]))
    with pytest.raises(ValueError, match="result JSON"):
        campisi_reconciliation_check(json.dumps(top_level), 1e-10)

    nested_sector = json.loads(json.dumps(result))
    nested_sector["sectors"][0]["bogus_field"] = 1.0
    with pytest.raises(ValueError, match="result JSON"):
        campisi_reconciliation_check(json.dumps(nested_sector), 1e-10)

    nested_components = json.loads(json.dumps(result))
    nested_components["portfolio_components"]["bogus_field"] = 1.0
    with pytest.raises(ValueError, match="result JSON"):
        campisi_reconciliation_check(json.dumps(nested_components), 1e-10)


def test_campisi_reconciliation_check_reports_residual_and_honours_tolerance() -> None:
    """The reconciliation gate is reachable from Python and its tolerance bites."""
    from finstack_quant.portfolio import campisi_attribution, campisi_reconciliation_check

    result_json = campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG))
    report = json.loads(campisi_reconciliation_check(result_json, 1e-10))

    assert report["is_reconciled"] is True
    assert report["tolerance"] == pytest.approx(1e-10)
    assert abs(report["total_residual"]) <= 1e-10

    # Hand-check against the five totals the caller would otherwise re-sum.
    result = json.loads(result_json)
    reconstructed = (
        result["total_allocation"]
        + result["total_active_carry"]
        + result["total_active_treasury"]
        + result["total_active_spread"]
        + result["total_selection"]
    )
    assert report["total_residual"] == pytest.approx(result["active_return"] - reconstructed, abs=1e-18)

    # Tolerance is load-bearing: a tampered active_return breaks the identity.
    tampered = {**result, "active_return": result["active_return"] + 0.01}
    assert json.loads(campisi_reconciliation_check(json.dumps(tampered), 1e-10))["is_reconciled"] is False
    assert json.loads(campisi_reconciliation_check(json.dumps(tampered), 1.0))["is_reconciled"] is True


def test_campisi_reconciliation_check_accepts_keyword_arguments() -> None:
    """Keyword names match the documented ``text_signature``."""
    from finstack_quant.portfolio import campisi_attribution, campisi_reconciliation_check

    result_json = campisi_attribution(json.dumps(_portfolio()), json.dumps(_benchmark()), json.dumps(_CONFIG))
    report = json.loads(campisi_reconciliation_check(result_json=result_json, tolerance=1e-10))
    assert report["is_reconciled"] is True


def test_campisi_linked_result_stamps_the_shared_spread_mode() -> None:
    """Linked results restate the spread convention linking enforced."""
    from finstack_quant.portfolio import campisi_carino_link_from_snapshots

    period = {"portfolio": _portfolio(), "benchmark": _benchmark()}
    linked = json.loads(campisi_carino_link_from_snapshots(json.dumps([period, period]), json.dumps(_CONFIG)))
    assert linked["spread_mode"] == "spread_duration"

    dts_config = {"period_years": 0.25, "spread_mode": "dts"}
    linked_dts = json.loads(campisi_carino_link_from_snapshots(json.dumps([period, period]), json.dumps(dts_config)))
    assert linked_dts["spread_mode"] == "dts"
