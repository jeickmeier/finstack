"""Typed InterestRateSwap / FixedLegSpec / FloatLegSpec bindings."""

from __future__ import annotations

import datetime
import json

import pytest

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import DayCount, Tenor
from finstack_quant.core.money import Money
from finstack_quant.valuations.instruments import (
    FixedLegSpec,
    FloatLegSpec,
    InterestRateSwap,
    TermLoan,
    price_instrument,
)

# Every serde wire value of the Rust `BusinessDayConvention` enum
# (`#[serde(rename_all = "snake_case")]` over Unadjusted/Following/
# ModifiedFollowing/Preceding/ModifiedPreceding). Kept in sync manually with
# `finstack-quant/core/src/dates/calendar/business_days.rs`; a stub `Literal`
# that drifts from this set is exactly the bug this test guards against.
_VALID_BDC_VALUES = (
    "unadjusted",
    "following",
    "modified_following",
    "preceding",
    "modified_preceding",
)


def _market_json() -> str:
    return json.dumps({
        "version": 2,
        "curves": [
            {
                "type": "discount",
                "id": "USD-OIS",
                "base": "2024-01-01",
                "day_count": "Act360",
                "knot_points": [[0.0, 1.0], [5.0, 0.90], [10.0, 0.80]],
                "interp_style": "monotone_convex",
                "extrapolation": "flat_forward",
                "min_forward_rate": None,
                "allow_non_monotonic": False,
                "min_forward_tenor": 1e-6,
            },
            {
                "type": "forward",
                "id": "USD-SOFR-3M",
                "base": "2024-01-01",
                "reset_lag": 2,
                "day_count": "Act360",
                "tenor": 0.25,
                "knot_points": [[0.0, 0.04], [10.0, 0.045]],
                "interp_style": "linear",
                "extrapolation": "flat_forward",
            },
        ],
        "fx": None,
        "surfaces": [],
        "prices": {},
        "series": [],
        "inflation_indices": [],
        "dividends": [],
        "credit_indices": [],
        "fx_delta_vol_surfaces": [],
        "vol_cubes": [],
        "collateral": {},
    })


def _payer_swap() -> InterestRateSwap:
    start = datetime.date(2024, 1, 15)
    end = datetime.date(2029, 1, 15)
    fixed = FixedLegSpec("USD-OIS", 0.04, Tenor.semi_annual(), DayCount.THIRTY_360, start, end)
    float_leg = FloatLegSpec("USD-OIS", "USD-SOFR-3M", 0.0, Tenor.quarterly(), DayCount.ACT_360, start, end)
    return (
        InterestRateSwap
        .builder()
        .id("IRS-1")
        .notional(Money(10_000_000.0, Currency("USD")))
        .side("pay")
        .fixed(fixed)
        .float(float_leg)
        .build()
    )


def _without_timestamp(result_json: str) -> dict[str, object]:
    parsed = json.loads(result_json)
    parsed["meta"].pop("timestamp", None)
    return parsed


class TestInterestRateSwapTyped:
    def test_builder_produces_swap_with_id(self) -> None:
        swap = _payer_swap()
        assert swap.id == "IRS-1"
        assert "IRS-1" in repr(swap)

    def test_to_json_is_tagged(self) -> None:
        payload = json.loads(_payer_swap().to_json())
        assert payload["type"] == "interest_rate_swap"
        assert payload["spec"]["id"] == "IRS-1"

    def test_from_json_round_trip(self) -> None:
        original = _payer_swap().to_json()
        assert json.loads(InterestRateSwap.from_json(original).to_json()) == json.loads(original)

    def test_from_json_rejects_wrong_type(self) -> None:
        with pytest.raises(ValueError, match="interest_rate_swap"):
            InterestRateSwap.from_json(TermLoan.example().to_json())

    def test_builder_missing_required_field_raises(self) -> None:
        with pytest.raises(ValueError, match="Invalid input data"):
            InterestRateSwap.builder().id("IRS-BAD").build()

    def test_invalid_side_raises_value_error(self) -> None:
        with pytest.raises(ValueError, match="invalid side"):
            InterestRateSwap.builder().side("sideways")

    def test_golden_typed_pv_equals_json_pv(self) -> None:
        """Payer IRS: typed path and JSON path produce identical ValuationResult."""
        swap = _payer_swap()
        typed = price_instrument(swap, _market_json(), "2024-01-01", "discounting")
        via_json = price_instrument(swap.to_json(), _market_json(), "2024-01-01", "discounting")
        assert _without_timestamp(typed) == _without_timestamp(via_json)

    def test_builder_setters_accept_keyword_value(self) -> None:
        """Every builder setter's `value` parameter name must match its text_signature."""
        start = datetime.date(2024, 1, 15)
        end = datetime.date(2029, 1, 15)
        fixed = FixedLegSpec("USD-OIS", 0.04, Tenor.semi_annual(), DayCount.THIRTY_360, start, end)
        float_leg = FloatLegSpec("USD-OIS", "USD-SOFR-3M", 0.0, Tenor.quarterly(), DayCount.ACT_360, start, end)
        swap = (
            InterestRateSwap
            .builder()
            .id(value="IRS-KW")
            .notional(value=Money(10_000_000.0, Currency("USD")))
            .side(value="pay")
            .fixed(value=fixed)
            .float(value=float_leg)
            .build()
        )
        assert swap.id == "IRS-KW"


class TestFixedLegSpecTyped:
    def test_keyword_arguments(self) -> None:
        """Every keyword-only parameter name must match its text_signature."""
        leg = FixedLegSpec(
            discount_curve_id="USD-OIS",
            rate=0.04,
            frequency=Tenor.semi_annual(),
            day_count=DayCount.THIRTY_360,
            start=datetime.date(2024, 1, 15),
            end=datetime.date(2029, 1, 15),
            bdc="modified_following",
            calendar_id=None,
            stub="ShortFront",
            compounding_simple=False,
            payment_lag_days=0,
            end_of_month=False,
        )
        assert "0.04" in repr(leg)

    @pytest.mark.parametrize("bdc", _VALID_BDC_VALUES)
    def test_every_bdc_literal_value_accepted(self, bdc: str) -> None:
        """Every value in the `bdc` stub Literal must be a real accepted wire value."""
        leg = FixedLegSpec(
            "USD-OIS",
            0.04,
            Tenor.semi_annual(),
            DayCount.THIRTY_360,
            datetime.date(2024, 1, 15),
            datetime.date(2029, 1, 15),
            bdc=bdc,
            compounding_simple=False,
        )
        assert "0.04" in repr(leg)


class TestFloatLegSpecTyped:
    @pytest.mark.parametrize("bdc", _VALID_BDC_VALUES)
    def test_every_bdc_literal_value_accepted(self, bdc: str) -> None:
        """Every value in the `bdc` stub Literal must be a real accepted wire value."""
        leg = FloatLegSpec(
            "USD-OIS",
            "USD-SOFR-3M",
            0.0,
            Tenor.quarterly(),
            DayCount.ACT_360,
            datetime.date(2024, 1, 15),
            datetime.date(2029, 1, 15),
            bdc=bdc,
        )
        assert "spread_bp=0" in repr(leg)

    def test_keyword_arguments(self) -> None:
        """Every keyword-only parameter name must match its text_signature."""
        leg = FloatLegSpec(
            discount_curve_id="USD-OIS",
            forward_curve_id="USD-SOFR-3M",
            spread_bp=0.0,
            frequency=Tenor.quarterly(),
            day_count=DayCount.ACT_360,
            start=datetime.date(2024, 1, 15),
            end=datetime.date(2029, 1, 15),
            bdc="modified_following",
            calendar_id=None,
            stub="ShortFront",
            reset_lag_days=-1,
            fixing_calendar_id=None,
            payment_lag_days=0,
            end_of_month=False,
        )
        assert "spread_bp=0" in repr(leg)
