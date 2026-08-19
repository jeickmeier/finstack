"""Python binding coverage for the listed-derivatives routing catalog."""

from __future__ import annotations

import pytest

from finstack_quant.valuations.market import listed_product_catalog


def test_listed_product_catalog_filters_exact_venue() -> None:
    """Return Montréal rows with canonical instrument routes and reject aliases."""
    rows = listed_product_catalog("montreal")

    assert rows
    assert all(row["exchange"] == "montreal" for row in rows)
    assert any("CRA" in str(row["symbols"]) for row in rows)
    assert any(row["instrument_type"] == "interest_rate_future" for row in rows)
    with pytest.raises(ValueError, match="unknown listed exchange"):
        listed_product_catalog("mx")  # type: ignore[arg-type]


def test_listed_product_catalog_covers_all_four_venues() -> None:
    """Expose all maintained exchange routes when no filter is provided."""
    rows = listed_product_catalog()
    venues = {row["exchange"] for row in rows}

    assert venues == {"cme", "eurex", "montreal", "sgx"}
    assert any(row["instrument_type"] == "commodity_future" for row in rows)
    option_routes = {row["instrument_type"] for row in rows if row["product_kind"] == "option_on_future"}
    assert option_routes == {
        "commodity_future_option",
        "equity_future_option",
        "fx_future_option",
        "interest_rate_future_option",
        "volatility_index_future_option",
    }
