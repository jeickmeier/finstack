# finstack-quant-py/tests/test_reporting_format.py
from __future__ import annotations

import datetime as dt
from decimal import Decimal
from types import SimpleNamespace

from finstack_quant.reporting import format as fmt


def test_escape_html_preserves_current_byte_encoding() -> None:
    raw = """A & B <tag> "quoted" 'single'"""
    assert fmt._escape_html(raw) == "A &amp; B &lt;tag&gt; &quot;quoted&quot; &#x27;single&#x27;"
    assert fmt._escape_html(None) == "None"


def test_dates_of_normalizes_date_like_indexes_only() -> None:
    frame = SimpleNamespace(index=[dt.datetime(2024, 1, 2, 15, 30), dt.date(2024, 1, 3), "plain"])
    assert fmt._dates_of(frame) == [dt.date(2024, 1, 2), dt.date(2024, 1, 3), "plain"]


def test_missing_preserves_supported_predicate_semantics() -> None:
    assert fmt._missing(None)
    assert fmt._missing(float("nan"))
    assert not fmt._missing(float("inf"))
    assert not fmt._missing(Decimal("NaN"))
    assert not fmt._missing("nan")


def test_pct_basic_and_signed() -> None:
    assert fmt.pct(0.132 * 100) == "13.2%"
    assert fmt.pct(0.132 * 100, signed=True) == "+13.2%"
    assert fmt.pct(-0.146 * 100, signed=True) == "-14.6%"


def test_pct_handles_nan_and_none() -> None:
    assert fmt.pct(float("nan")) == "·"
    assert fmt.pct(None) == "·"


def test_ratio() -> None:
    assert fmt.ratio(1.4234) == "1.42"


def test_sign_class() -> None:
    assert fmt.sign_class(1.0) == "pos"
    assert fmt.sign_class(-1.0) == "neg"
    assert fmt.sign_class(0.0) == ""
    assert fmt.sign_class(float("nan")) == ""


def test_fmt_date_accepts_date_and_timestamp() -> None:
    assert fmt.fmt_date(dt.date(2026, 6, 19)) == "19 Jun 2026"
