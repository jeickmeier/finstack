"""
Schedule-driven accrued interest: methods, ex-coupon rules, accrual index.

Typed bindings for ``finstack_quant_cashflows::accrual``. These read a
``CashFlowSchedule`` only: coupon shape, PIK splits, amortization, and
ex-coupon rules are inferred from ``CFKind``-tagged flows and the schedule's
outstanding path, not from instrument specs.

Example::

    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.accrual import accrued_interest_amount
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule.builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build(None)
    ... )
    >>> accrued_interest_amount(schedule, datetime.date(2025, 4, 15))
    12500.0

Examples
--------
>>> from finstack_quant.cashflows.accrual import AccrualMethod
>>> str(AccrualMethod.LINEAR)
'AccrualMethod(Linear)'

"""

from __future__ import annotations

import datetime

from finstack_quant.cashflows.builder import CashFlowSchedule
from finstack_quant.core.dates import Tenor

__all__ = [
    "AccrualConfig",
    "AccrualIndex",
    "AccrualMethod",
    "ExCouponRule",
    "accrued_interest_amount",
]

class AccrualMethod:
    """
    Generic accrual method usable across instruments.

    Mirrors the semantics of bond accrual methods but is defined at the
    cashflow layer so it can be reused by any instrument that exposes a
    ``CashFlowSchedule``.

    Examples
    --------
    >>> from finstack_quant.cashflows.accrual import AccrualMethod
    >>> AccrualMethod.LINEAR != AccrualMethod.COMPOUNDED
    True
    """

    LINEAR: AccrualMethod
    """
    Linear accrual (simple interest interpolation), the default.

    ``Accrued = Coupon x (elapsed / period)``. ICMA Rule 251.1 prescribes
    linear accrual for bond accrued-interest calculations; use this method
    for bond-style instruments.
    """

    COMPOUNDED: AccrualMethod
    """
    Compounded accrual.

    ``Accrued = N x [(1 + r)^f - 1]``, where ``r`` is the per-period coupon
    rate and ``f`` is the elapsed fraction of the period. This variant uses
    true exponential compounding and is **not** ICMA-compliant; do not cite
    it as ICMA-style accrual. It is intended for instruments that genuinely
    compound within a coupon period (e.g. some leveraged loans). Use
    ``AccrualMethod.LINEAR`` for bond markets that follow ICMA Rule 251.1.
    """

class ExCouponRule:
    """
    Ex-coupon convention applied to coupon flows.

    From the ex-coupon date (inclusive) until the coupon payment date
    (exclusive), the instrument trades ex-coupon: the seller keeps the
    coupon and accrued interest is negative.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.accrual import ExCouponRule
    >>> rule = ExCouponRule(days_before_coupon=7)
    >>> rule.ex_date(datetime.date(2025, 7, 15))
    datetime.date(2025, 7, 8)
    """

    def __init__(
        self,
        days_before_coupon: int,
        calendar_id: str | None = None,
    ) -> None:
        """
        Construct an ex-coupon rule.

        Parameters
        ----------
        days_before_coupon : int
            Number of days before the coupon payment date that go ex (max
            366; larger values are rejected by :meth:`ex_date`).
        calendar_id : str, optional
            Business-day calendar identifier; when omitted, calendar days
            are used instead of business days.

        Raises
        ------
        ValueError
            If constructor inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.accrual import ExCouponRule
        >>> ExCouponRule(days_before_coupon=7).days_before_coupon
        7
        """
        ...

    @property
    def days_before_coupon(self) -> int:
        """
        Days before the coupon payment date that go ex.

        Returns
        -------
        int
            The configured number of days before the coupon date.
        """
        ...

    @property
    def calendar_id(self) -> str | None:
        """
        Optional business-day calendar identifier.

        Returns
        -------
        str or None
            The calendar id used to count business days, or ``None`` when
            calendar days are used instead.
        """
        ...

    def ex_date(self, payment_date: datetime.date) -> datetime.date:
        """
        Compute the ex-coupon date for a coupon paid on *payment_date*.

        Parameters
        ----------
        payment_date : datetime.date
            Payment date of the coupon this ex-coupon window precedes.

        Returns
        -------
        datetime.date
            The ex-coupon date; from this date (inclusive) until
            *payment_date* (exclusive), the instrument trades ex-coupon.

        Raises
        ------
        ValueError
            If ``days_before_coupon`` exceeds 366.
        KeyError
            If the configured calendar id cannot be resolved.
        """
        ...

class AccrualConfig:
    """
    Generic configuration for schedule-driven interest accrual.

    Bundles the accrual method, optional ex-coupon rule, whether PIK
    interest is included, and the coupon frequency needed for ACT/ACT ISMA
    day-count calculations.

    Examples
    --------
    >>> from finstack_quant.cashflows.accrual import AccrualConfig, AccrualMethod
    >>> repr(AccrualConfig(method=AccrualMethod.LINEAR)).startswith("AccrualConfig(")
    True
    """

    def __init__(
        self,
        method: AccrualMethod | None = None,
        ex_coupon: ExCouponRule | None = None,
        include_pik: bool = True,
        frequency: Tenor | str | None = None,
    ) -> None:
        """
        Construct an accrual configuration.

        Parameters
        ----------
        method : AccrualMethod, optional
            Linear (default; ICMA 251.1) or compounded accrual (not
            ICMA-compliant — see :class:`AccrualMethod`).
        ex_coupon : ExCouponRule, optional
            Ex-coupon window rule; omit for instruments with no ex-coupon
            convention.
        include_pik : bool
            Whether to include PIK interest in the accrued amount, default
            ``True``.
        frequency : Tenor or str, optional
            Coupon frequency, required only when the schedule uses ACT/ACT
            ISMA day count.

        Raises
        ------
        ValueError
            If constructor inputs violate the documented type or domain
            constraints.

        Examples
        --------
        >>> from finstack_quant.cashflows.accrual import AccrualConfig
        >>> repr(AccrualConfig()).startswith("AccrualConfig(")
        True
        """
        ...

class AccrualIndex:
    """
    Precomputed accrual state for repeated ``accrued_at`` queries.

    Builds coupon periods and the outstanding path once for a
    ``(schedule, config)`` pair. Prefer this over
    :func:`accrued_interest_amount` when accruing the same schedule on many
    dates.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.accrual import AccrualIndex
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build(None)
    ... )
    >>> index = AccrualIndex.build(schedule)
    >>> index.accrued_at(datetime.date(2025, 4, 15))
    12500.0
    """

    @classmethod
    def build(
        cls,
        schedule: CashFlowSchedule,
        config: AccrualConfig | None = None,
    ) -> AccrualIndex:
        """
        Build reusable accrual state for repeated ``accrued_at`` queries.

        Parameters
        ----------
        schedule : CashFlowSchedule
            Canonical cashflow schedule containing coupon, PIK, and notional
            flows.
        config : AccrualConfig, optional
            Accrual method and ex-coupon configuration bound into the index
            (default linear, PIK included). Build a separate index to
            accrue under a different config.

        Returns
        -------
        AccrualIndex
            Prebuilt accrual state; call :meth:`accrued_at` for repeated
            queries against the same schedule and config.

        Raises
        ------
        ValueError
            If the schedule fails validation, mixes currencies across
            coupon flows, or carries a non-finite accrual factor.

        Examples
        --------
        >>> import datetime
        >>> from decimal import Decimal
        >>> from finstack_quant.cashflows.accrual import AccrualIndex
        >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
        >>> from finstack_quant.core.money import Money
        >>> schedule = (
        ...     CashFlowSchedule
        ...     .builder()
        ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
        ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
        ...     .build(None)
        ... )
        >>> AccrualIndex.build(schedule) is not None
        True
        """
        ...

    def accrued_at(self, as_of: datetime.date) -> float:
        """
        Accrued interest as of *as_of* using the prebuilt periods.

        Parameters
        ----------
        as_of : datetime.date
            Accrual cut-off date; dates outside all coupon periods return
            ``0.0``.

        Returns
        -------
        float
            Accrued interest in the schedule's currency space; negative
            inside an active ex-coupon window.

        Raises
        ------
        KeyError
            If a configured ex-coupon calendar id cannot be resolved.
        """
        ...

def accrued_interest_amount(
    schedule: CashFlowSchedule,
    as_of: datetime.date,
    config: AccrualConfig | None = None,
) -> float:
    """
    Compute accrued interest for a schedule as of *as_of*.

    Parameters
    ----------
    schedule : CashFlowSchedule
        Canonical cashflow schedule containing coupon, PIK, and notional
        flows.
    as_of : datetime.date
        Accrual cut-off date; dates outside all coupon periods return
        ``0.0``.
    config : AccrualConfig, optional
        Accrual method and ex-coupon configuration (default linear, PIK
        included).

    Returns
    -------
    float
        Accrued interest in the schedule's currency space; negative inside
        an active ex-coupon window.

    Raises
    ------
    ValueError
        If the schedule fails validation, mixes currencies across coupon
        flows, or carries a non-finite accrual factor.
    KeyError
        If a configured ex-coupon calendar id cannot be resolved.

    Examples
    --------
    >>> import datetime
    >>> from decimal import Decimal
    >>> from finstack_quant.cashflows.accrual import accrued_interest_amount
    >>> from finstack_quant.cashflows.builder import CashFlowSchedule, FixedCouponSpec, ScheduleParams
    >>> from finstack_quant.core.money import Money
    >>> schedule = (
    ...     CashFlowSchedule
    ...     .builder()
    ...     .principal(Money(1_000_000.0, "USD"), datetime.date(2025, 1, 15), datetime.date(2026, 1, 15))
    ...     .fixed_cf(FixedCouponSpec(rate=Decimal("0.05"), schedule=ScheduleParams.semiannual_30360()))
    ...     .build(None)
    ... )
    >>> accrued_interest_amount(schedule, datetime.date(2025, 4, 15))
    12500.0
    """
    ...
