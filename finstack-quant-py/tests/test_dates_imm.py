"""IMM, CDS-roll, and listed-option expiry date bindings."""

from __future__ import annotations

import datetime

import pytest

from finstack_quant.core import dates


def test_third_wednesday_and_friday_match_the_market_conventions() -> None:
    """March 2025: IMM lands on the 19th, listed-option expiry on the 21st."""
    assert dates.third_wednesday(3, 2025) == datetime.date(2025, 3, 19)
    assert dates.third_friday(3, 2025) == datetime.date(2025, 3, 21)
    # February 2025 starts on a Saturday — the third Friday is the 21st.
    assert dates.third_friday(2, 2025) == datetime.date(2025, 2, 21)


def test_month_out_of_range_raises() -> None:
    with pytest.raises(ValueError, match="invalid month: 13"):
        dates.third_wednesday(13, 2025)
    with pytest.raises(ValueError, match="invalid month: 0"):
        dates.imm_option_expiry(0, 2025)


def test_imm_and_cds_roll_navigation() -> None:
    """IMM dates are quarterly third Wednesdays; CDS rolls are the 20ths."""
    ref = datetime.date(2025, 5, 1)
    assert dates.next_imm(ref) == datetime.date(2025, 6, 18)
    assert dates.is_imm_date(datetime.date(2025, 3, 19))
    assert not dates.is_imm_date(datetime.date(2025, 3, 20))

    assert dates.next_cds_date(ref) == datetime.date(2025, 6, 20)
    assert dates.prev_cds_date(ref) == datetime.date(2025, 3, 20)
    assert dates.is_cds_date(datetime.date(2025, 6, 20))

    # Semi-annual rolls are the March and September dates only.
    assert dates.prev_cds_semiannual_roll(ref) == datetime.date(2025, 3, 20)
    assert dates.next_semiannual_cds_maturity(ref) == datetime.date(2025, 6, 20)


def test_option_expiries() -> None:
    """IMM option expiry precedes its future; equity expiry is the third Friday."""
    imm_expiry = dates.imm_option_expiry(3, 2025)
    assert imm_expiry == datetime.date(2025, 3, 14)
    assert imm_expiry < dates.third_wednesday(3, 2025)

    ref = datetime.date(2025, 5, 1)
    assert dates.next_imm_option_expiry(ref) == datetime.date(2025, 6, 13)
    assert dates.next_equity_option_expiry(ref) == datetime.date(2025, 5, 16)
    assert dates.next_equity_option_expiry(ref) == dates.third_friday(5, 2025)


def test_next_helpers_are_strictly_forward_looking() -> None:
    """`next_*` never returns its own argument."""
    imm = datetime.date(2025, 3, 19)
    assert dates.next_imm(imm) > imm
    expiry = dates.next_equity_option_expiry(datetime.date(2025, 5, 1))
    assert dates.next_equity_option_expiry(expiry) > expiry
