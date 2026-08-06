"""Tests for the pandas ``DataFrame`` accessors on leaf result types.

Covers the newly added exports across five namespaces:

- ``analytics``: ``BetaResult.to_dataframe``, ``GreeksResult.to_dataframe``,
  ``MultiFactorResult.to_dataframe``.
- ``core.credit``: ``pd.MasterScaleResult.to_dataframe``,
  ``recovery_waterfall.RecoveryWaterfallResult.to_dataframe``,
  ``liability_management.ExchangeOfferAnalysis.to_dataframe`` and
  ``LmeAnalysis.to_dataframe``.
- ``core.market_data``: ``fx.FxRateResult.to_dataframe``,
  ``scalars.ScalarTimeSeries.to_dataframe``.
- ``monte_carlo``: ``GbmPathSummary.to_dataframe``.
- ``factor_model.credit``: ``PeriodDecomposition.to_level_dataframe`` and
  ``to_adder_dataframe``.
- ``valuations.correlation``: ``PortfolioLossResult.to_distribution_dataframe``
  / ``to_summary_dataframe``, ``TrancheLossStatistics.to_dataframe``.

Everything is built through public constructors and calculators, so the tests
stay self-contained.
"""

from __future__ import annotations

import datetime
import json
import math
from datetime import date as dt_date
from datetime import timedelta

import pandas as pd
import pytest

from finstack_quant.analytics import MultiFactorResult, Performance
from finstack_quant.core.credit import liability_management, recovery_waterfall
from finstack_quant.core.credit import pd as credit_pd
from finstack_quant.core.market_data import FxMatrix, ScalarTimeSeries
from finstack_quant.factor_model.credit import (
    CreditCalibrator,
    PeriodDecomposition,
    decompose_levels,
    decompose_period,
)
from finstack_quant.monte_carlo import simulate_gbm_paths
from finstack_quant.valuations.correlation import (
    CopulaSpec,
    CreditExposure,
    PortfolioLossConfig,
    PortfolioLossResult,
    simulate_portfolio_loss,
)

# analytics


def _benchmarked_performance() -> Performance:
    """Two perfectly collinear series so the regressions are well determined."""
    dates = [dt_date(2024, 1, 1) + timedelta(days=i) for i in range(8)]
    benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03, 0.015, -0.005]
    return Performance.from_returns_arrays(
        dates,
        [[2.0 * value for value in benchmark], benchmark],
        ["FUND", "BENCH"],
        benchmark_ticker="BENCH",
        frequency="monthly",
    )


def test_beta_result_to_dataframe_is_one_row() -> None:
    result = _benchmarked_performance().beta()[0]
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {"beta", "std_err", "ci_lower", "ci_upper"}
    assert df["beta"].iloc[0] == pytest.approx(result.beta)
    assert df["ci_upper"].iloc[0] == pytest.approx(result.ci_upper)


def test_greeks_result_to_dataframe_is_one_row() -> None:
    result = _benchmarked_performance().greeks()[0]
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {"alpha", "beta", "r_squared", "adjusted_r_squared"}
    assert df["beta"].iloc[0] == pytest.approx(result.beta)
    assert df["r_squared"].iloc[0] == pytest.approx(result.r_squared)


def _multi_factor_result() -> MultiFactorResult:
    dates = [dt_date(2024, 1, 1) + timedelta(days=i) for i in range(8)]
    factor_a = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03, 0.015, -0.005]
    factor_b = [0.01, 0.0, -0.01, 0.02, 0.0, 0.01, -0.02, 0.005]
    returns = [2.0 * a + 0.5 * b for a, b in zip(factor_a, factor_b, strict=True)]
    perf = Performance.from_returns_arrays(dates, [returns], ["FUND"], frequency="monthly")
    return perf.multi_factor_greeks(0, [factor_a, factor_b])


def test_multi_factor_result_to_dataframe_is_one_row_per_factor() -> None:
    result = _multi_factor_result()
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(result.betas)
    assert list(df.columns) == [
        "factor",
        "beta",
        "alpha",
        "r_squared",
        "adjusted_r_squared",
        "residual_vol",
    ]
    assert list(df["factor"]) == ["factor_0", "factor_1"]
    assert list(df["beta"]) == pytest.approx(list(result.betas))
    # Regression-level statistics repeat on every row.
    assert df["r_squared"].nunique() == 1


def test_multi_factor_result_to_dataframe_accepts_factor_names() -> None:
    result = _multi_factor_result()
    df = result.to_dataframe(["value", "momentum"])
    assert list(df["factor"]) == ["value", "momentum"]


def test_multi_factor_result_to_dataframe_rejects_mismatched_names() -> None:
    result = _multi_factor_result()
    with pytest.raises(ValueError, match="factor_names"):
        result.to_dataframe(["only_one"])


def test_multi_factor_result_to_dataframe_is_stable_across_calls() -> None:
    result = _multi_factor_result()
    pd.testing.assert_frame_equal(result.to_dataframe(), result.to_dataframe())


# core.credit.pd


def test_master_scale_result_to_dataframe_is_one_row() -> None:
    result = credit_pd.MasterScale.sp_assumptions().map_pd(0.003)
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {"grade", "grade_index", "input_pd", "central_pd"}
    assert df["grade"].iloc[0] == result.grade
    assert df["grade_index"].iloc[0] == result.grade_index
    assert df["input_pd"].iloc[0] == pytest.approx(0.003)
    assert df["central_pd"].iloc[0] == pytest.approx(result.central_pd)


def test_master_scale_results_concat_into_a_grading_table() -> None:
    scale = credit_pd.MasterScale.sp_assumptions()
    pds = [0.0005, 0.003, 0.05]
    table = pd.concat([scale.map_pd(value).to_dataframe() for value in pds], ignore_index=True)
    assert len(table) == len(pds)
    assert list(table["input_pd"]) == pytest.approx(pds)


# core.credit.recovery_waterfall

_ALLOCATION_COLUMNS = [
    "id",
    "seniority",
    "priority",
    "total_claim",
    "collateral_recovery",
    "general_recovery",
    "total_recovery",
    "recovery_rate",
    "deficiency",
]


def _waterfall() -> recovery_waterfall.RecoveryWaterfallResult:
    """Three claims supplied out of priority order to exercise the sort."""
    claims = [
        recovery_waterfall.RecoveryClaim("SUB", "subordinated", 3, 100.0),
        recovery_waterfall.RecoveryClaim("SEN", "senior_secured", 1, 100.0),
        recovery_waterfall.RecoveryClaim("MEZZ", "senior_unsecured", 2, 50.0),
    ]
    return recovery_waterfall.allocate_recovery(120.0, claims)


def test_recovery_waterfall_to_dataframe_is_one_row_per_claim() -> None:
    result = _waterfall()
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(result.allocations)
    assert set(_ALLOCATION_COLUMNS) <= set(df.columns)


def test_recovery_waterfall_to_dataframe_is_ordered_by_priority() -> None:
    df = _waterfall().to_dataframe()
    assert list(df["priority"]) == sorted(df["priority"])
    assert list(df["id"]) == ["SEN", "MEZZ", "SUB"]


def test_recovery_waterfall_to_dataframe_keeps_schema_when_empty() -> None:
    result = recovery_waterfall.allocate_recovery(100.0, [])
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == _ALLOCATION_COLUMNS


def test_recovery_waterfall_to_dataframe_is_stable_across_calls() -> None:
    result = _waterfall()
    pd.testing.assert_frame_equal(result.to_dataframe(), result.to_dataframe())


# core.credit.liability_management


def test_exchange_offer_analysis_to_dataframe_is_one_row() -> None:
    analysis = liability_management.analyze_exchange_offer(60.0, 75.0, consent_fee=2.0)
    df = analysis.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {
        "exchange_type",
        "old_npv",
        "new_npv",
        "consent_fee",
        "equity_sweetener_value",
        "tender_total",
        "delta_npv",
        "breakeven_recovery",
        "tender_recommended",
    }
    assert df["exchange_type"].iloc[0] == analysis.exchange_type
    assert df["delta_npv"].iloc[0] == pytest.approx(analysis.delta_npv)
    assert bool(df["tender_recommended"].iloc[0]) is analysis.tender_recommended


_LME_COLUMNS = {
    "lme_type",
    "cost",
    "notional_reduction",
    "discount_capture",
    "discount_capture_pct",
    "remaining_holder_impact_pct",
    "pre_total_debt",
    "post_total_debt",
    "pre_leverage",
    "post_leverage",
    "leverage_reduction",
}


def test_lme_analysis_to_dataframe_flattens_leverage_impact() -> None:
    analysis = liability_management.analyze_lme("open_market_repurchase", 100.0, 0.70, 0.50, ebitda=12.5)
    df = analysis.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == _LME_COLUMNS
    impact = analysis.leverage_impact
    assert impact is not None
    assert df["pre_leverage"].iloc[0] == pytest.approx(impact.pre_leverage)
    assert df["leverage_reduction"].iloc[0] == pytest.approx(impact.leverage_reduction)
    assert df["notional_reduction"].iloc[0] == pytest.approx(analysis.notional_reduction)


def test_lme_analysis_to_dataframe_nulls_leverage_without_ebitda() -> None:
    analysis = liability_management.analyze_lme("open_market_repurchase", 100.0, 0.70, 0.50)
    df = analysis.to_dataframe()
    assert len(df) == 1
    assert set(df.columns) == _LME_COLUMNS
    assert analysis.leverage_impact is None
    assert df["pre_leverage"].isna().all()
    assert df["leverage_reduction"].isna().all()


# core.market_data.fx


def test_fx_rate_result_to_dataframe_is_one_row() -> None:
    matrix = FxMatrix()
    matrix.set_quote("EUR", "USD", 1.1)
    result = matrix.rate("EUR", "USD", datetime.date(2025, 1, 1))
    df = result.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {"rate", "triangulated"}
    assert df["rate"].iloc[0] == pytest.approx(result.rate)
    assert bool(df["triangulated"].iloc[0]) is result.triangulated


# core.market_data.scalars


def test_scalar_time_series_to_dataframe_is_date_indexed() -> None:
    observations = [
        (datetime.date(2025, 1, 3), 0.04),
        (datetime.date(2025, 1, 1), 0.03),
        (datetime.date(2025, 1, 2), 0.035),
    ]
    series = ScalarTimeSeries("SOFR", observations)
    df = series.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["value"]
    assert len(df) == len(series.observations)
    assert isinstance(df.index, pd.DatetimeIndex)
    # Rows are chronological, not insertion order.
    assert list(df.index) == [pd.Timestamp(d) for d, _ in sorted(observations)]
    assert list(df["value"]) == pytest.approx([0.03, 0.035, 0.04])


def test_scalar_time_series_to_dataframe_is_stable_across_calls() -> None:
    series = ScalarTimeSeries(
        "SOFR",
        [(datetime.date(2025, 1, 1), 0.03), (datetime.date(2025, 1, 2), 0.04)],
    )
    pd.testing.assert_frame_equal(series.to_dataframe(), series.to_dataframe())


# monte_carlo


def test_gbm_path_summary_to_dataframe_is_time_by_path() -> None:
    summary = simulate_gbm_paths(100.0, 0.05, 0.0, 0.2, 1.0, 2, 3, seed=7)
    df = summary.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(summary.times)
    assert list(df.columns) == [f"path_{i}" for i in range(len(summary.paths))]
    assert list(df.index) == pytest.approx(summary.times)
    assert list(df["path_0"]) == pytest.approx(summary.paths[0])


def test_gbm_path_summary_to_dataframe_is_stable_across_calls() -> None:
    summary = simulate_gbm_paths(100.0, 0.05, 0.0, 0.2, 1.0, 2, 3, seed=7)
    pd.testing.assert_frame_equal(summary.to_dataframe(), summary.to_dataframe())


# factor_model.credit

_LEVEL_DELTA_COLUMNS = [
    "from_date",
    "to_date",
    "level_index",
    "dimension",
    "bucket",
    "delta",
]
_ADDER_DELTA_COLUMNS = ["from_date", "to_date", "issuer_id", "d_adder"]


def _monthly_dates(n: int, end: dt_date) -> list[str]:
    dates = []
    current = end
    for _ in range(n):
        dates.append(current.isoformat())
        current = current - timedelta(days=30)
    dates.reverse()
    return dates


def _hierarchy_calibration_inputs() -> dict:
    """Synthetic 24-month panel with 6 issuers over a rating x region hierarchy.

    Mirrors the fixture in ``test_credit_factor_model_bindings.py`` so the
    calibration is known to converge for a 2-level hierarchy.
    """
    n = 24
    dates = _monthly_dates(n, dt_date(2024, 3, 31))
    generic_values = [100.0 + 0.5 * math.sin(i) for i in range(n)]

    issuer_specs = [
        ("ISSUER-A", "IG", "EU"),
        ("ISSUER-B", "IG", "NA"),
        ("ISSUER-C", "IG", "APAC"),
        ("ISSUER-D", "HY", "EU"),
        ("ISSUER-E", "HY", "NA"),
        ("ISSUER-F", "HY", "APAC"),
    ]

    spreads: dict[str, list[float]] = {}
    tags: dict[str, dict[str, str]] = {}
    as_of_spreads: dict[str, float] = {}
    for idx, (issuer_id, rating, region) in enumerate(issuer_specs):
        base = 100.0 + idx * 25.0
        beta_pc = 0.7 + 0.05 * idx
        series = [base + beta_pc * (generic_values[i] - 100.0) + 0.1 * math.cos(idx + i * 0.5) for i in range(n)]
        spreads[issuer_id] = series
        tags[issuer_id] = {"rating": rating, "region": region}
        as_of_spreads[issuer_id] = float(series[-1])

    return {
        "history_panel": {"dates": dates, "spreads": spreads},
        "issuer_tags": {"tags": tags},
        "generic_factor": {
            "spec": {"name": "CDX IG 5Y", "series_id": "cdx.ig.5y"},
            "values": generic_values,
        },
        "as_of": "2024-03-31",
        "as_of_spreads": as_of_spreads,
        "idiosyncratic_overrides": {},
    }


def _hierarchy_period_decomposition() -> PeriodDecomposition:
    config = {
        "policy": "globally_off",
        "hierarchy": {"levels": ["rating", "region"]},
        "min_bucket_size_per_level": {"per_level": [3, 3]},
        "vol_model": "sample",
        "covariance_strategy": "diagonal",
        "beta_shrinkage": "none",
        "use_returns_or_levels": "returns",
        "annualization_factor": 12.0,
    }
    inputs = _hierarchy_calibration_inputs()
    model = CreditCalibrator(json.dumps(config)).calibrate(json.dumps(inputs))
    spreads_json = json.dumps(inputs["as_of_spreads"])
    start = decompose_levels(model, spreads_json, 100.0, "2024-02-29")
    end = decompose_levels(model, spreads_json, 101.5, "2024-03-31")
    return decompose_period(start, end)


def _flat_period_decomposition() -> PeriodDecomposition:
    """A level-free hierarchy, so ``to_level_dataframe`` has nothing to emit."""
    config = {
        "policy": "globally_off",
        "hierarchy": {"levels": []},
        "min_bucket_size_per_level": {"per_level": []},
        "vol_model": "sample",
        "covariance_strategy": "diagonal",
        "beta_shrinkage": "none",
        "use_returns_or_levels": "returns",
        "annualization_factor": 12.0,
    }
    inputs = {
        "history_panel": {
            "dates": ["2024-01-01", "2024-02-01"],
            "spreads": {"ZEBRA": [100.0, 101.0], "ALPHA": [90.0, 91.0]},
        },
        "issuer_tags": {"tags": {"ZEBRA": {}, "ALPHA": {}}},
        "generic_factor": {
            "spec": {"name": "G", "series_id": "G"},
            "values": [100.0, 101.0],
        },
        "as_of": "2024-02-01",
        "as_of_spreads": {"ZEBRA": 101.0, "ALPHA": 91.0},
        "idiosyncratic_overrides": {},
    }
    model = CreditCalibrator(json.dumps(config)).calibrate(json.dumps(inputs))
    spreads_json = json.dumps({"ZEBRA": 105.0, "ALPHA": 95.0})
    start = decompose_levels(model, spreads_json, 100.0, "2024-03-01")
    end = decompose_levels(model, json.dumps({"ZEBRA": 106.5, "ALPHA": 96.0}), 101.5, "2024-03-02")
    return decompose_period(start, end)


def test_period_decomposition_level_dataframe_is_long_and_sorted() -> None:
    period = _hierarchy_period_decomposition()
    df = period.to_level_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert set(_LEVEL_DELTA_COLUMNS) <= set(df.columns)

    expected_rows = sum(len(period.level_deltas(k)) for k in range(period.n_levels))
    assert len(df) == expected_rows
    assert expected_rows > 0

    assert list(df["level_index"]) == sorted(df["level_index"])
    for level_index, group in df.groupby("level_index"):
        assert list(group["bucket"]) == sorted(group["bucket"])
        assert set(group["bucket"]) == set(period.level_deltas(int(level_index)))

    assert set(df["from_date"]) == {period.from_date}
    assert set(df["to_date"]) == {period.to_date}


def test_period_decomposition_level_dataframe_keeps_schema_when_no_levels() -> None:
    period = _flat_period_decomposition()
    assert period.n_levels == 0
    df = period.to_level_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == _LEVEL_DELTA_COLUMNS


def test_period_decomposition_adder_dataframe_is_sorted_by_issuer() -> None:
    period = _flat_period_decomposition()
    adders = period.d_adder()
    df = period.to_adder_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert set(_ADDER_DELTA_COLUMNS) <= set(df.columns)
    assert len(df) == len(adders)
    # "ALPHA" was declared after "ZEBRA"; the export is key-sorted.
    assert list(df["issuer_id"]) == sorted(adders)
    assert set(df["from_date"]) == {period.from_date}


def test_period_decomposition_dataframes_are_stable_across_calls() -> None:
    period = _hierarchy_period_decomposition()
    pd.testing.assert_frame_equal(period.to_level_dataframe(), period.to_level_dataframe())
    pd.testing.assert_frame_equal(period.to_adder_dataframe(), period.to_adder_dataframe())


# valuations.correlation


def _portfolio_loss_result() -> PortfolioLossResult:
    exposures = [
        CreditExposure("A", 100.0, 0.05, 0.6, [0.3]),
        CreditExposure("B", 100.0, 0.03, 0.6, [0.3]),
    ]
    config = PortfolioLossConfig(200, 42, 0.99, CopulaSpec.gaussian())
    return simulate_portfolio_loss(exposures, config)


def test_portfolio_loss_distribution_dataframe_is_one_row_per_path() -> None:
    result = _portfolio_loss_result()
    df = result.to_distribution_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["loss"]
    assert len(df) == len(result.losses)
    assert list(df["loss"]) == pytest.approx(list(result.losses))


def test_portfolio_loss_summary_dataframe_is_one_row() -> None:
    result = _portfolio_loss_result()
    df = result.to_summary_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {
        "expected_loss",
        "var",
        "expected_shortfall",
        "confidence",
        "num_paths",
    }
    assert df["expected_loss"].iloc[0] == pytest.approx(result.expected_loss)
    assert df["var"].iloc[0] == pytest.approx(result.var)
    assert df["num_paths"].iloc[0] == len(result.losses)


_TRANCHE_COLUMNS = {
    "attachment",
    "detachment",
    "tranche_notional",
    "expected_loss_fraction",
    "expected_loss_amount",
    "var_fraction",
    "var_amount",
    "expected_shortfall_fraction",
    "expected_shortfall_amount",
    "prob_attachment_breached",
    "prob_full_writedown",
}


def test_tranche_loss_statistics_to_dataframe_is_one_row() -> None:
    stats = _portfolio_loss_result().tranche_loss_statistics(0.0, 0.03, 200.0)
    df = stats.to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == _TRANCHE_COLUMNS
    assert df["attachment"].iloc[0] == pytest.approx(0.0)
    assert df["detachment"].iloc[0] == pytest.approx(0.03)
    assert df["tranche_notional"].iloc[0] == pytest.approx(stats.tranche_notional)


def test_tranche_loss_statistics_concat_into_a_capital_structure() -> None:
    result = _portfolio_loss_result()
    tranches = [(0.0, 0.03), (0.03, 0.07), (0.07, 1.0)]
    table = pd.concat(
        [result.tranche_loss_statistics(a, d, 200.0).to_dataframe() for a, d in tranches],
        ignore_index=True,
    )
    assert len(table) == len(tranches)
    assert list(table["attachment"]) == pytest.approx([a for a, _ in tranches])
    assert list(table["detachment"]) == pytest.approx([d for _, d in tranches])
