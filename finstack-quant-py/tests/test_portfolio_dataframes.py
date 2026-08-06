"""Tests for the portfolio-domain ``to_*_dataframe`` accessors.

Every portfolio result type that owns a row collection (or is a flat set of
scalars) gets a pandas exit. These tests pin, for each new frame: the return
type, the documented columns, and the row count against the underlying
collection.

Fixtures are JSON payloads patched inline, or objects built through public
constructors and pipeline functions, so the tests stay self-contained and do
not depend on notebook fixtures or on-disk golden data.
"""

from __future__ import annotations

from datetime import date
import json

import pandas as pd
import pytest

from finstack_quant.core.market_data import DiscountCurve, MarketContext
from finstack_quant.factor_model.credit import CreditFactorModel
from finstack_quant.portfolio import (
    Constraint,
    FactorAssignmentReport,
    MetricExpr,
    MissingMetricPolicy,
    Objective,
    PerPositionMetric,
    Portfolio,
    PortfolioMetrics,
    PortfolioOptimizationResult,
    PortfolioOptimizationSpec,
    PositionBudgetEntry,
    PositionFilter,
    PositionRiskDecomposition,
    PositionVarContribution,
    RiskBudgetResult,
    RiskDecomposition,
    StressAttribution,
    StressResult,
    TailScenarioBreakdown,
    TradeSpec,
    WeightingScheme,
    WhatIfResult,
    aggregate_full_cashflows,
    attribute_portfolio_pnl,
    build_credit_vol_report,
    build_stress_attribution,
    optimize_portfolio,
    value_portfolio,
)

AS_OF = "2025-01-15"
EMPTY_PORTFOLIO = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'


# Shared fixtures


def _portfolio_json() -> str:
    """Single-deposit portfolio priced off the ``USD-OIS`` discount curve."""
    return json.dumps({
        "id": "DF-EXITS",
        "as_of": AS_OF,
        "base_currency": "USD",
        "entities": {"FUND": {"id": "FUND"}},
        "positions": [
            {
                "position_id": "USD-POS",
                "entity_id": "FUND",
                "instrument_id": "USD-DEP",
                "instrument_spec": {
                    "type": "deposit",
                    "spec": {
                        "id": "USD-DEP",
                        "notional": {"amount": "1000000", "currency": "USD"},
                        "start_date": AS_OF,
                        "maturity": "2025-07-15",
                        "day_count": "act_360",
                        "quote_rate": "0.04",
                        "discount_curve_id": "USD-OIS",
                        "attributes": {},
                    },
                },
                "quantity": 1.0,
                "unit": "units",
            }
        ],
    })


def _market() -> MarketContext:
    market = MarketContext()
    market.insert(
        DiscountCurve(
            "USD-OIS",
            date.fromisoformat(AS_OF),
            [(0.0, 1.0), (0.5, 0.98), (1.0, 0.95)],
            day_count="act_365f",
        )
    )
    return market


def _materialization_bundle() -> str:
    return json.dumps({
        "schema": "finstack_quant.portfolio_materialization/1",
        "portfolio": {
            "id": "materialized-deposit",
            "base_currency": "USD",
            "as_of": "2025-01-01",
            "entities": {"entity": {"id": "entity", "name": None}},
        },
        "instruments": [
            {
                "artifact_id": "artifact-0",
                "envelope": {
                    "schema": "finstack_quant.instrument/1",
                    "instrument": {
                        "type": "deposit",
                        "spec": {
                            "id": "DEP-0",
                            "notional": {"amount": "1000000", "currency": "USD"},
                            "start_date": "2025-01-01",
                            "maturity": "2025-02-01",
                            "day_count": "act_360",
                            "discount_curve_id": "USD-OIS",
                            "attributes": {},
                        },
                    },
                },
            }
        ],
        "positions": [
            {
                "id": "position-0",
                "entity_id": "entity",
                "instrument_id": "DEP-0",
                "artifact_id": "artifact-0",
                "quantity": 1.0,
                "unit": "units",
            }
        ],
    })


def _risk_decomposition_payload() -> dict[str, object]:
    """Two factors, three position x factor rows, two residual rows."""
    return {
        "total_risk": 1.0,
        "measure": "variance",
        "factor_contributions": [
            {
                "factor_id": "credit::generic",
                "absolute_risk": 0.6,
                "relative_risk": 0.6,
                "marginal_risk": 0.3,
            },
            {
                "factor_id": "rates::USD",
                "absolute_risk": 0.4,
                "relative_risk": 0.4,
                "marginal_risk": 0.2,
            },
        ],
        "residual_risk": 0.0,
        "position_factor_contributions": [
            {
                "position_id": "P1",
                "factor_id": "credit::generic",
                "risk_contribution": 0.4,
            },
            {
                "position_id": "P1",
                "factor_id": "rates::USD",
                "risk_contribution": 0.3,
            },
            {
                "position_id": "P2",
                "factor_id": "credit::generic",
                "risk_contribution": 0.2,
            },
        ],
        "position_residual_contributions": [
            {
                "position_id": "P1",
                "residual_variance": 0.05,
                "source": {"kind": "from_credit_model", "issuer_id": "ACME"},
            },
            {
                "position_id": "P2",
                "residual_variance": 0.02,
                "source": {"kind": "other"},
            },
        ],
    }


def _risk_decomposition() -> RiskDecomposition:
    return RiskDecomposition.from_json(json.dumps(_risk_decomposition_payload()))


def _credit_model() -> CreditFactorModel:
    return CreditFactorModel.from_json(
        json.dumps({
            "schema": "finstack_quant.credit_factor_model/1",
            "as_of": "2024-03-29",
            "calibration_window": {"start": "2022-03-29", "end": "2024-03-29"},
            "policy": "globally_off",
            "generic_factor": {"name": "CDX IG", "series_id": "cdx.ig.5y"},
            "hierarchy": {"levels": ["rating", "region"]},
            "config": {
                "factors": [],
                "covariance": {"n": 0, "factor_ids": [], "data": []},
                "matching": {"mapping_table": []},
                "pricing_mode": "delta_based",
            },
            "issuer_betas": [],
            "anchor_state": {"pc": 0.0, "by_level": []},
            "static_correlation": {"factor_ids": [], "data": []},
            "vol_state": {"factors": {}, "idiosyncratic": {}},
            "factor_histories": None,
            "diagnostics": {
                "mode_counts": {},
                "bucket_sizes_per_level": [],
                "fold_ups": [],
                "r_squared_histogram": None,
                "tag_taxonomy": {},
            },
        })
    )


def _credit_vol_decomposition() -> RiskDecomposition:
    """Decomposition whose factor ids follow the credit naming convention."""
    return RiskDecomposition.from_json(
        json.dumps({
            "total_risk": 1.0,
            "measure": "variance",
            "factor_contributions": [
                {
                    "factor_id": "credit::generic",
                    "absolute_risk": 0.10,
                    "relative_risk": 0.10,
                    "marginal_risk": 0.0,
                },
                {
                    "factor_id": "credit::level0::rating::IG",
                    "absolute_risk": 0.20,
                    "relative_risk": 0.20,
                    "marginal_risk": 0.0,
                },
                {
                    "factor_id": "credit::level1::rating.region::IG.EU",
                    "absolute_risk": 0.30,
                    "relative_risk": 0.30,
                    "marginal_risk": 0.0,
                },
            ],
            "residual_risk": 0.40,
            "position_factor_contributions": [
                {
                    "position_id": "POS1",
                    "factor_id": "credit::generic",
                    "risk_contribution": 0.10,
                },
                {
                    "position_id": "POS1",
                    "factor_id": "credit::level0::rating::IG",
                    "risk_contribution": 0.20,
                },
            ],
            "position_residual_contributions": [],
        })
    )


# PortfolioAttribution


def test_portfolio_attribution_to_dataframe_is_single_row() -> None:
    """Flat factor totals collapse to exactly one row plus one currency column."""
    result = attribute_portfolio_pnl(
        Portfolio.from_spec(EMPTY_PORTFOLIO),
        MarketContext(),
        MarketContext(),
        "2025-01-01",
        "2025-01-02",
        "parallel",
    )
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "currency",
        "total_pnl",
        "carry",
        "rates_curves_pnl",
        "credit_curves_pnl",
        "inflation_curves_pnl",
        "correlations_pnl",
        "fx_pnl",
        "fx_translation_pnl",
        "cross_factor_pnl",
        "vol_pnl",
        "model_params_pnl",
        "market_scalars_pnl",
        "residual",
        "result_invalid",
    } == set(df.columns)
    # Money is flattened: a float column plus one shared currency column,
    # never a nested dict.
    assert df.iloc[0]["currency"] == "USD"
    assert df.iloc[0]["total_pnl"] == pytest.approx(result.total_pnl.amount)


# RiskDecomposition


def test_risk_decomposition_to_factor_dataframe() -> None:
    """One row per factor contribution, with the pinned column order.

    The column list is deliberately identical to
    ``FactorRiskDecomposition.to_factor_dataframe`` — both wrappers render the
    same Rust ``RiskDecomposition``, so they must not diverge.
    """
    df = _risk_decomposition().to_factor_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "factor_id",
        "absolute_risk",
        "relative_risk",
        "marginal_risk",
    ]
    assert len(df) == 2
    assert list(df["factor_id"]) == ["credit::generic", "rates::USD"]
    assert df["absolute_risk"].sum() == pytest.approx(1.0)


def test_risk_decomposition_to_position_factor_dataframe() -> None:
    """One row per position x factor pair, with the pinned column order."""
    df = _risk_decomposition().to_position_factor_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "factor_id", "risk_contribution"]
    assert len(df) == 3
    assert list(df["position_id"]) == ["P1", "P1", "P2"]


def test_risk_decomposition_to_position_residual_dataframe() -> None:
    """Residual rows flatten the tagged ``source`` enum into two columns."""
    df = _risk_decomposition().to_position_residual_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "residual_variance",
        "source_kind",
        "source_issuer_id",
    ]
    assert len(df) == 2
    assert list(df["source_kind"]) == ["from_credit_model", "other"]
    assert df.iloc[0]["source_issuer_id"] == "ACME"
    # A `None` issuer lands in a pandas string column, so it reads back as the
    # column's NA sentinel (NaN) rather than the Python `None` that was serialized.
    assert pd.isna(df.iloc[1]["source_issuer_id"])


def test_risk_decomposition_residual_dataframe_keeps_schema_when_empty() -> None:
    """No residual allocation still yields the documented columns."""
    payload = _risk_decomposition_payload()
    payload["position_residual_contributions"] = []
    df = RiskDecomposition.from_json(json.dumps(payload)).to_position_residual_dataframe()

    assert len(df) == 0
    assert "residual_variance" in df.columns
    assert "source_kind" in df.columns


# Position VaR / ES


def _position_risk_payload() -> dict[str, object]:
    return {
        "portfolio_var": -100.0,
        "portfolio_es": -140.0,
        "confidence": 0.95,
        "method": "parametric",
        "var_contributions": [
            {
                "position_id": "P1",
                "component_var": -60.0,
                "relative_var": 0.6,
                "marginal_var": -1.2,
                "incremental_var": None,
            },
            {
                "position_id": "P2",
                "component_var": -40.0,
                "relative_var": 0.4,
                "marginal_var": -0.8,
                "incremental_var": None,
            },
        ],
        "es_contributions": [
            {
                "position_id": "P1",
                "component_es": -84.0,
                "relative_es": 0.6,
                "marginal_es": -1.7,
            },
            {
                "position_id": "P2",
                "component_es": -56.0,
                "relative_es": 0.4,
                "marginal_es": None,
            },
        ],
        "n_positions": 2,
        "euler_residual": 0.0,
    }


def test_position_var_contribution_to_dataframe_is_single_row() -> None:
    """Leaf row type exposes a one-row frame for symmetry with its parent."""
    item = PositionVarContribution.from_json(
        json.dumps({
            "position_id": "P1",
            "component_var": -1.0,
            "relative_var": 1.0,
            "marginal_var": -1.0,
            "incremental_var": None,
        })
    )
    df = item.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "position_id",
        "component_var",
        "relative_var",
        "marginal_var",
        "incremental_var",
    } == set(df.columns)
    assert df.iloc[0]["position_id"] == "P1"


def test_position_risk_decomposition_joins_var_and_es() -> None:
    """VaR and ES share a position key space and land on the same row."""
    decomposition = PositionRiskDecomposition.from_json(json.dumps(_position_risk_payload()))
    df = decomposition.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "component_var",
        "relative_var",
        "marginal_var",
        "incremental_var",
        "component_es",
        "relative_es",
        "marginal_es",
    ]
    assert len(df) == len(decomposition.var_contributions) == 2

    p1 = df[df["position_id"] == "P1"].iloc[0]
    assert p1["component_var"] == pytest.approx(-60.0)
    assert p1["component_es"] == pytest.approx(-84.0)
    # Portfolio-level scalars are header metadata, not per-row columns.
    assert "portfolio_var" not in df.columns
    assert "confidence" not in df.columns


# Risk budget


def test_position_budget_entry_to_dataframe_is_single_row() -> None:
    item = PositionBudgetEntry.from_json(
        json.dumps({
            "position_id": "P1",
            "actual_component_var": 1.0,
            "target_component_var": 0.8,
            "utilization": 1.25,
            "excess": 0.2,
        })
    )
    df = item.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "position_id",
        "actual_component_var",
        "target_component_var",
        "utilization",
        "excess",
    } == set(df.columns)


def test_risk_budget_result_to_dataframe() -> None:
    """One row per budgeted position; the breach scalars stay off the rows."""
    result = RiskBudgetResult.from_json(
        json.dumps({
            "positions": [
                {
                    "position_id": "P1",
                    "actual_component_var": 1.0,
                    "target_component_var": 0.8,
                    "utilization": 1.25,
                    "excess": 0.2,
                },
                {
                    "position_id": "P2",
                    "actual_component_var": 0.4,
                    "target_component_var": 0.8,
                    "utilization": 0.5,
                    "excess": -0.4,
                },
            ],
            "total_overbudget": 0.2,
            "has_breach": True,
        })
    )
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "actual_component_var",
        "target_component_var",
        "utilization",
        "excess",
    ]
    assert len(df) == len(result.positions) == 2
    assert "has_breach" not in df.columns


# What-if


def test_what_if_result_to_dataframe_covers_delta_only() -> None:
    """Per-factor deltas become rows; before/after stay as nested wrappers."""
    risk = json.dumps(_risk_decomposition_payload())
    result = WhatIfResult.from_json(
        '{"before":' + risk + ',"after":' + risk + ',"delta":[{"factor_id":"rates::USD","absolute_change":0.1,'
        '"relative_change":0.05}]}'
    )
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["factor_id", "absolute_change", "relative_change"]
    assert len(df) == len(result.delta) == 1
    assert df.iloc[0]["factor_id"] == "rates::USD"


# Stress


def test_stress_result_to_dataframe() -> None:
    """One row per ``(position_id, pnl)`` entry."""
    risk = json.dumps(_risk_decomposition_payload())
    result = StressResult.from_json(
        '{"total_pnl":-12.0,"position_pnl":[["P1",-10.0],["P2",-2.0]],"stressed_decomposition":' + risk + "}"
    )
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "pnl"]
    assert len(df) == len(result.position_pnl) == 2
    assert df["pnl"].sum() == pytest.approx(result.total_pnl)


def test_tail_scenario_breakdown_to_dataframe_takes_parent_ids() -> None:
    """Ids live on the parent, so they are supplied as an argument."""
    breakdown = TailScenarioBreakdown.from_json(
        json.dumps({
            "scenario_index": 3,
            "portfolio_pnl": -12.0,
            "position_pnls": [-10.0, -2.0],
        })
    )
    df = breakdown.to_dataframe(["A", "B"])

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "pnl"]
    assert len(df) == len(breakdown.position_pnls) == 2
    assert list(df["position_id"]) == ["A", "B"]


def test_tail_scenario_breakdown_rejects_id_length_mismatch() -> None:
    """A wrong-length id list is a ValueError, never a silent mislabelling."""
    breakdown = TailScenarioBreakdown.from_json(
        json.dumps({
            "scenario_index": 0,
            "portfolio_pnl": -12.0,
            "position_pnls": [-10.0, -2.0],
        })
    )
    with pytest.raises(ValueError, match="position_ids length"):
        breakdown.to_dataframe(["A"])


def _stress_attribution() -> StressAttribution:
    position_ids = ["A", "B"]
    pnls_a = [-8.0, -2.0] + [0.5] * 38
    pnls_b = [-2.0, -4.0] + [0.5] * 38
    return build_stress_attribution(position_ids, [pnls_a, pnls_b], confidence=0.95)


def test_stress_attribution_to_dataframe() -> None:
    """One row per position contribution, largest driver first."""
    attribution = _stress_attribution()
    df = attribution.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "avg_tail_pnl",
        "pct_of_tail_loss",
        "worst_scenario_pnl",
    ]
    assert len(df) == len(attribution.position_contributions) == 2
    # pct_of_tail_loss is a fraction, not a percentage.
    assert df["pct_of_tail_loss"].sum() == pytest.approx(1.0)


def test_stress_attribution_to_scenario_dataframe_is_a_matrix() -> None:
    """Scenario x position matrix: positions as columns, scenario index as index."""
    attribution = _stress_attribution()
    df = attribution.to_scenario_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == attribution.position_ids
    assert len(df) == len(attribution.tail_scenarios) == 2
    assert list(df.index) == [s.scenario_index for s in attribution.tail_scenarios]
    assert df.loc[0, "A"] == pytest.approx(-8.0)
    assert df.loc[1, "B"] == pytest.approx(-4.0)


# Factor assignment


def _assignment_report() -> FactorAssignmentReport:
    return FactorAssignmentReport.from_json(
        json.dumps({
            "assignments": [
                {
                    "position_id": "P1",
                    "mappings": [
                        [
                            {"curve": {"id": "USD-OIS", "curve_type": "discount"}},
                            "rates::USD",
                            1.0,
                        ],
                        [
                            {"credit_curve": {"id": "ACME-CDS"}},
                            "credit::generic",
                            0.8,
                        ],
                    ],
                },
                {
                    "position_id": "P2",
                    "mappings": [
                        [
                            {"curve": {"id": "USD-OIS", "curve_type": "discount"}},
                            "rates::USD",
                            1.0,
                        ]
                    ],
                },
            ],
            "unmatched": [{"position_id": "P3", "dependency": {"spot": {"id": "AAPL"}}}],
        })
    )


def test_factor_assignment_report_to_dataframe_is_long_format() -> None:
    """Row count is total mappings, not the number of positions."""
    report = _assignment_report()
    df = report.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "factor_id", "beta", "dependency_json"]
    expected_rows = sum(a.n_mappings for a in report.assignments)
    assert len(df) == expected_rows == 3
    assert list(df["position_id"]) == ["P1", "P1", "P2"]
    # The MarketDependency variant tree stays a JSON string, as documented.
    assert json.loads(df.iloc[0]["dependency_json"])["curve"]["id"] == "USD-OIS"


def test_factor_assignment_report_to_unmatched_dataframe() -> None:
    report = _assignment_report()
    df = report.to_unmatched_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "dependency_json"]
    assert len(df) == len(report.unmatched) == 1
    assert json.loads(df.iloc[0]["dependency_json"])["spot"]["id"] == "AAPL"


# Credit vol report (the non-Serialize special case)


def test_credit_vol_report_to_level_dataframe() -> None:
    """One row per hierarchy level; columns built explicitly, not via serde."""
    report = build_credit_vol_report(_credit_vol_decomposition(), _credit_model())
    df = report.to_level_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["level_name", "total"]
    assert len(df) == len(report.by_level) == 2
    assert list(df["level_name"]) == ["Rating", "Region"]


def test_credit_vol_report_to_position_dataframe() -> None:
    report = build_credit_vol_report(_credit_vol_decomposition(), _credit_model(), by_position=True)
    df = report.to_position_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "factor_total", "idiosyncratic", "total"]
    assert report.by_position is not None
    assert len(df) == len(report.by_position) == 1
    assert df.iloc[0]["position_id"] == "POS1"


def test_credit_vol_report_position_dataframe_keeps_schema_when_absent() -> None:
    """``by_position=None`` must still yield the column schema, not an empty frame.

    ``LevelVolContribution`` / ``PositionVolContribution`` do not derive
    ``Serialize``, so the binding hand-rolls the zero-row-with-schema case that
    ``serde_rows_to_dataframe_with_schema`` would otherwise provide.
    """
    report = build_credit_vol_report(_credit_vol_decomposition(), _credit_model(), by_position=False)
    df = report.to_position_dataframe()

    assert report.by_position is None
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == ["position_id", "factor_total", "idiosyncratic", "total"]


# Optimization


def test_trade_spec_to_dataframe_is_single_row() -> None:
    trade = TradeSpec.from_json(
        json.dumps({
            "position_id": "P1",
            "instrument_id": "I1",
            "trade_type": "existing",
            "direction": "buy",
            "current_quantity": 1.0,
            "target_quantity": 2.0,
            "delta_quantity": 1.0,
            "current_weight": 0.1,
            "target_weight": 0.2,
        })
    )
    df = trade.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "position_id",
        "instrument_id",
        "trade_type",
        "current_quantity",
        "target_quantity",
        "delta_quantity",
        "direction",
        "current_weight",
        "target_weight",
    } == set(df.columns)
    assert df.iloc[0]["direction"] == "buy"


def _optimization_result() -> PortfolioOptimizationResult:
    objective = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
    spec = (
        PortfolioOptimizationSpec
        .new(_portfolio_json(), objective)
        .with_constraint(Constraint.weight_bounds(PositionFilter.all(), 0.0, 1.0, label="position_limits"))
        .with_weighting(WeightingScheme.value_weight())
        .with_missing_metric_policy(MissingMetricPolicy.zero())
        .with_label("dataframe_exits")
    )
    return optimize_portfolio(spec, _market())


def test_portfolio_optimization_result_to_dataframe_joins_weight_maps() -> None:
    """The four position-keyed maps join into one frame indexed by position."""
    result = _optimization_result()
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "current_weight",
        "optimal_weight",
        "weight_delta",
        "implied_quantity",
    ]
    expected = set(result.current_weights) | set(result.optimal_weights)
    expected |= set(result.weight_deltas) | set(result.implied_quantities)
    assert set(df.index) == expected
    assert len(df) == len(expected)


def test_portfolio_optimization_result_to_trade_dataframe() -> None:
    """Trade rows mirror ``to_trade_list`` one-for-one, schema always present."""
    result = _optimization_result()
    df = result.to_trade_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == len(result.to_trade_list())
    assert {
        "position_id",
        "instrument_id",
        "trade_type",
        "current_quantity",
        "target_quantity",
        "delta_quantity",
        "direction",
        "current_weight",
        "target_weight",
    } == set(df.columns)


# Portfolio runtime types


def test_portfolio_valuation_to_dataframe_matches_arrow_columns() -> None:
    """The pandas exit reuses the same table envelope as the Arrow exit."""
    valuation = value_portfolio(Portfolio.from_spec(_portfolio_json()), _market())
    df = valuation.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "entity_id",
        "value_native",
        "value_base",
        "currency_native",
        "currency_base",
    ]
    assert len(df) == len(valuation) == 1
    assert df.iloc[0]["position_id"] == "USD-POS"
    assert df.iloc[0]["currency_base"] == "USD"


def test_portfolio_cashflows_to_dataframe() -> None:
    """One row per dated event, with Money flattened into amount + currency."""
    cashflows = aggregate_full_cashflows(Portfolio.from_spec(_portfolio_json()), _market())
    df = cashflows.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == [
        "position_id",
        "instrument_id",
        "instrument_type",
        "date",
        "amount",
        "currency",
        "kind",
        "reset_date",
        "accrual_factor",
        "rate",
    ]
    assert len(df) == len(cashflows)
    if len(df) > 0:
        assert set(df["currency"]) == {"USD"}
        # Dates are ISO strings, never Rust date objects.
        assert date.fromisoformat(df.iloc[0]["date"]) >= date.fromisoformat(AS_OF)


def _portfolio_metrics() -> PortfolioMetrics:
    return PortfolioMetrics.from_json(
        json.dumps({
            "aggregated": {
                "pv_base": {"metric_id": "pv_base", "total": 300.0, "by_entity": {"FUND": 300.0}},
                "dv01": {"metric_id": "dv01", "total": -12.0, "by_entity": {"FUND": -12.0}},
            },
            "by_position": {
                "P1": {"currency": "USD", "metrics": {"pv_base": 200.0, "dv01": -8.0}},
                "P2": {"currency": "EUR", "metrics": {"pv_base": 100.0}},
            },
        })
    )


def test_portfolio_metrics_to_aggregated_dataframe() -> None:
    df = _portfolio_metrics().to_aggregated_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["metric_id", "total"]
    assert len(df) == 2
    assert list(df["metric_id"]) == ["pv_base", "dv01"]


def test_portfolio_metrics_to_position_dataframe_is_long_format() -> None:
    """Row count is (position, metric) pairs, and currency travels with them."""
    df = _portfolio_metrics().to_position_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["position_id", "currency", "metric_id", "value"]
    assert len(df) == 3
    assert list(df["position_id"]) == ["P1", "P1", "P2"]
    assert df[df["position_id"] == "P2"].iloc[0]["currency"] == "EUR"


def test_materialization_report_to_dataframe_is_single_row() -> None:
    """Flat counters plus flattened phase timings on one row."""
    _, report = Portfolio.from_materialization(_materialization_bundle())
    df = report.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert {
        "unique_instruments",
        "positions",
        "dependencies",
        "cache_hits",
        "input_bytes",
        "truncated",
        "timing_available",
        "phase_parse_nanos",
        "phase_validate_versions_nanos",
        "phase_decode_instruments_nanos",
        "phase_build_positions_nanos",
        "phase_index_build_nanos",
    } == set(df.columns)
    assert df.iloc[0]["positions"] == report.positions == 1
