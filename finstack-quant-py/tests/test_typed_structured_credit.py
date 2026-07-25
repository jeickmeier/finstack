"""Typed structured-credit spec models and StructuredCredit deal."""

from __future__ import annotations

import datetime
import json

import pytest

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import DayCount
from finstack_quant.core.money import Money
from finstack_quant.valuations.instruments import (
    AssetPool,
    RepLine,
    StructuredCredit,
    Tranche,
    TrancheStructure,
)


def _pool() -> AssetPool:
    pool = AssetPool("POOL-1", "ABS", Currency("USD"))
    return pool.with_rep_lines([
        RepLine(
            "LINE-1",
            Money(80_000_000.0, Currency("USD")),
            0.07,
            datetime.date(2031, 1, 15),
            12,
            DayCount.ACT_360,
            cpr=0.10,
            cdr=0.02,
            recovery_rate=0.45,
        )
    ])


def _tranches() -> TrancheStructure:
    senior = (
        Tranche
        .builder()
        .id("A")
        .attachment_point(10.0)
        .detachment_point(100.0)
        .seniority("Senior")
        .original_balance(Money(72_000_000.0, Currency("USD")))
        .coupon_fixed(0.05)
        .maturity(datetime.date(2031, 1, 15))
        .build()
    )
    equity = (
        Tranche
        .builder()
        .id("E")
        .attachment_point(0.0)
        .detachment_point(10.0)
        .seniority("Equity")
        .original_balance(Money(8_000_000.0, Currency("USD")))
        .coupon_fixed(0.0)
        .maturity(datetime.date(2031, 1, 15))
        .build()
    )
    return TrancheStructure([senior, equity])


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
