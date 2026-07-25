"""Typed FxForward / FxOption bindings."""

from __future__ import annotations

import datetime
import json

import pytest

from finstack_quant.core.currency import Currency
from finstack_quant.core.money import Money
from finstack_quant.valuations.instruments import FxForward, FxOption
from tests.tests_typed_helpers import build_fx_forward as _forward


class TestFxForwardTyped:
    def test_tagged_json_and_round_trip(self) -> None:
        payload = json.loads(_forward().to_json())
        assert payload["type"] == "fx_forward"
        assert FxForward.from_json(_forward().to_json()).id == "EURUSD-FWD-6M"

    def test_same_base_and_quote_currency_rejected(self) -> None:
        with pytest.raises(ValueError, match="must differ from quote_currency"):
            (
                FxForward
                .builder()
                .id("BAD")
                .base_currency(Currency("USD"))
                .quote_currency(Currency("USD"))
                .maturity(datetime.date(2025, 6, 15))
                .notional(Money(1.0, Currency("USD")))
                .domestic_discount_curve_id("USD-OIS")
                .foreign_discount_curve_id("USD-OIS")
                .build()
            )


class TestFxOptionTyped:
    def test_builder_round_trip(self) -> None:
        option = (
            FxOption
            .builder()
            .id("EURUSD-CALL-1Y")
            .base_currency(Currency("EUR"))
            .quote_currency(Currency("USD"))
            .strike(1.12)
            .option_type("call")
            .expiry(datetime.date(2025, 12, 15))
            .notional(Money(1_000_000.0, Currency("EUR")))
            .domestic_discount_curve_id("USD-OIS")
            .foreign_discount_curve_id("EUR-OIS")
            .vol_surface_id("EURUSD-VOL")
            .build()
        )
        payload = json.loads(option.to_json())
        assert payload["type"] == "fx_option"
        assert FxOption.from_json(option.to_json()).id == "EURUSD-CALL-1Y"

    def test_invalid_option_type_raises(self) -> None:
        with pytest.raises(ValueError, match="invalid option_type"):
            FxOption.builder().option_type("straddle")

    def test_exercise_style_wrong_case_is_rejected(self) -> None:
        with pytest.raises(ValueError, match="invalid exercise_style"):
            FxOption.builder().exercise_style("European")

    @pytest.mark.parametrize("value", ["european", "american", "bermudan"])
    def test_every_exercise_style_literal_value_accepted(self, value: str) -> None:
        """Every value in the `exercise_style` stub Literal must be a real accepted wire value.

        Only "european" is currently priceable (see
        `finstack_quant_valuations::instruments::fx::fx_option::pricer::validate_exercise_style`);
        "american"/"bermudan" are still accepted by the builder itself.
        """
        option = (
            FxOption
            .builder()
            .id("EURUSD-EX")
            .base_currency(Currency("EUR"))
            .quote_currency(Currency("USD"))
            .strike(1.12)
            .option_type("call")
            .exercise_style(value)
            .expiry(datetime.date(2025, 12, 15))
            .notional(Money(1_000_000.0, Currency("EUR")))
            .domestic_discount_curve_id("USD-OIS")
            .foreign_discount_curve_id("EUR-OIS")
            .vol_surface_id("EURUSD-VOL")
            .build()
        )
        assert option.id == "EURUSD-EX"
