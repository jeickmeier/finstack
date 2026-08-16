"""
Cashflow primitives: CashFlow, CFKind, settlement classification.

Typed bindings for ``finstack_quant_cashflows::primitives``, mirroring the
Rust-canonical ``CFKind`` classification enum and the ``CashFlow`` dated
payment record used throughout schedule construction and accrual.

Example::

    >>> import datetime
    >>> from finstack_quant.cashflows.primitives import CashFlow, CFKind
    >>> from finstack_quant.core.money import Money
    >>> cf = CashFlow(
    ...     date=datetime.date(2025, 6, 15),
    ...     amount=Money(50_000.0, "USD"),
    ...     kind=CFKind.FIXED,
    ...     accrual_factor=0.5,
    ...     rate=0.05,
    ... )
    >>> cf.kind.name
    'fixed'

Examples
--------
>>> from finstack_quant.cashflows.primitives import CFKind
>>> CFKind.parse("fixed") == CFKind.FIXED
True

"""

from __future__ import annotations

import datetime

from finstack_quant.core.money import Money

__all__ = ["CFKind", "CashFlow", "is_cash_settlement_kind"]

class CFKind:
    """
    Cashflow classification (mirrors ``finstack_quant_core::cashflow::CFKind``).

    Each class attribute is a singleton instance for one Rust enum variant.
    Instances compare and hash by the underlying kind, and round-trip through
    their snake_case ``Display``/``FromStr`` label (e.g. ``"float_reset"``).

    Examples
    --------
    >>> from finstack_quant.cashflows.primitives import CFKind
    >>> CFKind.FIXED.name
    'fixed'
    >>> CFKind.parse("amortization") == CFKind.AMORTIZATION
    True
    """

    FIXED: CFKind
    FLOAT_RESET: CFKind
    INFLATION_COUPON: CFKind
    FEE: CFKind
    COMMITMENT_FEE: CFKind
    USAGE_FEE: CFKind
    FACILITY_FEE: CFKind
    NOTIONAL: CFKind
    PIK: CFKind
    AMORTIZATION: CFKind
    PRE_PAYMENT: CFKind
    REVOLVING_DRAW: CFKind
    REVOLVING_REPAYMENT: CFKind
    DEFAULTED_NOTIONAL: CFKind
    RECOVERY: CFKind
    ACCRUED_ON_DEFAULT: CFKind
    STUB: CFKind
    INITIAL_MARGIN_POST: CFKind
    INITIAL_MARGIN_RETURN: CFKind
    VARIATION_MARGIN_RECEIVE: CFKind
    VARIATION_MARGIN_PAY: CFKind
    MARGIN_INTEREST: CFKind
    COLLATERAL_SUBSTITUTION_IN: CFKind
    COLLATERAL_SUBSTITUTION_OUT: CFKind

    @classmethod
    def parse(cls, name: str) -> CFKind:
        """
        Parse a cashflow kind from its snake_case label or a documented alias.

        Parameters
        ----------
        name : str
            Snake_case label such as ``"fixed"`` or ``"float_reset"``, or a
            documented alias such as ``"amort"`` for ``AMORTIZATION``.
            Matching is case-insensitive and separator-normalized.

        Returns
        -------
        CFKind
            The kind whose canonical label or alias matches *name*.

        Raises
        ------
        ValueError
            If *name* does not match any known label or alias.

        Examples
        --------
        >>> from finstack_quant.cashflows.primitives import CFKind
        >>> CFKind.parse("fixed") == CFKind.FIXED
        True
        """
        ...

    @property
    def name(self) -> str:
        """
        Canonical snake_case label of this kind.

        Returns
        -------
        str
            The Rust ``Display`` label, such as ``"float_reset"`` or
            ``"defaulted_notional"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def is_interest_like(self) -> bool:
        """
        Return whether this kind is interest-bearing.

        Covers ``FIXED``, ``FLOAT_RESET``, ``INFLATION_COUPON``, and ``STUB``;
        all other kinds (principal, fees, credit events, margin) return
        ``False``.

        Returns
        -------
        bool
            ``True`` for interest-bearing coupon kinds, ``False`` otherwise.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

class CashFlow:
    """
    A single dated cash-flow (payment or reset).

    Represents a monetary flow at a specific date together with the
    classification and accrual metadata needed for downstream schedule
    aggregation, accrual, and risk calculations.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.cashflows.primitives import CashFlow, CFKind
    >>> from finstack_quant.core.money import Money
    >>> cf = CashFlow(datetime.date(2025, 6, 15), Money(100.0, "USD"), CFKind.FIXED)
    >>> cf.kind == CFKind.FIXED
    True
    """

    def __init__(
        self,
        date: datetime.date,
        amount: Money,
        kind: CFKind | str,
        reset_date: datetime.date | None = None,
        accrual_factor: float = 0.0,
        rate: float | None = None,
    ) -> None:
        """
        Construct a dated cashflow.

        Parameters
        ----------
        date : datetime.date
            Payment date (or reset date for ``CFKind.FLOAT_RESET`` fixing
            events).
        amount : Money
            Monetary amount including its currency.
        kind : CFKind | str
            Cashflow classification, either a ``CFKind`` instance or its
            snake_case label string (e.g. ``"fixed"``).
        reset_date : datetime.date, optional
            Index reset date for floating coupons; must not be after *date*.
        accrual_factor : float
            Accrual year fraction used for the coupon amount, default
            ``0.0``.
        rate : float, optional
            Effective annual rate used to compute this cashflow, when known.

        Raises
        ------
        ValueError
            If *kind* is a string that does not match a known label, or if
            *amount* has a non-finite value.
        """
        ...

    @property
    def date(self) -> datetime.date:
        """
        Payment date for this cashflow.

        Returns
        -------
        datetime.date
            The payment (or reset, for ``FLOAT_RESET``) date of this flow.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def reset_date(self) -> datetime.date | None:
        """
        Optional index reset date for floating coupons.

        Returns
        -------
        datetime.date or None
            The reset date, or ``None`` when this flow has no separate
            reset event.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def amount(self) -> Money:
        """
        Monetary amount including its currency.

        Returns
        -------
        Money
            The currency-tagged amount of this cashflow.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def kind(self) -> CFKind:
        """
        Cashflow classification.

        Returns
        -------
        CFKind
            The classification of this flow (interest, principal, fee, etc.).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def accrual_factor(self) -> float:
        """
        Accrual year fraction used for the coupon amount.

        Returns
        -------
        float
            The accrual year fraction, non-negative.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rate(self) -> float | None:
        """
        Effective annual rate used to calculate this cashflow, when known.

        Returns
        -------
        float or None
            The rate used for interest/fee flows, or ``None`` when this flow
            is not rate-based or the rate is unknown.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def validate(self) -> None:
        """
        Validate amount, accrual factor, rate, and reset-date ordering.

        Zero amounts are valid (e.g. floored coupons); only non-finite
        values, a negative accrual factor, or a reset date after the
        payment date are rejected.

        Raises
        ------
        ValueError
            If a value is non-finite, the accrual factor is negative, or
            the reset date is after the payment date.
        """
        ...

def is_cash_settlement_kind(kind: CFKind | str) -> bool:
    """
    Return whether a classified flow represents a cash settlement.

    ``PIK`` is a capitalization event and ``DEFAULTED_NOTIONAL`` is a
    write-down; both return ``False``. All other (settlement) kinds return
    ``True``.

    Parameters
    ----------
    kind : CFKind | str
        Classified cashflow kind to test, either a ``CFKind`` instance or
        its snake_case label string.

    Returns
    -------
    bool
        ``True`` for settlement kinds, ``False`` for ``PIK`` and
        ``DEFAULTED_NOTIONAL``.

    Raises
    ------
    ValueError
        If *kind* is a string that does not match a known label.

    Examples
    --------
    >>> from finstack_quant.cashflows.primitives import CFKind, is_cash_settlement_kind
    >>> is_cash_settlement_kind(CFKind.FIXED)
    True
    >>> is_cash_settlement_kind(CFKind.PIK)
    False
    """
    ...
