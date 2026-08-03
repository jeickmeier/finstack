"""
Core finstack_quant types: rates, identifiers, credit ratings, and attributes.

Provides typed wrappers for financial primitives used throughout the
``finstack_quant`` library.

Example::

    >>> from finstack_quant.core.types import Rate, Bps, Percentage
    >>> r = Rate(0.05)
    >>> r.as_percent
    5.0
    >>> r.as_bp
    500
    >>> Bps(250).as_decimal
    0.025
    >>> Percentage(12.5).as_decimal
    0.125

Examples
--------
>>> from finstack_quant.core.types import Rate
>>> Rate.from_percent(5.0).as_decimal
0.05

"""

from __future__ import annotations

from typing import Optional

__all__ = [
    "Rate",
    "Bps",
    "Percentage",
    "CreditRating",
    "CurveId",
    "InstrumentId",
    "Attributes",
]

class Rate:
    """
    A financial rate expressed as a decimal fraction.

    Immutable, hashable value type. Supports arithmetic and conversion between
    decimal, percent, and basis-point representations.

    Parameters
    ----------
    decimal : float
        Rate as a decimal fraction (e.g. ``0.05`` for 5%).

    Raises
    ------
    ValueError
        If *decimal* is not finite.

    Examples
    --------
    >>> from finstack_quant.core.types import Rate
    >>> r = Rate(0.05)
    >>> r.as_percent
    5.0
    >>> r.as_bp
    500
    >>> Rate.from_percent(5.0) == r
    True
    """

    ZERO: Rate
    """Zero rate (0% as a decimal rate)."""

    def __init__(self, decimal: float) -> None:
        """
        Construct a rate from a decimal fraction.

        Parameters
        ----------
        decimal : float
            Rate as a decimal (e.g. ``0.05`` for 5%).

        Raises
        ------
        ValueError
            If *decimal* is not finite.
        """
        ...

    @classmethod
    def from_percent(cls, percent: float) -> Rate:
        """
        Build from a percent value.

        Parameters
        ----------
        percent : float
            Rate in percent (e.g. ``5.0`` for 5%).

        Returns
        -------
        Rate

            Rate stored as ``percent / 100`` in decimal-rate form.
        Raises
        ------
        ValueError
            If *percent* is not finite.

        Examples
        --------
        >>> from finstack_quant.core.types import Rate
        >>> Rate.from_percent(5.0).as_decimal
        0.05

        """
        ...

    @classmethod
    def from_bp(cls, bp: int) -> Rate:
        """
        Build from an integer basis-point amount.

        Parameters
        ----------
        bp : int
            Basis points (e.g. ``500`` for 5%).

        Returns
        -------
        Rate
            Rate stored as ``bp / 10_000`` in decimal-rate form.


        Examples
        --------
        >>> from finstack_quant.core.types import Rate
        >>> Rate.from_bp(25).as_decimal
        0.0025

        """
        ...

    @property
    def as_decimal(self) -> float:
        """
        Rate as a decimal fraction.

        Returns
        -------
        float
            The as decimal exposed by this `Rate`.
        """
        ...

    @property
    def as_percent(self) -> float:
        """
        Rate as a percent value.

        Returns
        -------
        float
            The as percent exposed by this `Rate`.
        """
        ...

    @property
    def as_bp(self) -> int:
        """
        Rate rounded to the nearest basis point.

        Returns
        -------
        int
            The as bp exposed by this `Rate`.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __lt__(self, other: Rate) -> bool: ...
    def __le__(self, other: Rate) -> bool: ...
    def __gt__(self, other: Rate) -> bool: ...
    def __ge__(self, other: Rate) -> bool: ...
    def __add__(self, other: Rate) -> Rate: ...
    def __sub__(self, other: Rate) -> Rate: ...
    def __mul__(self, rhs: float) -> Rate: ...
    def __truediv__(self, rhs: float) -> Rate: ...
    def __neg__(self) -> Rate: ...

class Bps:
    """
    A value measured in basis points (1 bp = 0.0001).

    Immutable, hashable value type. Integer-valued internally; fractional
    input is rejected rather than rounded.

    Parameters
    ----------
    bp : float
        Whole basis-point value.

    Raises
    ------
    ValueError
        If *bp* is not finite or not a whole number of basis points. Use a
        decimal ``Rate`` or the JSON instrument path for sub-bp precision.

    Examples
    --------
    >>> from finstack_quant.core.types import Bps
    >>> Bps(250).as_decimal
    0.025
    >>> Bps(100).as_bp
    100
    """

    ZERO: Bps
    """Zero basis points."""

    def __init__(self, bp: float) -> None:
        """
        Construct from a whole basis-point value.

        Parameters
        ----------
        bp : float
            Whole basis-point value.

        Raises
        ------
        ValueError
            If *bp* is not finite or not a whole number of basis points.

        Examples
        --------
        >>> from finstack_quant.core.types import Bps
        >>> Bps(250).as_decimal
        0.025
        """
        ...

    @property
    def as_decimal(self) -> float:
        """
        Value as a decimal fraction.

        Returns
        -------
        float

            The as decimal exposed by this `Bps`.
        Examples
        --------
        >>> from finstack_quant.core.types import Bps
        >>> Bps(250).as_decimal
        0.025
        """
        ...

    @property
    def as_bp(self) -> int:
        """
        Value as whole basis points.

        Returns
        -------
        int

            The as bp exposed by this `Bps`.
        Examples
        --------
        >>> from finstack_quant.core.types import Bps
        >>> Bps(250).as_bp
        250
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __lt__(self, other: Bps) -> bool: ...
    def __le__(self, other: Bps) -> bool: ...
    def __gt__(self, other: Bps) -> bool: ...
    def __ge__(self, other: Bps) -> bool: ...
    def __add__(self, other: Bps) -> Bps: ...
    def __sub__(self, other: Bps) -> Bps: ...
    def __mul__(self, rhs: int) -> Bps: ...
    def __truediv__(self, rhs: int) -> Bps: ...
    def __neg__(self) -> Bps: ...

class Percentage:
    """
    A percentage value (e.g. 12.5 means 12.5%).

    Immutable, hashable value type.

    Parameters
    ----------
    percent : float
        Percentage value (e.g. ``12.5`` for 12.5%).

    Raises
    ------
    ValueError
        If *percent* is not finite.

    Examples
    --------
    >>> from finstack_quant.core.types import Percentage
    >>> Percentage(12.5).as_decimal
    0.125
    >>> Percentage(50.0).as_percent
    50.0
    """

    ZERO: Percentage
    """Zero percent."""

    def __init__(self, percent: float) -> None:
        """
        Construct from a percent value.

        Parameters
        ----------
        percent : float
            Percentage value (e.g. ``12.5`` for 12.5%).

        Raises
        ------
        ValueError
            If *percent* is not finite.

        Examples
        --------
        >>> from finstack_quant.core.types import Percentage
        >>> Percentage(12.5).as_decimal
        0.125
        """
        ...

    @property
    def as_decimal(self) -> float:
        """
        Value as a decimal fraction.

        Returns
        -------
        float

            The as decimal exposed by this `Percentage`.
        Examples
        --------
        >>> from finstack_quant.core.types import Percentage
        >>> Percentage(12.5).as_decimal
        0.125
        """
        ...

    @property
    def as_percent(self) -> float:
        """
        Value in percent terms.

        Returns
        -------
        float

            The as percent exposed by this `Percentage`.
        Examples
        --------
        >>> from finstack_quant.core.types import Percentage
        >>> Percentage(12.5).as_percent
        12.5
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __lt__(self, other: Percentage) -> bool: ...
    def __le__(self, other: Percentage) -> bool: ...
    def __gt__(self, other: Percentage) -> bool: ...
    def __ge__(self, other: Percentage) -> bool: ...

class CreditRating:
    """
    Standardised credit rating category.

    Immutable, hashable enum-style type with class attributes for each
    rating level. Notched ratings (e.g. ``"BBB+"`` and ``"Baa1"``) preserve
    their notch-level precision.

    Parameters
    ----------
    None
        Use class attributes (e.g. ``CreditRating.AAA``) or
        :meth:`from_name` to construct.

    Examples
    --------
    >>> from finstack_quant.core.types import CreditRating
    >>> CreditRating.AAA.name
    'AAA'
    >>> CreditRating.from_name("bbb+") == CreditRating.BBB_PLUS
    True
    """

    AAA: CreditRating
    """Highest quality rating."""
    AA_PLUS: CreditRating
    """AA+ / Aa1."""
    AA: CreditRating
    """AA category."""
    AA_MINUS: CreditRating
    """AA- / Aa3."""
    A_PLUS: CreditRating
    """A+ / A1."""
    A: CreditRating
    """Single-A category."""
    A_MINUS: CreditRating
    """A- / A3."""
    BBB_PLUS: CreditRating
    """BBB+ / Baa1."""
    BBB: CreditRating
    """BBB category."""
    BBB_MINUS: CreditRating
    """BBB- / Baa3."""
    BB_PLUS: CreditRating
    """BB+ / Ba1."""
    BB: CreditRating
    """BB category."""
    BB_MINUS: CreditRating
    """BB- / Ba3."""
    B_PLUS: CreditRating
    """B+ / B1."""
    B: CreditRating
    """B category."""
    B_MINUS: CreditRating
    """B- / B3."""
    CCC_PLUS: CreditRating
    """CCC+ / Caa1."""
    CCC: CreditRating
    """CCC category."""
    CCC_MINUS: CreditRating
    """CCC- / Caa3."""
    CC: CreditRating
    """CC category."""
    C: CreditRating
    """C category."""
    D: CreditRating
    """Default rating."""
    NR: CreditRating
    """Not rated."""

    @classmethod
    def from_name(cls, name: str) -> CreditRating:
        """
        Parse a rating string case-insensitively while preserving notches.

        Parameters
        ----------
        name : str
            Rating string (e.g. ``"BBB"``, ``"bbb+"``, ``"Baa1"``).

        Returns
        -------
        CreditRating

            Canonical S&P/Fitch grade after case normalization, agency-alias
            mapping, and notch preservation.
        Raises
        ------
        ValueError
            If *name* cannot be parsed.

        Examples
        --------
        >>> from finstack_quant.core.types import CreditRating
        >>> CreditRating.from_name("bbb+").name
        'BBB+'
        """
        ...

    @property
    def name(self) -> str:
        """
        Canonical S&P/Fitch-style rating name (e.g. ``"BBB-"``).

        Returns
        -------
        str

            The name exposed by this `CreditRating`.
        Examples
        --------
        >>> from finstack_quant.core.types import CreditRating
        >>> CreditRating.AAA.name
        'AAA'
        """
        ...

    @property
    def warf(self) -> float:
        """
        Moody's weighted-average rating factor for this exact notch.

        Returns
        -------
        float
            The warf exposed by this `CreditRating`.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class CurveId:
    """
    A unique identifier for a market data curve.

    Immutable, hashable string-wrapper type.

    Parameters
    ----------
    value : str
        Curve identifier string.

    Examples
    --------
    >>> from finstack_quant.core.types import CurveId
    >>> CurveId("USD-OIS").as_str()
    'USD-OIS'
    """

    def __init__(self, value: str) -> None:
        """
        Create a curve identifier from its string value.

        Parameters
        ----------
        value : str
            Curve identifier.

        Examples
        --------
        >>> from finstack_quant.core.types import CurveId
        >>> CurveId("USD-OIS").as_str()
        'USD-OIS'

        """
        ...

    def as_str(self) -> str:
        """
        Underlying string value.

        Returns
        -------
        str

            Exact curve-identifier text supplied at construction.
        Examples
        --------
        >>> from finstack_quant.core.types import CurveId
        >>> CurveId("USD-OIS").as_str()
        'USD-OIS'
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this curve id.

        Returns
        -------
        str
        """
        ...
    def __str__(self) -> str:
        """Return the string value of this curve id.

        Returns
        -------
        str
        """
        ...
    def __hash__(self) -> int:
        """Return a hash for this curve id.

        Returns
        -------
        int
        """
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether two curve ids are equal.

        Returns
        -------
        bool
        """
        ...
    def __ne__(self, other: object) -> bool:
        """Return whether two curve ids are not equal.

        Returns
        -------
        bool
        """
        ...

class InstrumentId:
    """
    A unique identifier for a financial instrument.

    Immutable, hashable string-wrapper type.

    Parameters
    ----------
    value : str
        Instrument identifier string.

    Examples
    --------
    >>> from finstack_quant.core.types import InstrumentId
    >>> InstrumentId("BOND_A").as_str()
    'BOND_A'
    """

    def __init__(self, value: str) -> None:
        """
        Create an instrument identifier from its string value.

        Parameters
        ----------
        value : str
            Instrument identifier.

        Examples
        --------
        >>> from finstack_quant.core.types import InstrumentId
        >>> InstrumentId("BOND_A").as_str()
        'BOND_A'

        """
        ...

    def as_str(self) -> str:
        """
        Underlying string value.

        Returns
        -------
        str

            Exact instrument-identifier text supplied at construction.
        Examples
        --------
        >>> from finstack_quant.core.types import InstrumentId
        >>> InstrumentId("BOND_A").as_str()
        'BOND_A'
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this instrument id.

        Returns
        -------
        str
        """
        ...
    def __str__(self) -> str:
        """Return the string value of this instrument id.

        Returns
        -------
        str
        """
        ...
    def __hash__(self) -> int:
        """Return a hash for this instrument id.

        Returns
        -------
        int
        """
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether two instrument ids are equal.

        Returns
        -------
        bool
        """
        ...
    def __ne__(self, other: object) -> bool:
        """Return whether two instrument ids are not equal.

        Returns
        -------
        bool
        """
        ...

class Attributes:
    """
    A mutable key-value metadata bag.

    Stores string-typed metadata entries with set/get semantics.

    Examples
    --------
    >>> from finstack_quant.core.types import Attributes
    >>> attributes = Attributes()
    >>> attributes.set_meta("desk", "credit")
    >>> (attributes.get_meta("desk"), attributes.contains_meta_key("desk"), attributes.keys())
    ('credit', True, ['desk'])

    """

    def __init__(self) -> None:
        """
        Create an empty attribute set.

        """
        ...

    def get_meta(self, key: str) -> Optional[str]:
        """
        Fetch metadata by key.

        Parameters
        ----------
        key : str
            Metadata key.

        Returns
        -------
        str | None
            Value if present, otherwise ``None``.

        """
        ...

    def set_meta(self, key: str, value: str) -> None:
        """
        Insert or replace a metadata entry.

        Parameters
        ----------
        key : str
            Metadata key.
        value : str
            Metadata value.

        """
        ...

    def contains_meta_key(self, key: str) -> bool:
        """
        Return whether *key* exists in metadata.

        Parameters
        ----------
        key : str
            Metadata key.

        Returns
        -------
        bool

            ``True`` when the metadata map contains ``key``; otherwise ``False``.
        """
        ...

    def keys(self) -> list[str]:
        """
        Metadata keys in sorted order.

        Returns
        -------
        list[str]

            Metadata keys sorted lexicographically for deterministic iteration.
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this attribute set.

        Returns
        -------
        str
        """
        ...
    def __len__(self) -> int:
        """Return the number of metadata entries.

        Returns
        -------
        int
        """
        ...
