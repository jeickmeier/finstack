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


class TestBuilderSpecs:
    def test_schedule_params_presets(self) -> None:
        from finstack_quant.cashflows.builder import ScheduleParams

        p = ScheduleParams.usd_sofr_swap()
        assert p.calendar_id == "usny"
        assert p.payment_lag_days == 2
        assert p.adjust_accrual_dates is True
        q = ScheduleParams.quarterly_act360()
        assert q.calendar_id == "weekends_only"
        assert q.payment_lag_days == 0
        assert q.adjust_accrual_dates is False
        # all 10 presets exist and return ScheduleParams
        for name in (
            "quarterly_act360",
            "semiannual_30360",
            "annual_actact",
            "usd_sofr_swap",
            "usd_corporate_bond",
            "usd_treasury",
            "eur_estr_swap",
            "eur_gov_bond",
            "gbp_sonia_swap",
            "jpy_tona_swap",
        ):
            assert isinstance(getattr(ScheduleParams, name)(), ScheduleParams)

    def test_schedule_params_constructor(self) -> None:
        from finstack_quant.cashflows.builder import ScheduleParams
        from finstack_quant.core.dates import DayCount, Tenor

        p = ScheduleParams(
            freq=Tenor.quarterly(),
            dc=DayCount.ACT_360,
            calendar_id="weekends_only",
        )
        assert p.calendar_id == "weekends_only"
        assert p.end_of_month is False

    def test_fixed_coupon_spec_decimal_rate(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import FixedCouponSpec, ScheduleParams

        spec = FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360())
        assert spec.rate == Decimal("0.05")

    def test_coupon_type_and_roll_rule(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import CouponType, RollRule

        assert CouponType.CASH is not None
        assert CouponType.PIK is not None
        assert CouponType.split(Decimal("0.5"), Decimal("0.5")) is not None
        assert RollRule.NONE != RollRule.CDS_IMM

    def test_amortization_and_notional(self) -> None:
        from finstack_quant.cashflows.builder import AmortizationSpec, Notional

        n = Notional.par(1_000_000.0, "USD")
        assert n.initial.amount == pytest.approx(1_000_000.0)
        assert n.currency().code == "USD"
        n.validate()
        spec = AmortizationSpec.linear_to(Money(0.0, "USD"))
        n2 = Notional(Money(1_000_000.0, "USD"), amort=spec)
        n2.validate()
        with pytest.raises(ValueError, match="cannot exceed"):
            # LinearTo target above initial notional is invalid
            Notional(
                Money(100.0, "USD"),
                amort=AmortizationSpec.linear_to(Money(200.0, "USD")),
            ).validate()

    def test_fee_spec_factories(self) -> None:
        from finstack_quant.cashflows.builder import FeeAccrualBasis, FeeBase, FeeSpec
        from finstack_quant.core.dates import DayCount, Tenor

        fixed = FeeSpec.fixed(dt.date(2025, 1, 15), Money(-5_000.0, "USD"))
        assert fixed is not None
        periodic = FeeSpec.periodic_bps(
            base=FeeBase.undrawn(facility_limit=Money(10_000_000.0, "USD")),
            bps=50,
            freq=Tenor.quarterly(),
            dc=DayCount.ACT_360,
            bdc="modified_following",
            calendar_id="weekends_only",
            accrual_basis=FeeAccrualBasis.TIME_WEIGHTED_AVERAGE,
        )
        assert periodic is not None

    def test_floating_rate_spec_validate(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import FloatingRateSpec

        spec = FloatingRateSpec(
            index_id="USD-SOFR-3M",
            spread_bp=Decimal("200"),
            reset_freq="3M",
            index_floor_bp=Decimal("0"),
        )
        spec.validate()
        bad = FloatingRateSpec(
            index_id="USD-SOFR-3M",
            spread_bp=Decimal("200"),
            reset_freq="3M",
            index_floor_bp=Decimal("100"),
            index_cap_bp=Decimal("50"),
        )
        with pytest.raises(ValueError, match="index_floor_bp"):
            bad.validate()

    def test_credit_model_specs(self) -> None:
        from finstack_quant.cashflows.builder import (
            DefaultModelSpec,
            PrepaymentModelSpec,
            RecoveryModelSpec,
        )

        psa = PrepaymentModelSpec.psa(1.0)
        assert psa.smm(30) > psa.smm(1)
        assert PrepaymentModelSpec.constant_cpr(0.06).cpr == pytest.approx(0.06)
        sda = DefaultModelSpec.sda(1.0)
        assert sda.mdr(30) > sda.mdr(1)
        assert DefaultModelSpec.cdr_2pct().cdr == pytest.approx(0.02)
        rec = RecoveryModelSpec(rate=0.40, recovery_lag=12)
        rec.validate()
        assert rec.recovery_lag == 12
        with pytest.raises(ValueError, match="must be in"):
            RecoveryModelSpec(rate=1.5, recovery_lag=0).validate()
