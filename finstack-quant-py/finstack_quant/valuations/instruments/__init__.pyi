"""
Python bindings for the corresponding finstack-quant Rust API.

Examples
--------
>>> import finstack_quant.valuations.instruments as instruments
>>> instruments.__name__
'finstack_quant.valuations.instruments'
"""

from __future__ import annotations

import datetime
from typing import Literal

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import DayCount, Tenor
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.types import Bps, Rate

__all__ = [
    "Bond",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSTranche",
    "CDSTrancheBuilder",
    "CapFloor",
    "CapFloorBuilder",
    "ConvertibleBond",
    "ConvertibleBondBuilder",
    "CreditDefaultSwap",
    "CreditDefaultSwapBuilder",
    "EquityOption",
    "EquityOptionBuilder",
    "FixedLegSpec",
    "FloatLegSpec",
    "FxForward",
    "FxForwardBuilder",
    "FxOption",
    "FxOptionBuilder",
    "InterestRateSwap",
    "InterestRateSwapBuilder",
    "PremiumLegSpec",
    "ProtectionLegSpec",
    "Swaption",
    "SwaptionBuilder",
    "TermLoan",
    "bond_from_cashflows_json",
    "instrument_cashflows_json",
    "list_models",
    "list_models_grouped",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "price_instrument",
    "price_instrument_with_metrics",
    "structured_credit_tranche_breakeven_cdr",
    "structured_credit_tranche_discount_margin",
    "structured_credit_tranche_metrics",
    "structured_credit_tranche_oas",
    "structured_credit_tranche_scenario_table",
    "validate_instrument_json",
]

class Bond:
    """
    Typed wrapper for the canonical Rust ``Bond`` instrument.

    Construct via :meth:`Bond.fixed`, :meth:`Bond.floating`, or
    :meth:`Bond.from_json`; serialize with :meth:`Bond.to_json`. Instances
    are accepted directly by :func:`price_instrument`,
    :func:`price_instrument_with_metrics`, and
    :func:`instrument_cashflows_json`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import Bond
    >>> bond = Bond.fixed(
    ...     "BOND-1",
    ...     Money(1_000_000.0, Currency("USD")),
    ...     Rate(0.05),
    ...     datetime.date(2024, 1, 1),
    ...     datetime.date(2034, 1, 1),
    ...     "USD-OIS",
    ... )
    >>> bond.id
    'BOND-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def fixed(
        id: str,
        notional: Money,
        coupon_rate: Rate,
        issue: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> Bond:
        """
        Create a standard fixed-rate bond (semi-annual, 30/360, T+2).

        Mirrors Rust ``Bond::fixed``.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        notional : Money
            Principal amount of the bond.
        coupon_rate : Rate
            Annual coupon rate.
        issue : datetime.date
            Issue date.
        maturity : datetime.date
            Maturity date.
        discount_curve_id : str
            Discount curve identifier used for pricing.

        Returns
        -------
        Bond
            A validated fixed-rate bond.

        Raises
        ------
        ValueError
            If validation fails (e.g. maturity not after issue).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Bond
        >>> callable(Bond.fixed)
        True
        """
        ...

    @staticmethod
    def floating(
        id: str,
        notional: Money,
        index_id: str,
        margin_bp: Bps,
        issue: datetime.date,
        maturity: datetime.date,
        freq: Tenor,
        dc: DayCount,
        discount_curve_id: str,
    ) -> Bond:
        """
        Create a floating-rate bond (FRN) linked to a forward index.

        Mirrors Rust ``Bond::floating``.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        notional : Money
            Principal amount of the bond.
        index_id : str
            Forward curve identifier (e.g. ``"USD-SOFR-3M"``).
        margin_bp : Bps
            Spread over the index in basis points.
        issue : datetime.date
            Issue date.
        maturity : datetime.date
            Maturity date.
        freq : Tenor
            Payment frequency (e.g. ``Tenor.quarterly()``).
        dc : DayCount
            Day count convention (e.g. ``DayCount.act360()``).
        discount_curve_id : str
            Discount curve identifier used for pricing.

        Returns
        -------
        Bond
            A validated floating-rate note.

        Raises
        ------
        ValueError
            If validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Bond
        >>> callable(Bond.floating)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> Bond:
        """
        Deserialize a bond from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"bond"``
            (``{"type": "bond", "spec": {...}}``).

        Returns
        -------
        Bond
            The validated bond.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Bond
        >>> callable(Bond.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "bond", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`Bond.from_json`.
        """
        ...

class TermLoan:
    """
    Typed wrapper for the canonical Rust ``TermLoan`` instrument.

    Rust has no ``fixed``/``floating`` convenience constructors for term
    loans; construct via :meth:`TermLoan.from_json` with tagged JSON
    (``{"type": "term_loan", "spec": ...}``) or start from
    :meth:`TermLoan.example`. Instances are accepted directly by
    :func:`price_instrument`, :func:`price_instrument_with_metrics`, and
    :func:`instrument_cashflows_json`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import TermLoan
    >>> loan = TermLoan.example()
    >>> loan.id
    'TERM-LOAN-USD-5Y'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> TermLoan:
        """
        Deserialize a term loan from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"term_loan"``
            (``{"type": "term_loan", "spec": {...}}``).

        Returns
        -------
        TermLoan
            The validated term loan.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import TermLoan
        >>> callable(TermLoan.from_json)
        True
        """
        ...

    @staticmethod
    def example() -> TermLoan:
        """
        Canonical example term loan (mirrors Rust ``TermLoan::example``).

        Returns
        -------
        TermLoan
            A 5-year USD fixed-rate loan (6%, quarterly, Act/360, 2.5%
            per-period amortization).

        Raises
        ------
        ValueError
            If construction fails (should not occur).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import TermLoan
        >>> TermLoan.example().id
        'TERM-LOAN-USD-5Y'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "term_loan", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`TermLoan.from_json`.
        """
        ...

class FixedLegSpec:
    """
    Fixed leg of an interest-rate swap.

    Thin typed wrapper for the canonical Rust ``FixedLegSpec``. Used to build
    the fixed leg of an :class:`InterestRateSwap` via
    :meth:`InterestRateSwapBuilder.fixed`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import FixedLegSpec
    >>> leg = FixedLegSpec(
    ...     "USD-OIS",
    ...     0.04,
    ...     Tenor.semi_annual(),
    ...     DayCount.THIRTY_360,
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ...     compounding_simple=False,
    ... )
    >>> "0.04" in repr(leg)
    True
    """

    def __init__(
        self,
        discount_curve_id: str,
        rate: float,
        frequency: Tenor,
        day_count: DayCount,
        start: datetime.date,
        end: datetime.date,
        *,
        compounding_simple: bool,
        bdc: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
        stub: Literal["ShortFront", "ShortBack", "LongFront", "LongBack"] = "ShortFront",
        payment_lag_days: int = 0,
        end_of_month: bool = False,
    ) -> None:
        """
        Construct a fixed leg specification.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        rate : float
            Fixed rate as a decimal (0.04 = 4%).
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        start : datetime.date
            Start date of the fixed leg.
        end : datetime.date
            End date of the fixed leg.
        compounding_simple : bool
            If true, use simple interest on the accrual fraction. Required:
            the canonical Rust ``FixedLegSpec`` field has no default.
        bdc : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.
        stub : {"ShortFront", "ShortBack", "LongFront", "LongBack"}, default "ShortFront"
            Stub period handling rule.
        payment_lag_days : int, default 0
            Payment lag in business days after period end.
        end_of_month : bool, default False
            End-of-month roll convention.

        Raises
        ------
        ValueError
            If an enum value is invalid or the accrual period is malformed
            (``start >= end``).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FixedLegSpec
        >>> callable(FixedLegSpec)
        True
        """
        ...

class FloatLegSpec:
    """
    Floating leg of an interest-rate swap.

    Thin typed wrapper for the canonical Rust ``FloatLegSpec``. Used to build
    the floating leg of an :class:`InterestRateSwap` via
    :meth:`InterestRateSwapBuilder.float`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import FloatLegSpec
    >>> leg = FloatLegSpec(
    ...     "USD-OIS",
    ...     "USD-SOFR-3M",
    ...     0.0,
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ... )
    >>> "spread_bp=0" in repr(leg)
    True
    """

    def __init__(
        self,
        discount_curve_id: str,
        forward_curve_id: str,
        spread_bp: float,
        frequency: Tenor,
        day_count: DayCount,
        start: datetime.date,
        end: datetime.date,
        *,
        bdc: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
        stub: Literal["ShortFront", "ShortBack", "LongFront", "LongBack"] = "ShortFront",
        reset_lag_days: int = -1,
        fixing_calendar_id: str | None = None,
        payment_lag_days: int = 0,
        end_of_month: bool = False,
    ) -> None:
        """
        Construct a floating leg specification.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        forward_curve_id : str
            Forward curve identifier for rate projections.
        spread_bp : float
            Spread over the index in basis points.
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        start : datetime.date
            Start date of the floating leg.
        end : datetime.date
            End date of the floating leg.
        bdc : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.
        stub : {"ShortFront", "ShortBack", "LongFront", "LongBack"}, default "ShortFront"
            Stub period handling rule.
        reset_lag_days : int, default -1
            Reset lag in business days for the floating rate fixing.
            ``-1`` is a sentinel meaning "use the convention default".
        fixing_calendar_id : str, optional
            Calendar used for rate fixing (reset lag).
        payment_lag_days : int, default 0
            Payment lag in business days after period end.
        end_of_month : bool, default False
            End-of-month roll convention.

        Raises
        ------
        ValueError
            If an enum value is invalid or the accrual period is malformed
            (``start >= end``).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FloatLegSpec
        >>> callable(FloatLegSpec)
        True
        """
        ...

class PremiumLegSpec:
    """
    Premium (fixed coupon) leg of a CDS or CDS index.

    Thin typed wrapper for the canonical Rust ``PremiumLegSpec``. Used to
    build the premium leg of a :class:`CreditDefaultSwap` or :class:`CDSIndex`
    via :meth:`CreditDefaultSwapBuilder.premium` or
    :meth:`CDSIndexBuilder.premium`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import PremiumLegSpec
    >>> leg = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 6, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     100.0,
    ...     "USD-OIS",
    ... )
    >>> "spread_bp=100" in repr(leg)
    True
    """

    def __init__(
        self,
        start: datetime.date,
        end: datetime.date,
        frequency: Tenor,
        day_count: DayCount,
        spread_bp: float,
        discount_curve_id: str,
        *,
        stub: Literal["ShortFront", "ShortBack", "LongFront", "LongBack"] = "ShortFront",
        bdc: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
    ) -> None:
        """
        Construct a premium leg specification.

        Parameters
        ----------
        start : datetime.date
            Start date of protection / premium accrual.
        end : datetime.date
            End date of protection / premium accrual.
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        spread_bp : float
            Fixed running spread in basis points (e.g. 100.0 = 100bp = 1%).
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        stub : {"ShortFront", "ShortBack", "LongFront", "LongBack"}, default "ShortFront"
            Stub period handling rule.
        bdc : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.

        Raises
        ------
        ValueError
            If an enum value is invalid.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PremiumLegSpec
        >>> callable(PremiumLegSpec)
        True
        """
        ...

class ProtectionLegSpec:
    """
    Protection (default-contingent) leg of a CDS or CDS index.

    Thin typed wrapper for the canonical Rust ``ProtectionLegSpec``. Used to
    build the protection leg of a :class:`CreditDefaultSwap` or
    :class:`CDSIndex` via :meth:`CreditDefaultSwapBuilder.protection` or
    :meth:`CDSIndexBuilder.protection`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import ProtectionLegSpec
    >>> leg = ProtectionLegSpec("ACME-CDS", 0.4, 3)
    >>> "recovery_rate=0.4" in repr(leg)
    True
    """

    def __init__(
        self,
        credit_curve_id: str,
        recovery_rate: float,
        settlement_delay: int = 3,
    ) -> None:
        """
        Construct a protection leg specification.

        Parameters
        ----------
        credit_curve_id : str
            Hazard/credit curve identifier for default probabilities.
        recovery_rate : float
            Recovery rate in ``[0.0, 1.0]`` (e.g. 0.4 = 40%).
        settlement_delay : int, default 3
            Settlement delay in business days.

        Raises
        ------
        ValueError
            If ``recovery_rate`` is outside ``[0.0, 1.0]``.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ProtectionLegSpec
        >>> callable(ProtectionLegSpec)
        True
        """
        ...

class InterestRateSwap:
    """
    Typed wrapper for the canonical Rust ``InterestRateSwap``.

    Build with :meth:`InterestRateSwap.builder`; instances are accepted
    directly by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     FixedLegSpec,
    ...     FloatLegSpec,
    ...     InterestRateSwap,
    ... )
    >>> start = datetime.date(2024, 1, 15)
    >>> end = datetime.date(2029, 1, 15)
    >>> swap = (
    ...     InterestRateSwap
    ...     .builder()
    ...     .id("IRS-1")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .side("pay")
    ...     .fixed(FixedLegSpec("USD-OIS", 0.04, Tenor.semi_annual(), DayCount.THIRTY_360, start, end))
    ...     .float(FloatLegSpec("USD-OIS", "USD-SOFR-3M", 0.0, Tenor.quarterly(), DayCount.ACT_360, start, end))
    ...     .build()
    ... )
    >>> swap.id
    'IRS-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> InterestRateSwapBuilder:
        """
        Create a fluent builder (mirrors Rust ``InterestRateSwap::builder()``).

        Returns
        -------
        InterestRateSwapBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> InterestRateSwap:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"interest_rate_swap"``
            (``{"type": "interest_rate_swap", "spec": {...}}``).

        Returns
        -------
        InterestRateSwap
            The validated swap.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "interest_rate_swap", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`InterestRateSwap.from_json`.
        """
        ...

class InterestRateSwapBuilder:
    """
    Fluent builder returned by :meth:`InterestRateSwap.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import InterestRateSwap
    >>> isinstance(InterestRateSwap.builder(), InterestRateSwap.builder().__class__)
    True
    """

    def id(self, value: str) -> InterestRateSwapBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the swap.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().id)
        True
        """
        ...

    def notional(self, value: Money) -> InterestRateSwapBuilder:
        """
        Set the notional (both legs).

        Parameters
        ----------
        value : Money
            Notional amount shared by both legs.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().notional)
        True
        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> InterestRateSwapBuilder:
        """
        Set the swap direction: ``"pay"`` or ``"receive"`` (fixed leg).

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to pay fixed/receive floating, ``"receive"`` for the
            opposite.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().side)
        True
        """
        ...

    def fixed(self, value: FixedLegSpec) -> InterestRateSwapBuilder:
        """
        Set the fixed leg specification.

        Parameters
        ----------
        value : FixedLegSpec
            Fixed leg specification.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().fixed)
        True
        """
        ...

    def float(self, value: FloatLegSpec) -> InterestRateSwapBuilder:
        """
        Set the floating leg specification.

        Parameters
        ----------
        value : FloatLegSpec
            Floating leg specification.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().float)
        True
        """
        ...

    def build(self) -> InterestRateSwap:
        """
        Build the validated swap.

        Returns
        -------
        InterestRateSwap
            The validated swap.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> callable(InterestRateSwap.builder().build)
        True
        """
        ...

class Swaption:
    """
    Typed wrapper for the canonical Rust ``Swaption`` instrument.

    Build with :meth:`Swaption.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     FixedLegSpec,
    ...     FloatLegSpec,
    ...     Swaption,
    ... )
    >>> start = datetime.date(2025, 1, 15)
    >>> end = datetime.date(2030, 1, 15)
    >>> fixed = FixedLegSpec(
    ...     "USD-OIS",
    ...     0.04,
    ...     Tenor.semi_annual(),
    ...     DayCount.THIRTY_360,
    ...     start,
    ...     end,
    ...     compounding_simple=False,
    ... )
    >>> floating = FloatLegSpec(
    ...     "USD-OIS",
    ...     "USD-SOFR-3M",
    ...     0.0,
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     start,
    ...     end,
    ... )
    >>> swaption = (
    ...     Swaption
    ...     .builder()
    ...     .id("SWPT-1")
    ...     .option_type("call")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .expiry(datetime.date(2025, 1, 13))
    ...     .exercise_style("european")
    ...     .settlement("cash")
    ...     .cash_settlement_method("par_yield")
    ...     .vol_model("normal")
    ...     .vol_surface_id("USD-SWPT-VOL")
    ...     .underlying_fixed_leg(fixed)
    ...     .underlying_float_leg(floating)
    ...     .build()
    ... )
    >>> swaption.id
    'SWPT-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> SwaptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``Swaption::builder()``).

        Returns
        -------
        SwaptionBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> Swaption:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"swaption"``
            (``{"type": "swaption", "spec": {...}}``).

        Returns
        -------
        Swaption
            The validated swaption.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "swaption", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`Swaption.from_json`.
        """
        ...

class SwaptionBuilder:
    """
    Fluent builder returned by :meth:`Swaption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import Swaption
    >>> isinstance(Swaption.builder(), Swaption.builder().__class__)
    True
    """

    def id(self, value: str) -> SwaptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().id)
        True
        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> SwaptionBuilder:
        """
        Set the option type: ``"call"`` (payer) or ``"put"`` (receiver).

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().option_type)
        True
        """
        ...

    def notional(self, value: Money) -> SwaptionBuilder:
        """
        Set the notional amount of the underlying swap.

        Parameters
        ----------
        value : Money
            Notional amount of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().notional)
        True
        """
        ...

    def expiry(self, value: datetime.date) -> SwaptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().expiry)
        True
        """
        ...

    def exercise_style(self, value: Literal["european", "bermudan", "american"]) -> SwaptionBuilder:
        """
        Set the exercise style.

        Parameters
        ----------
        value : {"european", "bermudan", "american"}
            Exercise style of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().exercise_style)
        True
        """
        ...

    def settlement(self, value: Literal["physical", "cash"]) -> SwaptionBuilder:
        """
        Set the settlement method.

        Parameters
        ----------
        value : {"physical", "cash"}
            Settlement method of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized settlement method.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().settlement)
        True
        """
        ...

    def cash_settlement_method(self, value: Literal["par_yield", "isda_par_par", "zero_coupon"]) -> SwaptionBuilder:
        """
        Set the cash settlement annuity method.

        Only affects pricing when ``settlement`` is ``"cash"``.

        Parameters
        ----------
        value : {"par_yield", "isda_par_par", "zero_coupon"}
            Cash settlement annuity method.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized cash settlement method.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().cash_settlement_method)
        True
        """
        ...

    def vol_model(self, value: Literal["black", "normal"]) -> SwaptionBuilder:
        """
        Set the volatility model.

        Parameters
        ----------
        value : {"black", "normal"}
            Volatility model used for pricing.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized volatility model.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().vol_model)
        True
        """
        ...

    def vol_surface_id(self, value: str) -> SwaptionBuilder:
        """
        Set the volatility surface identifier.

        Parameters
        ----------
        value : str
            Volatility surface identifier for option pricing.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().vol_surface_id)
        True
        """
        ...

    def underlying_fixed_leg(self, value: FixedLegSpec) -> SwaptionBuilder:
        """
        Set the complete fixed leg of the underlying swap.

        Parameters
        ----------
        value : FixedLegSpec
            Fixed leg of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().underlying_fixed_leg)
        True
        """
        ...

    def underlying_float_leg(self, value: FloatLegSpec) -> SwaptionBuilder:
        """
        Set the complete floating leg of the underlying swap.

        Parameters
        ----------
        value : FloatLegSpec
            Floating leg of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().underlying_float_leg)
        True
        """
        ...

    def sabr_params_json(self, value: str) -> SwaptionBuilder:
        """
        Set the SABR volatility model parameters from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded SABR parameters object with fields ``alpha``,
            ``beta``, ``nu``, ``rho`` and optional ``shift``.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the SABR parameters shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().sabr_params_json)
        True
        """
        ...

    def build(self) -> Swaption:
        """
        Build the validated swaption.

        Returns
        -------
        Swaption
            The validated swaption.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> callable(Swaption.builder().build)
        True
        """
        ...

class CapFloor:
    """
    Typed wrapper for the canonical Rust ``CapFloor`` instrument.

    Build with :meth:`CapFloor.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import CapFloor
    >>> cap = (
    ...     CapFloor
    ...     .builder()
    ...     .id("CAP-1")
    ...     .rate_option_type("cap")
    ...     .notional(Money(5_000_000.0, Currency("USD")))
    ...     .strike(0.05)
    ...     .start_date(datetime.date(2024, 1, 15))
    ...     .maturity(datetime.date(2027, 1, 15))
    ...     .frequency(Tenor.quarterly())
    ...     .day_count(DayCount.ACT_360)
    ...     .discount_curve_id("USD-OIS")
    ...     .forward_curve_id("USD-SOFR-3M")
    ...     .vol_surface_id("USD-CAP-VOL")
    ...     .vol_type("normal")
    ...     .build()
    ... )
    >>> cap.id
    'CAP-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> CapFloorBuilder:
        """
        Create a fluent builder (mirrors Rust ``CapFloor::builder()``).

        Returns
        -------
        CapFloorBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CapFloor:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"cap_floor"``
            (``{"type": "cap_floor", "spec": {...}}``).

        Returns
        -------
        CapFloor
            The validated cap/floor.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "cap_floor", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`CapFloor.from_json`.
        """
        ...

class CapFloorBuilder:
    """
    Fluent builder returned by :meth:`CapFloor.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CapFloor
    >>> isinstance(CapFloor.builder(), CapFloor.builder().__class__)
    True
    """

    def id(self, value: str) -> CapFloorBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the cap/floor.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().id)
        True
        """
        ...

    def rate_option_type(self, value: Literal["cap", "floor"]) -> CapFloorBuilder:
        """
        Set the option type.

        Parameters
        ----------
        value : {"cap", "floor"}
            Option type of the instrument.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().rate_option_type)
        True
        """
        ...

    def notional(self, value: Money) -> CapFloorBuilder:
        """
        Set the notional amount of the cap or floor.

        Parameters
        ----------
        value : Money
            Notional amount.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().notional)
        True
        """
        ...

    def strike(self, value: float) -> CapFloorBuilder:
        """
        Set the strike rate of the cap or floor.

        Parameters
        ----------
        value : float
            Strike as a decimal (0.05 = 5%).

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not finite.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().strike)
        True
        """
        ...

    def spread(self, value: float) -> CapFloorBuilder:
        """
        Set the contractual spread added to the referenced rate.

        Parameters
        ----------
        value : float
            Spread in decimal rate units, added after projecting the index.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not finite.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().spread)
        True
        """
        ...

    def start_date(self, value: datetime.date) -> CapFloorBuilder:
        """
        Set the start date of the underlying period.

        Parameters
        ----------
        value : datetime.date
            Start date of the underlying period.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().start_date)
        True
        """
        ...

    def maturity(self, value: datetime.date) -> CapFloorBuilder:
        """
        Set the end date of the underlying period.

        Parameters
        ----------
        value : datetime.date
            End date of the underlying period.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().maturity)
        True
        """
        ...

    def frequency(self, value: Tenor) -> CapFloorBuilder:
        """
        Set the payment frequency.

        Parameters
        ----------
        value : Tenor
            Payment frequency for caps/floors.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().frequency)
        True
        """
        ...

    def day_count(self, value: DayCount) -> CapFloorBuilder:
        """
        Set the day count convention.

        Parameters
        ----------
        value : DayCount
            Day count convention.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().day_count)
        True
        """
        ...

    def calendar_id(self, value: str) -> CapFloorBuilder:
        """
        Set the holiday calendar identifier for schedule and roll conventions.

        Parameters
        ----------
        value : str
            Holiday calendar identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().calendar_id)
        True
        """
        ...

    def discount_curve_id(self, value: str) -> CapFloorBuilder:
        """
        Set the discount curve identifier.

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().discount_curve_id)
        True
        """
        ...

    def forward_curve_id(self, value: str) -> CapFloorBuilder:
        """
        Set the forward curve identifier.

        Parameters
        ----------
        value : str
            Forward curve identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().forward_curve_id)
        True
        """
        ...

    def vol_surface_id(self, value: str) -> CapFloorBuilder:
        """
        Set the volatility surface identifier.

        Parameters
        ----------
        value : str
            Volatility surface identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().vol_surface_id)
        True
        """
        ...

    def vol_type(self, value: Literal["lognormal", "normal", "shifted_lognormal"]) -> CapFloorBuilder:
        """
        Set the volatility type convention.

        Parameters
        ----------
        value : {"lognormal", "normal", "shifted_lognormal"}
            Volatility convention. Must match the convention of the
            configured volatility surface.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized volatility type.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().vol_type)
        True
        """
        ...

    def vol_shift(self, value: float) -> CapFloorBuilder:
        """
        Set the displacement shift used for shifted-lognormal pricing.

        Parameters
        ----------
        value : float
            Displacement added to forward and strike. Must be non-negative.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().vol_shift)
        True
        """
        ...

    def build(self) -> CapFloor:
        """
        Build the validated cap/floor.

        Returns
        -------
        CapFloor
            The validated cap/floor.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> callable(CapFloor.builder().build)
        True
        """
        ...

class CreditDefaultSwap:
    """
    Typed wrapper for the canonical Rust ``CreditDefaultSwap`` instrument.

    Build with :meth:`CreditDefaultSwap.builder`; instances are accepted
    directly by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     CreditDefaultSwap,
    ...     PremiumLegSpec,
    ...     ProtectionLegSpec,
    ... )
    >>> premium = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 6, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     100.0,
    ...     "USD-OIS",
    ... )
    >>> protection = ProtectionLegSpec("ACME-CDS", 0.4, 3)
    >>> cds = (
    ...     CreditDefaultSwap
    ...     .builder()
    ...     .id("CDS-1")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .side("pay")
    ...     .convention("isda_na")
    ...     .premium(premium)
    ...     .protection(protection)
    ...     .build()
    ... )
    >>> cds.id
    'CDS-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> CreditDefaultSwapBuilder:
        """
        Create a fluent builder (mirrors Rust ``CreditDefaultSwap::builder()``).

        Returns
        -------
        CreditDefaultSwapBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CreditDefaultSwap:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"credit_default_swap"``
            (``{"type": "credit_default_swap", "spec": {...}}``).

        Returns
        -------
        CreditDefaultSwap
            The validated CDS.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "credit_default_swap", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`CreditDefaultSwap.from_json`.
        """
        ...

class CreditDefaultSwapBuilder:
    """
    Fluent builder returned by :meth:`CreditDefaultSwap.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
    >>> isinstance(CreditDefaultSwap.builder(), CreditDefaultSwap.builder().__class__)
    True
    """

    def id(self, value: str) -> CreditDefaultSwapBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the CDS.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().id)
        True
        """
        ...

    def notional(self, value: Money) -> CreditDefaultSwapBuilder:
        """
        Set the notional amount.

        Parameters
        ----------
        value : Money
            Notional amount of protection.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().notional)
        True
        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> CreditDefaultSwapBuilder:
        """
        Set the protection buyer/seller perspective.

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to buy protection (pay premium), ``"receive"`` to sell
            protection (receive premium).

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().side)
        True
        """
        ...

    def convention(self, value: Literal["isda_na", "isda_eu", "isda_as"]) -> CreditDefaultSwapBuilder:
        """
        Set the ISDA regional convention.

        Parameters
        ----------
        value : {"isda_na", "isda_eu", "isda_as"}
            ISDA CDS convention (North American, European, or Asian).

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized convention.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().convention)
        True
        """
        ...

    def premium(self, value: PremiumLegSpec) -> CreditDefaultSwapBuilder:
        """
        Set the premium leg specification.

        Parameters
        ----------
        value : PremiumLegSpec
            Premium leg specification.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().premium)
        True
        """
        ...

    def protection(self, value: ProtectionLegSpec) -> CreditDefaultSwapBuilder:
        """
        Set the protection leg specification.

        Parameters
        ----------
        value : ProtectionLegSpec
            Protection leg specification.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().protection)
        True
        """
        ...

    def doc_clause(self, value: Literal["cr14", "mr14", "mm14", "xr14"]) -> CreditDefaultSwapBuilder:
        """
        Set the ISDA documentation clause for restructuring credit events.

        Parameters
        ----------
        value : {"cr14", "mr14", "mm14", "xr14"}
            Restructuring documentation clause. If never set, the effective
            clause is derived from the CDS convention.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized documentation clause.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().doc_clause)
        True
        """
        ...

    def protection_effective_date(self, value: datetime.date) -> CreditDefaultSwapBuilder:
        """
        Set the protection effective date for a forward-starting CDS.

        Parameters
        ----------
        value : datetime.date
            Date on which credit protection begins. Must satisfy
            ``premium.start <= value <= premium.end``. When never set,
            protection starts on the premium leg start date.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().protection_effective_date)
        True
        """
        ...

    def build(self) -> CreditDefaultSwap:
        """
        Build the validated CDS.

        Returns
        -------
        CreditDefaultSwap
            The validated CDS.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> callable(CreditDefaultSwap.builder().build)
        True
        """
        ...

class CDSIndex:
    """
    Typed wrapper for the canonical Rust ``CDSIndex`` instrument.

    Build with :meth:`CDSIndex.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     CDSIndex,
    ...     PremiumLegSpec,
    ...     ProtectionLegSpec,
    ... )
    >>> premium = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 12, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     60.0,
    ...     "USD-OIS",
    ... )
    >>> protection = ProtectionLegSpec("CDX.NA.IG.HAZARD", 0.4, 3)
    >>> index = (
    ...     CDSIndex
    ...     .builder()
    ...     .id("CDX-IG-42")
    ...     .index_name("CDX.NA.IG")
    ...     .series(42)
    ...     .version(1)
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .index_factor(1.0)
    ...     .side("pay")
    ...     .convention("isda_na")
    ...     .premium(premium)
    ...     .protection(protection)
    ...     .pricing("SingleCurve")
    ...     .num_constituents(125)
    ...     .build()
    ... )
    >>> index.id
    'CDX-IG-42'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> CDSIndexBuilder:
        """
        Create a fluent builder (mirrors Rust ``CDSIndex::builder()``).

        The builder pre-seeds an empty ``constituents`` list (the Rust field
        has no default) so ``build()`` succeeds without calling
        :meth:`CDSIndexBuilder.constituents_json` when the index is priced in
        ``"SingleCurve"`` mode.

        Returns
        -------
        CDSIndexBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CDSIndex:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"cds_index"``
            (``{"type": "cds_index", "spec": {...}}``).

        Returns
        -------
        CDSIndex
            The validated CDS index.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "cds_index", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`CDSIndex.from_json`.
        """
        ...

class CDSIndexBuilder:
    """
    Fluent builder returned by :meth:`CDSIndex.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CDSIndex
    >>> isinstance(CDSIndex.builder(), CDSIndex.builder().__class__)
    True
    """

    def id(self, value: str) -> CDSIndexBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the index trade.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().id)
        True
        """
        ...

    def index_name(self, value: str) -> CDSIndexBuilder:
        """
        Set the index name.

        Parameters
        ----------
        value : str
            Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx Europe"``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().index_name)
        True
        """
        ...

    def series(self, value: int) -> CDSIndexBuilder:
        """
        Set the series number.

        Parameters
        ----------
        value : int
            Series number, e.g. ``42``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().series)
        True
        """
        ...

    def version(self, value: int) -> CDSIndexBuilder:
        """
        Set the version number within the series.

        Parameters
        ----------
        value : int
            Version number, e.g. ``1``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().version)
        True
        """
        ...

    def notional(self, value: Money) -> CDSIndexBuilder:
        """
        Set the notional amount of the index.

        Parameters
        ----------
        value : Money
            Notional amount of the index.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().notional)
        True
        """
        ...

    def index_factor(self, value: float) -> CDSIndexBuilder:
        """
        Set the index factor (fraction of surviving notional).

        Parameters
        ----------
        value : float
            Index factor in ``[0.0, 1.0]``. ``1.0`` means no constituent has
            defaulted since series inception.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().index_factor)
        True
        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> CDSIndexBuilder:
        """
        Set the protection buyer/seller perspective.

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to buy protection (pay premium), ``"receive"`` to sell
            protection (receive premium).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().side)
        True
        """
        ...

    def convention(self, value: Literal["isda_na", "isda_eu", "isda_as"]) -> CDSIndexBuilder:
        """
        Set the ISDA regional convention.

        Parameters
        ----------
        value : {"isda_na", "isda_eu", "isda_as"}
            ISDA CDS convention (North American, European, or Asian).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized convention.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().convention)
        True
        """
        ...

    def premium(self, value: PremiumLegSpec) -> CDSIndexBuilder:
        """
        Set the premium leg specification.

        Parameters
        ----------
        value : PremiumLegSpec
            Premium leg specification (coupon schedule and discounting).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().premium)
        True
        """
        ...

    def protection(self, value: ProtectionLegSpec) -> CDSIndexBuilder:
        """
        Set the protection leg specification.

        Parameters
        ----------
        value : ProtectionLegSpec
            Protection leg specification (credit curve and settlement).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().protection)
        True
        """
        ...

    def pricing(self, value: Literal["SingleCurve", "Constituents"]) -> CDSIndexBuilder:
        """
        Set the pricing aggregation mode.

        Parameters
        ----------
        value : {"SingleCurve", "Constituents"}
            ``"SingleCurve"`` prices the index against a single index hazard
            curve (synthetic CDS). ``"Constituents"`` prices each issuer
            separately and aggregates by weight; requires
            :meth:`CDSIndexBuilder.constituents_json` to be set.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized pricing mode.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().pricing)
        True
        """
        ...

    def constituents_json(self, value: str) -> CDSIndexBuilder:
        """
        Set the index constituents from a JSON array.

        Parameters
        ----------
        value : str
            JSON array of ``CDSIndexConstituent`` objects (``credit``,
            ``weight``, and optional ``defaulted``).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the constituent-list shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().constituents_json)
        True
        """
        ...

    def num_constituents(self, value: int) -> CDSIndexBuilder:
        """
        Set the number of reference entities in the index pool.

        Parameters
        ----------
        value : int
            Number of names in the index pool, e.g. ``125`` for CDX.NA.IG.
            Required for portfolio-level analytics (e.g. jump-to-default)
            when ``constituents`` is empty.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().num_constituents)
        True
        """
        ...

    def build(self) -> CDSIndex:
        """
        Build the validated CDS index.

        Returns
        -------
        CDSIndex
            The validated CDS index.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> callable(CDSIndex.builder().build)
        True
        """
        ...

class CDSTranche:
    """
    Typed wrapper for the canonical Rust ``CDSTranche`` instrument.

    Build with :meth:`CDSTranche.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import CDSTranche
    >>> tranche = (
    ...     CDSTranche
    ...     .builder()
    ...     .id("CDX-IG-42-3-7")
    ...     .index_name("CDX.NA.IG")
    ...     .series(42)
    ...     .attach_pct(3.0)
    ...     .detach_pct(7.0)
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .maturity(datetime.date(2029, 6, 20))
    ...     .running_coupon_bp(100.0)
    ...     .frequency(Tenor.quarterly())
    ...     .day_count(DayCount.ACT_360)
    ...     .discount_curve_id("USD-OIS")
    ...     .credit_index_id("CDX-IG-42-CURVE")
    ...     .side("buy_protection")
    ...     .build()
    ... )
    >>> tranche.id
    'CDX-IG-42-3-7'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> CDSTrancheBuilder:
        """
        Create a fluent builder (mirrors Rust ``CDSTranche::builder()``).

        The builder pre-seeds ``accumulated_loss(0.0)`` and
        ``standard_imm_dates(True)`` (the Rust fields have no defaults),
        which :meth:`CDSTrancheBuilder.accumulated_loss` and
        :meth:`CDSTrancheBuilder.standard_imm_dates` can override.

        Returns
        -------
        CDSTrancheBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CDSTranche:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"cds_tranche"``
            (``{"type": "cds_tranche", "spec": {...}}``).

        Returns
        -------
        CDSTranche
            The validated CDS tranche.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "cds_tranche", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`CDSTranche.from_json`.
        """
        ...

class CDSTrancheBuilder:
    """
    Fluent builder returned by :meth:`CDSTranche.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CDSTranche
    >>> isinstance(CDSTranche.builder(), CDSTranche.builder().__class__)
    True
    """

    def id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the tranche trade.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().id)
        True
        """
        ...

    def index_name(self, value: str) -> CDSTrancheBuilder:
        """
        Set the underlying index name.

        Parameters
        ----------
        value : str
            Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx EUR"``.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().index_name)
        True
        """
        ...

    def series(self, value: int) -> CDSTrancheBuilder:
        """
        Set the series number.

        Parameters
        ----------
        value : int
            Series number, e.g. ``42``.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().series)
        True
        """
        ...

    def attach_pct(self, value: float) -> CDSTrancheBuilder:
        """
        Set the attachment point.

        Parameters
        ----------
        value : float
            Attachment point quoted in percent (e.g. ``0.0`` for equity;
            ``3.0`` for a tranche attaching at 3%).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().attach_pct)
        True
        """
        ...

    def detach_pct(self, value: float) -> CDSTrancheBuilder:
        """
        Set the detachment point.

        Parameters
        ----------
        value : float
            Detachment point quoted in percent (e.g. ``3.0`` for a 0-3%
            tranche).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().detach_pct)
        True
        """
        ...

    def notional(self, value: Money) -> CDSTrancheBuilder:
        """
        Set the notional amount of the tranche.

        Parameters
        ----------
        value : Money
            Notional amount of the tranche.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().notional)
        True
        """
        ...

    def maturity(self, value: datetime.date) -> CDSTrancheBuilder:
        """
        Set the maturity date of the tranche.

        Parameters
        ----------
        value : datetime.date
            Maturity date of the tranche.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().maturity)
        True
        """
        ...

    def running_coupon_bp(self, value: float) -> CDSTrancheBuilder:
        """
        Set the running coupon.

        Parameters
        ----------
        value : float
            Running coupon in basis points (e.g. ``100.0`` = 1.00%).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().running_coupon_bp)
        True
        """
        ...

    def frequency(self, value: Tenor) -> CDSTrancheBuilder:
        """
        Set the payment frequency.

        Parameters
        ----------
        value : Tenor
            Payment frequency (typically quarterly).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().frequency)
        True
        """
        ...

    def day_count(self, value: DayCount) -> CDSTrancheBuilder:
        """
        Set the day count convention.

        Parameters
        ----------
        value : DayCount
            Day count convention (typically Act/360).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().day_count)
        True
        """
        ...

    def calendar_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the holiday calendar identifier.

        Parameters
        ----------
        value : str
            Holiday calendar identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().calendar_id)
        True
        """
        ...

    def discount_curve_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the discount curve identifier (by quote currency).

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().discount_curve_id)
        True
        """
        ...

    def credit_index_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the credit index identifier for survival/loss modeling.

        Parameters
        ----------
        value : str
            Credit index identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().credit_index_id)
        True
        """
        ...

    def side(self, value: Literal["buy_protection", "sell_protection"]) -> CDSTrancheBuilder:
        """
        Set the tranche side (buy/sell protection).

        Parameters
        ----------
        value : {"buy_protection", "sell_protection"}
            Tranche side.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().side)
        True
        """
        ...

    def effective_date(self, value: datetime.date) -> CDSTrancheBuilder:
        """
        Set the effective date for schedule anchoring.

        Parameters
        ----------
        value : datetime.date
            Effective date. If never set, uses the as-of date (or standard
            IMM-date rolling, if ``standard_imm_dates`` is true).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().effective_date)
        True
        """
        ...

    def accumulated_loss(self, value: float) -> CDSTrancheBuilder:
        """
        Set the accumulated realized loss.

        Parameters
        ----------
        value : float
            Accumulated realized loss as a fraction of the original
            portfolio notional. Defaults to ``0.0`` when never set
            explicitly.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().accumulated_loss)
        True
        """
        ...

    def standard_imm_dates(self, value: bool) -> CDSTrancheBuilder:
        """
        Set whether to enforce standard IMM dates.

        Parameters
        ----------
        value : bool
            Whether to enforce standard IMM dates (20th of Mar, Jun, Sep,
            Dec). Defaults to ``True`` when never set explicitly.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().standard_imm_dates)
        True
        """
        ...

    def build(self) -> CDSTranche:
        """
        Build the validated CDS tranche.

        Returns
        -------
        CDSTranche
            The validated CDS tranche.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> callable(CDSTranche.builder().build)
        True
        """
        ...

class ConvertibleBond:
    """
    Typed wrapper for the canonical Rust ``ConvertibleBond`` instrument.

    Build with :meth:`ConvertibleBond.builder`; nested conversion/call/coupon
    terms are set via ``*_json`` setters (JSON sub-fields, per the
    nested-spec rule). Instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> import json
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import ConvertibleBond
    >>> conversion = json.dumps({
    ...     "ratio": 20.0,
    ...     "price": None,
    ...     "policy": "Voluntary",
    ...     "anti_dilution": "FullRatchet",
    ...     "dividend_adjustment": "None",
    ...     "dilution_events": [],
    ... })
    >>> bond = (
    ...     ConvertibleBond
    ...     .builder()
    ...     .id("CONV-1")
    ...     .notional(Money(1_000.0, Currency("USD")))
    ...     .issue_date(datetime.date(2024, 1, 15))
    ...     .maturity(datetime.date(2029, 1, 15))
    ...     .discount_curve_id("USD-OIS")
    ...     .conversion_json(conversion)
    ...     .underlying_equity_id("ACME")
    ...     .build()
    ... )
    >>> bond.id
    'CONV-1'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> ConvertibleBondBuilder:
        """
        Create a fluent builder (mirrors Rust ``ConvertibleBond::builder()``).

        Returns
        -------
        ConvertibleBondBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> ConvertibleBond:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"convertible_bond"``
            (``{"type": "convertible_bond", "spec": {...}}``).

        Returns
        -------
        ConvertibleBond
            The validated convertible bond.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "convertible_bond", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`ConvertibleBond.from_json`.
        """
        ...

class ConvertibleBondBuilder:
    """
    Fluent builder returned by :meth:`ConvertibleBond.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import ConvertibleBond
    >>> isinstance(ConvertibleBond.builder(), ConvertibleBond.builder().__class__)
    True
    """

    def id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the convertible bond.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().id)
        True
        """
        ...

    def notional(self, value: Money) -> ConvertibleBondBuilder:
        """
        Set the principal amount.

        Parameters
        ----------
        value : Money
            Principal amount.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().notional)
        True
        """
        ...

    def issue_date(self, value: datetime.date) -> ConvertibleBondBuilder:
        """
        Set the issue date.

        Parameters
        ----------
        value : datetime.date
            Issue date.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().issue_date)
        True
        """
        ...

    def maturity(self, value: datetime.date) -> ConvertibleBondBuilder:
        """
        Set the maturity date.

        Parameters
        ----------
        value : datetime.date
            Maturity date.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().maturity)
        True
        """
        ...

    def discount_curve_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the discount curve identifier for the debt component.

        Parameters
        ----------
        value : str
            Discount curve identifier for the debt component (risk-free or
            funding).

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().discount_curve_id)
        True
        """
        ...

    def credit_curve_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the credit curve identifier for risky discounting (bond floor).

        Parameters
        ----------
        value : str
            Credit curve identifier. If not provided, falls back to
            ``discount_curve_id`` (implies no credit spread). Must represent
            zero-recovery (pure hazard) risky discounting.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().credit_curve_id)
        True
        """
        ...

    def conversion_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the conversion terms from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``ConversionSpec`` object with fields ``ratio``,
            ``price``, ``policy``, ``anti_dilution``, ``dividend_adjustment``
            and ``dilution_events``. At least one of ``ratio`` / ``price``
            must be set. The Rust enums have no ``rename_all`` attribute, so
            variant values use their exact PascalCase Rust names, e.g.
            ``"Voluntary"``, ``"FullRatchet"``, ``"None"``.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``ConversionSpec`` shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().conversion_json)
        True
        """
        ...

    def underlying_equity_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the underlying equity identifier.

        Parameters
        ----------
        value : str
            Underlying equity identifier (ticker or instrument id).

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().underlying_equity_id)
        True
        """
        ...

    def call_put_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the call/put schedule from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``CallPutSchedule`` object with ``calls`` and
            ``puts`` arrays of call/put windows.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``CallPutSchedule`` shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().call_put_json)
        True
        """
        ...

    def soft_call_trigger_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the soft-call trigger condition from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``SoftCallTrigger`` object with fields
            ``threshold_pct``, ``observation_days`` and
            ``required_days_above``.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``SoftCallTrigger`` shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().soft_call_trigger_json)
        True
        """
        ...

    def settlement_days(self, value: int) -> ConvertibleBondBuilder:
        """
        Set the settlement lag.

        Parameters
        ----------
        value : int
            Number of business days from trade date to settlement date
            (e.g. ``2`` for US corporate convertibles). If never set,
            settlement is assumed same-day.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().settlement_days)
        True
        """
        ...

    def recovery_rate(self, value: float) -> ConvertibleBondBuilder:
        """
        Set the assumed recovery rate on default.

        Parameters
        ----------
        value : float
            Recovery rate as a fraction (e.g. ``0.40`` = 40%). Used in the
            Tsiveriotis-Zhang credit model; only relevant when
            ``credit_curve_id`` is set.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().recovery_rate)
        True
        """
        ...

    def fixed_coupon_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the fixed coupon specification from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``FixedCouponSpec`` object.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``FixedCouponSpec`` shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().fixed_coupon_json)
        True
        """
        ...

    def floating_coupon_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the floating coupon specification from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``FloatingCouponSpec`` object.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``FloatingCouponSpec``
            shape.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().floating_coupon_json)
        True
        """
        ...

    def build(self) -> ConvertibleBond:
        """
        Build the validated convertible bond.

        Returns
        -------
        ConvertibleBond
            The validated convertible bond.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            neither ``ratio`` nor ``price`` set on the conversion terms).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> callable(ConvertibleBond.builder().build)
        True
        """
        ...

class FxForward:
    """
    Typed wrapper for the canonical Rust ``FxForward``.

    Build with :meth:`FxForward.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import FxForward
    >>> forward = (
    ...     FxForward
    ...     .builder()
    ...     .id("EURUSD-FWD-6M")
    ...     .base_currency(Currency("EUR"))
    ...     .quote_currency(Currency("USD"))
    ...     .maturity(datetime.date(2025, 6, 15))
    ...     .notional(Money(1_000_000.0, Currency("EUR")))
    ...     .contract_rate(1.10)
    ...     .domestic_discount_curve_id("USD-OIS")
    ...     .foreign_discount_curve_id("EUR-OIS")
    ...     .build()
    ... )
    >>> forward.id
    'EURUSD-FWD-6M'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> FxForwardBuilder:
        """
        Create a fluent builder (mirrors Rust ``FxForward::builder()``).

        Returns
        -------
        FxForwardBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> FxForward:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"fx_forward"``
            (``{"type": "fx_forward", "spec": {...}}``).

        Returns
        -------
        FxForward
            The validated FX forward.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "fx_forward", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`FxForward.from_json`.
        """
        ...

class FxForwardBuilder:
    """
    Fluent builder returned by :meth:`FxForward.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import FxForward
    >>> isinstance(FxForward.builder(), FxForward.builder().__class__)
    True
    """

    def id(self, value: str) -> FxForwardBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the FX forward.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().id)
        True
        """
        ...

    def base_currency(self, value: Currency) -> FxForwardBuilder:
        """
        Set the base currency (foreign currency, numerator of the pair).

        Parameters
        ----------
        value : Currency
            Base (foreign) currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().base_currency)
        True
        """
        ...

    def quote_currency(self, value: Currency) -> FxForwardBuilder:
        """
        Set the quote currency (domestic currency, denominator of the pair).

        Parameters
        ----------
        value : Currency
            Quote (domestic) currency; also the PV currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().quote_currency)
        True
        """
        ...

    def maturity(self, value: datetime.date) -> FxForwardBuilder:
        """
        Set the maturity/settlement date.

        Parameters
        ----------
        value : datetime.date
            Maturity/settlement date.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().maturity)
        True
        """
        ...

    def notional(self, value: Money) -> FxForwardBuilder:
        """
        Set the notional amount in base currency.

        Parameters
        ----------
        value : Money
            Notional amount, denominated in the base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().notional)
        True
        """
        ...

    def contract_rate(self, value: float) -> FxForwardBuilder:
        """
        Set the contract forward rate (quote per base).

        If not set, the forward is valued at-market (zero PV at inception).

        Parameters
        ----------
        value : float
            Contract forward rate, quote currency per unit of base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().contract_rate)
        True
        """
        ...

    def domestic_discount_curve_id(self, value: str) -> FxForwardBuilder:
        """
        Set the domestic (quote currency) discount curve identifier.

        Parameters
        ----------
        value : str
            Domestic (quote currency) discount curve identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().domestic_discount_curve_id)
        True
        """
        ...

    def foreign_discount_curve_id(self, value: str) -> FxForwardBuilder:
        """
        Set the foreign (base currency) discount curve identifier.

        Parameters
        ----------
        value : str
            Foreign (base currency) discount curve identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().foreign_discount_curve_id)
        True
        """
        ...

    def spot_rate_override(self, value: float) -> FxForwardBuilder:
        """
        Set an explicit spot rate override (quote per base).

        If not set, the spot rate is sourced from the market's FX matrix.

        Parameters
        ----------
        value : float
            Spot FX rate, quote currency per unit of base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().spot_rate_override)
        True
        """
        ...

    def base_calendar_id(self, value: str) -> FxForwardBuilder:
        """
        Set the base currency calendar identifier for business day adjustment.

        Parameters
        ----------
        value : str
            Base currency holiday calendar identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().base_calendar_id)
        True
        """
        ...

    def quote_calendar_id(self, value: str) -> FxForwardBuilder:
        """
        Set the quote currency calendar identifier for business day adjustment.

        Parameters
        ----------
        value : str
            Quote currency holiday calendar identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().quote_calendar_id)
        True
        """
        ...

    def build(self) -> FxForward:
        """
        Build the validated FX forward.

        Returns
        -------
        FxForward
            The validated FX forward.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            ``base_currency`` equals ``quote_currency``).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> callable(FxForward.builder().build)
        True
        """
        ...

class FxOption:
    """
    Typed wrapper for the canonical Rust ``FxOption``.

    Build with :meth:`FxOption.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import FxOption
    >>> option = (
    ...     FxOption
    ...     .builder()
    ...     .id("EURUSD-CALL-1Y")
    ...     .base_currency(Currency("EUR"))
    ...     .quote_currency(Currency("USD"))
    ...     .strike(1.12)
    ...     .option_type("call")
    ...     .expiry(datetime.date(2025, 12, 15))
    ...     .notional(Money(1_000_000.0, Currency("EUR")))
    ...     .domestic_discount_curve_id("USD-OIS")
    ...     .foreign_discount_curve_id("EUR-OIS")
    ...     .vol_surface_id("EURUSD-VOL")
    ...     .build()
    ... )
    >>> option.id
    'EURUSD-CALL-1Y'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> FxOptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``FxOption::builder()``).

        Returns
        -------
        FxOptionBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> FxOption:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"fx_option"``
            (``{"type": "fx_option", "spec": {...}}``).

        Returns
        -------
        FxOption
            The validated FX option.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "fx_option", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`FxOption.from_json`.
        """
        ...

class FxOptionBuilder:
    """
    Fluent builder returned by :meth:`FxOption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import FxOption
    >>> isinstance(FxOption.builder(), FxOption.builder().__class__)
    True
    """

    def id(self, value: str) -> FxOptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the FX option.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().id)
        True
        """
        ...

    def base_currency(self, value: Currency) -> FxOptionBuilder:
        """
        Set the base currency (foreign currency).

        Parameters
        ----------
        value : Currency
            Base (foreign) currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().base_currency)
        True
        """
        ...

    def quote_currency(self, value: Currency) -> FxOptionBuilder:
        """
        Set the quote currency (domestic currency).

        Parameters
        ----------
        value : Currency
            Quote (domestic) currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().quote_currency)
        True
        """
        ...

    def strike(self, value: float) -> FxOptionBuilder:
        """
        Set the strike exchange rate (quote per base).

        Parameters
        ----------
        value : float
            Strike exchange rate, quote currency per unit of base currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().strike)
        True
        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> FxOptionBuilder:
        """
        Set the option type: ``"call"`` or ``"put"`` on base currency.

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the FX option.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().option_type)
        True
        """
        ...

    def exercise_style(self, value: Literal["european", "american"]) -> FxOptionBuilder:
        """
        Set the exercise style.

        Parameters
        ----------
        value : {"european", "american"}
            Exercise style of the FX option.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().exercise_style)
        True
        """
        ...

    def expiry(self, value: datetime.date) -> FxOptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().expiry)
        True
        """
        ...

    def notional(self, value: Money) -> FxOptionBuilder:
        """
        Set the notional amount in base currency.

        Parameters
        ----------
        value : Money
            Notional amount, denominated in the base currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().notional)
        True
        """
        ...

    def domestic_discount_curve_id(self, value: str) -> FxOptionBuilder:
        """
        Set the domestic currency discount curve identifier.

        Parameters
        ----------
        value : str
            Domestic currency discount curve identifier.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().domestic_discount_curve_id)
        True
        """
        ...

    def foreign_discount_curve_id(self, value: str) -> FxOptionBuilder:
        """
        Set the foreign currency discount curve identifier.

        Parameters
        ----------
        value : str
            Foreign currency discount curve identifier.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().foreign_discount_curve_id)
        True
        """
        ...

    def vol_surface_id(self, value: str) -> FxOptionBuilder:
        """
        Set the FX volatility surface identifier.

        Parameters
        ----------
        value : str
            FX volatility surface identifier for option pricing.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().vol_surface_id)
        True
        """
        ...

    def build(self) -> FxOption:
        """
        Build the validated FX option.

        Returns
        -------
        FxOption
            The validated FX option.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            ``base_currency`` equals ``quote_currency``).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> callable(FxOption.builder().build)
        True
        """
        ...

class EquityOption:
    """
    Typed wrapper for the canonical Rust ``EquityOption`` instrument.

    Build with :meth:`EquityOption.builder`; instances are accepted directly
    by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import EquityOption
    >>> option = (
    ...     EquityOption
    ...     .builder()
    ...     .id("AAPL-C-200")
    ...     .underlying_ticker("AAPL")
    ...     .strike(200.0)
    ...     .option_type("call")
    ...     .expiry(datetime.date(2025, 6, 20))
    ...     .notional(Money(100.0, Currency("USD")))
    ...     .discount_curve_id("USD-OIS")
    ...     .spot_id("AAPL")
    ...     .vol_surface_id("AAPL-VOL")
    ...     .build()
    ... )
    >>> option.id
    'AAPL-C-200'
    """

    @property
    def id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The unique instrument identifier.
        """
        ...

    @staticmethod
    def builder() -> EquityOptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``EquityOption::builder()``).

        Returns
        -------
        EquityOptionBuilder
            A builder with fluent, consuming setter methods.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder)
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> EquityOption:
        """
        Deserialize from tagged instrument JSON.

        Parameters
        ----------
        json : str
            Tagged instrument JSON with type ``"equity_option"``
            (``{"type": "equity_option", "spec": {...}}``).

        Returns
        -------
        EquityOption
            The validated equity option.

        Raises
        ------
        ValueError
            If the JSON is malformed, has a different instrument type, or
            fails validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.from_json)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to tagged instrument JSON.

        Returns
        -------
        str
            ``{"type": "equity_option", "spec": ...}`` JSON accepted by
            :func:`price_instrument` and :meth:`EquityOption.from_json`.
        """
        ...

class EquityOptionBuilder:
    """
    Fluent builder returned by :meth:`EquityOption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import EquityOption
    >>> isinstance(EquityOption.builder(), EquityOption.builder().__class__)
    True
    """

    def id(self, value: str) -> EquityOptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the equity option.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().id)
        True
        """
        ...

    def underlying_ticker(self, value: str) -> EquityOptionBuilder:
        """
        Set the underlying equity ticker symbol.

        Parameters
        ----------
        value : str
            Underlying equity ticker symbol.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().underlying_ticker)
        True
        """
        ...

    def strike(self, value: float) -> EquityOptionBuilder:
        """
        Set the strike price.

        Parameters
        ----------
        value : float
            Strike price. Must be finite and positive.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().strike)
        True
        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> EquityOptionBuilder:
        """
        Set the option type.

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the equity option.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().option_type)
        True
        """
        ...

    def exercise_style(self, value: Literal["european", "american", "bermudan"]) -> EquityOptionBuilder:
        """
        Set the exercise style.

        Parameters
        ----------
        value : {"european", "american", "bermudan"}
            Exercise style of the equity option. Defaults to ``"european"``
            when never set.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().exercise_style)
        True
        """
        ...

    def expiry(self, value: datetime.date) -> EquityOptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().expiry)
        True
        """
        ...

    def notional(self, value: Money) -> EquityOptionBuilder:
        """
        Set the notional amount for valuation scaling.

        Parameters
        ----------
        value : Money
            Notional amount for valuation scaling.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().notional)
        True
        """
        ...

    def discount_curve_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the discount curve identifier for present value calculations.

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().discount_curve_id)
        True
        """
        ...

    def spot_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the equity spot price identifier.

        Parameters
        ----------
        value : str
            Equity spot price identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().spot_id)
        True
        """
        ...

    def vol_surface_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the equity volatility surface identifier.

        Parameters
        ----------
        value : str
            Equity volatility surface identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().vol_surface_id)
        True
        """
        ...

    def div_yield_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the continuous dividend yield identifier.

        Parameters
        ----------
        value : str
            Continuous dividend yield identifier. If never set, the pricer
            treats the underlying as having zero continuous dividend yield.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().div_yield_id)
        True
        """
        ...

    def discrete_dividends(self, value: list[tuple[datetime.date, float]]) -> EquityOptionBuilder:
        """
        Set the discrete dividend schedule.

        Parameters
        ----------
        value : list[tuple[datetime.date, float]]
            Discrete dividend schedule as ``(ex_date, dividend_amount)``
            pairs. When provided, the escrowed dividend model is used for
            pricing.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().discrete_dividends)
        True
        """
        ...

    def exercise_schedule(self, value: list[datetime.date]) -> EquityOptionBuilder:
        """
        Set the exercise schedule for Bermudan options.

        Parameters
        ----------
        value : list[datetime.date]
            Dates on which early exercise is permitted. Required when
            ``exercise_style`` is ``"bermudan"``.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().exercise_schedule)
        True
        """
        ...

    def build(self) -> EquityOption:
        """
        Build the validated equity option.

        Returns
        -------
        EquityOption
            The validated equity option.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> callable(EquityOption.builder().build)
        True
        """
        ...

def bond_from_cashflows_json(
    instrument_id: str,
    schedule_json: str,
    discount_curve_id: str,
    quoted_clean: float | None = None,
) -> str:
    """
    Construct tagged bond instrument JSON from a cashflow schedule.

    Parameters
    ----------
    instrument_id : str
        Identifier for the bond instrument.
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.
    discount_curve_id : str
        Discount curve ID required for pricing.
    quoted_clean : float, optional
        Clean quoted price as a percent of par.

    Returns
    -------
    str
        JSON-encoded tagged ``InstrumentJson::Bond``.

    Raises
    ------
    ValueError
        If the schedule is invalid or bond construction fails.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import bond_from_cashflows_json
    >>> callable(bond_from_cashflows_json)
    True
    """
    ...

def validate_instrument_json(json: str) -> str:
    """
    Validate tagged instrument JSON and return canonical JSON.

    Parameters
    ----------
    json : str
        JSON string for a tagged valuation instrument.

    Returns
    -------
    str
        Canonical pretty-printed instrument JSON after Rust serde validation.

    Raises
    ------
    ValueError
        If the JSON is malformed, has an unknown instrument tag, or
        fails instrument-specific validation.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import validate_instrument_json
    >>> callable(validate_instrument_json)
    True
    """
    ...

def price_instrument(
    instrument_json: str | Bond | TermLoan | InterestRateSwap | Swaption | CapFloor | CreditDefaultSwap | CDSIndex,
    market: MarketContext | str,
    as_of: str,
    model: str = "default",
) -> str:
    """
    Price one instrument and return a ``ValuationResult`` JSON string.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex
        Tagged instrument JSON accepted by
        :func:`validate_instrument_json`, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : str
        ISO 8601 valuation date.
    model : str, default "default"
        Pricing model selector. Common values include ``"default"``,
        ``"discounting"``, ``"hazard_rate"``, and option-model keys such
        as ``"black76"`` where supported by the instrument.

    Returns
    -------
    str
        JSON-serialized valuation result containing value, currency, metrics,
        and covenant flags when applicable.

    Raises
    ------
    ValueError
        If any input JSON is malformed, required market data is
        missing, or the selected model is unsupported for the instrument.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import price_instrument
    >>> callable(price_instrument)
    True
    """
    ...

def price_instrument_with_metrics(
    instrument_json: str | Bond | TermLoan | InterestRateSwap | Swaption | CapFloor | CreditDefaultSwap | CDSIndex,
    market: MarketContext | str,
    as_of: str,
    model: str = "default",
    metrics: list[str] = [],
    pricing_options: str | None = None,
    market_history: str | None = None,
) -> str:
    """
    Price one instrument and compute explicit risk metric requests.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex
        Tagged instrument JSON, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : str
        ISO 8601 valuation date.
    model : str, default "default"
        Pricing model selector.
    metrics : list[str], default []
        Metric IDs to compute, such as ``"ytm"``, ``"dv01"``,
        ``"modified_duration"``, ``"hvar"``, or ``"expected_shortfall"``
        when supported by the instrument.
    pricing_options : str, optional
        Optional JSON string for metric pricing overrides.
    market_history : str, optional
        Optional JSON market-history payload required by
        historical risk metrics.

    Returns
    -------
    str
        JSON-serialized valuation result including requested metric values.

    Raises
    ------
    ValueError
        If a metric is unknown, not applicable, or cannot be
        calculated from the supplied market and history inputs.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import price_instrument_with_metrics
    >>> callable(price_instrument_with_metrics)
    True
    """
    ...

def instrument_cashflows_json(
    instrument_json: str | Bond | TermLoan | InterestRateSwap | Swaption | CapFloor | CreditDefaultSwap | CDSIndex,
    market: MarketContext | str,
    as_of: str,
    model: str,
) -> str:
    """
    Per-flow cashflow envelope for a discountable instrument.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex
        Tagged instrument JSON, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : str
        ISO 8601 valuation date.
    model : str
        ``"discounting"`` or ``"hazard_rate"``.

    Returns
    -------
    str
        JSON-serialized ``InstrumentCashflowEnvelope``.

    Raises
    ------
    ValueError
        If the model is unsupported, the instrument is unsupported
        for cashflow export, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import instrument_cashflows_json
    >>> callable(instrument_cashflows_json)
    True
    """
    ...

def list_models() -> list[str]:
    """
    Return every pricing model key registered in the standard pricer registry.

    The list is registry-derived rather than enum-derived, so it reflects real
    dispatch coverage: a model with no registered pricer is omitted. The names
    are the canonical keys accepted by the ``model`` argument of
    :func:`price_instrument`.

    Returns
    -------
    list[str]
        Canonical model keys such as ``"discounting"`` or ``"black76"``,
        deduplicated and sorted.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_models
    >>> callable(list_models)
    True
    """
    ...

def list_models_grouped() -> dict[str, list[str]]:
    """
    Return the standard registry's pricing models grouped by instrument type.

    Only instrument types with at least one registered pricer appear as keys,
    and each entry lists only the models that can price that instrument.

    Returns
    -------
    dict[str, list[str]]
        Mapping from canonical instrument-type name to its sorted model keys.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_models_grouped
    >>> callable(list_models_grouped)
    True
    """
    ...

def list_standard_metrics() -> list[str]:
    """
    Return all standard metric IDs registered by the Rust valuation engine.

    Returns
    -------
    list[str]
        Sorted list of fully qualified metric keys.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_standard_metrics
    >>> callable(list_standard_metrics)
    True
    """
    ...

def list_standard_metrics_grouped() -> dict[str, list[str]]:
    """
    Return standard metric IDs grouped by human-readable category.

    Returns
    -------
    dict[str, list[str]]
        Mapping from group label to sorted metric ID lists.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_standard_metrics_grouped
    >>> callable(list_standard_metrics_grouped)
    True
    """
    ...

def structured_credit_tranche_discount_margin(
    instrument_json: str,
    tranche_id: str,
    market: MarketContext | str,
    as_of: str,
    target_pv: float,
) -> float:
    """Solve a z-spread-equivalent discount margin for a floating-rate tranche.

    Contractual cashflows are projected without changing coupon projection,
    then a constant additive spread is applied to the discount curve. The
    result is zero at model PV, negative for a richer (higher) target PV, and
    positive for a cheaper (lower) target PV; it is not the contractual quoted
    margin.

    Parameters
    ----------
    instrument_json : str
        Tagged JSON for a ``StructuredCredit`` deal.
    tranche_id : str
        Identifier of the floating-rate tranche whose contractual cashflows
        are spread-discounted.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        the discount curve and any forward curves or historical fixings
        required for cashflow projection.
    as_of : str
        ISO 8601 valuation date used for projection and discounting.
    target_pv : float
        Target present value in the tranche's currency. Values above model PV
        produce a negative result; values below model PV produce a positive
        result.

    Returns
    -------
    float
        Z-spread-equivalent discount margin in decimal (``0.015`` = 150 bp).

    Raises
    ------
    ValueError
        If the JSON or date is malformed, the deal fails validation, the
        tranche is missing or fixed-rate, ``target_pv`` is not finite, required
        market data is unavailable, or the spread solve fails or exceeds
        ±5000 bp.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_discount_margin
    >>> callable(structured_credit_tranche_discount_margin)
    True
    """
    ...

def structured_credit_tranche_breakeven_cdr(
    instrument_json: str,
    tranche_id: str,
    market: MarketContext | str,
    as_of: str,
) -> float:
    """Solve the constant default rate at which a tranche first takes a writedown.

    Parameters
    ----------
    instrument_json : str
        Tagged JSON for a ``StructuredCredit`` deal.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : str
        ISO 8601 valuation date.

    Returns
    -------
    float
        Break-even annual CDR in decimal.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_breakeven_cdr
    >>> callable(structured_credit_tranche_breakeven_cdr)
    True
    """
    ...

def structured_credit_tranche_oas(
    instrument_json: str,
    tranche_id: str,
    market_price_pct: float,
    market: MarketContext | str,
    as_of: str,
    config_json: str | None = None,
) -> str:
    """Compute option-adjusted spread for a tranche. Returns JSON ``OasResult``.

    Parameters
    ----------
    instrument_json : str
        Tagged JSON for a ``StructuredCredit`` deal.
    tranche_id : str
        Identifier of the tranche within the deal.
    market_price_pct : float
        Market price as a percentage of original balance (100.0 = par).
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : str
        ISO 8601 valuation date.
    config_json : str or None, optional
        Serialized ``OasConfig``. All fields are required when supplied.

    Returns
    -------
    str
        JSON-serialized ``OasResult``.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_oas
    >>> callable(structured_credit_tranche_oas)
    True
    """
    ...

def structured_credit_tranche_metrics(
    instrument_json: str,
    tranche_id: str,
    market: MarketContext | str,
    as_of: str,
    market_price_pct: float | None = None,
) -> str:
    """Summary risk/pricing metrics for a tranche. Returns JSON ``TrancheMetrics``.

    Parameters
    ----------
    instrument_json : str
        Tagged JSON for a ``StructuredCredit`` deal.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : str
        ISO 8601 valuation date.
    market_price_pct : float or None, optional
        Market price as a percentage of original balance; the model price is
        used when omitted.

    Returns
    -------
    str
        JSON-serialized ``TrancheMetrics``.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_metrics
    >>> callable(structured_credit_tranche_metrics)
    True
    """
    ...

def structured_credit_tranche_scenario_table(
    instrument_json: str,
    tranche_id: str,
    market: MarketContext | str,
    as_of: str,
    grid_json: str,
) -> str:
    """Price a tranche across a CPR x CDR x severity grid. Returns JSON ``ScenarioTable``.

    Parameters
    ----------
    instrument_json : str
        Tagged JSON for a ``StructuredCredit`` deal.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : str
        ISO 8601 valuation date.
    grid_json : str
        Serialized ``ScenarioGrid``. Capped at 10,000 cells because each cell
        reprices the entire deal.

    Returns
    -------
    str
        JSON-serialized ``ScenarioTable``.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_scenario_table
    >>> callable(structured_credit_tranche_scenario_table)
    True
    """
    ...
