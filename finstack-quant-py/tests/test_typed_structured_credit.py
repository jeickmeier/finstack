"""Typed structured-credit spec models and StructuredCredit deal."""

from __future__ import annotations

import datetime
import json
from pathlib import Path

import pytest

from finstack_quant.core.currency import Currency
from finstack_quant.core.market_data import (
    DiscountCurve,
    ForwardCurve,
    MarketContext,
    ScalarTimeSeries,
)
from finstack_quant.core.money import Money
from finstack_quant.valuations.instruments import (
    AssetPool,
    StructuredCredit,
    Tranche,
)
from tests.tests_typed_helpers import structured_credit_pool as _pool, structured_credit_tranches as _tranches

# Fixture path + helpers copied from test_structured_credit_bindings.py so the
# typed golden below exercises the exact same deal/market fixtures as the
# JSON-path tests.
FIXTURE = (
    Path(__file__).resolve().parents[2]
    / "finstack-quant"
    / "valuations"
    / "tests"
    / "instruments"
    / "json_examples"
    / "structured_credit_full.json"
)


def _valid_deal_json() -> str:
    instrument = json.loads(FIXTURE.read_text())["instrument"]
    instrument["spec"]["payment_calendar_id"] = "nyse"
    return json.dumps(instrument)


def _metrics_tranche_id(deal_json: str) -> str:
    """Return a tranche id whose z-spread solve is well-posed.

    ``spec["tranches"]["tranches"][0]`` is the fixture's equity tranche,
    whose near-zero residual cashflows make the z-spread bracket solve in
    ``structured_credit_tranche_metrics`` degenerate (``f(a) == f(b) == 0``,
    no sign change) — a pre-existing Rust-side numerical property of that
    tranche, unrelated to typed-vs-JSON parity. Picking the first non-equity
    tranche lands on "SENIOR", the same tranche
    ``test_structured_credit_bindings.py`` exercises for this call.
    """
    tranches = json.loads(deal_json)["spec"]["tranches"]["tranches"]
    for tranche in tranches:
        if tranche.get("seniority") != "Equity":
            return tranche["id"]
    return tranches[0]["id"]


def _market() -> MarketContext:
    as_of = datetime.date(2024, 1, 1)
    market = (
        MarketContext()
        .insert(DiscountCurve.flat("USD-SOFR-DISC", as_of, 0.04))
        .insert(
            ForwardCurve(
                "SOFR-3M",
                0.25,
                [(0.0, 0.04), (10.0, 0.04)],
                as_of,
                day_count="act_360",
            )
        )
    )
    market.insert_series(ScalarTimeSeries("FIXING:SOFR-3M", [(datetime.date(2023, 12, 28), 0.04)]))
    return market


class TestStructuredCreditTyped:
    def test_new_abs_round_trips(self) -> None:
        deal = StructuredCredit.new_abs(
            "ABS-1",
            _pool(),
            _tranches(),
            datetime.date(2024, 1, 15),
            datetime.date(2031, 1, 15),
            "USD-SOFR-DISC",
        )
        payload = json.loads(deal.to_json())
        assert payload["type"] == "structured_credit"
        # DealType has no `#[serde(rename_all)]` in Rust, so the wire value is
        # the exact PascalCase/acronym variant name ("ABS"), and that is also
        # exactly what the typed Python constructors accept (see
        # `test_wire_casing_round_trips_without_translation` below).
        assert payload["spec"]["deal_type"] == "ABS"
        assert StructuredCredit.from_json(deal.to_json()).id == "ABS-1"

    def test_tranche_builder_validates_attach_detach(self) -> None:
        with pytest.raises(ValueError, match="Invalid"):
            (
                Tranche
                .builder()
                .id("BAD")
                .attachment_point(50.0)
                .detachment_point(10.0)
                .seniority("Senior")
                .original_balance(Money(1.0, Currency("USD")))
                .coupon_fixed(0.05)
                .maturity(datetime.date(2031, 1, 15))
                .build()
            )

    def test_deal_type_string_is_validated(self) -> None:
        with pytest.raises(ValueError, match="invalid deal_type"):
            AssetPool("POOL-X", "cdo_squared", Currency("USD"))

    def test_wire_casing_round_trips_without_translation(self) -> None:
        """The typed surface must accept exactly the strings `to_json()` emits.

        `DealType`/`TrancheSeniority` have no `#[serde(rename_all)]` in Rust,
        so their wire form is the literal PascalCase/acronym variant name
        ("ABS", "Senior", ...). This asserts the round trip end to end: build
        a deal, serialize it, pull the `deal_type` and a tranche `seniority`
        straight out of that JSON, and feed those exact strings back into the
        typed constructors. If a lowercase-translation shim like the old
        `deal_type_from_str`/`seniority_from_str` helpers were reintroduced,
        this would fail because the JSON values ("ABS", "Senior") no longer
        match the shim's lowercase-only vocabulary.
        """
        deal = StructuredCredit.new_abs(
            "ABS-1",
            _pool(),
            _tranches(),
            datetime.date(2024, 1, 15),
            datetime.date(2031, 1, 15),
            "USD-SOFR-DISC",
        )
        payload = json.loads(deal.to_json())
        deal_type_wire = payload["spec"]["deal_type"]
        tranche_seniority_wire = payload["spec"]["tranches"]["tranches"][0]["seniority"]

        # Feed the exact wire strings straight back into the typed
        # constructors: this must succeed without any casing translation.
        AssetPool("POOL-RT", deal_type_wire, Currency("USD"))
        Tranche.builder().seniority(tranche_seniority_wire)
        StructuredCredit.builder().deal_type(deal_type_wire)

    def test_all_deal_type_literal_values_accepted(self) -> None:
        for value in ("CLO", "CBO", "ABS", "RMBS", "CMBS", "Auto", "Card"):
            AssetPool("POOL-DT", value, Currency("USD"))
            StructuredCredit.builder().deal_type(value)

    def test_all_seniority_literal_values_accepted(self) -> None:
        for value in ("Senior", "Mezzanine", "Subordinated", "Equity"):
            Tranche.builder().seniority(value)

    def test_lowercase_deal_type_is_rejected(self) -> None:
        with pytest.raises(ValueError, match="invalid deal_type"):
            AssetPool("POOL-LC", "abs", Currency("USD"))
        with pytest.raises(ValueError, match="invalid deal_type"):
            StructuredCredit.builder().deal_type("abs")

    def test_lowercase_seniority_is_rejected(self) -> None:
        with pytest.raises(ValueError, match="invalid seniority"):
            Tranche.builder().seniority("senior")

    def test_from_json_accepts_repo_fixture(self) -> None:
        from pathlib import Path

        fixture = (
            Path(__file__).resolve().parents[2]
            / "finstack-quant"
            / "valuations"
            / "tests"
            / "instruments"
            / "json_examples"
            / "structured_credit_full.json"
        )
        instrument = json.loads(fixture.read_text())["instrument"]
        deal = StructuredCredit.from_json(json.dumps(instrument))
        assert json.loads(deal.to_json())["type"] == "structured_credit"


class TestStructuredCreditTypedAnalytics:
    def test_typed_metrics_equal_json_metrics_on_fixture(self) -> None:
        """Golden: typed deal in == JSON deal in, identical TrancheMetrics JSON."""
        from finstack_quant.valuations.instruments import (
            StructuredCredit,
            TrancheMetrics,
            structured_credit_tranche_metrics,
        )

        deal_json = _valid_deal_json()
        tranche_id = _metrics_tranche_id(deal_json)
        market = _market()

        via_json = structured_credit_tranche_metrics(deal_json, tranche_id, market, "2024-01-01")
        typed_deal = StructuredCredit.from_json(deal_json)
        via_typed = structured_credit_tranche_metrics(typed_deal, tranche_id, market, "2024-01-01")
        assert json.loads(via_typed) == json.loads(via_json)

        metrics = TrancheMetrics.from_json(via_typed)
        assert metrics.tranche_id == tranche_id

        # Field-by-field with pytest.approx rather than exact string/dict
        # equality: serde_json's default (non-`float_roundtrip`) float parser
        # is not always bit-exact on reparse, so a few of this fixture's
        # values shift by 1 ULP across a parse -> reserialize cycle. That is
        # an unrelated, pre-existing serde_json characteristic (present for
        # every f64-bearing struct in this workspace) rather than anything
        # about typed-vs-JSON parity, which the assert above already proved.
        decoded = json.loads(via_typed)
        reencoded = json.loads(metrics.to_json())
        assert reencoded.keys() == decoded.keys()
        for key, expected in decoded.items():
            if isinstance(expected, float):
                assert reencoded[key] == pytest.approx(expected)
            else:
                assert reencoded[key] == expected

    def test_oas_result_wrapper_round_trips(self) -> None:
        from finstack_quant.valuations.instruments import OasResult

        payload = json.dumps({
            "oas": 0.0125,
            "model_price": 99.5,
            "market_price": 98.75,
            "num_paths": 256,
            "price_std_error": 0.05,
        })
        result = OasResult.from_json(payload)
        assert result.oas == pytest.approx(0.0125)
        assert json.loads(result.to_json()) == json.loads(payload)

    def test_scenario_table_wrapper_exposes_cells(self) -> None:
        from finstack_quant.valuations.instruments import ScenarioTable

        payload = json.dumps({
            "tranche_id": "A",
            "cells": [
                {
                    "cpr": 0.06,
                    "cdr": 0.02,
                    "severity": 0.6,
                    "price": 98.2,
                    "wal": 4.1,
                    "writedown": 0.0,
                }
            ],
        })
        table = ScenarioTable.from_json(payload)
        assert table.tranche_id == "A"
        assert table.cells()[0]["price"] == pytest.approx(98.2)
