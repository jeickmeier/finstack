"""Behavioral tests for the typed finstack_quant.cashflows bindings."""

from __future__ import annotations

import datetime as dt

import pytest

from finstack_quant.core.money import Money


class TestPrimitives:
    def test_cfkind_classattrs_parse_and_display(self) -> None:
        from finstack_quant.cashflows.primitives import CFKind

        assert CFKind.parse("fixed") == CFKind.FIXED
        assert CFKind.FLOAT_RESET.name == "float_reset"
        assert str(CFKind.AMORTIZATION) == "amortization"
        assert CFKind.PIK != CFKind.FIXED
        assert CFKind.FIXED.is_interest_like()
        assert not CFKind.FEE.is_interest_like()
        # FromStr accepts the documented alias set (e.g. "amort").
        assert CFKind.parse("amort") == CFKind.AMORTIZATION
        with pytest.raises(ValueError, match="unknown variant"):
            CFKind.parse("not_a_kind")

    def test_cashflow_construction_getters_and_validate(self) -> None:
        from finstack_quant.cashflows.primitives import CashFlow, CFKind

        cf = CashFlow(
            date=dt.date(2025, 6, 15),
            amount=Money(50_000.0, "USD"),
            kind=CFKind.FIXED,
            accrual_factor=0.5,
            rate=0.05,
        )
        assert cf.date == dt.date(2025, 6, 15)
        assert cf.reset_date is None
        assert cf.amount.amount == pytest.approx(50_000.0)
        assert cf.kind == CFKind.FIXED
        assert cf.accrual_factor == pytest.approx(0.5)
        assert cf.rate == pytest.approx(0.05)
        cf.validate()  # must not raise

    def test_cashflow_validate_rejects_negative_accrual_factor(self) -> None:
        from finstack_quant.cashflows.primitives import CashFlow, CFKind

        cf = CashFlow(
            date=dt.date(2025, 6, 15),
            amount=Money(1.0, "USD"),
            kind=CFKind.FIXED,
            accrual_factor=-0.1,
        )
        with pytest.raises(ValueError, match="accrual_factor"):
            cf.validate()

    def test_is_cash_settlement_kind(self) -> None:
        from finstack_quant.cashflows.primitives import CFKind, is_cash_settlement_kind

        assert is_cash_settlement_kind(CFKind.FIXED)
        assert is_cash_settlement_kind(CFKind.AMORTIZATION)
        assert not is_cash_settlement_kind(CFKind.PIK)
        assert not is_cash_settlement_kind(CFKind.DEFAULTED_NOTIONAL)
        # str form is accepted too
        assert not is_cash_settlement_kind("pik")
