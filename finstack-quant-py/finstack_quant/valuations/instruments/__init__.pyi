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

from finstack_quant.core.dates import DayCount, Tenor
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.types import Bps, Rate

__all__ = [
    "Bond",
    "FixedLegSpec",
    "FloatLegSpec",
    "InterestRateSwap",
    "InterestRateSwapBuilder",
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
    instrument_json: str | Bond | TermLoan | InterestRateSwap,
    market: MarketContext | str,
    as_of: str,
    model: str = "default",
) -> str:
    """
    Price one instrument and return a ``ValuationResult`` JSON string.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap
        Tagged instrument JSON accepted by
        :func:`validate_instrument_json`, or a typed :class:`Bond` /
        :class:`TermLoan` instance.
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
    instrument_json: str | Bond | TermLoan | InterestRateSwap,
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
    instrument_json : str or Bond or TermLoan or InterestRateSwap
        Tagged instrument JSON, or a typed :class:`Bond` /
        :class:`TermLoan` instance.
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
    instrument_json: str | Bond | TermLoan | InterestRateSwap,
    market: MarketContext | str,
    as_of: str,
    model: str,
) -> str:
    """
    Per-flow cashflow envelope for a discountable instrument.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap
        Tagged instrument JSON, or a typed :class:`Bond` /
        :class:`TermLoan` instance.
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
