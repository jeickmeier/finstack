"""
Instrument pricing, risk metrics, P&L attribution, and market-context bootstrapping.

The canonical path to build a :class:`finstack_quant.core.market_data.MarketContext`
from raw market quotes is :func:`calibrate`. A typical envelope has this shape::

    {
        "schema": "finstack_quant.calibration/1",
        "plan": {
            "id": "usd_curves",
            "quote_sets": {"usd_quotes": ["USD-SOFR-DEP-1M", "USD-OIS-SWAP-1Y"]},
            "steps": [{"id": "USD-OIS", "quote_set": "usd_quotes", "kind": "discount"}],
            "settings": {},
        },
        "market_data": [
            {"kind": "rate_quote", "type": "deposit", "id": "USD-SOFR-DEP-1M"},
            {"kind": "rate_quote", "type": "swap", "id": "USD-OIS-SWAP-1Y"},
        ],
    }

Pass that JSON to :func:`calibrate` and read ``result.market`` after
``result.success`` is true.

The :class:`CalibrationResult` wrapper carries the :class:`MarketContext` next
to per-step residuals (:meth:`step_report_json`, :meth:`to_report_dataframe`)
so users can verify their curves actually fit before consuming them downstream.

A ``CalibrationEnvelope`` carries inputs in three sections:

- ``plan`` — execution recipe. ``plan.steps`` declares calibration steps in
  declared order; ``plan.quote_sets`` maps a set name to a list of quote IDs
  that resolve into ``market_data``.
- ``market_data`` — flat, id-addressable list of all input data. Each entry
  has a ``"kind"`` discriminator. Quotes (``rate_quote``, ``cds_quote``,
  ``fx_quote``, ``inflation_quote``, ``vol_quote``, ``xccy_quote``, ``bond_quote``,
  ``cds_tranche_quote``) feed calibration steps. Snapshot data
  (``fx_spot``, ``price``, ``dividend_schedule``, ``fixing_series``,
  ``inflation_fixings``, ``credit_index``, ``fx_vol_surface``, ``vol_cube``,
  ``collateral``) is passed through into the resulting :class:`MarketContext`.
- ``prior_market`` — optional list of pre-built calibrated curves or surfaces
  from a previous run, layered in before steps execute.

Reference envelope JSON examples covering both Track-A (bootstrap from quotes)
and Track-B (snapshot-only) live under
``finstack-quant/valuations/examples/market_bootstrap/`` in the repository.

Instrument pricing helpers live under :mod:`finstack_quant.valuations.instruments`.
Reusable model engines live under :mod:`finstack_quant.models`. Portfolio
factor sensitivities and risk decomposition live under
:mod:`finstack_quant.portfolio`.

Examples
--------
>>> from finstack_quant.valuations import instruments
>>> hasattr(instruments, "price_instrument")
True

"""

from __future__ import annotations

import datetime
from typing import Any

import pandas as pd

from finstack_quant.core.dates import StubKind
from finstack_quant.core.market_data import MarketContext
from finstack_quant.valuations import composite as composite
from finstack_quant.valuations import credit_derivatives as credit_derivatives
from finstack_quant.valuations import instruments as instruments
from finstack_quant.valuations import market as market
from finstack_quant.valuations import schema as schema
from finstack_quant.valuations.envelope import CalibrationEnvelope as CalibrationEnvelope

__all__ = [
    "composite",
    "credit_derivatives",
    "instruments",
    "market",
    "schema",
    "ValuationResult",
    "CalibrationEnvelope",
    "CalibrationEnvelopeError",
    "CalibrationResult",
    "validate_calibration_json",
    "calibrate",
    "dry_run",
    "envelope",
    "dependency_graph_json",
    "tarn_coupon_profile",
    "snowball_coupon_profile",
    "inverse_floater_coupon_profile",
    "cms_spread_option_intrinsic",
    "callable_range_accrual_accrued",
    "instrument_cashflows",
]

class ValuationResult:
    """
    Valuation envelope: PV, currency, risk metrics, covenant flags, and JSON round-trip.

    Returned directly by the ``price_*`` helpers; :meth:`from_json` rebuilds one
    from a previously serialized payload.

    The rich ``details`` (model-specific pricing detail) and ``meta`` (numeric
    mode, rounding context, FX policy stamps) fields of the Rust envelope have
    no typed getters yet; they are reachable through ``to_json()`` only.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import StubKind
    >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import Bond, price_instrument
    >>> as_of = datetime.date(2024, 1, 15)
    >>> bond = Bond.fixed(
    ...     "B", Money(1000.0, Currency("USD")), Rate(0.05), as_of, datetime.date(2026, 1, 15), StubKind.NONE, "USD-OIS"
    ... )
    >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", as_of, 0.04))
    >>> result = price_instrument(bond, market, "2024-01-15")
    >>> (result.instrument_id, round(result.price, 2), result.currency)
    ('B', 1018.16, 'USD')

    """

    @staticmethod
    def from_json(json: str) -> ValuationResult:
        """
        Deserialize a ``ValuationResult`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        ValuationResult
            Parsed ``ValuationResult`` instance.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import StubKind
        >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Rate
        >>> from finstack_quant.valuations import ValuationResult
        >>> from finstack_quant.valuations.instruments import Bond, price_instrument
        >>> as_of = datetime.date(2024, 1, 15)
        >>> bond = Bond.fixed(
        ...     "B",
        ...     Money(1000.0, Currency("USD")),
        ...     Rate(0.05),
        ...     as_of,
        ...     datetime.date(2026, 1, 15),
        ...     StubKind.NONE,
        ...     "USD-OIS",
        ... )
        >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", as_of, 0.04))
        >>> result = ValuationResult.from_json(price_instrument(bond, market, "2024-01-15").to_json())
        >>> (result.instrument_id, round(result.price, 2), result.currency)
        ('B', 1018.16, 'USD')

        Raises
        ------
        ValueError
            If ``json`` is malformed or cannot be deserialized as a valuation result.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this result to pretty-printed JSON.

        Returns
        -------
        str
            Pretty-printed JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Instrument identifier assigned by the pricer.

        Returns
        -------
        str
            Instrument ID string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def as_of(self) -> datetime.date:
        """
        Valuation date (T+0) for the calculation.

        Returns
        -------
        datetime.date
            The valuation date stamped on this result.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def schema_version(self) -> int:
        """
        Wire-format schema version of the result envelope.

        Returns
        -------
        int
            Schema version number (currently ``1``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def price(self) -> float:
        """
        Present value amount (NPV).

        Returns
        -------
        float
            PV amount as a float.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def price_decimal(self) -> str:
        """
        Return the exact Decimal price as a string, without a float round-trip.

        Unlike the ``price`` property (a lossy ``float``), this preserves the
        internal Decimal representation exactly. Pass the result to
        ``decimal.Decimal`` for lossless arithmetic in Python.

        Returns
        -------
        str
            Exact decimal string of the valuation amount, e.g. ``"1000000.00"``.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def currency(self) -> str:
        """
        Currency code for the present value.

        Returns
        -------
        str
            Currency code string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def get_metric(self, key: str) -> float | None:
        """
        Return a scalar risk measure by string key.

        Parameters
        ----------
        key : str
            Metric identifier (e.g. ``"ytm"``, ``"dv01"``).

        Returns
        -------
        float or None
            Metric value, or ``None`` if missing.

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.
        """
        ...

    def metric_series(self, base: str) -> list[tuple[list[str], float]]:
        """
        Return decoded components and values for a composite base metric.

        Entries retain the deterministic insertion order of the serialized
        ``measures`` map. The scalar aggregate stored directly under ``base``
        is excluded. Malformed legacy escapes remain literal; decoded
        coordinate collisions fall back to literal wire components so no
        entries are dropped or deduplicated.

        Parameters
        ----------
        base : str
            Unqualified metric base key, such as ``"bucketed_dv01"``, used to
            select its encoded coordinate series from the valuation measures.

        Returns
        -------
        list[tuple[list[str], float]]
            Ordered ``(coordinate_components, value)`` pairs for matching
            composite metrics; the scalar aggregate stored at ``base`` is omitted.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def metric_keys(self) -> list[str]:
        """
        List metric keys present on this result.

        Returns
        -------
        list[str]
            All measure keys as strings.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def metric_count(self) -> int:
        """
        Count of measures stored on this result.

        Returns
        -------
        int
            Number of entries in the measures map.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def all_covenants_passed(self) -> bool:
        """
        Whether every covenant passed (or none were evaluated).

        Returns
        -------
        bool
            ``True`` if no covenant failures are recorded.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

    def failed_covenants(self) -> list[str]:
        """
        Covenant IDs that failed, if any.

        Returns
        -------
        list[str]
            List of failed covenant identifiers.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the headline result as a single-row pandas DataFrame.

        Columns: ``instrument_id``, ``as_of_date`` (ISO 8601 string), ``pv``,
        ``currency``, then one column per metric key in ``measures``
        insertion order.

        This is the default export, built from the Rust crate's own
        ``ValuationResult::to_row`` flattener. Stack a book with
        ``pd.concat([r.to_dataframe() for r in results])``; instruments with
        different metric sets align on column name and leave ``NaN``
        elsewhere.

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame with the identity columns followed by one
            column per metric.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_metrics_dataframe(self) -> pd.DataFrame:
        """
        Export as a single-row pandas DataFrame.

        Columns include ``instrument_id``, ``price``, ``currency``, plus one
        column per metric key.  Useful for stacking multiple results with
        ``pd.concat``.

        Prefer :meth:`to_dataframe`, which additionally carries the valuation
        date.

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame with one column per metric.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug string for this result.

        Returns
        -------
        str
            ``ValuationResult(id=..., price=..., currency=..., metrics=...)`` text.
        """
        ...

def instrument_cashflows(
    instrument_json: str,
    market: MarketContext | str,
    as_of: str,
    *,
    model: str,
) -> tuple[dict[str, Any], pd.DataFrame]:
    """
        DataFrame-friendly wrapper around :func:`instrument_cashflows_json`.

        Parses the JSON envelope returned by the low-level binding and constructs
        a per-flow ``pandas.DataFrame`` with ``date`` / ``reset_date`` parsed as
        ``datetime64``. See :func:`instrument_cashflows_json` for argument and
        error semantics.

        Parameters
        ----------
        instrument_json : str
    Canonical ``finstack_quant.instrument/1`` envelopes accepted by the valuation bindings.
        market : MarketContext or str
            Market context object or canonical market JSON containing the curves,
            fixings, and scalar data required by the requested pricing model.
        as_of : str
            ISO-8601 valuation date used to exclude settled flows and calculate
            schedule-relative discount factors.
        model : str
            Must be ``"discounting"`` or ``"hazard_rate"``. ``"default"`` is
            not accepted on cashflow export.

        Returns
        -------
        tuple[dict[str, Any], pd.DataFrame]
            ``(envelope, df)`` where ``envelope`` is the parsed dict and ``df``
            carries one row per flow with columns ``date``, ``amount``,
            ``currency``, ``kind``, ``accrual_factor``, ``year_fraction``,
            ``rate``, ``reset_date``, ``discount_factor``, ``discount_curve_id``,
            ``survival_probability``, ``conditional_default_prob``, ``inflation_index_ratio``,
            ``prepayment_smm``, ``beginning_balance``, ``ending_balance``, and
            ``pv``.

        Raises
        ------
        TypeError
            If ``instrument_json`` is neither a supported typed instrument nor
            a JSON string, or ``market`` is neither a ``MarketContext`` nor a
            JSON string.
        ValueError
            If instrument or market JSON is malformed, ``as_of`` or ``model``
            is invalid, the instrument/model pair is unsupported, or the
            generated cashflow schedule fails validation.
        KeyError
            If a curve, fixing, or other market datum required for cashflow
            generation or pricing is missing.
        RuntimeError
            If native pricing reports an internal, calibration, or solver failure.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import StubKind
        >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Rate
        >>> from finstack_quant.valuations.instruments import Bond
        >>> as_of = datetime.date(2024, 1, 1)
        >>> bond = Bond.fixed(
        ...     "B",
        ...     Money(1000.0, Currency("USD")),
        ...     Rate(0.05),
        ...     as_of,
        ...     datetime.date(2026, 1, 1),
        ...     StubKind.NONE,
        ...     "USD-OIS",
        ... )
        >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", as_of, 0.04))
        >>> from finstack_quant.valuations import instrument_cashflows
        >>> header, frame = instrument_cashflows(bond.to_json(), market, "2024-01-01", model="discounting")
        >>> (header["instrument_id"], len(frame))
        ('B', 6)

    """
    ...

# Calibration

class CalibrationResult:
    """
    Result of a calibration plan execution.

    Provides access to the calibrated market context, per-step reports,
    and overall success status.  Construct via :func:`calibrate` or
    :meth:`from_json`.

    Examples
    --------
    >>> import json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> from finstack_quant.valuations import calibrate
    >>> result = calibrate(json.dumps(envelope))
    >>> (result.success, result.rmse)
    (True, 0.0)

    """

    @staticmethod
    def from_json(json: str) -> CalibrationResult:
        """
        Deserialize a ``CalibrationResult`` from JSON.

        Parameters
        ----------
        json : str
            JSON string (a ``CalibrationResultEnvelope``).

        Returns
        -------
        CalibrationResult
            Parsed ``CalibrationResult`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or cannot be deserialized as a calibration result.

        Examples
        --------
        >>> import json
        >>> envelope = {
        ...     "schema": "finstack_quant.calibration/1",
        ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
        ... }
        >>> from finstack_quant.valuations import CalibrationResult, calibrate
        >>> restored = CalibrationResult.from_json(calibrate(json.dumps(envelope)).to_json())
        >>> (restored.success, restored.rmse)
        (True, 0.0)

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to pretty-printed JSON.

        Returns
        -------
        str
            Pretty-printed JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def success(self) -> bool:
        """
        Whether the overall calibration succeeded (all steps passed).

        Returns
        -------
        bool
            ``True`` if all steps passed.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def market(self) -> MarketContext:
        """
        The calibrated ``MarketContext`` containing all produced curves.

        Returns
        -------
        MarketContext
            Live market context ready for pricing and attribution.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def market_json(self) -> str:
        """
        The calibrated market serialized as a JSON string.

        Returns
        -------
        str
            JSON snapshot of the calibrated market.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def report_json(self) -> str:
        """
        The aggregated calibration report as a JSON string.

        Returns
        -------
        str
            JSON-serialized calibration report.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def step_ids(self) -> list[str]:
        """
        List of step identifiers ordered lexicographically by step ID.

        Returns
        -------
        list[str]
            Step IDs in lexicographic order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def iterations(self) -> int:
        """
        Total solver iterations across all steps.

        Returns
        -------
        int
            Sum of solver iterations.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def max_residual(self) -> float:
        """
        Maximum absolute dimensionless fit ratio across all steps.

        Returns
        -------
        float
            Largest ``abs(residual) / step_tolerance`` ratio.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rmse(self) -> float:
        """
        Root mean square dimensionless fit ratio across all steps.

        Returns
        -------
        float
            RMSE of ``abs(residual) / step_tolerance`` ratios.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def step_report_json(self, step_id: str) -> str:
        """
        Per-step calibration report as a JSON string.

        Parameters
        ----------
        step_id : str
            Identifier of the calibration step.

        Returns
        -------
        str
            JSON-serialized calibration report for the step.

        Raises
        ------
        ValueError
            If no step with the given *step_id* exists.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-step summary as a pandas DataFrame.

        Columns: ``step_id``, ``success``, ``iterations``, ``max_residual``,
        ``rmse``, ``convergence_reason``. Rows are ordered lexicographically
        by step ID.

        This is the default export and the same table as
        :meth:`to_report_dataframe`. The plan-level roll-ups (``success``,
        ``iterations``, ``max_residual``, ``rmse``) are properties on the
        result and are not repeated per row.

        Returns
        -------
        pd.DataFrame
            DataFrame with one row per calibration step.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_report_dataframe(self) -> pd.DataFrame:
        """
        Per-step summary as a pandas DataFrame.

        Columns: ``step_id``, ``success``, ``iterations``, ``max_residual``,
        ``rmse``, ``convergence_reason``. Identical to :meth:`to_dataframe`.

        Returns
        -------
        pd.DataFrame
            DataFrame with one row per calibration step.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str: ...

class CalibrationEnvelopeError(RuntimeError):
    """
    Structured calibration ingestion or execution failure.

    Attributes
    ----------
    kind : str
        Programmatic failure category such as ``"strict_load"``,
        ``"missing_dependency"``, ``"validation"``, or
        ``"solver_not_converged"``.
    stage : str
        Pipeline stage: ``"ingestion"``, ``"configuration"``, ``"context"``,
        ``"preflight"``, ``"target"``, or ``"solver"``.
    step_id : str or None
        Identifier of the offending step for step-scoped failures.
    solver_diagnostics : str or None
        JSON-serialized solver diagnostics for fit-acceptance failures.
    details : str
        JSON-serialized stable execution-error payload.

    Examples
    --------
    >>> from finstack_quant.valuations import CalibrationEnvelopeError, dry_run
    >>> try:
    ...     dry_run("{ malformed")
    ... except CalibrationEnvelopeError as exc:
    ...     print((exc.kind, exc.stage, exc.step_id))
    ('strict_load', 'ingestion', None)
    """

    kind: str
    stage: str
    step_id: str | None
    solver_diagnostics: str | None
    details: str

def validate_calibration_json(json: str) -> str:
    """
    Validate a calibration plan JSON and return canonical pretty-printed form.

    Parameters
    ----------
    json : str
        JSON-serialized ``CalibrationEnvelope``.

    Returns
    -------
    str
        Canonical pretty-printed JSON.

    Raises
    ------
    CalibrationEnvelopeError
        If the JSON is not a valid calibration envelope. Inherits from
        :class:`RuntimeError`.

    Examples
    --------
    >>> import json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> from finstack_quant.valuations import validate_calibration_json
    >>> json.loads(validate_calibration_json(json.dumps(envelope)))["plan"]["id"]
    'smoke'

    """
    ...

def dry_run(json: str) -> str:
    """
    Pre-flight envelope validation without invoking the solver.

    Runs all structural checks (missing dependencies, undefined ``quote_set``s,
    cycles) in a single pass and returns a JSON-serialized
    ``CalibrationValidationReport`` listing every error found plus the dependency graph.
    Microseconds — suitable as a fast pre-flight check before invoking
    :func:`calibrate`.

    Parameters
    ----------
    json : str
        JSON-serialized ``CalibrationEnvelope``.

    Returns
    -------
    str
        Pretty-printed JSON ``CalibrationValidationReport``. Inspect ``report["errors"]``
        for any structural problems and ``report["dependency_graph"]`` for the
        step DAG.

    Raises
    ------
    CalibrationEnvelopeError
        If the envelope JSON is malformed.

    Examples
    --------
    >>> import json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> from finstack_quant.valuations import dry_run
    >>> report = json.loads(dry_run(json.dumps(envelope)))
    >>> (report["errors"], report["dependency_graph"]["nodes"])
    ([], [])

    """
    ...

def dependency_graph_json(json: str) -> str:
    """
    Dump the static dependency graph of a calibration plan as JSON.

    Parameters
    ----------
    json : str
        JSON-serialized ``CalibrationEnvelope``.

    Returns
    -------
    str
        Pretty-printed JSON ``DependencyGraph`` with ``initial_ids`` (curve,
        surface, and scalar IDs supplied by ``market_data`` snapshots or
        ``prior_market``) and ``nodes`` (declared-order ``reads``/``writes``).

    Raises
    ------
    CalibrationEnvelopeError
        If the envelope JSON is malformed.

    Examples
    --------
    >>> import json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> from finstack_quant.valuations import dependency_graph_json
    >>> graph = json.loads(dependency_graph_json(json.dumps(envelope)))
    >>> (graph["initial_ids"], graph["nodes"])
    ([], [])

    """
    ...

def calibrate(json: str) -> CalibrationResult:
    """
    Build a :class:`MarketContext` from raw market quotes — the canonical entry point.

    Accepts a JSON-serialized ``CalibrationEnvelope``. The envelope carries
    quotes in two complementary places:

    - ``plan.quote_sets`` + ``plan.steps`` — quote-driven calibration steps
      (discount, forward, hazard, vol surface, swaption vol, base correlation,
      etc.). Each step reads its named list of quote IDs and resolves them
      against ``market_data``.
    - ``market_data`` — flat, id-addressable list of inputs. Quotes drive
      calibration steps; snapshot data (FX spots, prices, dividends, fixings,
      etc.) is passed through. Snapshot ``MarketQuote`` variants for FX and
      Bond exist for documentation but are not consumed by any calibration
      step today; pass FX rates as ``"kind": "fx_spot"`` entries and prices
      as ``"kind": "price"`` entries.
    - ``prior_market`` — pre-built curves and surfaces from a prior
      calibration, layered in before steps execute.

    Parameters
    ----------
    json : str
        JSON-serialized ``CalibrationEnvelope`` (schema string is
        ``"finstack_quant.calibration/1"``).

    Returns
    -------
    CalibrationResult
        :class:`CalibrationResult` with:
        - ``.market`` — the live :class:`MarketContext` (use this for
          pricing, attribution, scenarios, portfolio).
        - ``.market_json`` — same context as a JSON snapshot for
          persistence or comparison.
        - ``.report_json`` / ``.step_report_json(id)`` /
          ``.to_report_dataframe()`` — diagnostics. Always check
          ``.success`` and ``.rmse`` before relying on the produced market.
        - ``.iterations``, ``.max_residual``, ``.step_ids`` — summary stats.

    Raises
    ------
    CalibrationEnvelopeError
        If the JSON is malformed or calibration fails (e.g., missing
        dependency, solver non-convergence). The exception carries ``kind``,
        ``step_id``, and ``details`` attributes for programmatic handling.
        Inherits from :class:`RuntimeError` so legacy ``except RuntimeError``
        handlers continue to catch it.

    Examples
    --------
    >>> import json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> from finstack_quant.valuations import calibrate
    >>> result = calibrate(json.dumps(envelope))
    >>> (result.success, result.rmse)
    (True, 0.0)

    See Also
    --------
    - ``finstack-quant/valuations/examples/market_bootstrap/`` — reference
      envelope JSON files (discount curve, single-name hazard, FX matrix).
    - :func:`validate_calibration_json` — pre-flight envelope check.
    """
    ...

def tarn_coupon_profile(
    fixed_rate: float,
    coupon_floor: float,
    floating_fixings: list[float],
    target_coupon: float,
    day_count_fraction: float,
) -> dict[str, Any]:
    """
    Simulate a TARN coupon profile along a deterministic rate path.

    Each period coupon is ``max(fixed_rate - L_i, coupon_floor) * dcf``;
    payments accumulate until the cumulative reaches ``target_coupon``, at
    which point the final coupon is capped so the cumulative hits the
    target exactly and the note redeems early.

    Parameters
    ----------
    fixed_rate : float
        Fixed strike rate.
    coupon_floor : float
        Per-period floor on ``fixed_rate - L_i``.
    floating_fixings : list[float]
        Floating rate fixings (one per period).
    target_coupon : float
        Cumulative target that triggers knockout (> 0).
    day_count_fraction : float
        Year fraction applied to each period coupon.

    Returns
    -------
    dict[str, Any]
        Dict with keys ``coupons_paid`` (list[float]), ``cumulative``
        (list[float]), ``redemption_index`` (int | None) and
        ``redeemed_early`` (bool).

    Raises
    ------
    ValueError
        If ``fixed_rate`` or a fixing is non-finite; ``coupon_floor`` is
        non-finite or negative; or ``target_coupon`` or
        ``day_count_fraction`` is non-finite or non-positive.

    Examples
    --------
    >>> from finstack_quant.valuations import tarn_coupon_profile
    >>> profile = tarn_coupon_profile(0.05, 0.0, [0.02, 0.03, 0.04], 0.025, 0.5)
    >>> (profile["redeemed_early"], profile["redemption_index"], round(profile["cumulative"][-1], 3))
    (True, 1, 0.025)

    """
    ...

def snowball_coupon_profile(
    initial_coupon: float,
    fixed_rate: float,
    floating_fixings: list[float],
    floor: float,
    cap: float,
) -> list[float]:
    """
    Compute a snowball coupon schedule.

    Snowball: ``c_i = clip(c_{i-1} + fixed_rate - L_i, floor, cap)``
    with ``c_0 = initial_coupon``.

    Pass ``float('inf')`` as ``cap`` for an uncapped coupon.

    Parameters
    ----------
    initial_coupon : float
        First-period coupon for snowball mode.
    fixed_rate : float
        Fixed strike rate.
    floating_fixings : list[float]
        Floating rate fixings (one per period).
    floor : float
        Per-period coupon floor.
    cap : float
        Per-period coupon cap (use ``float('inf')`` for uncapped).
    is_inverse_floater : bool
        ``True`` for inverse floater mode, ``False`` for snowball.
    leverage : float, default 1.0
        Leverage multiplier for inverse floater mode.

    Returns
    -------
    list[float]
        Coupon schedule, one per period.

    Raises
    ------
    ValueError
        If ``fixed_rate``, ``initial_coupon``, ``floor``, or a fixing is
        non-finite; ``initial_coupon`` or ``floor`` is negative; or ``cap`` is
        NaN or is not strictly greater than ``floor``. Positive infinity is
        accepted as an uncapped ``cap``.

    Examples
    --------
    >>> from finstack_quant.valuations import snowball_coupon_profile
    >>> snowball_coupon_profile(0.03, 0.04, [0.02, 0.03, 0.05], 0.0, 0.10)
    [0.05, 0.06, 0.05]

    """
    ...

def inverse_floater_coupon_profile(
    fixed_rate: float,
    floating_fixings: list[float],
    floor: float,
    cap: float,
    leverage: float,
) -> list[float]:
    """
    Compute a path-independent inverse-floater coupon schedule.

    Parameters
    ----------
    fixed_rate : float
        Fixed strike rate in decimal annual-rate units.
    floating_fixings : list[float]
        Floating reference-rate fixings in decimal annual-rate units, one per
        coupon period in the returned schedule.
    floor : float
        Per-period minimum coupon rate in decimal annual-rate units.
    cap : float
        Per-period maximum coupon rate in decimal annual-rate units; use
        ``float("inf")`` for no cap.
    leverage : float
        Multiplier applied to each floating fixing before it offsets the fixed rate.

    Returns
    -------
    list[float]
        Coupon rate for each fixing after applying ``fixed_rate - leverage *
        fixing`` and clamping the result to ``[floor, cap]``.

    Raises
    ------
    ValueError
        If ``fixed_rate``, ``floor``, ``leverage``, or a fixing is non-finite;
        ``floor`` is negative; ``leverage`` is non-positive; or ``cap`` is NaN
        or is not strictly greater than ``floor``. Positive infinity is
        accepted as an uncapped ``cap``.

    Examples
    --------
    >>> from finstack_quant.valuations import inverse_floater_coupon_profile
    >>> [round(value, 3) for value in inverse_floater_coupon_profile(0.08, [0.02, 0.03, 0.05], 0.0, 0.10, 1.5)]
    [0.05, 0.035, 0.005]

    """
    ...

def cms_spread_option_intrinsic(
    long_cms: float,
    short_cms: float,
    strike: float,
    is_call: bool,
    notional: float,
) -> float:
    """
    Undiscounted intrinsic payoff of a CMS spread option.

    Call: ``notional * max(long_cms - short_cms - strike, 0)``.
    Put: ``notional * max(strike - (long_cms - short_cms), 0)``.

    Ignores CMS convexity, vol smile, and correlation adjustments — the
    full product pricer applies those on top of a copula model with
    SABR marginals.

    Parameters
    ----------
    long_cms : float
        Long CMS rate.
    short_cms : float
        Short CMS rate.
    strike : float
        Spread strike.
    is_call : bool
        ``True`` for a call, ``False`` for a put.
    notional : float
        Notional amount.

    Returns
    -------
    float
        Undiscounted intrinsic payoff.

    Raises
    ------
    ValueError
        If a CMS rate or ``strike`` is non-finite, or ``notional`` is
        non-finite or negative.

    Examples
    --------
    >>> from finstack_quant.valuations import cms_spread_option_intrinsic
    >>> round(cms_spread_option_intrinsic(0.05, 0.03, 0.01, True, 1_000_000.0), 2)
    10000.0

    """
    ...

def callable_range_accrual_accrued(
    lower: float,
    upper: float,
    observations: list[float],
    coupon_rate: float,
    day_count_fraction: float,
) -> float:
    """
    Accrued coupon over a range-accrual period.

    Counts the fraction of ``observations`` within the inclusive interval
    ``[lower, upper]`` and returns
    ``coupon_rate * day_count_fraction * fraction``.

    The call provision is not applied here — this is the coupon that
    would accrue assuming the note is not called before period end.

    Parameters
    ----------
    lower : float
        Lower bound of the accrual range.
    upper : float
        Upper bound of the accrual range.
    observations : list[float]
        Observed values (one per day in the period).
    coupon_rate : float
        Coupon rate (decimal).
    day_count_fraction : float
        Year fraction for the period.

    Returns
    -------
    float
        Accrued coupon amount.

    Raises
    ------
    ValueError
        If ``lower`` or ``upper`` is non-finite or ``lower >= upper``;
        ``observations`` is empty or contains a non-finite value; or
        ``coupon_rate`` or ``day_count_fraction`` is non-finite or negative.

    Examples
    --------
    >>> from finstack_quant.valuations import callable_range_accrual_accrued
    >>> callable_range_accrual_accrued(0.01, 0.03, [0.005, 0.02, 0.03, 0.04], 0.08, 0.25)
    0.01

    """
    ...
