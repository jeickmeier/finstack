"""
Composable cashflow builder: coupon/fee/amortization specs, CashFlowBuilder, CashFlowSchedule.

Typed bindings for the spec types under ``finstack_quant_cashflows::builder``:
schedule generation parameters, fixed/floating/step-up coupon specs,
amortization and notional rules, fee specs, and the credit behavior models
(prepayment, default, recovery) used by structured/private-credit cashflow
programs. ``CashFlowSchedule.builder()`` is the fluent entry point into
``CashFlowBuilder``, whose chained methods assemble a principal, coupon
legs, fees, and principal events into a canonical ``CashFlowSchedule``.
Fuller schedule accessors (notional, day count, metadata, validation) are
added in a later task; ``get_flows()`` is available today. The canonical
JSON program bridge on ``finstack_quant.cashflows`` remains available for
the full build pipeline as well.

Examples
--------
>>> import finstack_quant.cashflows.builder as builder
>>> builder.__name__
'finstack_quant.cashflows.builder'
"""

from __future__ import annotations

import datetime
from decimal import Decimal

import pandas as pd

from finstack_quant.cashflows.primitives import CashFlow, CFKind
from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import BusinessDayConvention, DayCount, Period, StubKind, Tenor
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money

__all__ = [
    "AmortizationSpec",
    "CashFlowBuilder",
    "CashFlowMeta",
    "CashFlowSchedule",
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
    "PrincipalEvent",
    "RecoveryModelSpec",
    "RollRule",
    "ScheduleParams",
    "StepUpCouponSpec",
    "merge_cashflow_schedules",
]

class AmortizationSpec:
    """
    Amortization rule for principal over the life of a cashflow leg.

    Immutable, hashable value type. ``NONE`` is a class-attribute singleton
    for the fieldless variant; the remaining factory functions each
    construct a data-carrying Rust ``AmortizationSpec`` variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import AmortizationSpec
    >>> AmortizationSpec.NONE is not None
    True
    """

    NONE: AmortizationSpec
    """No amortization; principal remains constant until final redemption."""

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

class CashFlowBuilder:
    """
    Fluent builder assembling a principal, coupon legs, fees, and principal
    events into a :class:`CashFlowSchedule`.

    Created only via :meth:`CashFlowSchedule.builder`; there is no direct
    constructor. Every chaining method mutates the builder in place and
    returns that same instance, matching the Rust ``&mut self`` fluent API.
    Configuration errors on individual calls (e.g. an invalid principal
    event sign) are deferred and surfaced from :meth:`build`.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
    ...     .build()
    ... )
    >>> len(schedule.get_flows())
    6
    """

    def principal(
        self,
        initial: Money,
        issue_date: datetime.date,
        maturity: datetime.date,
    ) -> CashFlowBuilder:
        """
        Set the principal amount and instrument horizon.

        Parameters
        ----------
        initial : Money
            Initial notional amount outstanding at issue.
        issue_date : datetime.date
            Issue date; also the date of the initial funding cashflow.
        maturity : datetime.date
            Final maturity date of the instrument.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If *issue_date* is not strictly before *maturity*, or a date
            cannot be converted from the supplied value.
        """
        ...

    def amortization(self, spec: AmortizationSpec) -> CashFlowBuilder:
        """
        Configure amortization for the instrument notional.

        Parameters
        ----------
        spec : AmortizationSpec
            Amortization rule applied to the principal over the schedule.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If the amortization schedule is inconsistent with the
            principal set via :meth:`principal`.
        """
        ...

    def add_principal_event(
        self,
        date: datetime.date,
        delta: Money,
        kind: CFKind | str,
        cash: Money | None = None,
    ) -> CashFlowBuilder:
        """
        Add a single principal event (draw, repayment, or exchange).

        Parameters
        ----------
        date : datetime.date
            Event date.
        delta : Money
            Outstanding delta; sign convention depends on *kind* (e.g.
            ``CFKind.NOTIONAL`` draws require ``delta >= 0``).
        kind : CFKind | str
            Classification for the emitted cashflow. Required; matches the
            Rust API, which has no default.
        cash : Money, optional
            Cash leg paid or received, when it differs from *delta*
            (default: equal to *delta*).

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If *delta* violates the sign convention for *kind*, or *cash*
            is supplied in a different currency than *delta*. The error is
            deferred and raised from :meth:`build`.
        """
        ...

    def fixed_cf(self, spec: FixedCouponSpec) -> CashFlowBuilder:
        """
        Add a full-horizon fixed-rate coupon leg.

        Parameters
        ----------
        spec : FixedCouponSpec
            Fixed coupon rate, settlement, and schedule conventions.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If the coupon schedule cannot be generated between the
            principal's issue and maturity dates. The error is deferred
            and raised from :meth:`build`.
        """
        ...

    def floating_cf(self, spec: FloatingCouponSpec) -> CashFlowBuilder:
        """
        Add a full-horizon floating-rate coupon leg.

        Parameters
        ----------
        spec : FloatingCouponSpec
            Floating rate index, settlement, and schedule conventions.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If the coupon schedule cannot be generated between the
            principal's issue and maturity dates. The error is deferred
            and raised from :meth:`build`.
        """
        ...

    def step_up_cf(self, spec: StepUpCouponSpec) -> CashFlowBuilder:
        """
        Add a full-horizon step-up/step-down coupon leg.

        Parameters
        ----------
        spec : StepUpCouponSpec
            Initial rate, step schedule, and schedule conventions.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If the coupon schedule cannot be generated between the
            principal's issue and maturity dates. The error is deferred
            and raised from :meth:`build`.
        """
        ...

    def fee(self, spec: FeeSpec) -> CashFlowBuilder:
        """
        Add a fee specification (one-time fixed or periodic basis-point).

        Parameters
        ----------
        spec : FeeSpec
            Fixed or periodic basis-point fee specification.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If a periodic fee's schedule cannot be generated. The error is
            deferred and raised from :meth:`build`.
        """
        ...

    def add_fixed_window(
        self,
        start: datetime.date,
        end: datetime.date,
        spec: FixedCouponSpec,
    ) -> CashFlowBuilder:
        """
        Add a fixed-rate coupon over the half-open window ``[start, end)``.

        Parameters
        ----------
        start : datetime.date
            Window start date (inclusive).
        end : datetime.date
            Window end date (exclusive).
        spec : FixedCouponSpec
            Fixed coupon rate, settlement, and schedule conventions applied
            within the window.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If *start* is not strictly before *end*, or the coupon
            schedule cannot be generated over the window.
        """
        ...

    def add_floating_window(
        self,
        start: datetime.date,
        end: datetime.date,
        spec: FloatingCouponSpec,
    ) -> CashFlowBuilder:
        """
        Add a floating-rate coupon over the half-open window ``[start, end)``.

        Parameters
        ----------
        start : datetime.date
            Window start date (inclusive).
        end : datetime.date
            Window end date (exclusive).
        spec : FloatingCouponSpec
            Floating rate index, settlement, and schedule conventions
            applied within the window.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If *start* is not strictly before *end*, or the coupon
            schedule cannot be generated over the window.
        """
        ...

    def add_payment_window(
        self,
        start: datetime.date,
        end: datetime.date,
        split: CouponType,
    ) -> CashFlowBuilder:
        """
        Set the payment split (cash/PIK/split) over ``[start, end)``.

        Parameters
        ----------
        start : datetime.date
            Window start date (inclusive).
        end : datetime.date
            Window end date (exclusive).
        split : CouponType
            Settlement split applied to coupons accruing within the window.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If *start* is not strictly before *end*.
        """
        ...

    def payment_split_program(
        self,
        steps: list[tuple[datetime.date, CouponType]],
    ) -> CashFlowBuilder:
        """
        Configure a payment split program from ordered boundary dates.

        Parameters
        ----------
        steps : list[tuple[datetime.date, CouponType]]
            Ordered ``(effective_date, split)`` pairs describing successive
            cash/PIK toggle windows.

        Returns
        -------
        CashFlowBuilder
            This same builder, for chaining.

        Raises
        ------
        ValueError
            If the step dates are not strictly increasing.
        """
        ...

    def build(self, market: MarketContext | None = None) -> CashFlowSchedule:
        """
        Compile the configured legs into a canonical cashflow schedule.

        Parameters
        ----------
        market : MarketContext, optional
            Market context supplying forward curves for floating-rate
            projection. Fixed coupons and deterministic fees do not
            require one.

        Returns
        -------
        CashFlowSchedule
            The compiled, deterministically ordered cashflow schedule.

        Raises
        ------
        KeyError
            If required inputs are missing, such as an unset principal.
        ValueError
            If a spec or date validation fails, including any deferred
            error recorded by an earlier fluent call, or a floating-rate
            projection fails and its fallback policy does not resolve it.
        """
        ...

    def __repr__(self) -> str: ...

class CashFlowMeta:
    """
    Metadata shared by an entire cashflow schedule.

    Tracks the schedule's representation label, referenced calendar IDs,
    optional facility limit, and instrument issue/maturity dates. Obtained
    via :meth:`CashFlowSchedule.get_meta`; there is no public constructor.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
    ...     .build()
    ... )
    >>> schedule.get_meta().issue_date
    datetime.date(2025, 1, 15)
    """

    @property
    def representation(self) -> str:
        """
        Schedule representation label.

        Returns
        -------
        str
            One of ``"contractual"``, ``"projected"``, ``"placeholder"``,
            or ``"no_residual"``, describing the schedule's meaning
            relative to waterfall policy.
        """
        ...

    @property
    def calendar_ids(self) -> list[str]:
        """
        Holiday calendar identifiers used by the schedule.

        Returns
        -------
        list[str]
            Calendar ids referenced while generating schedule dates.
        """
        ...

    @property
    def facility_limit(self) -> Money | None:
        """
        Optional facility limit / commitment.

        Returns
        -------
        Money, optional
            The facility limit for instruments like RCFs, or ``None`` when
            not applicable.
        """
        ...

    @property
    def issue_date(self) -> datetime.date | None:
        """
        Instrument issue date, when known.

        Returns
        -------
        datetime.date, optional
            The issue date used by the accrual engine to establish the
            first coupon period start, or ``None`` when unset.
        """
        ...

    @property
    def maturity_date(self) -> datetime.date | None:
        """
        Contractual maturity date, when known.

        Returns
        -------
        datetime.date, optional
            The final maturity date, or ``None`` when unset.
        """
        ...

    def __repr__(self) -> str: ...

class CashFlowSchedule:
    """
    Canonical, deterministically ordered cashflow schedule.

    The output of :meth:`builder`, which is the only construction path.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
    ...     .build()
    ... )
    >>> len(schedule.get_flows())
    6
    """

    @staticmethod
    def builder() -> CashFlowBuilder:
        """
        Create a new fluent cashflow builder (the only builder entry point).

        Returns
        -------
        CashFlowBuilder
            A fresh, empty builder ready for chained configuration.

        Examples
        --------
        >>> from finstack_quant.cashflows.builder import CashFlowSchedule
        >>> CashFlowSchedule.builder() is not None
        True
        """
        ...

    def get_flows(self) -> list[CashFlow]:
        """
        Return the canonical, deterministically ordered cashflows.

        Returns
        -------
        list[CashFlow]
            All cashflows in the schedule (coupons, principal payments,
            fees) sorted into deterministic schedule order.
        """
        ...

    def coupons(self) -> list[CashFlow]:
        """
        Interest-like coupon cashflows.

        Returns
        -------
        list[CashFlow]
            Flows whose kind is Fixed, FloatReset, InflationCoupon, or
            Stub, in schedule order.
        """
        ...

    def dates(self) -> list[datetime.date]:
        """
        Return the list of dates for all flows in schedule order.

        Returns
        -------
        list[datetime.date]
            One date per flow, matching the order of :meth:`get_flows`.
        """
        ...

    def get_notional(self) -> Notional:
        """
        Return the schedule notional.

        Returns
        -------
        Notional
            The representative notional amount and amortization rule.
        """
        ...

    def get_day_count(self) -> DayCount:
        """
        Return the representative day-count convention.

        Returns
        -------
        DayCount
            The day-count convention recorded on the schedule.
        """
        ...

    def get_meta(self) -> CashFlowMeta:
        """
        Return schedule-level metadata.

        Returns
        -------
        CashFlowMeta
            The representation label, calendar ids, facility limit, and
            issue/maturity dates recorded on the schedule.
        """
        ...

    def validate(self) -> None:
        """
        Validate all schedule-level and per-flow invariants.

        Checks the representative notional, each flow, nondecreasing flow
        dates, and cross-flow economic invariants reconciling funding,
        outstanding balances, currencies, and recorded metadata.

        Raises
        ------
        ValueError
            If the notional, any flow, the date ordering, or a cross-flow
            economic invariant is violated.
        """
        ...

    def scale_amounts(self, scale: float) -> CashFlowSchedule:
        """
        Return a new schedule with every amount scaled by ``scale``.

        Parameters
        ----------
        scale : float
            Multiplier applied to every flow amount; a negative value
            reverses the cashflow direction.

        Returns
        -------
        CashFlowSchedule
            A new schedule with scaled amounts; the representative
            notional and flow classification/date metadata are unchanged.

        Raises
        ------
        ValueError
            If *scale* is NaN or infinite.
        """
        ...

    def weighted_average_life(self, as_of: datetime.date) -> float:
        """
        Weighted Average Life (WAL) in years from ``as_of``.

        Computed on an Act/365F basis regardless of the schedule's accrual
        day count, matching conventional desk reporting.

        Parameters
        ----------
        as_of : datetime.date
            Valuation date from which the WAL is measured.

        Returns
        -------
        float
            The weighted average life in years; ``0.0`` if there are no
            future principal flows.

        Raises
        ------
        ValueError
            If the day-count year-fraction calculation fails.
        """
        ...

    def outstanding_by_date(self) -> list[tuple[datetime.date, Money]]:
        """
        Outstanding principal balance path over the schedule's life.

        Returns
        -------
        list[tuple[datetime.date, Money]]
            ``(date, outstanding_balance)`` pairs in schedule order, one
            per distinct flow date.

        Raises
        ------
        ValueError
            If ``meta.issue_date`` is unset, or a currency mismatch is
            found between flows and the notional.
        """
        ...

    def pv_by_period(
        self,
        periods: list[Period],
        market: MarketContext,
        disc_curve_id: str,
        base: datetime.date,
        day_count: DayCount | None = None,
        hazard_curve_id: str | None = None,
    ) -> dict[str, dict[str, Money]]:
        """
        Periodized present values resolved from a market context.

        Parameters
        ----------
        periods : list[Period]
            Reporting periods that define the output buckets (half-open
            boundaries).
        market : MarketContext
            Market context containing the required discount (and optional
            hazard) curves.
        disc_curve_id : str
            Discount curve identifier.
        base : datetime.date
            Valuation date used to convert cashflow dates into discount
            times.
        day_count : DayCount, optional
            Day-count convention for discount times (default Act/365F).
        hazard_curve_id : str, optional
            Hazard curve identifier for credit-adjusted present value.

        Returns
        -------
        dict[str, dict[str, Money]]
            Mapping from period id label (e.g. ``"2025Q1"``) to a mapping
            of currency code to the present-value total for that period
            and currency; periods with no flows are omitted.

        Raises
        ------
        KeyError
            If the discount or hazard curve id cannot be found in *market*.
        ValueError
            If day-count conversion fails or credit-adjusted inputs are
            internally inconsistent.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize the canonical schedule to JSON.

        Returns
        -------
        str
            Canonical JSON encoding of the schedule, compatible with
            :func:`finstack_quant.cashflows.validate_cashflow_schedule_json`
            and other JSON-bridge functions on
            ``finstack_quant.cashflows``.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CashFlowSchedule:
        """
        Deserialize a schedule from canonical JSON.

        Parameters
        ----------
        json : str
            JSON-encoded ``CashFlowSchedule`` with strict field names.

        Returns
        -------
        CashFlowSchedule
            The deserialized schedule.

        Raises
        ------
        ValueError
            If *json* is not valid JSON or does not match the canonical
            ``CashFlowSchedule`` schema (unknown fields are rejected).

        Examples
        --------
        >>> import datetime
        >>> from decimal import Decimal
        >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
        >>> from finstack_quant.core.money import Money
        >>> schedule = (
        ...     CashFlowSchedule
        ...     .builder()
        ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
        ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
        ...     .build()
        ... )
        >>> round_tripped = CashFlowSchedule.from_json(schedule.to_json())
        >>> len(round_tripped.get_flows())
        6
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Flows as a pandas DataFrame.

        Returns
        -------
        pandas.DataFrame
            One row per flow with columns ``date``, ``kind``, ``amount``,
            ``currency``, in schedule order.
        """
        ...

    def __repr__(self) -> str: ...

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

    ``DRAWN`` is a class-attribute singleton for the fieldless variant;
    :meth:`undrawn` constructs the data-carrying ``Undrawn`` variant.

    Examples
    --------
    >>> from finstack_quant.cashflows.builder import FeeBase
    >>> FeeBase.DRAWN is not None
    True
    """

    DRAWN: FeeBase
    """Fee accrues on the drawn outstanding balance."""

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

    Constructed via :meth:`fixed` or :meth:`periodic_bp`, mirroring the
    two Rust ``FeeSpec`` variants. Negative amounts and bp quotes are
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
    def periodic_bp(
        base: FeeBase,
        bp: Decimal | float,
        frequency: Tenor | str,
        day_count: DayCount,
        business_day_convention: BusinessDayConvention | str,
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
        bp : decimal.Decimal | float
            Fee quote in basis points per annum.
        frequency : Tenor | str
            Accrual and payment frequency for the fee schedule.
        day_count : DayCount
            Day-count convention used to annualize the fee accrual.
        business_day_convention : BusinessDayConvention | str
            Business-day convention applied to generated fee dates.
        calendar_id : str
            Holiday calendar id used with *business_day_convention*.
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
        >>> spec = FeeSpec.periodic_bp(
        ...     base=FeeBase.undrawn(facility_limit=Money(10_000_000.0, "USD")),
        ...     bp=50,
        ...     frequency=Tenor.quarterly(),
        ...     day_count=DayCount.ACT_360,
        ...     business_day_convention="modified_following",
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
    >>> rate_spec = FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("200"), reset_frequency="3M")
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
    >>> spec = FloatingRateSpec(index_id="USD-SOFR-3M", spread_bp=Decimal("200"), reset_frequency="3M")
    >>> spec.index_id
    'USD-SOFR-3M'
    """

    def __init__(
        self,
        index_id: str,
        spread_bp: Decimal | float,
        reset_frequency: Tenor | str,
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
        reset_frequency : Tenor | str
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
            differs from *reset_frequency*.
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

class PrincipalEvent:
    """
    Principal event applied during schedule build (a draw or a repayment).

    Constructed directly or emitted internally by :meth:`CashFlowBuilder.principal`
    and :meth:`CashFlowBuilder.add_principal_event`; ``delta`` is the source
    of truth for outstanding-balance movement, while ``cash`` is the
    settled cash leg (which can differ for OID/fee-adjusted draws).

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.builder import PrincipalEvent
    >>> from finstack_quant.cashflows.primitives import CFKind
    >>> from finstack_quant.core.money import Money
    >>> event = PrincipalEvent(
    ...     datetime.date(2025, 1, 15), Money(1_000_000.0, "USD"), Money(1_000_000.0, "USD"), CFKind.NOTIONAL
    ... )
    >>> event.kind == CFKind.NOTIONAL
    True
    """

    def __init__(
        self,
        date: datetime.date,
        delta: Money,
        cash: Money,
        kind: CFKind | str,
    ) -> None:
        """
        Construct a principal event.

        Parameters
        ----------
        date : datetime.date
            Event date.
        delta : Money
            Outstanding delta (positive increases the balance, negative
            repays it).
        cash : Money
            Cash leg paid or received; may differ from *delta* for
            OID/fee-adjusted draws.
        kind : CFKind | str
            Classification for the emitted cashflow.

        Raises
        ------
        ValueError
            If a date cannot be converted from the supplied value, or
            *kind* is not a recognized ``CFKind`` label.
        """
        ...

    @property
    def date(self) -> datetime.date:
        """
        Date on which this principal event occurs.

        Returns
        -------
        datetime.date
            The date on which this principal event occurs.
        """
        ...

    @property
    def delta(self) -> Money:
        """
        Outstanding delta.

        Returns
        -------
        Money
            The balance movement (positive increases, negative repays).
        """
        ...

    @property
    def cash(self) -> Money:
        """
        Cash leg paid or received.

        Returns
        -------
        Money
            The settled cash amount, which may differ from *delta*.
        """
        ...

    @property
    def kind(self) -> CFKind:
        """
        Emitted cashflow classification.

        Returns
        -------
        CFKind
            The classification recorded on the emitted cashflow.
        """
        ...

    def __repr__(self) -> str: ...

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
        frequency: Tenor | str,
        day_count: DayCount,
        calendar_id: str,
        business_day_convention: BusinessDayConvention | str | None = None,
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
        frequency : Tenor | str
            Accrual and payment frequency (e.g. ``"3M"``).
        day_count : DayCount
            Day-count convention for accrual year fractions.
        calendar_id : str
            Holiday calendar id (``"weekends_only"`` for weekend-only
            rolling).
        business_day_convention : BusinessDayConvention | str, optional
            Payment-date rolling convention (default Modified Following).
        stub : StubKind, optional
            Stub rule (default short-front).
        end_of_month : bool
            Preserve end-of-month rolling (default ``False``).
        payment_lag_days : int
            Payment lag in business days (default ``0``).
        adjust_accrual_dates : bool
            Roll accrual boundaries with *business_day_convention*, i.e. swap/ISDA convention
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
    def frequency(self) -> Tenor:
        """
        Accrual and payment frequency.

        Returns
        -------
        Tenor
            The frequency used to generate schedule boundaries.
        """
        ...

    @property
    def day_count(self) -> DayCount:
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

def merge_cashflow_schedules(
    schedules: list[CashFlowSchedule],
    notional: Notional,
    day_count: DayCount,
) -> CashFlowSchedule:
    """
    Merge multiple schedules into one deterministic composite schedule.

    Parameters
    ----------
    schedules : list[CashFlowSchedule]
        Schedules to combine.
    notional : Notional
        Representative notional stamped on the merged schedule.
    day_count : DayCount
        Day-count convention attached to the merged schedule.

    Returns
    -------
    CashFlowSchedule
        The merged schedule. Flows from all inputs are combined and
        re-sorted into canonical order. Metadata is merged: ``representation``
        takes the most conservative value across inputs (``Projected``
        dominates, then ``Placeholder``, ``Contractual``, ``NoResidual``);
        ``calendar_ids`` is the sorted, deduplicated union; ``facility_limit``
        and ``issue_date`` are kept only when every input agrees, otherwise
        ``None``. Empty input yields an empty schedule with default metadata.

    Raises
    ------
    TypeError
        If an element of *schedules* is not a ``CashFlowSchedule``, or
        *notional* / *day_count* are not the expected types.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.builder import (
    ...     CashFlowSchedule,
    ...     FixedCouponSpec,
    ...     Notional,
    ...     ScheduleParams,
    ...     merge_cashflow_schedules,
    ... )
    >>> from finstack_quant.core.dates import DayCount
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.quarterly_act360()))
    ...     .build()
    ... )
    >>> merged = merge_cashflow_schedules([schedule], Notional.par(1_000_000.0, "USD"), DayCount.ACT_360)
    >>> len(merged.get_flows())
    6
    """
    ...
