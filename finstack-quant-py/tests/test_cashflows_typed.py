"""Behavioral tests for the typed finstack_quant.cashflows bindings."""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

import pytest

from finstack_quant.core.money import Money

if TYPE_CHECKING:
    from finstack_quant.cashflows.builder import CashFlowSchedule


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


class TestCashFlowBuilder:
    @staticmethod
    def _quarterly_bond() -> CashFlowSchedule:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import (
            CashFlowSchedule,
            FixedCouponSpec,
            ScheduleParams,
        )

        return (
            CashFlowSchedule
            .builder()
            .principal(Money(1_000_000.0, "USD"), dt.date(2025, 1, 15), dt.date(2026, 1, 15))
            .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
            .build(None)
        )

    def test_fixed_quarterly_bond_flows(self) -> None:
        from finstack_quant.cashflows.primitives import CFKind

        schedule = self._quarterly_bond()
        flows = schedule.get_flows()
        assert len(flows) == 6

        # Initial funding: -1MM notional at issue.
        assert flows[0].date == dt.date(2025, 1, 15)
        assert flows[0].kind == CFKind.NOTIONAL
        assert flows[0].amount.amount == pytest.approx(-1_000_000.0)

        # Four Act/360 coupons: 90/91/92/92 day accrual periods.
        expected_coupons = [
            (dt.date(2025, 4, 15), 1_000_000.0 * 0.05 * 90 / 360),
            (dt.date(2025, 7, 15), 1_000_000.0 * 0.05 * 91 / 360),
            (dt.date(2025, 10, 15), 1_000_000.0 * 0.05 * 92 / 360),
            (dt.date(2026, 1, 15), 1_000_000.0 * 0.05 * 92 / 360),
        ]
        coupon_flows = [f for f in flows if f.kind == CFKind.FIXED]
        assert len(coupon_flows) == 4
        for flow, (date, amount) in zip(coupon_flows, expected_coupons, strict=True):
            assert flow.date == date
            assert flow.amount.amount == pytest.approx(amount, abs=1e-6)
            assert flow.amount.currency.code == "USD"
            assert flow.rate == pytest.approx(0.05)

        # Redemption at maturity, after the same-date final coupon.
        assert flows[5].date == dt.date(2026, 1, 15)
        assert flows[5].kind == CFKind.NOTIONAL
        assert flows[5].amount.amount == pytest.approx(1_000_000.0)

    def test_build_requires_principal(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import (
            CashFlowSchedule,
            FixedCouponSpec,
            ScheduleParams,
        )

        builder = CashFlowSchedule.builder().fixed_cf(
            FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360())
        )
        with pytest.raises(KeyError):
            # Missing principal maps to InputError::NotFound -> KeyError.
            builder.build(None)

    def test_floating_without_curve_raises(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import (
            CashFlowSchedule,
            FloatingCouponSpec,
            FloatingRateSpec,
            ScheduleParams,
        )

        spec = FloatingCouponSpec(
            rate_spec=FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("200"), reset_freq="3M"),
            schedule=ScheduleParams.quarterly_act360(),
        )
        builder = (
            CashFlowSchedule
            .builder()
            .principal(Money(1_000_000.0, "USD"), dt.date(2025, 1, 15), dt.date(2026, 1, 15))
            .floating_cf(spec)
        )
        # Default FloatingRateFallback.ERROR: missing curve must fail the build.
        with pytest.raises((KeyError, ValueError)):
            builder.build(None)

    def test_floating_with_market_context_builds(self) -> None:
        from decimal import Decimal

        from finstack_quant.cashflows.builder import (
            CashFlowSchedule,
            FloatingCouponSpec,
            FloatingRateSpec,
            ScheduleParams,
        )
        from finstack_quant.cashflows.primitives import CFKind
        from finstack_quant.core.market_data import ForwardCurve, MarketContext

        # base_date must be on/before every reset date; the first reset
        # trails the issue date by the default 2-business-day reset lag.
        curve = ForwardCurve(
            id="USD-SOFR-3M",
            tenor=0.25,
            knots=[(0.0, 0.04), (2.0, 0.04)],
            base_date=dt.date(2025, 1, 1),
        )
        market = MarketContext().insert(curve)
        spec = FloatingCouponSpec(
            rate_spec=FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("0"), reset_freq="3M"),
            schedule=ScheduleParams.quarterly_act360(),
        )
        schedule = (
            CashFlowSchedule
            .builder()
            .principal(Money(1_000_000.0, "USD"), dt.date(2025, 1, 15), dt.date(2026, 1, 15))
            .floating_cf(spec)
            .build(market)
        )
        floats = [f for f in schedule.get_flows() if f.kind == CFKind.FLOAT_RESET]
        assert len(floats) == 4
        assert all(f.amount.amount > 0.0 for f in floats)


class TestCashFlowSchedule:
    @staticmethod
    def _bond() -> CashFlowSchedule:
        return TestCashFlowBuilder._quarterly_bond()

    def test_accessors(self) -> None:
        from finstack_quant.core.dates import DayCount

        schedule = self._bond()
        assert len(schedule.coupons()) == 4
        assert schedule.dates()[0] == dt.date(2025, 1, 15)
        assert schedule.get_notional().initial.amount == pytest.approx(1_000_000.0)
        assert schedule.get_day_count() == DayCount.ACT_360
        meta = schedule.get_meta()
        assert meta.issue_date == dt.date(2025, 1, 15)
        assert meta.maturity_date == dt.date(2026, 1, 15)
        assert "weekends_only" in meta.calendar_ids
        schedule.validate()

    def test_scale_amounts_flips_direction(self) -> None:
        schedule = self._bond()
        flipped = schedule.scale_amounts(-1.0)
        # Original untouched; new schedule has negated amounts.
        assert schedule.get_flows()[0].amount.amount == pytest.approx(-1_000_000.0)
        total_orig = sum(f.amount.amount for f in schedule.get_flows())
        total_flip = sum(f.amount.amount for f in flipped.get_flows())
        assert total_flip == pytest.approx(-total_orig, abs=1e-9)
        with pytest.raises(ValueError, match="finite"):
            schedule.scale_amounts(float("nan"))

    def test_weighted_average_life_bullet(self) -> None:
        schedule = self._bond()
        # Single 1MM principal repayment exactly one non-leap year out: Act/365F WAL = 1.0.
        wal = schedule.weighted_average_life(dt.date(2025, 1, 15))
        assert wal == pytest.approx(1.0, abs=1e-12)

    def test_outstanding_by_date(self) -> None:
        schedule = self._bond()
        path = schedule.outstanding_by_date()
        first_date, first_balance = path[0]
        last_date, last_balance = path[-1]
        assert first_date == dt.date(2025, 1, 15)
        assert first_balance.amount == pytest.approx(1_000_000.0)
        assert last_date == dt.date(2026, 1, 15)
        assert last_balance.amount == pytest.approx(0.0, abs=1e-9)

    def test_pv_by_period_zero_rate_equals_nominal(self) -> None:
        from finstack_quant.core.dates import build_periods
        from finstack_quant.core.market_data import DiscountCurve, MarketContext

        schedule = self._bond()
        periods = build_periods("2025Q1..2026Q1").periods
        market = MarketContext().insert(DiscountCurve.flat("USD-OIS", dt.date(2025, 1, 15), 0.0))
        pv = schedule.pv_by_period(
            periods=periods,
            market=market,
            disc_curve_id="USD-OIS",
            base=dt.date(2025, 1, 15),
        )
        # Zero rate: PV == nominal sum per period. 2025Q2 holds the 90-day coupon.
        assert pv["2025Q2"]["USD"].amount == pytest.approx(12_500.0, abs=1e-6)

    def test_to_dataframe(self) -> None:
        pd = pytest.importorskip("pandas")

        schedule = self._bond()
        df = schedule.to_dataframe()
        assert list(df.columns) == ["date", "kind", "amount", "currency"]
        assert len(df) == 6
        assert set(df["kind"]) == {"fixed", "notional"}
        assert set(df["currency"]) == {"USD"}
        assert df["amount"].sum() == pytest.approx(sum(f.amount.amount for f in schedule.get_flows()), abs=1e-9)
        assert isinstance(df, pd.DataFrame)

    def test_json_round_trip_matches_json_bridge(self) -> None:
        from finstack_quant.cashflows import validate_cashflow_schedule_json
        from finstack_quant.cashflows.builder import CashFlowSchedule

        schedule = self._bond()
        payload = schedule.to_json()
        # The typed schedule serializes to the same canonical JSON the bridge accepts.
        canonical = validate_cashflow_schedule_json(payload)
        round_tripped = CashFlowSchedule.from_json(canonical)
        assert len(round_tripped.get_flows()) == len(schedule.get_flows())

    def test_merge_cashflow_schedules(self) -> None:
        from finstack_quant.cashflows.builder import Notional, merge_cashflow_schedules
        from finstack_quant.core.dates import DayCount

        a = self._bond()
        b = self._bond().scale_amounts(-1.0)
        merged = merge_cashflow_schedules([a, b], Notional.par(1_000_000.0, "USD"), DayCount.ACT_360)
        assert len(merged.get_flows()) == 12
        total = sum(f.amount.amount for f in merged.get_flows())
        assert total == pytest.approx(0.0, abs=1e-9)
