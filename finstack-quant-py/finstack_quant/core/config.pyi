"""
Configuration types from ``finstack-quant-core``: rounding, tolerances, and global config.

Provides :class:`RoundingMode`, :class:`ToleranceConfig`, and
:class:`FinstackConfig` for controlling rounding behaviour and
numerical tolerance thresholds across the library.

Examples
--------
>>> from finstack_quant.core.config import RoundingMode
>>> RoundingMode.from_name("bankers") == RoundingMode.BANKERS
True

"""

from __future__ import annotations

from typing import Any, Optional

__all__ = [
    "RoundingMode",
    "ToleranceConfig",
    "FinstackConfig",
]

class RoundingMode:
    """
    Rounding mode for monetary and rate calculations.

    Enum-style class with class-level constants for each supported mode.

    Examples
    --------
    >>> from finstack_quant.core.config import RoundingMode
    >>> RoundingMode.from_name("bankers") == RoundingMode.BANKERS
    True

    """

    BANKERS: RoundingMode
    """Banker's rounding (ties to even)."""
    AWAY_FROM_ZERO: RoundingMode
    """Round halves away from zero."""
    TOWARD_ZERO: RoundingMode
    """Round toward zero (truncate)."""
    FLOOR: RoundingMode
    """Round toward negative infinity."""
    CEIL: RoundingMode
    """Round toward positive infinity."""

    @classmethod
    def from_name(cls, name: str) -> RoundingMode:
        """
        Parse a rounding mode from its exact canonical lowercase label.

        Parameters
        ----------
        name : str
            Label such as ``"bankers"``, ``"away_from_zero"``, ``"floor"``.

        Returns
        -------
        RoundingMode

            Rounding policy matching the exact canonical lowercase name,
            including its documented tie-breaking convention.
        Raises
        ------
        ValueError
            If *name* is not recognised.

        Examples
        --------
        >>> from finstack_quant.core.config import RoundingMode
        >>> str(RoundingMode.from_name("bankers"))
        'bankers'
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this rounding mode.

        Returns
        -------
        str
        """
        ...
    def __str__(self) -> str:
        """Return a human-readable name for this rounding mode.

        Returns
        -------
        str
        """
        ...
    def __hash__(self) -> int:
        """Return a hash for this rounding mode.

        Returns
        -------
        int
        """
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether two rounding modes are equal.

        Returns
        -------
        bool
        """
        ...

class ToleranceConfig:
    """
    Numerical tolerance settings for rate and generic comparisons.

    Parameters
    ----------
    rate_epsilon : float | None
        Epsilon for rate-style comparisons. If ``None``, the library
        default is used.
    generic_epsilon : float | None
        Epsilon for generic floating-point comparisons. If ``None``,
        the library default is used.

    Examples
    --------
    >>> from finstack_quant.core.config import ToleranceConfig
    >>> tolerances = ToleranceConfig(rate_epsilon=1e-9, generic_epsilon=1e-12)
    >>> (tolerances.rate_epsilon, tolerances.generic_epsilon)
    (1e-09, 1e-12)

    """

    def __init__(
        self,
        rate_epsilon: Optional[float] = None,
        generic_epsilon: Optional[float] = None,
    ) -> None:
        """
        Create tolerance settings, optionally overriding default epsilons.

        Parameters
        ----------
        rate_epsilon : float | None
            Epsilon for rate-style comparisons.
        generic_epsilon : float | None
            Epsilon for generic floating-point comparisons.

        Raises
        ------
        ValueError
            If either supplied epsilon is non-finite or not strictly positive.

        """
        ...

    @property
    def rate_epsilon(self) -> float:
        """
        Epsilon used for rate-style comparisons.

        Returns
        -------
        float

            The rate epsilon exposed by this `ToleranceConfig`.
        """
        ...

    @property
    def generic_epsilon(self) -> float:
        """
        Epsilon used for generic floating-point comparisons.

        Returns
        -------
        float

            The generic epsilon exposed by this `ToleranceConfig`.
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this tolerance config.

        Returns
        -------
        str
        """
        ...

class FinstackConfig:
    """
    Top-level library configuration combining rounding and tolerances.

    Parameters
    ----------
    rounding_mode : RoundingMode | None
        Rounding mode override. If ``None``, the library default is used.
    tolerances : ToleranceConfig | None
        Tolerance configuration override. If ``None``, the library default
        is used.

    Examples
    --------
    >>> from finstack_quant.core.config import FinstackConfig
    >>> config = FinstackConfig()
    >>> config.set_extension("example.settings.v1", {"enabled": True})
    >>> (config.output_scale("USD"), config.ingest_scale("JPY"), config.extension_keys())
    (2, 6, ['example.settings.v1'])
    >>> (config.get_extension("example.settings.v1"), config.get_extension_json("example.settings.v1"))
    ({'enabled': True}, '{"enabled":true}')
    >>> (config.remove_extension("example.settings.v1"), config.get_extension("example.settings.v1"))
    (True, None)
    >>> FinstackConfig.from_json(config.to_json()).output_scale("USD")
    2

    """

    def __init__(
        self,
        rounding_mode: Optional[RoundingMode] = None,
        tolerances: Optional[ToleranceConfig] = None,
    ) -> None:
        """
        Create a configuration, optionally overriding rounding mode and tolerances.

        Parameters
        ----------
        rounding_mode : RoundingMode | None
            Rounding mode.
        tolerances : ToleranceConfig | None
            Tolerance configuration.

        """
        ...

    def output_scale(self, currency: str) -> int:
        """
        Effective output decimal scale for a currency.

        Parameters
        ----------
        currency : str
            ISO-4217 alphabetic currency code.

        Returns
        -------
        int
            Number of decimal places for output formatting.

        Raises
        ------
        ValueError
            If *currency* is not recognised.

        """
        ...

    def ingest_scale(self, currency: str) -> int:
        """
        Effective ingest decimal scale for a currency.

        Parameters
        ----------
        currency : str
            ISO-4217 alphabetic currency code.

        Returns
        -------
        int
            Number of decimal places for input parsing.

        Raises
        ------
        ValueError
            If *currency* is not recognised.

        """
        ...

    def set_extension(self, key: str, value: Any) -> None:
        """
        Set a versioned registry/config extension from Python data or a JSON string.

        Parameters
        ----------
        key:
            Namespaced extension key used to locate the versioned configuration
            payload in this process-wide registry.
        value:
            Python data or a JSON string.

        Raises
        ------
        ValueError
            If *key* is not a versioned namespaced key of the form
            ``namespace.domain.vN``, or if *value* cannot be represented as
            JSON data.

        """
        ...

    def remove_extension(self, key: str) -> bool:
        """
        Remove a versioned registry/config extension.

        Parameters
        ----------
        key:
            Extension key to remove.

        Returns
        -------
        bool
            ``True`` when an extension was present.

        """
        ...

    def extension_keys(self) -> list[str]:
        """
        Return configured extension keys.

        Returns
        -------
        list[str]
            Extension key list.

        """
        ...

    def get_extension_json(self, key: str) -> Optional[str]:
        """
        Return one extension as a JSON string, or ``None`` if absent.

        Parameters
        ----------
        key:
            Namespaced extension key whose serialized payload is requested.

        Returns
        -------
        str or None
            JSON string, or ``None``.

        """
        ...

    def get_extension(self, key: str) -> Optional[Any]:
        """
        Return one extension as native Python data, or ``None`` if absent.

        Parameters
        ----------
        key:
            Namespaced extension key whose JSON payload is decoded to Python.

        Returns
        -------
        Any or None
            Python data, or ``None``.

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this config, including extensions, to JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `FinstackConfig`, suitable for a matching `from_json` call.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> FinstackConfig:
        """
        Deserialize a config from JSON.

        Parameters
        ----------
        json:
            JSON document matching the config schema.

        Returns
        -------
        FinstackConfig
            Parsed configuration.

        Raises
        ------
        ValueError
            If JSON parsing or schema validation fails.

        Examples
        --------
        >>> from finstack_quant.core.config import FinstackConfig
        >>> FinstackConfig.from_json(FinstackConfig().to_json()).output_scale("USD")
        2

        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this config.

        Returns
        -------
        str
        """
        ...
