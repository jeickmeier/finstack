"""
Composable cashflow builder: coupon/fee/amortization specs, CashFlowBuilder, CashFlowSchedule.

Typed bindings for the spec types under ``finstack_quant_cashflows::builder``:
schedule generation parameters, fixed/floating/step-up coupon specs,
amortization and notional rules, fee specs, and the credit behavior models
(prepayment, default, recovery) used by structured/private-credit cashflow
programs. The fluent ``CashFlowBuilder`` and ``CashFlowSchedule`` types are
added in a later task; the canonical JSON program bridge on
``finstack_quant.cashflows`` remains available for the full build pipeline
today.

Examples
--------
>>> import finstack_quant.cashflows.builder as builder
>>> builder.__name__
'finstack_quant.cashflows.builder'
"""

from __future__ import annotations

import datetime
from decimal import Decimal

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import BusinessDayConvention, DayCount, StubKind, Tenor
from finstack_quant.core.money import Money

__all__ = [
    "AmortizationSpec",
    "CouponType",
    "DefaultModelSpec",
    "FeeAccrualBasis",
    "FeeBase",
    "FeeSpec",
    "FixedCouponSpec",
    "FloatingCouponSpec",
    "FloatingRateFallback",
    "FloatingRateSpec",
    "Notional",
    "OvernightCompoundingMethod",
    "OvernightIndexConstraintApplication",
    "PrepaymentModelSpec",
    "RecoveryModelSpec",
    "RollRule",
    "ScheduleParams",
    "StepUpCouponSpec",
]

class AmortizationSpec:
    """
    Amortization rule for principal over the life of a cashflow leg.

    Immutable, hashable value type constructed only through its named
    factory functions; each factory mirrors one Rust ``AmortizationSpec``
    variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import AmortizationSpec
    >>> AmortizationSpec.none() is not None
    True
    """

    @staticmethod
    def none() -> AmortizationSpec:
        """
        No amortization; principal remains constant until final redemption.

        Returns
        -------
        AmortizationSpec
            A bullet (non-amortising) amortization rule.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import AmortizationSpec
        >>> AmortizationSpec.none() is not None
        True
        """
        ...

    @staticmethod
    def linear_to(final_notional: Money) -> AmortizationSpec:
        """
        Linear principal paydown towards a target final notional.

        Parameters
        ----------
        final_notional : Money
            Target remaining principal at the end of the schedule; must be
            in the same currency as, and not exceed, the leg's initial
            notional.

        Returns
        -------
        AmortizationSpec
            A linear-paydown amortization rule.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import AmortizationSpec
        >>> from finstack_quant.core.money import Money
        >>> AmortizationSpec.linear_to(Money(0.0, "USD")) is not None
        True
        """
        ...

    @staticmethod
    def step_remaining(
        schedule: list[tuple[datetime.date, Money]],
    ) -> AmortizationSpec:
        """
        Explicit schedule of remaining principal after each listed date.

        Parameters
        ----------
        schedule : list[tuple[datetime.date, Money]]
            Ordered ``(date, remaining_principal_after_date)`` pairs; dates
            must be strictly increasing and amounts non-increasing.

        Returns
        -------
        AmortizationSpec
            A step-schedule amortization rule.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.cashflows.builder import AmortizationSpec
        >>> from finstack_quant.core.money import Money
        >>> spec = AmortizationSpec.step_remaining([(datetime.date(2026, 1, 1), Money(500_000.0, "USD"))])
        >>> spec is not None
        True
        """
        ...

    @staticmethod
    def percent_of_original_per_period(pct: float) -> AmortizationSpec:
        """
        Fixed percentage of original notional paid each period.

        Parameters
        ----------
        pct : float
            Fraction of original notional paid per period (e.g. ``0.05``
            for 5%); does not compound across periods.

        Returns
        -------
        AmortizationSpec
            A sinking-fund style amortization rule.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import AmortizationSpec
        >>> AmortizationSpec.percent_of_original_per_period(0.05) is not None
        True
        """
        ...

    @staticmethod
    def custom_principal(
        items: list[tuple[datetime.date, Money]],
    ) -> AmortizationSpec:
        """
        Custom principal exchanges on specific dates (absolute cash amounts).

        Parameters
        ----------
        items : list[tuple[datetime.date, Money]]
            ``(date, principal_amount)`` exchanges; positive amounts reduce
            outstanding principal.

        Returns
        -------
        AmortizationSpec
            A custom-principal-exchange amortization rule.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.cashflows.builder import AmortizationSpec
        >>> from finstack_quant.core.money import Money
        >>> spec = AmortizationSpec.custom_principal([(datetime.date(2026, 1, 1), Money(50_000.0, "USD"))])
        >>> spec is not None
        True
        """
        ...

class CouponType:
    """
    Coupon settlement type: cash, PIK, or an explicit cash/PIK split.

    Class attributes ``CASH`` and ``PIK`` are singletons for the fieldless
    variants; :meth:`split` constructs the data-carrying ``Split`` variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import CouponType
    >>> CouponType.CASH != CouponType.PIK
    True
    """

    CASH: CouponType
    """100% paid in cash."""
    PIK: CouponType
    """100% capitalized into principal."""

    @staticmethod
    def split(cash_pct: Decimal | float, pik_pct: Decimal | float) -> CouponType:
        """
        Split settlement with explicit cash and PIK fractions.

        Parameters
        ----------
        cash_pct : decimal.Decimal | float
            Fraction of the coupon paid in cash, in ``[0, 1]``.
        pik_pct : decimal.Decimal | float
            Fraction of the coupon capitalized as PIK, in ``[0, 1]``;
            ``cash_pct + pik_pct`` must sum to ``1``.

        Returns
        -------
        CouponType
            A ``Split`` coupon type with the given fractions.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from decimal import Decimal
        >>> from finstack_quant.cashflows.builder import CouponType
        >>> CouponType.split(Decimal("0.5"), Decimal("0.5")) is not None
        True
        """
        ...

class DefaultModelSpec:
    """
    Default rate model used to project defaults across a pool or portfolio.

    Constructed via named factories (:meth:`constant_cdr`, :meth:`sda`,
    :meth:`cdr_2pct`) mirroring the Rust curve presets.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import DefaultModelSpec
    >>> DefaultModelSpec.cdr_2pct().cdr
    0.02
    """

    @staticmethod
    def constant_cdr(cdr: float) -> DefaultModelSpec:
        """
        Constant CDR (Constant Default Rate) with no seasoning curve.

        Parameters
        ----------
        cdr : float
            Annual constant default rate (e.g. ``0.02`` for 2%).

        Returns
        -------
        DefaultModelSpec
            A default model using a flat annual CDR.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import DefaultModelSpec
        >>> DefaultModelSpec.constant_cdr(0.02).cdr
        0.02
        """
        ...

    @staticmethod
    def sda(speed_multiplier: float) -> DefaultModelSpec:
        """
        SDA (Standard Default Assumption) seasoning curve.

        Parameters
        ----------
        speed_multiplier : float
            Curve speed relative to 100% SDA (``1.0`` = 100% SDA).

        Returns
        -------
        DefaultModelSpec
            A default model driven by the SDA seasoning ramp.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import DefaultModelSpec
        >>> DefaultModelSpec.sda(1.0) is not None
        True
        """
        ...

    @staticmethod
    def cdr_2pct() -> DefaultModelSpec:
        """
        2% constant CDR baseline.

        Returns
        -------
        DefaultModelSpec
            A default model with a flat 2% annual CDR.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import DefaultModelSpec
        >>> DefaultModelSpec.cdr_2pct().cdr
        0.02
        """
        ...

    @property
    def cdr(self) -> float:
        """
        Annual constant default rate.

        Returns
        -------
        float
            The configured annual CDR, ignored when a seasoning curve
            supplies its own terminal rate.
        """
        ...

    def mdr(self, seasoning_months: int) -> float:
        """
        Monthly default rate (MDR) for the supplied seasoning.

        Parameters
        ----------
        seasoning_months : int
            Number of months since origination.

        Returns
        -------
        float
            The single-month default rate implied by this model at the
            given seasoning.

        Raises
        ------
        ValueError
            If the underlying curve parameters are invalid, such as a
            negative SDA speed multiplier.
        """
        ...

class FeeAccrualBasis:
    """
    Controls how the outstanding balance is sampled during fee accrual.

    Immutable, hashable enum-style type with one class attribute per
    Rust variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import FeeAccrualBasis
    >>> FeeAccrualBasis.POINT_IN_TIME != FeeAccrualBasis.TIME_WEIGHTED_AVERAGE
    True
    """

    POINT_IN_TIME: FeeAccrualBasis
    """Sample the outstanding balance at accrual-period start (default)."""
    TIME_WEIGHTED_AVERAGE: FeeAccrualBasis
    """Time-weighted average outstanding over the accrual period."""

class FeeBase:
    """
    Economic balance used as the base for a periodic basis-point fee.

    Constructed via :meth:`drawn` or :meth:`undrawn`, mirroring the Rust
    ``FeeBase`` variants.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import FeeBase
    >>> FeeBase.drawn() is not None
    True
    """

    @staticmethod
    def drawn() -> FeeBase:
        """
        Fee accrues on the drawn outstanding balance.

        Returns
        -------
        FeeBase
            A fee base tied to the drawn balance.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import FeeBase
        >>> FeeBase.drawn() is not None
        True
        """
        ...

    @staticmethod
    def undrawn(facility_limit: Money) -> FeeBase:
        """
        Fee accrues on undrawn = max(facility_limit - outstanding, 0).

        Parameters
        ----------
        facility_limit : Money
            Total facility commitment used to compute the undrawn amount.

        Returns
        -------
        FeeBase
            A fee base tied to the undrawn balance.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import FeeBase
        >>> from finstack_quant.core.money import Money
        >>> FeeBase.undrawn(Money(10_000_000.0, "USD")) is not None
        True
        """
        ...

class FeeSpec:
    """
    Fee specification: a one-time fixed fee or a periodic basis-point fee.

    Constructed via :meth:`fixed` or :meth:`periodic_bps`, mirroring the
    two Rust ``FeeSpec`` variants. Negative amounts and bps quotes are
    treated as rebates and flow through unchanged.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.builder import FeeSpec
    >>> from finstack_quant.core.money import Money
    >>> FeeSpec.fixed(datetime.date(2025, 1, 15), Money(-5_000.0, "USD")) is not None
    True
    """

    @staticmethod
    def fixed(date: datetime.date, amount: Money) -> FeeSpec:
        """
        Fixed fee paid once on a specified date.

        Parameters
        ----------
        date : datetime.date
            Payment date of the fixed fee.
        amount : Money
            Fee amount; negative amounts are rebates.

        Returns
        -------
        FeeSpec
            A one-time fixed fee specification.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.cashflows.builder import FeeSpec
        >>> from finstack_quant.core.money import Money
        >>> FeeSpec.fixed(datetime.date(2025, 1, 15), Money(-5_000.0, "USD")) is not None
        True
        """
        ...

    @staticmethod
    def periodic_bps(
        base: FeeBase,
        bps: Decimal | float,
        freq: Tenor | str,
        dc: DayCount,
        bdc: BusinessDayConvention | str,
        calendar_id: str,
        stub: StubKind | None = None,
        accrual_basis: FeeAccrualBasis | None = None,
    ) -> FeeSpec:
        """
        Periodic fee quoted in basis points per annum, accrued over generated periods.

        Parameters
        ----------
        base : FeeBase
            Economic balance the fee accrues against (drawn or undrawn).
        bps : decimal.Decimal | float
            Fee quote in basis points per annum.
        freq : Tenor | str
            Accrual and payment frequency for the fee schedule.
        dc : DayCount
            Day-count convention used to annualize the fee accrual.
        bdc : BusinessDayConvention | str
            Business-day convention applied to generated fee dates.
        calendar_id : str
            Holiday calendar id used with *bdc*.
        stub : StubKind, optional
            Stub-handling rule for irregular periods (default short-front).
        accrual_basis : FeeAccrualBasis, optional
            How the outstanding balance is sampled (default point-in-time).

        Returns
        -------
        FeeSpec
            A periodic basis-point fee specification.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import FeeBase, FeeSpec
        >>> from finstack_quant.core.dates import DayCount, Tenor
        >>> from finstack_quant.core.money import Money
        >>> spec = FeeSpec.periodic_bps(
        ...     base=FeeBase.undrawn(facility_limit=Money(10_000_000.0, "USD")),
        ...     bps=50,
        ...     freq=Tenor.quarterly(),
        ...     dc=DayCount.ACT_360,
        ...     bdc="modified_following",
        ...     calendar_id="weekends_only",
        ... )
        >>> spec is not None
        True
        """
        ...

class FixedCouponSpec:
    """
    Fixed-rate coupon specification.

    Combines the coupon quote, settlement behavior, and schedule
    conventions required to emit a fixed-rate leg.

    Examples
    --------
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import FixedCouponSpec, ScheduleParams
    >>> spec = FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360())
    >>> spec.rate
    Decimal('0.05')
    """

    def __init__(
        self,
        rate: Decimal | float,
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> None:
        """
        Construct a fixed-rate coupon specification.

        Parameters
        ----------
        rate : decimal.Decimal | float
            Annual coupon rate as a decimal (``0.05`` for 5%).
        schedule : ScheduleParams
            Accrual and payment schedule conventions.
        coupon_type : CouponType, optional
            Cash (default), PIK, or split settlement.

        Raises
        ------
        ValueError
            If *rate* is not representable as a finite decimal value.
        """
        ...

    @property
    def rate(self) -> Decimal:
        """
        Annual coupon rate.

        Returns
        -------
        decimal.Decimal
            The coupon rate as an exact decimal (e.g. ``Decimal("0.05")``).
        """
        ...

    @property
    def schedule(self) -> ScheduleParams:
        """
        Accrual and payment schedule conventions.

        Returns
        -------
        ScheduleParams
            The schedule-generation parameters for this coupon.
        """
        ...

class FloatingCouponSpec:
    """
    Floating coupon specification composing a :class:`FloatingRateSpec`.

    Combines a floating-rate index specification with coupon settlement
    behavior and schedule conventions.

    Examples
    --------
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import (
    ...     FloatingCouponSpec,
    ...     FloatingRateSpec,
    ...     ScheduleParams,
    ... )
    >>> rate_spec = FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("200"), reset_freq="3M")
    >>> spec = FloatingCouponSpec(rate_spec, ScheduleParams.usd_sofr_swap())
    >>> spec.rate_spec.index_id
    'USD-SOFR-3M'
    """

    def __init__(
        self,
        rate_spec: FloatingRateSpec,
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> None:
        """
        Construct a floating-rate coupon specification.

        Parameters
        ----------
        rate_spec : FloatingRateSpec
            Floating rate specification (index, spread, floors/caps, etc.).
        schedule : ScheduleParams
            Accrual and payment schedule conventions.
        coupon_type : CouponType, optional
            Cash (default), PIK, or split settlement.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.
        """
        ...

    @property
    def rate_spec(self) -> FloatingRateSpec:
        """
        Floating rate specification.

        Returns
        -------
        FloatingRateSpec
            The index, spread, floor/cap, and reset configuration.
        """
        ...

class FloatingRateFallback:
    """
    Policy for handling floating rate projection failures.

    ``ERROR`` and ``SPREAD_ONLY`` are class-attribute singletons;
    :meth:`fixed_rate` constructs the data-carrying ``FixedRate`` variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import FloatingRateFallback
    >>> FloatingRateFallback.ERROR != FloatingRateFallback.SPREAD_ONLY
    True
    """

    ERROR: FloatingRateFallback
    """Fail the build when the forward curve lookup fails (default, safest)."""
    SPREAD_ONLY: FloatingRateFallback
    """Project spread-only when no forward curve is available (explicit opt-in)."""

    @staticmethod
    def fixed_rate(rate: Decimal | float) -> FloatingRateFallback:
        """
        Use a fixed rate as the index component when projection fails.

        Parameters
        ----------
        rate : decimal.Decimal | float
            Decimal annual rate substituted for the projected index rate
            (e.g. ``0.045`` for 4.5%), not basis points.

        Returns
        -------
        FloatingRateFallback
            A fallback policy using the fixed rate.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import FloatingRateFallback
        >>> FloatingRateFallback.fixed_rate(0.045) is not None
        True
        """
        ...

class FloatingRateSpec:
    """
    Canonical floating rate specification for all instruments.

    Used by bonds, swaps, credit facilities, and structured products;
    mirrors ``finstack_quant_cashflows::builder::FloatingRateSpec``
    field-for-field.

    Examples
    --------
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import FloatingRateSpec
    >>> spec = FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("200"), reset_freq="3M")
    >>> spec.index_id
    'USD-SOFR-3M'
    """

    def __init__(
        self,
        index_id: str,
        spread_bp: Decimal | float,
        reset_freq: Tenor | str,
        gearing: Decimal | float | None = None,
        gearing_includes_spread: bool = True,
        index_floor_bp: Decimal | float | None = None,
        all_in_floor_bp: Decimal | float | None = None,
        all_in_cap_bp: Decimal | float | None = None,
        index_cap_bp: Decimal | float | None = None,
        overnight_index_constraints: OvernightIndexConstraintApplication | None = None,
        index_tenor: Tenor | str | None = None,
        reset_lag_days: int = 2,
        fixing_calendar_id: str | None = None,
        overnight_compounding: OvernightCompoundingMethod | None = None,
        overnight_basis: DayCount | None = None,
        fallback: FloatingRateFallback | None = None,
    ) -> None:
        """
        Construct a floating rate specification.

        Parameters
        ----------
        index_id : str
            Forward curve identifier (e.g. ``"USD-SOFR-3M"``).
        spread_bp : decimal.Decimal | float
            Spread/margin over the index in basis points.
        reset_freq : Tenor | str
            Reset frequency for rate fixings; also the default index tenor.
        gearing : decimal.Decimal | float, optional
            Leverage multiplier applied to the all-in rate (default ``1``);
            must be strictly positive.
        gearing_includes_spread : bool
            Whether gearing multiplies ``(index + spread)`` (``True``,
            default) or only the index component (``False``).
        index_floor_bp : decimal.Decimal | float, optional
            Floor applied to the index component, in basis points.
        all_in_floor_bp : decimal.Decimal | float, optional
            Floor applied to the final all-in rate, in basis points.
        all_in_cap_bp : decimal.Decimal | float, optional
            Cap applied to the final all-in rate, in basis points.
        index_cap_bp : decimal.Decimal | float, optional
            Cap applied to the index component, in basis points.
        overnight_index_constraints : OvernightIndexConstraintApplication, optional
            Where index floors/caps are applied for overnight-compounded
            coupons (default daily).
        index_tenor : Tenor | str, optional
            Underlying index tenor for the forward projection, when it
            differs from *reset_freq*.
        reset_lag_days : int
            Reset lag in business days (default ``2``, T-2 convention).
        fixing_calendar_id : str, optional
            Calendar for the reset lag; defaults to the coupon schedule
            calendar when omitted.
        overnight_compounding : OvernightCompoundingMethod, optional
            Overnight compounding method for RFR indices (SOFR/ESTR/SONIA);
            leave unset for term rates.
        overnight_basis : DayCount, optional
            Day-count basis for the overnight compounding denominator
            (default Act/360).
        fallback : FloatingRateFallback, optional
            Policy applied when forward curve lookup fails (default
            ``Error``).

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type, shape,
            finite-value, or domain constraints.
        """
        ...

    @property
    def index_id(self) -> str:
        """
        Forward curve identifier.

        Returns
        -------
        str
            The forward curve id this spec projects from.
        """
        ...

    @property
    def spread_bp(self) -> Decimal:
        """
        Spread over the index in basis points.

        Returns
        -------
        decimal.Decimal
            The spread quote, preserved at full precision.
        """
        ...

    def validate(self) -> None:
        """
        Validate reset lag and floor/cap ordering.

        Raises
        ------
        ValueError
            If the reset lag is negative or a floor exceeds its
            corresponding cap.
        """
        ...

class Notional:
    """
    Notional amount with an optional amortization rule.

    Combines an initial principal amount with an :class:`AmortizationSpec`
    describing how it evolves over the life of the leg.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import Notional
    >>> n = Notional.par(1_000_000.0, "USD")
    >>> n.initial.amount
    1000000.0
    """

    def __init__(
        self,
        initial: Money,
        amort: AmortizationSpec | None = None,
    ) -> None:
        """
        Construct a notional with an optional amortization rule.

        Parameters
        ----------
        initial : Money
            Initial principal amount outstanding at leg inception.
        amort : AmortizationSpec, optional
            Amortization rule applied after each period (default: none).

        Raises
        ------
        ValueError
            If *initial* has a non-finite amount.
        """
        ...

    @classmethod
    def par(cls, amount: float, currency: Currency | str) -> Notional:
        """
        Plain (non-amortising) notional helper.

        Parameters
        ----------
        amount : float
            Initial principal amount.
        currency : Currency | str
            Currency of the notional, as a ``Currency`` instance or ISO
            4217 code.

        Returns
        -------
        Notional
            A bullet notional with no amortization.

        Raises
        ------
        ValueError
            If *currency* is not a valid ISO 4217 code or *amount* is not
            finite.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import Notional
        >>> Notional.par(1_000_000.0, "USD").initial.amount
        1000000.0
        """
        ...

    @property
    def initial(self) -> Money:
        """
        Initial principal amount.

        Returns
        -------
        Money
            The principal amount outstanding at leg inception.
        """
        ...

    def currency(self) -> Currency:
        """
        Currency of the notional.

        Returns
        -------
        Currency
            The currency of the initial notional amount.
        """
        ...

    def validate(self) -> None:
        """
        Validate the notional and its amortization rule.

        Raises
        ------
        ValueError
            If the amortization schedule is inconsistent with the initial
            notional, such as a currency mismatch or a target above the
            initial amount.
        """
        ...

class OvernightCompoundingMethod:
    """
    Compounding method for overnight rate indices (SOFR, ESTR, SONIA).

    ``SIMPLE_AVERAGE`` and ``COMPOUNDED_IN_ARREARS`` are class-attribute
    singletons; the remaining variants carry a business-day parameter and
    are constructed via their named factories.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import OvernightCompoundingMethod
    >>> OvernightCompoundingMethod.COMPOUNDED_IN_ARREARS is not None
    True
    """

    SIMPLE_AVERAGE: OvernightCompoundingMethod
    """Arithmetic average of daily fixings weighted by accrual days."""
    COMPOUNDED_IN_ARREARS: OvernightCompoundingMethod
    """Compounded in arrears (ISDA 2021 standard; default)."""

    @staticmethod
    def compounded_with_lookback(lookback_days: int) -> OvernightCompoundingMethod:
        """
        Compounded in arrears with an observation lookback.

        Parameters
        ----------
        lookback_days : int
            Number of business days to look back for rate observations.

        Returns
        -------
        OvernightCompoundingMethod
            A lookback-compounding method with the given window.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import OvernightCompoundingMethod
        >>> OvernightCompoundingMethod.compounded_with_lookback(2) is not None
        True
        """
        ...

    @staticmethod
    def compounded_with_lockout(lockout_days: int) -> OvernightCompoundingMethod:
        """
        Compounded in arrears with a rate lockout near period end.

        Parameters
        ----------
        lockout_days : int
            Number of business days before period end to freeze the rate.

        Returns
        -------
        OvernightCompoundingMethod
            A lockout-compounding method with the given window.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import OvernightCompoundingMethod
        >>> OvernightCompoundingMethod.compounded_with_lockout(2) is not None
        True
        """
        ...

    @staticmethod
    def compounded_with_observation_shift(
        shift_days: int,
    ) -> OvernightCompoundingMethod:
        """
        Compounded in arrears with both dates and weights shifted back.

        Parameters
        ----------
        shift_days : int
            Number of business days to shift observations and weights.

        Returns
        -------
        OvernightCompoundingMethod
            An observation-shift compounding method with the given window.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import OvernightCompoundingMethod
        >>> OvernightCompoundingMethod.compounded_with_observation_shift(2) is not None
        True
        """
        ...

class OvernightIndexConstraintApplication:
    """
    Where overnight index floors/caps are applied for compounded rates.

    Immutable, hashable enum-style type with one class attribute per
    Rust variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import OvernightIndexConstraintApplication
    >>> OvernightIndexConstraintApplication.DAILY != OvernightIndexConstraintApplication.PERIOD
    True
    """

    DAILY: OvernightIndexConstraintApplication
    """Apply index floors/caps to each daily fixing before compounding (default)."""
    PERIOD: OvernightIndexConstraintApplication
    """Apply index floors/caps once to the compounded period index rate."""

class PrepaymentModelSpec:
    """
    Prepayment rate model used to project prepayments across a pool.

    Constructed via named factories (:meth:`constant_cpr`, :meth:`psa`,
    :meth:`psa_100`, :meth:`cmbs_with_lockout`) mirroring the Rust curve
    presets.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import PrepaymentModelSpec
    >>> PrepaymentModelSpec.constant_cpr(0.06).cpr
    0.06
    """

    @staticmethod
    def constant_cpr(cpr: float) -> PrepaymentModelSpec:
        """
        Constant CPR (Constant Prepayment Rate) with no seasoning curve.

        Parameters
        ----------
        cpr : float
            Annual constant prepayment rate (e.g. ``0.06`` for 6%).

        Returns
        -------
        PrepaymentModelSpec
            A prepayment model using a flat annual CPR.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import PrepaymentModelSpec
        >>> PrepaymentModelSpec.constant_cpr(0.06).cpr
        0.06
        """
        ...

    @staticmethod
    def psa(speed_multiplier: float) -> PrepaymentModelSpec:
        """
        PSA (Public Securities Association) seasoning curve.

        Parameters
        ----------
        speed_multiplier : float
            Curve speed relative to 100% PSA (``1.0`` = 100% PSA).

        Returns
        -------
        PrepaymentModelSpec
            A prepayment model driven by the PSA seasoning ramp.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import PrepaymentModelSpec
        >>> PrepaymentModelSpec.psa(1.0) is not None
        True
        """
        ...

    @staticmethod
    def psa_100() -> PrepaymentModelSpec:
        """
        100% PSA (standard prepayment assumption).

        Returns
        -------
        PrepaymentModelSpec
            A prepayment model at the 100% PSA speed.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import PrepaymentModelSpec
        >>> PrepaymentModelSpec.psa_100() is not None
        True
        """
        ...

    @staticmethod
    def cmbs_with_lockout(lockout_months: int, post_lockout_cpr: float) -> PrepaymentModelSpec:
        """
        CMBS-style lockout: zero prepayment, then a constant CPR.

        Parameters
        ----------
        lockout_months : int
            Number of months with zero prepayment from origination.
        post_lockout_cpr : float
            Annual constant CPR applied after the lockout period ends.

        Returns
        -------
        PrepaymentModelSpec
            A prepayment model with a lockout followed by constant CPR.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import PrepaymentModelSpec
        >>> PrepaymentModelSpec.cmbs_with_lockout(24, 0.10) is not None
        True
        """
        ...

    @property
    def cpr(self) -> float:
        """
        Annual constant prepayment rate.

        Returns
        -------
        float
            The configured annual CPR, ignored when the PSA curve supplies
            its own terminal rate.
        """
        ...

    def smm(self, seasoning_months: int) -> float:
        """
        Single-month mortality (SMM) for the supplied seasoning.

        Parameters
        ----------
        seasoning_months : int
            Number of months since origination.

        Returns
        -------
        float
            The single-month prepayment rate implied by this model at the
            given seasoning.

        Raises
        ------
        ValueError
            If the underlying curve parameters are invalid, such as a
            negative PSA speed multiplier.
        """
        ...

class RecoveryModelSpec:
    """
    Recovery model specification for defaulted credit exposures.

    Combines a recovery rate fraction with the lag between default and the
    recovery cashflow.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import RecoveryModelSpec
    >>> spec = RecoveryModelSpec(rate=0.40, recovery_lag=12)
    >>> spec.recovery_lag
    12
    """

    def __init__(self, rate: float, recovery_lag: int) -> None:
        """
        Construct a recovery model with a rate and recovery lag.

        Parameters
        ----------
        rate : float
            Recovery rate as a fraction in ``[0.0, 1.0]`` (e.g. ``0.40``
            for 40%).
        recovery_lag : int
            Number of months between default and the recovery cashflow.

        Raises
        ------
        ValueError
            If *rate* is not finite; range validation happens in
            :meth:`validate`.
        """
        ...

    @property
    def rate(self) -> float:
        """
        Recovery rate as a fraction.

        Returns
        -------
        float
            The configured recovery rate, expected in ``[0.0, 1.0]``.
        """
        ...

    @property
    def recovery_lag(self) -> int:
        """
        Recovery lag in months.

        Returns
        -------
        int
            The number of months between default and recovery cashflow.
        """
        ...

    def validate(self) -> None:
        """
        Validate that the rate is finite and in ``[0.0, 1.0]``.

        Raises
        ------
        ValueError
            If the rate is not finite or falls outside ``[0.0, 1.0]``.
        """
        ...

class RollRule:
    """
    Roll-date rule applied when generating schedule anchors.

    Immutable, hashable enum-style type with one class attribute per
    Rust variant (``NONE``, ``IMM``, ``CDS_IMM``).

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import RollRule
    >>> RollRule.NONE != RollRule.CDS_IMM
    True
    """

    NONE: RollRule
    """Plain tenor stepping from the schedule boundaries (default)."""
    IMM: RollRule
    """Standard IMM dates: third Wednesday of Mar/Jun/Sep/Dec."""
    CDS_IMM: RollRule
    """CDS IMM dates: 20th of Mar/Jun/Sep/Dec with Big-Bang front accrual."""

class ScheduleParams:
    """
    Canonical schedule-generation parameters for coupons and periodic fees.

    Controls how accrual boundaries and payment dates are generated. Ten
    market-convention presets (:meth:`quarterly_act360`,
    :meth:`usd_sofr_swap`, etc.) are available as staticmethods alongside
    the general constructor.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import ScheduleParams
    >>> ScheduleParams.usd_sofr_swap().calendar_id
    'usny'
    """

    def __init__(
        self,
        freq: Tenor | str,
        dc: DayCount,
        calendar_id: str,
        bdc: BusinessDayConvention | str | None = None,
        stub: StubKind | None = None,
        end_of_month: bool = False,
        payment_lag_days: int = 0,
        adjust_accrual_dates: bool = False,
        roll_rule: RollRule | None = None,
    ) -> None:
        """
        Construct schedule-generation parameters.

        Parameters
        ----------
        freq : Tenor | str
            Accrual and payment frequency (e.g. ``"3M"``).
        dc : DayCount
            Day-count convention for accrual year fractions.
        calendar_id : str
            Holiday calendar id (``"weekends_only"`` for weekend-only
            rolling).
        bdc : BusinessDayConvention | str, optional
            Payment-date rolling convention (default Modified Following).
        stub : StubKind, optional
            Stub rule (default short-front).
        end_of_month : bool
            Preserve end-of-month rolling (default ``False``).
        payment_lag_days : int
            Payment lag in business days (default ``0``).
        adjust_accrual_dates : bool
            Roll accrual boundaries with *bdc*, i.e. swap/ISDA convention
            (default ``False``, bond convention).
        roll_rule : RollRule, optional
            IMM/CDS-IMM anchor grid (default none).

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type, shape, or
            domain constraints.
        """
        ...

    @staticmethod
    def quarterly_act360() -> ScheduleParams:
        """
        Quarterly, Act/360, Modified Following, weekends-only calendar.

        Returns
        -------
        ScheduleParams
            Generic quarterly preset with a weekends-only calendar.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.quarterly_act360().calendar_id
        'weekends_only'
        """
        ...

    @staticmethod
    def semiannual_30360() -> ScheduleParams:
        """
        Semi-annual, 30/360, Modified Following, weekends-only calendar.

        Returns
        -------
        ScheduleParams
            Generic semi-annual preset with a weekends-only calendar.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.semiannual_30360().calendar_id
        'weekends_only'
        """
        ...

    @staticmethod
    def annual_actact() -> ScheduleParams:
        """
        Annual, Act/Act, Following, weekends-only calendar.

        Returns
        -------
        ScheduleParams
            Generic annual preset with a weekends-only calendar.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.annual_actact().calendar_id
        'weekends_only'
        """
        ...

    @staticmethod
    def usd_sofr_swap() -> ScheduleParams:
        """
        USD SOFR swap: quarterly, Act/360, MF, USNY, T+2 lag, adjusted accruals.

        Returns
        -------
        ScheduleParams
            Market-convention preset for USD SOFR swap legs.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.usd_sofr_swap().payment_lag_days
        2
        """
        ...

    @staticmethod
    def usd_corporate_bond() -> ScheduleParams:
        """
        USD corporate bond: semi-annual, 30/360, Following, USNY.

        Returns
        -------
        ScheduleParams
            Market-convention preset for USD corporate bonds.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.usd_corporate_bond().calendar_id
        'usny'
        """
        ...

    @staticmethod
    def usd_treasury() -> ScheduleParams:
        """
        USD Treasury: semi-annual, Act/Act ISMA, Following, USNY.

        Returns
        -------
        ScheduleParams
            Market-convention preset for US Treasury securities.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.usd_treasury().calendar_id
        'usny'
        """
        ...

    @staticmethod
    def eur_estr_swap() -> ScheduleParams:
        """
        EUR ESTR swap: annual, Act/360, MF, TARGET2, T+2 lag, adjusted accruals.

        Returns
        -------
        ScheduleParams
            Market-convention preset for EUR ESTR swap legs.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.eur_estr_swap().payment_lag_days
        2
        """
        ...

    @staticmethod
    def eur_gov_bond() -> ScheduleParams:
        """
        EUR government bond: annual, Act/Act ISMA, Following, TARGET2.

        Returns
        -------
        ScheduleParams
            Market-convention preset for EUR government bonds.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.eur_gov_bond() is not None
        True
        """
        ...

    @staticmethod
    def gbp_sonia_swap() -> ScheduleParams:
        """
        GBP SONIA swap: annual, Act/365F, MF, GBLO, no lag, adjusted accruals.

        Returns
        -------
        ScheduleParams
            Market-convention preset for GBP SONIA swap legs.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.gbp_sonia_swap().payment_lag_days
        0
        """
        ...

    @staticmethod
    def jpy_tona_swap() -> ScheduleParams:
        """
        JPY TONA swap: annual, Act/365F, MF, JPTO, T+2 lag, adjusted accruals.

        Returns
        -------
        ScheduleParams
            Market-convention preset for JPY TONA swap legs.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import ScheduleParams
        >>> ScheduleParams.jpy_tona_swap().payment_lag_days
        2
        """
        ...

    @property
    def freq(self) -> Tenor:
        """
        Accrual and payment frequency.

        Returns
        -------
        Tenor
            The frequency used to generate schedule boundaries.
        """
        ...

    @property
    def dc(self) -> DayCount:
        """
        Day-count convention.

        Returns
        -------
        DayCount
            The convention used to convert accrual periods to year
            fractions.
        """
        ...

    @property
    def calendar_id(self) -> str:
        """
        Holiday calendar identifier.

        Returns
        -------
        str
            The calendar id used together with the business-day convention.
        """
        ...

    @property
    def payment_lag_days(self) -> int:
        """
        Payment lag in business days.

        Returns
        -------
        int
            Business days added after the adjusted accrual end date.
        """
        ...

    @property
    def end_of_month(self) -> bool:
        """
        Whether end-of-month rolling is preserved.

        Returns
        -------
        bool
            ``True`` when schedule generation preserves end-of-month dates.
        """
        ...

    @property
    def adjust_accrual_dates(self) -> bool:
        """
        Whether accrual boundaries are business-day adjusted.

        Returns
        -------
        bool
            ``True`` for the swap/ISDA convention (both boundaries
            adjusted), ``False`` for the bond convention (only payment
            dates adjusted).
        """
        ...

class StepUpCouponSpec:
    """
    Step-up/step-down coupon specification.

    A fixed-style coupon whose rate changes at scheduled effective dates,
    used for step-up bonds and covenant-linked pricing grids.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import ScheduleParams, StepUpCouponSpec
    >>> spec = StepUpCouponSpec(
    ...     initial_rate=Decimal("0.04"),
    ...     step_schedule=[(datetime.date(2027, 1, 1), Decimal("0.045"))],
    ...     schedule=ScheduleParams.semiannual_30360(),
    ... )
    >>> spec is not None
    True
    """

    def __init__(
        self,
        initial_rate: Decimal | float,
        step_schedule: list[tuple[datetime.date, Decimal | float]],
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> None:
        """
        Construct a step-up/step-down coupon specification.

        Parameters
        ----------
        initial_rate : decimal.Decimal | float
            Rate in effect until the first step date.
        step_schedule : list[tuple[datetime.date, decimal.Decimal | float]]
            ``(effective_date, new_rate)`` pairs, strictly increasing by
            date; each rate is applied at its unadjusted accrual-period
            start.
        schedule : ScheduleParams
            Accrual and payment schedule conventions.
        coupon_type : CouponType, optional
            Cash (default), PIK, or split settlement.

        Raises
        ------
        ValueError
            If supplied inputs violate the documented type, shape, or
            domain constraints.
        """
        ...

    def validate(self) -> None:
        """
        Validate that step dates are strictly increasing.

        Raises
        ------
        ValueError
            If the step schedule dates are not strictly increasing.
        """
        ...
