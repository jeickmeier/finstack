"""
P&L attribution: decompose portfolio P&L into risk-factor contributions.

Bindings for ``finstack_quant_core::attribution``. Provides the
:class:`PnlAttribution` result type and the :func:`attribute_pnl` /
:func:`attribute_return_contribution` entry points, along with validation
helpers and default metric / waterfall ordering utilities.

Examples
--------
>>> from finstack_quant.attribution import default_waterfall_order
>>> default_waterfall_order()[:2]
['carry', 'rates_curves']
"""

from __future__ import annotations

import datetime

from typing import Any

import pandas as pd

__all__ = [
    "PnlAttribution",
    "attribute_pnl",
    "attribute_pnl_from_spec",
    "attribute_return_contribution",
    "default_attribution_metrics",
    "default_waterfall_order",
    "validate_attribution_json",
    "validate_return_contribution_json",
]

# P&L Attribution

class PnlAttribution:
    """
    P&L attribution result decomposing total P&L into risk factor contributions.

    Factors include carry, rates curves, credit curves, inflation, correlations,
    FX, volatility, cross-factor interactions, model parameters, market scalars,
    and residual.

    Construct via :meth:`from_json` or the :func:`attribute_pnl` helper.

    Examples
    --------
    >>> from finstack_quant.attribution import PnlAttribution
    >>> try:
    ...     PnlAttribution.from_json("{}")
    ... except ValueError as exc:
    ...     "total_pnl" in str(exc)
    True
    """

    @staticmethod
    def from_json(json: str) -> PnlAttribution:
        """
        Deserialize a ``PnlAttribution`` from JSON.

        Parameters
        ----------
        json : str
            JSON string (the ``attribution`` field from an
            ``AttributionResultEnvelope``).

        Returns
        -------
        PnlAttribution
            Parsed ``PnlAttribution`` instance.

        Examples
        --------
        >>> from finstack_quant.attribution import PnlAttribution
        >>> try:
        ...     PnlAttribution.from_json("{}")
        ... except ValueError as exc:
        ...     "total_pnl" in str(exc)
        True

        Raises
        ------
        ValueError
            If ``json`` is malformed or omits fields required by the P&L
            attribution schema.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        """
        ...

    def to_dict(self) -> dict[str, object]:
        """
        Export the canonical serde-shaped attribution payload as a dict.

        Returns
        -------
        dict[str, object]
            Attribution payload as a Python dict.

        """
        ...

    @property
    def total_pnl(self) -> float:
        """
        Total P&L amount (val_t1 − val_t0 + intra-period coupon income).

        For methods that follow the total-return convention (parallel,
        waterfall, Taylor), ``total_pnl`` includes coupon income received in
        the period. Use :attr:`mark_to_market_pnl` for the raw price change.

        Returns
        -------
        float
            Total P&L in :attr:`currency`.
        """
        ...

    @property
    def mark_to_market_pnl(self) -> float | None:
        """
        Raw mark-to-market P&L: ``val_t1 − val_t0`` with no cashflow adjustment.

        When the attribution method added coupon income to ``total_pnl`` (the
        standard total-return convention), this field still reports the raw
        mark-to-market change so a downstream consumer can reconcile against
        their own computation. ``None`` for attributions deserialized from a
        pre-audit JSON payload that did not carry the field.

        Returns
        -------
        float or None
            Raw MTM P&L in :attr:`currency`, or ``None`` when not stored.
        """
        ...

    @property
    def carry(self) -> float:
        """
        Carry (theta + accruals) P&L amount.

        Returns
        -------
        float
            Carry bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def rates_curves_pnl(self) -> float:
        """
        Interest rate curves P&L amount.

        Returns
        -------
        float
            Rates-curve bucket P&L in :attr:`currency`. Use
            :meth:`to_long_dataframe` for per-curve and per-tenor detail.
        """
        ...

    @property
    def credit_curves_pnl(self) -> float:
        """
        Credit hazard curves P&L amount.

        Returns
        -------
        float
            Credit-curve bucket P&L in :attr:`currency`. Use
            :meth:`to_credit_factor_dataframe` when a credit factor model was
            supplied.
        """
        ...

    @property
    def inflation_curves_pnl(self) -> float:
        """
        Inflation curves P&L amount.

        Returns
        -------
        float
            Inflation-curve bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def correlations_pnl(self) -> float:
        """
        Base correlation curves P&L amount.

        Returns
        -------
        float
            Correlation-curve bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def fx_pnl(self) -> float:
        """
        FX rate changes P&L amount.

        Pricing-impact FX P&L for cross-currency instruments. For pure
        single-currency instruments this is zero.

        Returns
        -------
        float
            FX pricing bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def fx_translation_pnl(self) -> float:
        """
        FX translation P&L amount.

        Reporting-currency FX P&L when ``AttributionConfig.target_currency`` was
        supplied and differs from native. Equals
        ``val_t0_native × (T1_fx − T0_fx)``. Zero by default.

        Returns
        -------
        float
            Translation bucket P&L in the reporting currency.
        """
        ...

    @property
    def vol_pnl(self) -> float:
        """
        Implied volatility changes P&L amount.

        Returns
        -------
        float
            Volatility bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def cross_factor_pnl(self) -> float:
        """
        Cross-factor interaction P&L amount.

        Returns
        -------
        float
            Cross-factor bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def model_params_pnl(self) -> float:
        """
        Model parameters P&L amount.

        Returns
        -------
        float
            Model-parameter bucket P&L in :attr:`currency`.
        """
        ...

    @property
    def market_scalars_pnl(self) -> float:
        """
        Market scalars P&L amount.

        Returns
        -------
        float
            Market-scalar bucket P&L in :attr:`currency` (dividends, repo, etc.).
        """
        ...

    @property
    def residual(self) -> float:
        """
        Residual (unexplained) P&L amount.

        Returns
        -------
        float
            ``total_pnl`` minus the sum of explained factor buckets, in
            :attr:`currency`.
        """
        ...

    @property
    def currency(self) -> str:
        """
        Currency code for all P&L amounts.

        Returns
        -------
        str
            The currency exposed by this `PnlAttribution`.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Instrument identifier.

        Returns
        -------
        str
            The instrument id exposed by this `PnlAttribution`.
        """
        ...

    @property
    def method(self) -> str:
        """
        Canonical attribution method name (``parallel``, ``waterfall``,
        ``metrics_based``, or ``taylor``).

        Returns
        -------
        str
            The method exposed by this `PnlAttribution`.
        """
        ...

    @property
    def t0(self) -> str:
        """
        Start date (T₀) as ISO string.

        Returns
        -------
        str
            The t0 exposed by this `PnlAttribution`.
        """
        ...

    @property
    def t1(self) -> str:
        """
        End date (T₁) as ISO string.

        Returns
        -------
        str
            The t1 exposed by this `PnlAttribution`.
        """
        ...

    @property
    def num_repricings(self) -> int:
        """
        Number of repricings performed.

        Returns
        -------
        int
            The num repricings exposed by this `PnlAttribution`.
        """
        ...

    @property
    def residual_pct(self) -> float:
        """
        Residual as percentage of total P&L.

        Returns
        -------
        float
            The residual pct exposed by this `PnlAttribution`.
        """
        ...

    @property
    def notes(self) -> list[str]:
        """
        Diagnostic notes and warnings.

        Returns
        -------
        list[str]
            The notes exposed by this `PnlAttribution`.
        """
        ...

    @property
    def result_invalid(self) -> bool:
        """
        True when attribution was flagged invalid and residual checks should fail.

        Returns
        -------
        bool
            The result invalid exposed by this `PnlAttribution`.
        """
        ...

    def residual_within_tolerance(
        self,
        pct_tolerance: float | None = None,
        abs_tolerance: float | None = None,
    ) -> bool:
        """
        Check if residual is within tolerance.

        Parameters
        ----------
        pct_tolerance : float or None
            Percentage tolerance (e.g. 0.1 for 0.1%).
            Defaults to the attribution's stored ``meta.tolerance_pct``.
        abs_tolerance : float or None
            Absolute tolerance (e.g. 100.0 for $100).
            Defaults to the attribution's stored ``meta.tolerance_abs``.

        Returns
        -------
        bool
            ``True`` if residual is within tolerance.

        """
        ...

    def residual_within_meta_tolerance(self) -> bool:
        """
        Check residual using the attribution's stored method-specific tolerances.

        Returns
        -------
        bool
            ``True`` when the residual satisfies the absolute or percentage
            tolerance stored in the attribution metadata; ``False`` for an
            invalid result or an out-of-tolerance residual.
        """
        ...

    def validate_currencies(self) -> None:
        """
        Validate that every factor's currency matches ``total_pnl.currency``.

        Useful before building a DataFrame or summing across instruments.

        Raises
        ------
        ValueError
            When any factor's currency differs from ``total_pnl.currency``.

        """
        ...

    def explain(self) -> str:
        """
        Human-readable tree explanation (non-zero factors only).

        Returns
        -------
        str
            Multi-line string with tree structure showing P&L breakdown.

        """
        ...

    def explain_verbose(self) -> str:
        """
        Verbose tree explanation including zero-valued factors.

        Returns
        -------
        str
            Multi-line string with tree structure showing all factors.

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export attribution as a single-row pandas DataFrame.

        Columns include ``instrument_id``, ``method``, ``t0``, ``t1``,
        ``currency``, ``total_pnl``, ``mark_to_market_pnl`` (nullable), all
        factor P&L amounts, ``residual``, ``residual_pct``,
        ``num_repricings``, and ``result_invalid``.

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame.

        """
        ...

    def to_long_dataframe(self) -> pd.DataFrame:
        """
        Export every populated detail breakdown as one long-format DataFrame.

        Columns:
            kind: dotted-path identifier (e.g. ``"rates.by_curve"``,
                ``"rates.by_tenor"``, ``"credit.by_curve"``, ``"fx.by_pair"``,
                ``"vol.by_surface"``, ``"cross_factor.by_pair"``,
                ``"scalars.dividends"``, ``"credit_factor.generic"``,
                ``"credit_factor.level"``, ``"credit_factor.adder"``,
                ``"credit_factor.curve_shape"``,
                ``"carry.coupon_income"``, ...).
            factor: parent factor family (``"rates"``, ``"credit"``, ``"fx"``,
                ``"vol"``, ``"cross_factor"``, ``"scalars"``,
                ``"credit_factor"``, ``"carry"``, ``"inflation"``,
                ``"correlations"``, ``"model_params"``).
            key_a: primary identifier (curve_id, pair label, vol_surface_id,
                equity_id, level_name, sub-component name).
            key_b: secondary key when present (tenor, ``to``-currency, bucket
                path); ``None`` otherwise.
            amount: float P&L amount.
            currency: 3-letter currency code.

        The DataFrame is empty (zero rows, schema columns present) when no
        detail breakdown was populated. Use
        ``df.query("kind.str.startswith('rates')")`` or
        ``df.pivot_table(index="key_a", columns="key_b", values="amount")``
        to slice the desired view.

        Returns
        -------
        pd.DataFrame
            Long-format DataFrame.

        """
        ...

    def to_carry_detail_dataframe(self) -> pd.DataFrame:
        """
        Export the carry decomposition as a typed long DataFrame.

        Columns are the same as :meth:`to_long_dataframe` but the kind values
        are limited to the ``"carry.*"`` family — useful when you only want the
        carry split (coupon income, pull-to-par, roll-down, funding cost),
        including the optional rates/credit splits when a credit
        factor model was supplied.

        Returns an empty DataFrame when ``carry_detail`` is not populated.

        Returns
        -------
        pd.DataFrame
            Carry-decomposition DataFrame.

        """
        ...

    def to_credit_factor_dataframe(self) -> pd.DataFrame:
        """
        Export the credit-factor hierarchy decomposition as a typed long DataFrame.

        Columns are the same as :meth:`to_long_dataframe`; rows are limited to
        the ``"credit_factor.*"`` family. Includes generic, per-level, adder,
        curve_shape, plus per-bucket and per-issuer rows when present.

        Returns an empty DataFrame when ``credit_factor_detail`` is not
        populated (no ``credit_factor_model`` was supplied, or the instrument
        has no resolvable issuer).

        Returns
        -------
        pd.DataFrame
            Credit-factor DataFrame.

        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this attribution.

        Returns
        -------
        str
        """
        ...

def attribute_pnl(
    instrument_json: str,
    market_t0_json: str,
    market_t1_json: str,
    as_of_t0: datetime.date | str,
    as_of_t1: datetime.date | str,
    method: str | dict[str, Any],
    config: dict[str, Any] | None = None,
    full_cross_attribution: bool | None = None,
) -> str:
    """
    Run P&L attribution for a single instrument.

    This is the main entry point. Accepts the instrument, two market
    snapshots, valuation dates, and a method descriptor and returns the
    canonical JSON form of the attribution. Use
    ``PnlAttribution.from_json(...)`` when you want the richer wrapper.

    Parameters
    ----------
    instrument_json : str
        Canonical v1 instrument envelope
        (``{"schema": "finstack_quant.instrument/1", "instrument": {...}}``).
    market_t0_json : str
        JSON-serialized ``MarketContext`` at T₀.
    market_t1_json : str
        JSON-serialized ``MarketContext`` at T₁.
    as_of_t0 : datetime.date | str
        Valuation date T₀ in ISO 8601 format.
    as_of_t1 : datetime.date | str
        Valuation date T₁ in ISO 8601 format.
    method : str or dict[str, Any]
        Attribution method — one of ``"parallel"``,
        ``{"waterfall": ["carry", "rates_curves", ...]}``,
        ``"metrics_based"``, or ``{"taylor": {"include_gamma": True, ...}}``.
    config : dict[str, Any] or None
        Optional config overrides (tolerance, metrics, bump sizes,
        target currency, or ``{"execution_policy": "serial"}`` for
        callers that already parallelize at the portfolio/batch level).
    full_cross_attribution : bool or None
        Option to compute all 36 cross-factor pairs when enabled.

    Returns
    -------
    str
        Compact JSON ``PnlAttribution`` payload.

    Examples
    --------
    >>> from finstack_quant.attribution import attribute_pnl
    >>> try:
    ...     attribute_pnl("{}", "{}", "{}", "2025-01-15", "2025-01-16", "parallel")
    ... except ValueError as exc:
    ...     "instrument envelope" in str(exc)
    True

    Raises
    ------
    ValueError
        If ``method`` or ``config`` cannot be serialized, an input JSON or ISO
        date is malformed, or attribution validation, pricing, or result
        serialization fails.
    KeyError
        If a required curve, market item, calendar, or FX triangulation leg is
        unavailable.
    RuntimeError
        If calibration or solver convergence fails, or attribution encounters
        an internal operational failure.
    """
    ...

def attribute_pnl_from_spec(spec_json: str) -> str:
    """
    Run attribution from a full JSON ``AttributionEnvelope``.

    Power-user variant for full envelope round-trip workflows.
    Most users should prefer :func:`attribute_pnl`.

    Parameters
    ----------
    spec_json : str
        JSON-serialized ``AttributionEnvelope``.

    Returns
    -------
    str
        JSON-serialized ``AttributionResultEnvelope``.

    Examples
    --------
    >>> from finstack_quant.attribution import attribute_pnl_from_spec
    >>> try:
    ...     attribute_pnl_from_spec("{}")
    ... except ValueError as exc:
    ...     "missing field" in str(exc)
    True

    Raises
    ------
    ValueError
        If ``spec_json`` is malformed or violates the exact attribution
        envelope schema, attribution validation or pricing fails, or the result
        cannot be serialized.
    KeyError
        If execution cannot find a required curve, market item, calendar, or FX
        triangulation leg.
    RuntimeError
        If calibration or solver convergence fails, or attribution encounters
        an internal operational failure.
    """
    ...

def attribute_return_contribution(spec_json: str) -> str:
    """
    Compute single-period return contribution attribution.

    Parameters
    ----------
    spec_json : str
        JSON-serialized return contribution specification.

    Returns
    -------
    str
        JSON-serialized return contribution result.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.attribution import attribute_return_contribution
    >>> spec = {
    ...     "as_of": "2026-01-02",
    ...     "weighting": "gross",
    ...     "factors": [],
    ...     "positions": [{"id": "A", "market_value": 100.0, "return": 0.02, "groups": {}}],
    ... }
    >>> json.loads(attribute_return_contribution(json.dumps(spec)))["portfolio_return"]
    0.02

    Raises
    ------
    ValueError
        If ``spec_json`` is malformed; required identifiers or positions are
        empty; numeric inputs are non-finite; position weighting modes are
        mixed or incomplete; factor or benchmark inputs are incomplete; or
        benchmark-relative weights do not sum to one.
    RuntimeError
        If result serialization fails or an internal post-validation invariant
        is violated.
    """
    ...

def validate_attribution_json(json: str) -> str:
    """
    Validate an attribution specification JSON.

    Deserializes against the ``AttributionEnvelope`` schema and returns
    the canonical (re-serialized) JSON.

    Parameters
    ----------
    json : str
        JSON-serialized ``AttributionEnvelope``.

    Returns
    -------
    str
        Canonical compact JSON.

    Examples
    --------
    >>> from finstack_quant.attribution import validate_attribution_json
    >>> try:
    ...     validate_attribution_json("{}")
    ... except ValueError as exc:
    ...     "missing field" in str(exc)
    True

    Raises
    ------
    ValueError
        If ``json`` is malformed, violates the exact attribution envelope
        schema, or cannot be canonically reserialized.
    """
    ...

def validate_return_contribution_json(spec_json: str) -> None:
    """
    Validate a return contribution specification JSON.

    Parameters
    ----------
    spec_json : str
        JSON-serialized return contribution specification.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.attribution import validate_return_contribution_json
    >>> spec = {
    ...     "as_of": "2026-01-02",
    ...     "weighting": "gross",
    ...     "factors": [],
    ...     "positions": [{"id": "A", "market_value": 100.0, "return": 0.02, "groups": {}}],
    ... }
    >>> validate_return_contribution_json(json.dumps(spec)) is None
    True

    Raises
    ------
    ValueError
        If ``spec_json`` is malformed; required identifiers or positions are
        empty; numeric inputs are non-finite; position weighting modes are
        mixed or incomplete; factor or benchmark inputs are incomplete; or
        benchmark-relative weights do not sum to one.
    RuntimeError
        If execution violates an internal invariant after validation.
    """
    ...

def default_waterfall_order() -> list[str]:
    """
    Return the default waterfall factor ordering.

    Returns
    -------
    list[str]
        Canonical snake-case factor names in the default waterfall order.

    Examples
    --------
    >>> from finstack_quant.attribution import default_waterfall_order
    >>> default_waterfall_order()[:3]
    ['carry', 'rates_curves', 'credit_curves']
    """
    ...

def default_attribution_metrics() -> list[str]:
    """
    Return the default metric IDs used by metrics-based attribution.

    Returns
    -------
    list[str]
        Metric identifier strings.

    Examples
    --------
    >>> from finstack_quant.attribution import default_attribution_metrics
    >>> default_attribution_metrics()[:3]
    ['theta', 'dv01', 'cs01']
    """
    ...
