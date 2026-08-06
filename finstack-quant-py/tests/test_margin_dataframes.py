"""Tests for the `margin` pandas ``DataFrame`` accessors.

Covers the newly added exports: `VmResult.to_dataframe`,
`ImResult.to_dataframe` / `to_breakdown_dataframe`, the three metric types,
and the two long-format sensitivity exports
(`FrtbSensitivities` / `SimmSensitivities`).

Everything is built through public constructors and calculators, so the tests
stay self-contained.
"""

from __future__ import annotations

import pandas as pd
import pytest

from finstack_quant.margin import (
    CsaSpec,
    ExcessCollateral,
    FrtbSensitivities,
    ImResult,
    MarginFundingCost,
    MarginUtilization,
    ScheduleImCalculator,
    SimmCalculator,
    SimmSensitivities,
    VmCalculator,
    VmResult,
)

SENSITIVITY_COLUMNS = ["risk_class", "bucket", "tenor", "issuer", "kind", "amount"]


def _sort_keys(df: pd.DataFrame) -> list[tuple[object, ...]]:
    """Reproduce the Rust row ordering key for the long-format exports.

    Rust sorts on ``(risk_class, kind, issuer, bucket, tenor)`` where the last
    three are ``Option<String>`` and ``None`` sorts before any value. Missing
    values are therefore mapped to a ``(0, "")`` prefix rather than compared as
    the literal string ``"None"``.
    """

    def optional(value: object) -> tuple[int, str]:
        return (1, str(value)) if pd.notna(value) else (0, "")

    return [
        (
            str(row["risk_class"]),
            str(row["kind"]),
            optional(row["issuer"]),
            optional(row["bucket"]),
            optional(row["tenor"]),
        )
        for _, row in df.iterrows()
    ]


# VmResult


def _vm_result() -> VmResult:
    csa = CsaSpec.usd_regulatory()
    return VmCalculator(csa).calculate(1_000_000.0, 250_000.0, "USD", 2025, 6, 30)


def test_vm_result_to_dataframe_is_one_row() -> None:
    result = _vm_result()
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "gross_exposure",
        "net_exposure",
        "delivery_amount",
        "return_amount",
        "net_margin",
        "requires_call",
        "currency",
    ]
    row = df.iloc[0]
    assert row["currency"] == "USD"
    assert row["gross_exposure"] == pytest.approx(result.gross_exposure)
    assert row["delivery_amount"] == pytest.approx(result.delivery_amount)
    assert bool(row["requires_call"]) == result.requires_call


# ImResult


def _im_result() -> ImResult:
    sensitivities = SimmSensitivities("USD")
    sensitivities.add_ir_delta("USD", "5Y", 100_000.0)
    sensitivities.add_equity_delta("AAPL", 250_000.0)
    sensitivities.add_fx_delta("EUR", 75_000.0)
    calculator = SimmCalculator("v2_6")
    return calculator.calculate_from_sensitivities(sensitivities, "USD", 2025, 6, 30)


def test_im_result_to_dataframe_is_one_row() -> None:
    result = _im_result()
    df = result.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "amount",
        "currency",
        "methodology",
        "mpor_days",
        "as_of",
        "approximation",
    ]
    row = df.iloc[0]
    assert row["amount"] == pytest.approx(result.amount)
    assert row["currency"] == "USD"
    assert row["as_of"] == "2025-06-30"
    assert row["mpor_days"] == result.mpor_days
    assert bool(row["approximation"]) == result.approximation


def test_im_result_breakdown_dataframe_is_sorted_by_risk_class() -> None:
    result = _im_result()
    df = result.to_breakdown_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert list(df.columns) == ["risk_class", "amount", "currency"]
    assert len(df) == len(result.breakdown_keys())
    if len(df):
        risk_classes = list(df["risk_class"])
        assert risk_classes == sorted(risk_classes), "breakdown must be deterministic"
        assert set(risk_classes) == set(result.breakdown_keys())
        first = risk_classes[0]
        assert df.iloc[0]["amount"] == pytest.approx(result.breakdown_amount(first))
        assert set(df["currency"]) == {"USD"}


def test_im_result_breakdown_dataframe_keeps_schema_when_empty() -> None:
    """A methodology with no published breakdown still yields all columns."""
    result = ScheduleImCalculator.bcbs_standard().calculate_for_notional(
        10_000_000.0, "USD", "interest_rate", 7.0, 2025, 6, 30
    )
    df = result.to_breakdown_dataframe()
    assert list(df.columns) == ["risk_class", "amount", "currency"]
    assert len(df) == len(result.breakdown_keys())


# Metrics


def test_margin_utilization_to_dataframe() -> None:
    metric = MarginUtilization(800_000.0, 1_000_000.0, "USD")
    df = metric.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "posted",
        "required",
        "ratio",
        "shortfall",
        "is_adequate",
        "currency",
    ]
    row = df.iloc[0]
    assert row["ratio"] == pytest.approx(0.8)
    assert row["currency"] == "USD"
    assert bool(row["is_adequate"]) is False


def test_excess_collateral_to_dataframe() -> None:
    metric = ExcessCollateral(1_200_000.0, 1_000_000.0, "USD")
    df = metric.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "collateral_value",
        "required_value",
        "excess",
        "excess_percentage",
        "has_excess",
        "has_shortfall",
        "currency",
    ]
    row = df.iloc[0]
    assert row["excess"] == pytest.approx(200_000.0)
    assert row["excess_percentage"] == pytest.approx(0.2)
    assert bool(row["has_excess"]) is True
    assert bool(row["has_shortfall"]) is False


def test_margin_funding_cost_to_dataframe() -> None:
    metric = MarginFundingCost(1_000_000.0, 0.05, 0.03, "USD")
    df = metric.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert len(df) == 1
    assert list(df.columns) == [
        "margin_posted",
        "funding_rate",
        "collateral_rate",
        "spread",
        "annual_cost",
        "currency",
    ]
    row = df.iloc[0]
    assert row["spread"] == pytest.approx(0.02)
    assert row["annual_cost"] == pytest.approx(20_000.0)
    assert row["currency"] == "USD"


# FrtbSensitivities long format


def test_frtb_sensitivities_to_dataframe_keeps_schema_when_empty() -> None:
    df = FrtbSensitivities("USD").to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == SENSITIVITY_COLUMNS


def test_frtb_sensitivities_to_dataframe_is_long_and_sorted() -> None:
    sens = FrtbSensitivities("USD")
    sens.add_girr_delta("5Y", 12_000.0)
    sens.add_girr_delta("2Y", 8_000.0)
    sens.add_csr_delta("ACME", 3, "5Y", 4_000.0)
    sens.add_equity_delta("AAPL", 1, 25_000.0)
    sens.add_fx_delta("EUR", "USD", 9_000.0)
    sens.add_girr_curvature(500.0, -400.0)
    sens.add_rrao_position("EXOTIC-1", 5_000_000.0, True)

    df = sens.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert set(SENSITIVITY_COLUMNS) <= set(df.columns)
    # 2 GIRR deltas + 1 CSR + 1 equity + 1 FX + 2 curvature halves + 1 RRAO.
    assert len(df) == 8

    keys = _sort_keys(df)
    assert keys == sorted(keys), "rows must be sorted by their key columns"

    girr_delta = df[(df["risk_class"] == "girr") & (df["kind"] == "delta")]
    assert len(girr_delta) == 2
    assert set(girr_delta["tenor"]) == {"2Y", "5Y"}
    assert set(girr_delta["issuer"]) == {"USD"}

    curvature = df[df["kind"].isin(["curvature_up", "curvature_down"])]
    assert len(curvature) == 2
    assert set(curvature["kind"]) == {"curvature_up", "curvature_down"}

    fx = df[df["risk_class"] == "fx"]
    assert list(fx["issuer"]) == ["EUR/USD"]

    csr = df[df["risk_class"] == "csr_non_sec"]
    assert list(csr["bucket"]) == ["3"], "bucket is exported as a string label"

    rrao = df[df["risk_class"] == "rrao"]
    assert list(rrao["kind"]) == ["exotic_notional"]
    assert rrao.iloc[0]["amount"] == pytest.approx(5_000_000.0)


def test_frtb_sensitivities_to_dataframe_is_stable_across_calls() -> None:
    sens = FrtbSensitivities("USD")
    for tenor in ("1Y", "2Y", "5Y", "10Y", "30Y"):
        sens.add_girr_delta(tenor, 1_000.0)
        sens.add_equity_delta(f"NAME_{tenor}", 1, 2_000.0)

    first = sens.to_dataframe()
    second = sens.to_dataframe()
    pd.testing.assert_frame_equal(first, second)


# SimmSensitivities long format


def test_simm_sensitivities_to_dataframe_keeps_schema_when_empty() -> None:
    df = SimmSensitivities("USD").to_dataframe()
    assert isinstance(df, pd.DataFrame)
    assert len(df) == 0
    assert list(df.columns) == SENSITIVITY_COLUMNS


def test_simm_sensitivities_to_dataframe_is_long_and_sorted() -> None:
    sens = SimmSensitivities("USD")
    sens.add_ir_delta("USD", "5Y", 100_000.0)
    sens.add_ir_delta("EUR", "2Y", 50_000.0)
    sens.add_ir_vega("USD", "5Y", 10_000.0)
    sens.add_credit_delta("ACME", True, "5Y", 20_000.0)
    sens.add_equity_delta("AAPL", 250_000.0)
    sens.add_fx_delta("EUR", 75_000.0)
    sens.add_commodity_delta("crude", 30_000.0)
    sens.add_curvature("interest_rate", 5_000.0)

    df = sens.to_dataframe()

    assert isinstance(df, pd.DataFrame)
    assert set(SENSITIVITY_COLUMNS) <= set(df.columns)
    assert len(df) == 8

    keys = _sort_keys(df)
    assert keys == sorted(keys), "rows must be sorted by their key columns"

    ir = df[df["risk_class"] == "interest_rate"]
    assert set(ir["kind"]) == {"delta", "vega", "curvature"}

    ir_delta = ir[ir["kind"] == "delta"]
    assert set(ir_delta["issuer"]) == {"USD", "EUR"}
    assert set(ir_delta["tenor"]) == {"5Y", "2Y"}

    commodity = df[df["risk_class"] == "commodity"]
    assert list(commodity["bucket"]) == ["crude"]
    assert commodity["issuer"].isna().all(), "commodity has no name axis"

    credit = df[df["risk_class"] == "credit_qualifying"]
    assert list(credit["issuer"]) == ["ACME"]
    assert credit["bucket"].isna().all(), "unbucketed credit has no sector"


def test_simm_sensitivities_bucketed_credit_carries_sector_label() -> None:
    sens = SimmSensitivities("USD")
    sens.add_credit_delta_bucketed("sovereign", "GOVT_A", "5Y", 40_000.0)
    df = sens.to_dataframe()

    assert len(df) == 1
    row = df.iloc[0]
    assert row["risk_class"] == "credit_qualifying"
    assert row["bucket"] == "sovereign"
    assert row["issuer"] == "GOVT_A"
    assert row["tenor"] == "5Y"
    assert row["amount"] == pytest.approx(40_000.0)


def test_simm_sensitivities_to_dataframe_is_stable_across_calls() -> None:
    sens = SimmSensitivities("USD")
    for tenor in ("1Y", "2Y", "5Y", "10Y", "30Y"):
        sens.add_ir_delta("USD", tenor, 1_000.0)
        sens.add_equity_delta(f"NAME_{tenor}", 2_000.0)

    first = sens.to_dataframe()
    second = sens.to_dataframe()
    pd.testing.assert_frame_equal(first, second)
