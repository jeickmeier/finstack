"""Tests for the `statements_analytics` pandas ``DataFrame`` accessors.

Covers every ``to_dataframe`` / ``to_*_dataframe`` on the domain's result and
spec types: `VarianceReport`, `SensitivityResult`, `ScenarioResultSet`,
`BridgeChart`, `ScorecardReport`, `CorkscrewReport`, `Exposure`, and the
real-estate template specs.

Fixtures are built inline (public constructors, or ``from_json`` with a
hand-written canonical payload) so the tests stay self-contained and never
require a full statement-evaluation pipeline.
"""

from __future__ import annotations

import json

import pandas as pd
import pytest

from finstack_quant.statements_analytics import (
    BridgeChart,
    CorkscrewReport,
    Exposure,
    LeaseSpec,
    PropertyTemplateNodes,
    RenewalSpec,
    RentRollOutputNodes,
    ScenarioResultSet,
    ScorecardReport,
    SensitivityConfig,
    SensitivityResult,
    SimpleLeaseSpec,
    VarianceReport,
)

VARIANCE_COLUMNS = [
    "period",
    "metric",
    "baseline",
    "comparison",
    "abs_var",
    "pct_var",
]


def _statement_result(revenue_by_period: dict[str, float]) -> dict[str, object]:
    """Minimal canonical `StatementResult` payload with one node."""
    return {
        "schema_version": 1,
        "nodes": {"revenue": revenue_by_period},
        "meta": {
            "eval_time_ms": None,
            "num_nodes": 1,
            "num_periods": len(revenue_by_period),
        },
    }


# VarianceReport


def _variance_report(rows: list[dict[str, object]]) -> VarianceReport:
    return VarianceReport.from_json(
        json.dumps({
            "baseline_label": "management_case",
            "comparison_label": "bank_case",
            "rows": rows,
        })
    )


def test_variance_report_to_dataframe_keeps_schema_when_empty() -> None:
    """An empty report must still carry every documented column."""
    df = _variance_report([]).to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == VARIANCE_COLUMNS


def test_variance_report_to_dataframe_row_per_metric_period() -> None:
    rows = [
        {
            "period": "2025Q1",
            "metric": "revenue",
            "baseline": 100.0,
            "comparison": 110.0,
            "abs_var": 10.0,
            "pct_var": 0.1,
        },
        {
            "period": "2025Q2",
            "metric": "revenue",
            "baseline": 200.0,
            "comparison": 180.0,
            "abs_var": -20.0,
            "pct_var": -0.1,
        },
    ]
    report = _variance_report(rows)
    df = report.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert set(VARIANCE_COLUMNS) <= set(df.columns)
    assert len(df) == len(report.rows) == 2
    assert list(df["period"]) == ["2025Q1", "2025Q2"]
    assert df.iloc[0]["abs_var"] == 10.0
    assert df.iloc[1]["pct_var"] == pytest.approx(-0.1)


def test_variance_report_to_dataframe_keeps_pct_var_column_when_all_null() -> None:
    """``pct_var`` is skipped by the Rust serde when ``None``.

    The binding emits it explicitly, so the column must survive a report where
    every baseline was effectively zero.
    """
    rows = [
        {
            "period": "2025Q1",
            "metric": "revenue",
            "baseline": 0.0,
            "comparison": 5.0,
            "abs_var": 5.0,
        }
    ]
    df = _variance_report(rows).to_dataframe()
    assert "pct_var" in df.columns
    assert df["pct_var"].isna().all()


# BridgeChart


def _bridge_chart(steps: list[dict[str, object]]) -> BridgeChart:
    return BridgeChart.from_json(
        json.dumps({
            "target_metric": "ebitda",
            "period": "2025Q1",
            "baseline_label": "management_case",
            "comparison_label": "bank_case",
            "baseline_value": 100.0,
            "comparison_value": 130.0,
            "steps": steps,
            "unexplained": 5.0,
        })
    )


def test_bridge_chart_to_dataframe_keeps_schema_when_empty() -> None:
    df = _bridge_chart([]).to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == ["driver", "contribution"]


def test_bridge_chart_to_dataframe_preserves_step_order() -> None:
    steps = [
        {"driver": "revenue", "contribution": 20.0},
        {"driver": "opex", "contribution": -5.0},
        {"driver": "cogs", "contribution": 10.0},
    ]
    chart = _bridge_chart(steps)
    df = chart.to_dataframe()

    # Column order follows serde_json's key ordering; only membership is pinned.
    assert set(df.columns) == {"driver", "contribution"}
    assert len(df) == len(chart.steps) == 3
    # Row order is decomposition order, not sorted.
    assert list(df["driver"]) == ["revenue", "opex", "cogs"]
    assert df.iloc[1]["contribution"] == pytest.approx(-5.0)


# SensitivityResult


def _sensitivity_result(scenarios: list[dict[str, object]]) -> SensitivityResult:
    config = SensitivityConfig(
        "Diagonal",
        [("revenue", "2025Q1", 100.0, [-0.1, 0.1])],
        ["revenue"],
    )
    payload = {"config": json.loads(config.to_json()), "scenarios": scenarios}
    return SensitivityResult.from_json(json.dumps(payload))


def test_sensitivity_result_to_dataframe_keeps_scenario_column_when_empty() -> None:
    df = _sensitivity_result([]).to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == ["scenario"]


def test_sensitivity_result_to_dataframe_row_per_scenario() -> None:
    scenarios = [
        {
            "parameter_values": {"revenue@2025Q1": 90.0},
            "results": _statement_result({"2025Q1": 90.0}),
        },
        {
            "parameter_values": {"revenue@2025Q1": 110.0},
            "results": _statement_result({"2025Q1": 110.0}),
        },
    ]
    result = _sensitivity_result(scenarios)
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(result) == 2
    assert "scenario" in df.columns
    assert "revenue@2025Q1" in df.columns
    assert sorted(df["scenario"]) == [0, 1]
    assert sorted(df["revenue@2025Q1"]) == [90.0, 110.0]


# ScenarioResultSet


def _scenario_result_set() -> ScenarioResultSet:
    return ScenarioResultSet.from_json(
        json.dumps({
            "base": _statement_result({"2025Q1": 100.0}),
            "downside": _statement_result({"2025Q1": 90.0}),
        })
    )


def test_scenario_result_set_to_dataframe_matches_comparison_table() -> None:
    """``to_dataframe`` and ``to_comparison_table`` share one Rust builder."""
    results = _scenario_result_set()
    df = results.to_dataframe(["revenue"])
    table = results.to_comparison_table(["revenue"])

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == table.column_names()
    assert len(df) == table.num_rows
    assert {"period", "metric", "base", "downside"} <= set(df.columns)
    assert df.iloc[0]["base"] == pytest.approx(100.0)
    assert df.iloc[0]["downside"] == pytest.approx(90.0)


def test_scenario_result_set_to_dataframe_rejects_empty_metrics() -> None:
    with pytest.raises(ValueError, match="metrics cannot be empty"):
        _scenario_result_set().to_dataframe([])


# ScorecardReport


def _scorecard_report(metric_scores: list[dict[str, object]]) -> ScorecardReport:
    return ScorecardReport.from_json(
        json.dumps({
            "status": "success",
            "message": "Scorecard complete",
            "data": {
                "rating": "BBB",
                "total_score": 3.5,
                "metric_scores": metric_scores,
                "rating_scale": "S&P",
                "period": "2025Q4",
                "partial": False,
                "weight_coverage": 1.0,
            },
            "warnings": ["leverage metric clipped"],
            "errors": [],
        })
    )


def test_scorecard_report_to_dataframe_is_one_row() -> None:
    df = _scorecard_report([]).to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    expected = {
        "status",
        "message",
        "rating",
        "rating_scale",
        "period",
        "total_score",
        "partial",
        "weight_coverage",
        "warning_count",
        "error_count",
    }
    assert expected <= set(df.columns)
    row = df.iloc[0]
    assert row["status"] == "success"
    assert row["rating"] == "BBB"
    assert row["period"] == "2025Q4"
    assert row["weight_coverage"] == pytest.approx(1.0)
    assert row["warning_count"] == 1
    assert row["error_count"] == 0


def test_scorecard_report_metric_scores_dataframe() -> None:
    scores = [
        {
            "metric": "leverage",
            "value": 3.2,
            "score": 4.0,
            "weight": 0.6,
            "weighted_score": 2.4,
        },
        {
            "metric": "coverage",
            "value": 2.1,
            "score": 3.0,
            "weight": 0.4,
            "weighted_score": 1.2,
        },
    ]
    df = _scorecard_report(scores).to_metric_scores_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(scores)
    assert {"metric", "value", "score", "weight", "weighted_score"} <= set(df.columns)
    assert list(df["metric"]) == ["leverage", "coverage"]
    assert df.iloc[0]["weighted_score"] == pytest.approx(2.4)


def test_scorecard_report_metric_scores_dataframe_keeps_schema_when_empty() -> None:
    df = _scorecard_report([]).to_metric_scores_dataframe()
    assert len(df) == 0
    assert list(df.columns) == [
        "metric",
        "value",
        "score",
        "weight",
        "weighted_score",
    ]


# CorkscrewReport


def _corkscrew_report(validations: list[dict[str, object]]) -> CorkscrewReport:
    return CorkscrewReport.from_json(
        json.dumps({
            "status": "success",
            "message": "Balanced",
            "data": {"validations": validations},
            "warnings": [],
            "errors": [],
        })
    )


def test_corkscrew_report_to_dataframe_is_one_row() -> None:
    validations = [
        {
            "account": "debt",
            "type": "liability",
            "periods_validated": 4,
            "max_error": 0.0,
            "is_valid": True,
        }
    ]
    df = _corkscrew_report(validations).to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "status",
        "message",
        "account_count",
        "warning_count",
        "error_count",
    } <= set(df.columns)
    assert df.iloc[0]["account_count"] == 1


def test_corkscrew_report_validations_dataframe() -> None:
    validations = [
        {
            "account": "debt",
            "type": "liability",
            "periods_validated": 4,
            "max_error": 0.005,
            "is_valid": True,
        },
        {
            "account": "ppe",
            "type": "asset",
            "periods_validated": 4,
            "max_error": 12.5,
            "is_valid": False,
        },
    ]
    df = _corkscrew_report(validations).to_validations_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(validations)
    assert {
        "account",
        "type",
        "periods_validated",
        "max_error",
        "is_valid",
    } <= set(df.columns)
    assert list(df["account"]) == ["debt", "ppe"]
    assert bool(df.iloc[1]["is_valid"]) is False


def test_corkscrew_report_validations_dataframe_keeps_schema_when_empty() -> None:
    df = _corkscrew_report([]).to_validations_dataframe()
    assert len(df) == 0
    assert list(df.columns) == [
        "account",
        "type",
        "periods_validated",
        "max_error",
        "is_valid",
    ]


# Exposure


def test_exposure_to_dataframe_is_one_row() -> None:
    exposure = Exposure("loan-1", 1_000_000.0, 0.4, 0.05, 3.0, 0.02, 0.01, dpd=45)
    df = exposure.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "id",
        "ead",
        "lgd",
        "eir",
        "remaining_maturity",
        "current_pd",
        "origination_pd",
        "dpd",
    ]
    row = df.iloc[0]
    assert row["id"] == "loan-1"
    assert row["ead"] == pytest.approx(1_000_000.0)
    assert row["lgd"] == pytest.approx(0.4)
    assert row["dpd"] == 45


def test_exposure_to_dataframe_defaults_dpd_to_zero() -> None:
    df = Exposure("loan-2", 500_000.0, 0.45, 0.06, 2.0, 0.03, 0.02).to_dataframe()
    assert df.iloc[0]["dpd"] == 0


# Real-estate template specs


def test_simple_lease_spec_to_dataframe() -> None:
    lease = SimpleLeaseSpec(
        "tenant_a",
        "2025Q1",
        25_000.0,
        end="2025Q4",
        growth_rate=0.03,
        free_rent_periods=1,
        occupancy=0.95,
    )
    df = lease.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {
        "node_id",
        "start",
        "end",
        "base_rent",
        "growth_rate",
        "free_rent_periods",
        "occupancy",
    }
    row = df.iloc[0]
    assert row["node_id"] == "tenant_a"
    assert row["start"] == "2025Q1"
    assert row["end"] == "2025Q4"
    assert row["occupancy"] == pytest.approx(0.95)


def test_simple_lease_spec_to_dataframe_keeps_end_column_when_open_ended() -> None:
    """``end`` is skipped by the Rust serde when ``None``; the column stays."""
    df = SimpleLeaseSpec("tenant_b", "2025Q1", 10_000.0).to_dataframe()
    assert "end" in df.columns
    assert df["end"].isna().all()


def test_renewal_spec_to_dataframe() -> None:
    renewal = RenewalSpec(4, 0.75, downtime_periods=1, rent_factor=1.05, free_rent_periods=1)
    df = renewal.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "downtime_periods",
        "term_periods",
        "probability",
        "rent_factor",
        "free_rent_periods",
    } <= set(df.columns)
    row = df.iloc[0]
    assert row["term_periods"] == 4
    assert row["probability"] == pytest.approx(0.75)
    assert row["rent_factor"] == pytest.approx(1.05)


def test_lease_spec_to_dataframe_summarises_nested_collections() -> None:
    lease = LeaseSpec(
        "tenant_c",
        "2025Q1",
        30_000.0,
        end="2026Q4",
        growth_rate=0.02,
        renewal=RenewalSpec(4, 0.6),
    )
    df = lease.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {
        "node_id",
        "start",
        "end",
        "base_rent",
        "growth_rate",
        "growth_convention",
        "free_rent_periods",
        "occupancy",
        "rent_step_count",
        "free_rent_window_count",
        "has_renewal",
    }
    row = df.iloc[0]
    assert row["growth_convention"] == "per_period"
    assert row["rent_step_count"] == 0
    assert row["free_rent_window_count"] == 0
    assert bool(row["has_renewal"]) is True


def test_rent_roll_output_nodes_to_dataframe() -> None:
    df = RentRollOutputNodes().to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "rent_pgi_node",
        "free_rent_node",
        "vacancy_loss_node",
        "rent_effective_node",
    } <= set(df.columns)
    assert df.iloc[0]["rent_pgi_node"] == "rent_pgi"


def test_property_template_nodes_to_dataframe_flattens_rent_roll() -> None:
    df = PropertyTemplateNodes().to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert set(df.columns) == {
        "rent_pgi_node",
        "free_rent_node",
        "vacancy_loss_node",
        "rent_effective_node",
        "other_income_total_node",
        "egi_node",
        "management_fee_node",
        "opex_total_node",
        "noi_node",
        "capex_total_node",
        "ncf_node",
    }
    row = df.iloc[0]
    # The rent-roll block is flattened, never a dict in a cell.
    assert row["rent_effective_node"] == "rent_effective"
    assert row["ncf_node"] == "ncf"
