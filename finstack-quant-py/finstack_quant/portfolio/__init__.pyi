"""
Portfolio construction, valuation, optimization, cashflows, scenarios, and metrics.

Examples
--------
>>> from finstack_quant.portfolio import Portfolio
>>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
>>> (Portfolio.from_spec(spec).id, len(Portfolio.from_spec(spec)))
('empty', 0)
"""

from __future__ import annotations

from typing import Any

import numpy as np
import numpy.typing as npt
import pandas as pd

from finstack_quant.core.market_data import DiscountCurve, MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.table import ArrowTable
from finstack_quant.factor_model.credit import CreditFactorModel

__all__ = [
    "ContractLimitExceededError",
    "ContractValidationError",
    "FinstackError",
    "FinstackFxError",
    "FinstackOptimizationError",
    "FinstackValuationError",
    "InstrumentArtifactCache",
    "MalformedContractSchemaError",
    "MaterializationReport",
    "MissingContractVersionError",
    "Portfolio",
    "PortfolioAttribution",
    "PortfolioCashflows",
    "PortfolioError",
    "PortfolioMetrics",
    "PortfolioResult",
    "PortfolioValuation",
    "UnsupportedContractVersionError",
    "aggregate_full_cashflows",
    "aggregate_metrics",
    "almgren_chriss_impact",
    "amihud_illiquidity",
    "apply_scenario_and_revalue",
    "attribute_portfolio_pnl",
    "allocate_weights",
    "brinson_fachler",
    "build_credit_vol_report",
    "build_portfolio_from_spec",
    "build_stress_attribution",
    "campisi_attribution",
    "campisi_carino_link",
    "campisi_carino_link_from_snapshots",
    "campisi_reconciliation_check",
    "carino_link",
    "cell_returns_from_curves",
    "cell_returns_from_reference",
    "days_to_liquidate",
    "evaluate_risk_budget",
    "excess_returns",
    "factor_brinson_attribution",
    "factor_stress",
    "grid_attribution",
    "grid_carino_link",
    "historical_var_decomposition",
    "kyle_lambda",
    "liquidity_tier",
    "lvar_bangia",
    "mwr_xirr",
    "optimize_portfolio",
    "parametric_es_decomposition",
    "parametric_var_decomposition",
    "parse_portfolio_spec",
    "portfolio_result_get_metric",
    "portfolio_result_total_value",
    "replay_portfolio",
    "position_what_if",
    "roll_effective_spread",
    "scenario_pnl",
    "scenario_pnl_batch",
    "twrr_linked",
    "twrr_modified_dietz",
    "validate_allocation_json",
    "value_portfolio",
    # factor_model typed result classes
    "FactorContribution",
    "PositionFactorContribution",
    "PositionResidualContribution",
    "RiskDecomposition",
    "FactorRiskDecomposition",
    "SensitivityMatrix",
    "FactorPnlProfile",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "decompose_factor_risk",
    "PositionVarContribution",
    "PositionEsContribution",
    "PositionRiskDecomposition",
    "PositionBudgetEntry",
    "RiskBudgetResult",
    "FactorContributionDelta",
    "WhatIfResult",
    "StressResult",
    "StressPositionEntry",
    "TailScenarioBreakdown",
    "StressAttribution",
    "PositionAssignment",
    "UnmatchedEntry",
    "FactorAssignmentReport",
    "LevelVolContribution",
    "PositionVolContribution",
    "CreditVolReport",
    "VolHorizon",
    "DecompositionConfig",
    "position_component_var",
    # optimization spec/result classes
    "WeightingScheme",
    "MissingMetricPolicy",
    "Inequality",
    "TradeDirection",
    "TradeType",
    "PerPositionMetric",
    "PositionFilter",
    "MetricExpr",
    "Objective",
    "Constraint",
    "CandidatePosition",
    "TradeUniverse",
    "OptimizationStatus",
    "TradeSpec",
    "PortfolioOptimizationSpec",
    "PortfolioOptimizationResult",
]

class FinstackError(ValueError):
    """
    Base class for the library's named exceptions.

    Catch this in a single ``except FinstackError`` clause to handle any error
    the library raises through one of its own exception classes, rather than
    enumerating them.

    It derives from ``ValueError``, so every subclass keeps ``ValueError`` in
    its MRO and existing ``except ValueError`` clauses are unaffected. Bare
    ``ValueError`` / ``KeyError`` raised by the lower-level mapping helpers are
    deliberately not reparented and are *not* caught by this class.

    Its canonical home is ``finstack_quant.core``; it is re-exported here
    alongside the portfolio exception family.

    Examples
    --------
    >>> from finstack_quant.portfolio import FinstackError, PortfolioError
    >>> issubclass(PortfolioError, FinstackError)
    True
    >>> issubclass(FinstackError, ValueError)
    True
    """

class PortfolioError(FinstackError):
    """
    Portfolio validation or calculation failure.

    Examples
    --------
    >>> from finstack_quant.portfolio import PortfolioError
    >>> str(PortfolioError("invalid portfolio"))
    'invalid portfolio'
    """

class FinstackValuationError(PortfolioError):
    """
    Portfolio valuation failure.

    Examples
    --------
    >>> from finstack_quant.portfolio import FinstackValuationError
    >>> str(FinstackValuationError("valuation failed"))
    'valuation failed'
    """

class FinstackFxError(PortfolioError):
    """
    Portfolio FX conversion or market-data failure.

    Examples
    --------
    >>> from finstack_quant.portfolio import FinstackFxError
    >>> str(FinstackFxError("missing FX rate"))
    'missing FX rate'
    """

class FinstackOptimizationError(PortfolioError):
    """
    Portfolio optimization failure.

    Examples
    --------
    >>> from finstack_quant.portfolio import FinstackOptimizationError
    >>> str(FinstackOptimizationError("infeasible"))
    'infeasible'
    """

class ContractValidationError(FinstackError):
    """
    Persisted contract validation failure with structured diagnostics.

    ``report`` is a list of dictionaries with stable ``code`` and optional
    RFC 6901 ``pointer`` entries suitable for form and ingestion error UIs.

    Examples
    --------
    >>> from finstack_quant.portfolio import ContractValidationError
    >>> issubclass(ContractValidationError, ValueError)
    True
    """

    report: list[dict[str, Any]]

class UnsupportedContractVersionError(ContractValidationError):
    """
    Persisted payload uses an unsupported positive contract version.

    Examples
    --------
    >>> from finstack_quant.portfolio import UnsupportedContractVersionError
    >>> issubclass(UnsupportedContractVersionError, ContractValidationError)
    True
    """

class MissingContractVersionError(ContractValidationError):
    """
    Strict persisted payload omits its required contract version.

    Examples
    --------
    >>> from finstack_quant.portfolio import MissingContractVersionError
    >>> issubclass(MissingContractVersionError, ContractValidationError)
    True
    """

class MalformedContractSchemaError(ContractValidationError):
    """
    Persisted payload has a malformed or mismatched schema marker.

    Examples
    --------
    >>> from finstack_quant.portfolio import MalformedContractSchemaError
    >>> issubclass(MalformedContractSchemaError, ContractValidationError)
    True
    """

class ContractLimitExceededError(ContractValidationError):
    """
    Persisted payload exceeds a configured byte, count, or depth bound.

    Examples
    --------
    >>> from finstack_quant.portfolio import ContractLimitExceededError
    >>> issubclass(ContractLimitExceededError, ContractValidationError)
    True
    """

class InstrumentArtifactCache:
    """
    Reusable bounded cache for decoded content-addressed instruments.

    ``capacity`` bounds retained artifacts; encoded source data is also bounded
    at 64 MiB. Instances are frozen and thread-safe.

    Examples
    --------
    >>> from finstack_quant.portfolio import InstrumentArtifactCache
    >>> cache = InstrumentArtifactCache(5_000)
    >>> (len(cache), cache.size, cache.decode_count)
    (0, 0, 0)
    """

    def __init__(self, capacity: int = 4_096) -> None:
        """
        Create an empty cache with an explicit artifact-entry bound.

        Parameters
        ----------
        capacity : int, default 4096
            Maximum decoded artifacts retained by the cache. Benchmarks should
            pass the fixture's unique-artifact count explicitly.

        Raises
        ------
        TypeError
            If ``capacity`` is not an integer.
        OverflowError
            If ``capacity`` is negative or exceeds the native ``usize`` range.

        Returns
        -------
        None
            Constructors initialize the cache in place.
        """
    @property
    def size(self) -> int:
        """
        Number of decoded artifacts currently retained.

        Returns
        -------
        int
            Entry count between zero and the configured capacity.
        """
    @property
    def decode_count(self) -> int:
        """
        Cumulative number of successful cache-miss decodes.

        Returns
        -------
        int
            Non-negative decode count for this cache instance.
        """
    def __len__(self) -> int:
        """
        Return the number of retained decoded artifacts.

        Returns
        -------
        int
            Same value as :attr:`size`.
        """

class MaterializationReport:
    """
    Immutable outcome metadata for one successful materialization call.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import Portfolio
    >>> bundle = {
    ...     "schema": "finstack_quant.portfolio_materialization/1",
    ...     "portfolio": {"id": "empty", "base_currency": "USD", "as_of": "2025-01-01", "entities": {}},
    ...     "instruments": [],
    ...     "positions": [],
    ... }
    >>> _, report = Portfolio.from_materialization(json.dumps(bundle))
    >>> (report.positions, report.unique_instruments)
    (0, 0)
    """

    @property
    def diagnostics(self) -> list[dict[str, Any]]:
        """
        Return bounded non-fatal structured diagnostics.

        Returns
        -------
        list[dict[str, Any]]
            Diagnostic dictionaries with stable codes, phases, severities,
            messages, optional JSON pointers, and artifact context.
        """
    @property
    def truncated(self) -> bool:
        """
        Whether diagnostics were omitted because the bound was reached.

        Returns
        -------
        bool
            ``True`` when one or more diagnostics were not retained.
        """
    @property
    def unique_instruments(self) -> int:
        """
        Number of unique instrument artifacts in the source bundle.

        Returns
        -------
        int
            Non-negative artifact count.
        """
    @property
    def positions(self) -> int:
        """
        Number of ordered positions materialized.

        Returns
        -------
        int
            Non-negative position count.
        """
    @property
    def dependencies(self) -> int:
        """
        Sum of normalized market-dependency keys over unique artifacts.

        Returns
        -------
        int
            Non-negative dependency count before position fan-out.
        """
    @property
    def cache_hits(self) -> int:
        """
        Number of unique artifacts served from the supplied cache.

        Returns
        -------
        int
            Non-negative hit count no greater than ``unique_instruments``.
        """
    @property
    def input_bytes(self) -> int:
        """
        Number of UTF-8 bytes in the supplied materialization bundle.

        Returns
        -------
        int
            Positive encoded byte count for the source document.
        """
    @property
    def timing_available(self) -> bool:
        """
        Whether every phase used an available monotonic host clock.

        Returns
        -------
        bool
            ``True`` for native Python builds, where ``time::Instant`` backs
            every phase measurement.
        """
    @property
    def phase_nanos(self) -> dict[str, int]:
        """
        Return sequential materialization phase timings.

        Returns
        -------
        dict[str, int]
            Nanosecond timings for parse, version validation, instrument
            decode, position build, and index build phases.
        """

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the materialization counters as a single-row DataFrame.

        The report is a flat set of counters, so it yields exactly one row —
        concatenate rows across loads to chart materialization cost over time.
        The nested ``phase_nanos`` mapping is flattened into one column per
        phase; the structured :attr:`diagnostics` payload is not included.

        Columns: ``unique_instruments``, ``positions``, ``dependencies``,
        ``cache_hits``, ``input_bytes``, ``truncated``, ``timing_available``,
        ``phase_parse_nanos``, ``phase_validate_versions_nanos``,
        ``phase_decode_instruments_nanos``, ``phase_build_positions_nanos``,
        ``phase_index_build_nanos``.

        Returns
        -------
        pd.DataFrame
            Exactly one row. The ``phase_*`` columns are zero — not measured
            durations — whenever ``timing_available`` is ``False``.
        """
        ...

class Portfolio:
    """
    Built runtime portfolio. Cheap to clone; pass directly to pipeline functions.

    Build once with :meth:`from_spec` and reuse across ``value_portfolio``,
    ``aggregate_full_cashflows``, ``aggregate_metrics``, and
    ``apply_scenario_and_revalue`` to skip the per-call spec parse + index
    rebuild.

    Examples
    --------
    >>> from finstack_quant.portfolio import Portfolio
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> (Portfolio.from_spec(spec).id, len(Portfolio.from_spec(spec)))
    ('empty', 0)
    """

    @staticmethod
    def from_spec(spec_json: str) -> Portfolio:
        """
        Parse a ``PortfolioSpec`` JSON string into a runtime portfolio.

        Parameters
        ----------
        spec_json : str
            Portfolio specification JSON, including positions, base currency,
            and as-of date, to validate and compile for reuse.

        Returns
        -------
        Portfolio
            Validated runtime portfolio built from ``spec_json`` with lookup indices rebuilt.

        Raises
        ------
        ValueError
            If ``spec_json`` is malformed or does not match the ``PortfolioSpec`` schema.
        PortfolioError
            If the decoded positions or portfolio identifiers violate portfolio
            construction invariants.

        Examples
        --------
        >>> from finstack_quant.portfolio import Portfolio
        >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
        >>> portfolio = Portfolio.from_spec(spec)
        >>> (portfolio.id, portfolio.base_currency, len(portfolio))
        ('empty', 'USD', 0)
        """
        ...

    @staticmethod
    def from_materialization(
        bundle: str | bytes | bytearray,
        cache: InstrumentArtifactCache | None = None,
    ) -> tuple[Portfolio, MaterializationReport]:
        """
        Parse and build a strict persisted portfolio bundle in one native call.

        Parsing, validation, unique instrument decoding, position construction,
        and index rebuilding run while the GIL is released. Pass a reusable
        cache to share decoded instruments across bundles.

        Parameters
        ----------
        bundle : str | bytes | bytearray
            Complete UTF-8 JSON document using the
            ``finstack_quant.portfolio_materialization/1`` contract.
        cache : InstrumentArtifactCache | None, default None
            Shared artifact cache, or ``None`` to use an ephemeral bounded
            cache for this call.

        Returns
        -------
        tuple[Portfolio, MaterializationReport]
            Frozen reusable portfolio handle and immutable load report.

        Raises
        ------
        TypeError
            If ``bundle`` is not ``str``, ``bytes``, or ``bytearray``.
        ContractValidationError
            If JSON parsing or structured contract validation fails. The
            exception's ``report`` attribute contains diagnostic dictionaries.
        ContractLimitExceededError
            If the bundle exceeds a configured resource bound.
        PortfolioError
            If native portfolio construction fails outside contract validation.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.portfolio import Portfolio
        >>> bundle = {
        ...     "schema": "finstack_quant.portfolio_materialization/1",
        ...     "portfolio": {"id": "empty", "base_currency": "USD", "as_of": "2025-01-01", "entities": {}},
        ...     "instruments": [],
        ...     "positions": [],
        ... }
        >>> built, report = Portfolio.from_materialization(json.dumps(bundle))
        >>> (built.id, report.positions)
        ('empty', 0)
        """
        ...

    @property
    def id(self) -> str:
        """
        Portfolio identifier.
        Returns
        -------
        str
            The id exposed by this `Portfolio`.
        """
        ...

    @property
    def as_of(self) -> str:
        """
        Portfolio as-of date as an ISO 8601 string.
        Returns
        -------
        str
            The as of exposed by this `Portfolio`.
        """
        ...

    @property
    def base_currency(self) -> str:
        """
        Base currency code used for valuation and aggregation.
        Returns
        -------
        str
            The base ccy exposed by this `Portfolio`.
        """
        ...

    def __len__(self) -> int:
        """Number of positions in the built portfolio.
        Returns
        -------
        int
        """
        ...

    def to_spec_json(self) -> str:
        """
        Serialize the portfolio back to its canonical ``PortfolioSpec`` JSON.
        Returns
        -------
        str
            Compact canonical JSON reconstructed as a ``PortfolioSpec``.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioAttribution:
    """
    Rust-computed portfolio P&L attribution in portfolio base currency.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, attribute_portfolio_pnl
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> result = attribute_portfolio_pnl(
    ...     Portfolio.from_spec(spec), MarketContext(), MarketContext(), "2025-01-01", "2025-01-02", "parallel"
    ... )
    >>> result.total_pnl.amount == 0
    True
    """

    def to_json(self) -> str:
        """
        Serialize the complete canonical attribution payload.

        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioAttribution`, suitable for a matching `from_json` call.
        """
        ...

    def by_position_json(self) -> str:
        """
        Serialize nested attribution in Rust ``IndexMap`` position order.

        Returns
        -------
        str
            Compact position-keyed attribution JSON in canonical insertion order.
        """
        ...

    def reconciliation_check(self, tolerance: float) -> dict[str, float | bool]:
        """
        Reconcile aggregate factor P&L to total P&L.

        Parameters
        ----------
        tolerance : float
            Absolute base-currency difference tolerated before reconciliation
            is reported as failing.

        Returns
        -------
        dict[str, float | bool]
            Base-currency ``total_residual``, ``is_reconciled`` flag, and ``tolerance``.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the portfolio-level factor totals as a single-row DataFrame.

        Every ``Money`` aggregate is flattened to a float column plus one
        shared ``currency`` column; the per-position nested breakdown stays on
        :meth:`by_position_json`.

        Columns: ``currency``, ``total_pnl``, ``carry``, ``rates_curves_pnl``,
        ``credit_curves_pnl``, ``inflation_curves_pnl``, ``correlations_pnl``,
        ``fx_pnl``, ``fx_translation_pnl``, ``cross_factor_pnl``, ``vol_pnl``,
        ``model_params_pnl``, ``market_scalars_pnl``, ``residual``,
        ``result_invalid``.

        Returns
        -------
        pd.DataFrame
            Exactly one row; concatenate rows across attribution runs to build
            a P&L time series.
        """
        ...

    @property
    def total_pnl(self) -> Money:
        """
        Total portfolio P&L between the two market snapshots.

        Returns
        -------
        Money
            Base-currency total; the factor components below decompose it.
        """
        ...

    @property
    def carry(self) -> Money:
        """
        Carry (time decay plus coupon/dividend income) component of total P&L.

        Returns
        -------
        Money
            Base-currency carry. Positive when the passage of time earns income.
        """
        ...

    @property
    def rates_curves_pnl(self) -> Money:
        """
        P&L attributed to interest-rate (discount and forward) curve moves.

        Returns
        -------
        Money
            Base-currency rates component. Excludes credit and inflation curves, which are reported separately.
        """
        ...

    @property
    def credit_curves_pnl(self) -> Money:
        """
        P&L attributed to credit-spread / hazard-rate curve moves.

        Returns
        -------
        Money
            Base-currency credit component, from spread and hazard-rate moves only.
        """
        ...

    @property
    def inflation_curves_pnl(self) -> Money:
        """
        P&L attributed to inflation curve moves.

        Returns
        -------
        Money
            Base-currency inflation component, from index-curve moves only.
        """
        ...

    @property
    def correlations_pnl(self) -> Money:
        """
        P&L attributed to correlation-input moves.

        Returns
        -------
        Money
            Base-currency correlation component, from correlation inputs only.
        """
        ...

    @property
    def fx_pnl(self) -> Money:
        """
        P&L attributed to FX spot/forward moves on FX-sensitive instruments.

        Returns
        -------
        Money
            Distinct from :attr:`fx_translation_pnl`, which covers restating
            unchanged foreign-currency values into the base currency.
        """
        ...

    @property
    def fx_translation_pnl(self) -> Money:
        """
        P&L from translating non-base-currency values at the new FX rates.

        Returns
        -------
        Money
            Base-currency translation effect only; economic FX exposure is reported in ``fx_pnl``.
        """
        ...

    @property
    def cross_factor_pnl(self) -> Money:
        """
        Second-order P&L from joint moves of two or more factors.

        Returns
        -------
        Money
            Base-currency interaction term. Non-zero only when factors move jointly, and not attributable to any single factor.
        """
        ...

    @property
    def vol_pnl(self) -> Money:
        """
        P&L attributed to implied-volatility surface moves.

        Returns
        -------
        Money
            Base-currency volatility component, from implied-vol surface moves only.
        """
        ...

    @property
    def model_params_pnl(self) -> Money:
        """
        P&L attributed to changes in calibrated model parameters.

        Returns
        -------
        Money
            Base-currency model component, from recalibration rather than market moves.
        """
        ...

    @property
    def market_scalars_pnl(self) -> Money:
        """
        P&L attributed to scalar market inputs (spots, dividends, recoveries).

        Returns
        -------
        Money
            Base-currency scalar component, covering inputs that are not curves or surfaces.
        """
        ...

    @property
    def residual(self) -> Money:
        """
        Unexplained P&L left after every factor has been attributed.

        Returns
        -------
        Money
            ``total_pnl`` less the sum of all factor components. Check it
            against a tolerance with :meth:`reconciliation_check`.
        """
        ...

    @property
    def result_invalid(self) -> bool:
        """
        Whether Rust flagged the attribution as unusable.

        Returns
        -------
        bool
            ``True`` when at least one position produced non-finite
            sensitivities or failed residual computation. Aggregating an
            invalid result across instruments is not meaningful.
        """
        ...

    def __repr__(self) -> str: ...

class PortfolioValuation:
    """
    Typed wrapper around a ``PortfolioValuation`` result.

    Wrap the JSON returned by :func:`value_portfolio` once and pass the typed
    object to :func:`aggregate_metrics` to skip re-parsing.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import PortfolioValuation
    >>> doc = {
    ...     "as_of": "2025-01-01",
    ...     "position_values": {},
    ...     "total_base_currency": {"amount": "0", "currency": "USD"},
    ...     "by_entity": {},
    ... }
    >>> PortfolioValuation.from_json(json.dumps(doc)).total_value
    0.0
    """

    @staticmethod
    def from_json(valuation_json: str) -> PortfolioValuation:
        """
        Deserialize a ``PortfolioValuation`` from JSON.

        Parameters
        ----------
        valuation_json : str
            Canonical valuation payload returned by ``value_portfolio`` or an
            equivalent serialized portfolio valuation.

        Returns
        -------
        PortfolioValuation
            Validated `PortfolioValuation` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.portfolio import PortfolioValuation
        >>> doc = {
        ...     "as_of": "2025-01-01",
        ...     "position_values": {},
        ...     "total_base_currency": {"amount": "0", "currency": "USD"},
        ...     "by_entity": {},
        ... }
        >>> valuation = PortfolioValuation.from_json(json.dumps(doc))
        >>> (valuation.total_value, valuation.base_currency)
        (0.0, 'USD')
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this valuation to canonical JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioValuation`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def total_value(self) -> float:
        """
        Total portfolio value in ``base_currency``.
        Returns
        -------
        float
            The total value exposed by this `PortfolioValuation`.
        """
        ...

    @property
    def base_currency(self) -> str:
        """
        Base currency code for this valuation.
        Returns
        -------
        str
            The base ccy exposed by this `PortfolioValuation`.
        """
        ...

    @property
    def as_of(self) -> str:
        """
        Valuation date as an ISO 8601 string.
        Returns
        -------
        str
            The as of exposed by this `PortfolioValuation`.
        """
        ...

    def to_arrow_positions(self) -> ArrowTable:
        """
        Export per-position values via Arrow (zero-copy for consumers).

        Columns: ``position_id``, ``entity_id``, ``value_native``,
        ``value_base``, ``currency_native``, ``currency_base`` (see
        ``finstack_quant_portfolio::positions_to_table``).

        Returns
        -------
        ArrowTable
            Arrow table with one row per valued position.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export per-position values as a pandas DataFrame.

        One row per entry of ``position_values``. Built from the same
        ``positions_to_table`` envelope that backs :meth:`to_arrow_positions`,
        so the two exits cannot drift apart.

        Columns: ``position_id``, ``entity_id``, ``value_native``,
        ``value_base``, ``currency_native``, ``currency_base``.

        Returns
        -------
        pd.DataFrame
            Values are floats and currency codes are strings. Prefer
            :meth:`to_arrow_positions` when a zero-copy handoff matters more
            than pandas ergonomics.
        """
        ...

    def __len__(self) -> int:
        """Number of valued positions.
        Returns
        -------
        int
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioCashflows:
    """
    Typed wrapper around a ``PortfolioCashflows`` ladder.

    Returned by :func:`aggregate_full_cashflows`; survives multiple drill-in
    calls (``events_json``, ``by_date_json``, ``issues_json``,
    :meth:`collapse_to_base_by_date_kind`) without re-parsing.

    Examples
    --------
    >>> from finstack_quant.portfolio import PortfolioCashflows
    >>> cashflows = PortfolioCashflows.from_json(
    ...     '{"events":[],"by_position":{},"by_date":{},"position_summaries":{},"issues":[]}'
    ... )
    >>> (cashflows.num_positions(), cashflows.num_issues())
    (0, 0)
    """

    @staticmethod
    def from_json(cashflows_json: str) -> PortfolioCashflows:
        """
        Deserialize a cashflow ladder from JSON.

        Parameters
        ----------
        cashflows_json : str
            Canonical classified-cashflow payload returned by the aggregation
            API or an equivalent serialized ladder.

        Returns
        -------
        PortfolioCashflows
            Validated `PortfolioCashflows` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PortfolioCashflows
        >>> cashflows = PortfolioCashflows.from_json(
        ...     '{"events":[],"by_position":{},"by_date":{},"position_summaries":{},"issues":[]}'
        ... )
        >>> (cashflows.num_positions(), cashflows.num_issues())
        (0, 0)
        """
        ...

    def to_json(self) -> str:
        """
        Serialize the full cashflow ladder to canonical JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioCashflows`, suitable for a matching `from_json` call.
        """
        ...

    def events_json(self) -> str:
        """
        Return all dated cashflow events as JSON.
        Returns
        -------
        str
            Compact JSON array of all dated cashflow events in ladder order.
        """
        ...

    def by_date_json(self) -> str:
        """
        Return cashflows grouped by date as JSON.
        Returns
        -------
        str
            Compact JSON for totals grouped by payment date, currency, and cashflow kind.
        """
        ...

    def issues_json(self) -> str:
        """
        Return cashflow aggregation or FX-conversion issues as JSON.
        Returns
        -------
        str
            Compact JSON array of cashflow-schedule extraction issues.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the flat ``events`` ladder as a pandas DataFrame.

        One row per dated cashflow event, in the ladder's canonical
        payment-date order. ``amount`` is flattened to a float column plus a
        ``currency`` column rather than a nested dict, so a multi-currency
        ladder stays currency-safe: group by ``currency`` before summing.

        Columns: ``position_id``, ``instrument_id``, ``instrument_type``,
        ``date`` (ISO 8601 string), ``amount`` (position-scaled), ``currency``,
        ``kind`` (``"fixed"``, ``"float_reset"``, ``"notional"``, ...),
        ``reset_date`` (ISO 8601 string, ``None`` outside floating coupons),
        ``accrual_factor``, ``rate`` (``None`` when the event carries no rate).

        Returns
        -------
        pd.DataFrame
            One row per event; ``len(df)`` equals ``len(cashflows)``.
        """
        ...

    def num_positions(self) -> int:
        """
        Number of positions represented in the ladder.
        Returns
        -------
        int
            Number of positions with schedule entries, including empty schedules.
        """
        ...

    def num_issues(self) -> int:
        """
        Number of diagnostic issues recorded on the ladder.
        Returns
        -------
        int
            Number of cashflow-schedule extraction issues recorded in the ladder.
        """
        ...

    def collapse_to_base_by_date_kind(
        self,
        market: MarketContext | str,
        base_currency: str,
        as_of: str,
    ) -> str:
        """
        Collapse the ladder to a base-currency ``(date, kind) → Money`` JSON.

        Uses **spot-equivalent** FX at each payment date. ``as_of`` is the
        valuation/run date used to flag far-future conversions.

        Parameters
        ----------
        market : MarketContext or str
            Market context object or JSON providing FX data for payment-date
            base-currency conversion.
        base_currency : str
            ISO currency code into which each classified cashflow is converted.
        as_of : str
            ISO-8601 valuation date used for conversion diagnostics and limits.

        Returns
        -------
        str
            Date-and-kind totals converted to ``base_currency`` using payment-date FX.

        Raises
        ------
        TypeError
            If ``market`` is neither a ``MarketContext`` nor a JSON string.
        ValueError
            If the market JSON, currency, or date is invalid, or monetary
            aggregation cannot represent the result.
        FinstackFxError
            If an FX rate required for base-currency conversion is unavailable.
        """
        ...

    def __len__(self) -> int:
        """Number of cashflow events.
        Returns
        -------
        int
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioResult:
    """
    Typed wrapper around a ``PortfolioResult`` envelope.

    Use the scalar accessors (``total_value``, ``get_metric``) to read single
    values without re-parsing the JSON envelope.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import PortfolioResult
    >>> valuation = {
    ...     "as_of": "2025-01-01",
    ...     "position_values": {},
    ...     "total_base_currency": {"amount": "0", "currency": "USD"},
    ...     "by_entity": {},
    ... }
    >>> rounding = {
    ...     "mode": "bankers",
    ...     "ingest_scale_by_currency": {},
    ...     "output_scale_by_currency": {},
    ...     "tolerances": {"rate_epsilon": 1e-12, "generic_epsilon": 1e-10},
    ...     "version": 1,
    ... }
    >>> doc = {
    ...     "schema_version": 1,
    ...     "valuation": valuation,
    ...     "metrics": {"aggregated": {}, "by_position": {}},
    ...     "meta": {"numeric_mode": "f64", "rounding": rounding},
    ... }
    >>> PortfolioResult.from_json(json.dumps(doc)).total_value
    0.0
    """

    @staticmethod
    def from_json(result_json: str) -> PortfolioResult:
        """
        Deserialize a portfolio result envelope from JSON.

        Parameters
        ----------
        result_json : str
            Canonical portfolio-result JSON containing total value and metrics.

        Returns
        -------
        PortfolioResult
            Validated `PortfolioResult` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.portfolio import PortfolioResult
        >>> valuation = {
        ...     "as_of": "2025-01-01",
        ...     "position_values": {},
        ...     "total_base_currency": {"amount": "0", "currency": "USD"},
        ...     "by_entity": {},
        ... }
        >>> rounding = {
        ...     "mode": "bankers",
        ...     "ingest_scale_by_currency": {},
        ...     "output_scale_by_currency": {},
        ...     "tolerances": {"rate_epsilon": 1e-12, "generic_epsilon": 1e-10},
        ...     "version": 1,
        ... }
        >>> doc = {
        ...     "schema_version": 1,
        ...     "valuation": valuation,
        ...     "metrics": {"aggregated": {}, "by_position": {}},
        ...     "meta": {"numeric_mode": "f64", "rounding": rounding},
        ... }
        >>> PortfolioResult.from_json(json.dumps(doc)).total_value
        0.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this result envelope to canonical JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioResult`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def total_value(self) -> float:
        """
        Total value stored in the result envelope.
        Returns
        -------
        float
            The total value exposed by this `PortfolioResult`.
        """
        ...

    def get_metric(self, metric_id: str) -> float | None:
        """
        Return a metric value, or ``None`` when it is absent.

        Parameters
        ----------
        metric_id : str
            Fully qualified metric key, such as ``"pv01::usd_ois"``.

        Returns
        -------
        float | None
            Aggregated scalar stored under ``metric_id``, or ``None`` when the
            result contains no matching metric.
        """
        ...

    def require_metric(self, metric_id: str) -> float:
        """
        Return a metric value, raising ``KeyError`` when it is absent.

        Parameters
        ----------
        metric_id : str
            Fully qualified metric key that must be present in the result.

        Returns
        -------
        float
            Aggregated scalar stored under the fully qualified ``metric_id``.

        Raises
        ------
        KeyError
            If ``metric_id`` is absent from the result.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioMetrics:
    """
    Typed wrapper around Rust-aggregated portfolio metrics.

    Examples
    --------
    >>> from finstack_quant.portfolio import PortfolioMetrics
    >>> metrics = PortfolioMetrics.from_json('{"aggregated":{},"by_position":{}}')
    >>> metrics.metric_series("pv")
    []
    """

    @staticmethod
    def from_json(metrics_json: str) -> PortfolioMetrics:
        """
        Deserialize canonical ``PortfolioMetrics`` JSON.

        Parameters
        ----------
        metrics_json : str
            Canonical metric payload returned by ``aggregate_metrics``.

        Returns
        -------
        PortfolioMetrics
            Validated `PortfolioMetrics` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PortfolioMetrics
        >>> metrics = PortfolioMetrics.from_json('{"aggregated":{},"by_position":{}}')
        >>> metrics.metric_series("pv")
        []
        """
        ...

    def to_json(self) -> str:
        """
        Serialize canonical ``PortfolioMetrics`` JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioMetrics`, suitable for a matching `from_json` call.
        """
        ...

    def metric_series(
        self,
        base: str,
    ) -> list[tuple[list[str], float, dict[str, float]]]:
        """
        Return decoded components, total, and entity values in wire order.

        Entity mappings preserve Rust ``IndexMap`` insertion order. Malformed
        legacy escapes remain literal; decoded coordinate collisions use
        literal wire components so no aggregate entry is lost.

        Parameters
        ----------
        base : str
            Metric namespace prefix used to select matching metric series.

        Returns
        -------
        list[tuple[list[str], float, dict[str, float]]]
            Ordered ``(components, total, by_entity)`` entries below ``base``;
            the scalar base entry is excluded.
        """
        ...

    def to_aggregated_dataframe(self) -> pd.DataFrame:
        """
        Export the portfolio-wide aggregated metrics as a pandas DataFrame.

        One row per entry of the ``aggregated`` map, in canonical Rust
        ``IndexMap`` insertion order. The per-entity breakdown is not
        flattened here — reach it through :meth:`metric_series`.

        Columns: ``metric_id``, ``total`` (sum across positions; only summable
        metrics are aggregated).

        Returns
        -------
        pd.DataFrame
            One row per aggregated metric.
        """
        ...

    def to_position_dataframe(self) -> pd.DataFrame:
        """
        Export the raw per-position metric values in long format.

        One row per ``(position, metric)`` pair — the row count is the total
        number of metric values across positions, not the number of positions.
        Pivot with ``df.pivot(index="position_id", columns="metric_id",
        values="value")`` for a wide view.

        Columns: ``position_id``, ``currency`` (the position's native currency;
        non-summable metrics are quoted in it), ``metric_id``, ``value``.

        Returns
        -------
        pd.DataFrame
            Long-format per-position metric values.
        """
        ...

    def __repr__(self) -> str: ...

def parse_portfolio_spec(json_str: str) -> str:
    """
    Parse and canonicalize a ``PortfolioSpec`` from JSON.

    Parameters
    ----------
    json_str : str
        Portfolio specification JSON to validate and normalize into canonical
        Rust serialization.

    Returns
    -------
    str
        Canonical compact JSON with defaults normalized and fields emitted in Rust wire order.

    Raises
    ------
    ValueError
        If ``json_str`` is malformed or does not match the ``PortfolioSpec`` schema.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import parse_portfolio_spec
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> json.loads(parse_portfolio_spec(spec))["id"]
    'empty'
    """
    ...

def build_portfolio_from_spec(spec_json: str) -> str:
    """
    Build a runtime portfolio from JSON and return the round-tripped spec.

    Prefer :meth:`Portfolio.from_spec` for real work — it returns the typed
    object that pipeline functions reuse without rebuilding.

    Parameters
    ----------
    spec_json : str
        Portfolio specification JSON to validate, compile, and serialize back.

    Returns
    -------
    str
        Canonical compact JSON for the validated, constructed portfolio specification.

    Raises
    ------
    ValueError
        If ``spec_json`` is malformed or does not match the ``PortfolioSpec`` schema.
    PortfolioError
        If the decoded positions or portfolio identifiers violate portfolio
        construction invariants.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import build_portfolio_from_spec
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> json.loads(build_portfolio_from_spec(spec))["positions"]
    []
    """
    ...

def portfolio_result_total_value(result: PortfolioResult | str) -> float:
    """
    Read total portfolio value from a ``PortfolioResult`` envelope.

    Accepts a typed :class:`PortfolioResult` (O(1)) or a JSON string
    (O(size-of-envelope)).

    Parameters
    ----------
    result : PortfolioResult or str
        Typed result envelope or canonical result JSON containing total value.

    Returns
    -------
    float
        Portfolio total in the result's base currency, converted to ``float``.

    Raises
    ------
    TypeError
        If ``result`` is neither a ``PortfolioResult`` nor a JSON string.
    ValueError
        If a supplied JSON string is malformed or does not match the result schema.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import portfolio_result_total_value
    >>> valuation = {
    ...     "as_of": "2025-01-01",
    ...     "position_values": {},
    ...     "total_base_currency": {"amount": "0", "currency": "USD"},
    ...     "by_entity": {},
    ... }
    >>> rounding = {
    ...     "mode": "bankers",
    ...     "ingest_scale_by_currency": {},
    ...     "output_scale_by_currency": {},
    ...     "tolerances": {"rate_epsilon": 1e-12, "generic_epsilon": 1e-10},
    ...     "version": 1,
    ... }
    >>> result = {
    ...     "schema_version": 1,
    ...     "valuation": valuation,
    ...     "metrics": {"aggregated": {}, "by_position": {}},
    ...     "meta": {"numeric_mode": "f64", "rounding": rounding},
    ... }
    >>> portfolio_result_total_value(json.dumps(result))
    0.0
    """
    ...

def portfolio_result_get_metric(result: PortfolioResult | str, metric_id: str) -> float | None:
    """
    Read one metric from a ``PortfolioResult``.

    Accepts a typed :class:`PortfolioResult` or a JSON string.

    Parameters
    ----------
    result : PortfolioResult or str
        Typed result envelope or canonical JSON containing portfolio metrics.
    metric_id : str
        Fully qualified metric key, such as ``"cs01::BOND_A"``.

    Returns
    -------
    float | None
        Aggregated metric total, or ``None`` when ``metric_id`` is absent.

    Raises
    ------
    TypeError
        If ``result`` is neither a ``PortfolioResult`` nor a JSON string.
    ValueError
        If a supplied JSON string is malformed or does not match the result schema.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import portfolio_result_get_metric
    >>> valuation = {
    ...     "as_of": "2025-01-01",
    ...     "position_values": {},
    ...     "total_base_currency": {"amount": "0", "currency": "USD"},
    ...     "by_entity": {},
    ... }
    >>> rounding = {
    ...     "mode": "bankers",
    ...     "ingest_scale_by_currency": {},
    ...     "output_scale_by_currency": {},
    ...     "tolerances": {"rate_epsilon": 1e-12, "generic_epsilon": 1e-10},
    ...     "version": 1,
    ... }
    >>> metrics = {"aggregated": {"dv01": {"metric_id": "dv01", "total": 12.5, "by_entity": {}}}, "by_position": {}}
    >>> result = {
    ...     "schema_version": 1,
    ...     "valuation": valuation,
    ...     "metrics": metrics,
    ...     "meta": {"numeric_mode": "f64", "rounding": rounding},
    ... }
    >>> portfolio_result_get_metric(json.dumps(result), "dv01")
    12.5
    """
    ...

def aggregate_metrics(
    valuation: PortfolioValuation | str,
    base_currency: str,
    market: MarketContext | str,
    as_of: str,
) -> str:
    """
    Aggregate portfolio metrics from a valuation.

    Accepts a typed :class:`PortfolioValuation` (fast path) or a JSON string.

    Parameters
    ----------
    valuation : PortfolioValuation or str
        Typed valuation or canonical valuation JSON to aggregate.
    base_currency : str
        ISO base-currency code in which aggregate values and metrics are stated.
    market : MarketContext or str
        Market context object or JSON supplying conversion and market inputs.
    as_of : str
        ISO-8601 valuation date used to resolve date-dependent market data.

    Returns
    -------
    str
        Canonical JSON for totals, per-position metrics, and skipped non-finite values.

    Raises
    ------
    TypeError
        If ``valuation`` or ``market`` is neither its typed wrapper nor a JSON string.
    ValueError
        If supplied JSON, ``base_currency``, or ``as_of`` is invalid.
    PortfolioError
        If the valuation base currency or date is inconsistent with the
        requested aggregation context.
    FinstackFxError
        If an FX rate required for base-currency aggregation is unavailable.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, aggregate_metrics, value_portfolio
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> valuation = value_portfolio(Portfolio.from_spec(spec), MarketContext())
    >>> json.loads(aggregate_metrics(valuation, "USD", MarketContext(), "2025-01-01"))["aggregated"]
    {}
    """
    ...

def value_portfolio(
    portfolio: Portfolio | str,
    market: MarketContext | str,
    strict_risk: bool = False,
    metrics: list[str] | None = None,
) -> PortfolioValuation:
    """
    Value a portfolio and return its typed result.

    Typed ``Portfolio`` and ``MarketContext`` inputs avoid rebuilding runtime
    state, while the result can be passed directly to :func:`aggregate_metrics`.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON to value.
    market : MarketContext or str
        Market context object or JSON supplying curves, quotes, and FX data.
    strict_risk : bool, default False
        Whether absent or failed risk calculations abort the valuation rather
        than being recorded as diagnostics.
    metrics : list[str] or None, default None
        Exact metric identifiers to compute. ``None`` requests the standard
        portfolio risk set; an empty list performs PV-only valuation.

    Returns
    -------
    PortfolioValuation
        Typed valuation result backed directly by the Rust calculation.

    Raises
    ------
    PortfolioError
        If portfolio construction, market lookup, FX conversion, pricing, or
        strict risk evaluation fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, value_portfolio
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> value_portfolio(Portfolio.from_spec(spec), MarketContext()).total_value
    0.0
    """
    ...

def aggregate_full_cashflows(portfolio: Portfolio | str, market: MarketContext | str) -> PortfolioCashflows:
    """
    Build the full classified cashflow ladder for the portfolio.

    Returns a typed :class:`PortfolioCashflows` wrapper; call ``to_json()``
    to get the raw ladder or use the typed accessors to drill in.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON to expand.
    market : MarketContext or str
        Market context object or JSON needed for instrument cashflow generation.

    Returns
    -------
    PortfolioCashflows
        Typed event ladder with per-position, per-date, and issue drill-downs.

    Raises
    ------
    TypeError
        If ``portfolio`` or ``market`` is neither its typed wrapper nor a JSON string.
    ValueError
        If supplied JSON is malformed or cashflow aggregation cannot represent
        a monetary result.
    PortfolioError
        If a JSON portfolio cannot be constructed from its decoded specification.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, aggregate_full_cashflows
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> cashflows = aggregate_full_cashflows(Portfolio.from_spec(spec), MarketContext())
    >>> (cashflows.num_positions(), cashflows.num_issues())
    (0, 0)
    """
    ...

def apply_scenario_and_revalue(
    portfolio: Portfolio | str,
    scenario_json: str,
    market: MarketContext | str,
) -> tuple[str, str]:
    """
    Apply a scenario and revalue the portfolio.

    Returns ``(valuation_json, report_json)``.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON to revalue.
    scenario_json : str
        Canonical scenario specification JSON describing market-data shocks.
    market : MarketContext or str
        Base market context object or JSON to shock before revaluation.

    Returns
    -------
    tuple[str, str]
        ``(valuation_json, diagnostics_json)`` for the shocked valuation and targets.

    Raises
    ------
    TypeError
        If ``portfolio`` or ``market`` is neither its typed wrapper nor a JSON string.
    ValueError
        If portfolio, scenario, or market JSON is malformed or schema-incompatible.
    PortfolioError
        If portfolio construction, scenario application, market lookup, FX
        conversion, or valuation fails.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, apply_scenario_and_revalue
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> scenario = '{"id":"s","name":"S","operations":[]}'
    >>> valuation_json, _ = apply_scenario_and_revalue(Portfolio.from_spec(spec), scenario, MarketContext())
    >>> json.loads(valuation_json)["position_values"]
    {}
    """
    ...

def scenario_pnl(
    portfolio: Portfolio | str,
    scenario_json: str,
    market: MarketContext | str,
) -> tuple[str, str]:
    """
    Compute the profit and loss attributable to a scenario.

    Values the portfolio against the unshocked market and against the
    scenario-shocked market, then reports the base-currency difference per
    position and in total. Positions added or removed by the scenario are
    zero-filled against the missing side, so ``by_position`` always sums to
    ``total``.

    Returns ``(pnl_json, report_json)``.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON valued on both the
        unshocked and shocked legs.
    scenario_json : str
        Canonical scenario specification JSON describing the market-data shocks
        whose profit-and-loss impact is measured.
    market : MarketContext or str
        Unshocked market context object or JSON used as the base leg and as the
        source the scenario operations are applied to.

    Returns
    -------
    tuple[str, str]
        ``(pnl_json, report_json)`` — the ``ScenarioPnl`` ladder (base-currency
        ``total`` and ``by_position`` amounts) and the scenario application
        report carrying which operations were applied.

    Raises
    ------
    TypeError
        If ``portfolio`` or ``market`` is neither its typed wrapper nor a JSON string.
    ValueError
        If a JSON input is malformed or does not match its serialized schema.
    PortfolioError
        If scenario application or either valuation fails, including for
        missing market data or mismatched base and shocked currencies.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, scenario_pnl
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> pnl_json, _ = scenario_pnl(Portfolio.from_spec(spec), '{"id":"s","name":"S","operations":[]}', MarketContext())
    >>> json.loads(pnl_json)["by_position"]
    {}
    """
    ...

def scenario_pnl_batch(
    portfolio: Portfolio | str,
    scenarios_json: str,
    market: MarketContext | str,
) -> str:
    """
    Compute ordered scenario P&L results while reusing one base valuation.

    The Rust portfolio engine values the unshocked portfolio once, applies
    each scenario independently, and returns results in the same order as the
    input JSON array. It is the batch counterpart to :func:`scenario_pnl`;
    use it when more than one scenario is evaluated against one portfolio and
    market snapshot.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON. The typed form
        avoids rebuilding the portfolio for the batch.
    scenarios_json : str
        Canonical JSON array of ``ScenarioSpec`` objects. Array order is
        preserved exactly. ``"[]"`` returns ``"[]"`` without valuation.
    market : MarketContext or str
        Unshocked market context or canonical market JSON used for the shared
        base valuation and each scenario application.

    Returns
    -------
    str
        Canonical JSON array. Each item has ``scenario_id``, ``pnl`` (the same
        base-currency ``ScenarioPnl`` shape returned by :func:`scenario_pnl`),
        and ``report`` (the corresponding scenario application report).

    Raises
    ------
    ValueError
        If ``scenarios_json`` is malformed or cannot deserialize to an ordered
        array of valid ``ScenarioSpec`` values.
    PortfolioError
        If scenario application, valuation, or base-currency P&L differencing
        fails. The reported error is for the earliest failing input scenario.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, scenario_pnl_batch
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> json.loads(scenario_pnl_batch(Portfolio.from_spec(spec), "[]", MarketContext()))
    []
    """
    ...

def attribute_portfolio_pnl(
    portfolio: Portfolio | str,
    market_t0: MarketContext | str,
    market_t1: MarketContext | str,
    as_of_t0: str,
    as_of_t1: str,
    method: str | dict[str, Any],
    config: dict[str, Any] | str | None = None,
) -> PortfolioAttribution:
    """
    Attribute portfolio P&L with Rust-owned aggregation and FX translation.

    Parameters
    ----------
    portfolio : Portfolio or str
        Built portfolio or canonical ``PortfolioSpec`` JSON being attributed.
    market_t0 : MarketContext or str
        Opening market context object or JSON at the start of the P&L interval.
    market_t1 : MarketContext or str
        Closing market context object or JSON at the end of the P&L interval.
    as_of_t0 : str
        ISO-8601 opening valuation date associated with ``market_t0``.
    as_of_t1 : str
        ISO-8601 closing valuation date associated with ``market_t1``.
    method : str or dict[str, Any]
        Attribution-method name or method configuration understood by Rust.
    config : dict[str, Any] or str or None
        Optional attribution configuration mapping or canonical JSON payload.

    Returns
    -------
    PortfolioAttribution
        Base-currency P&L decomposition by position, market move, and residual.

    Raises
    ------
    TypeError
        If a portfolio, market, method, or configuration input has an unsupported type.
    ValueError
        If supplied JSON, either ISO date, or the attribution method or
        configuration is invalid.
    PortfolioError
        If portfolio construction, valuation, FX conversion, or attribution fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, attribute_portfolio_pnl
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> result = attribute_portfolio_pnl(
    ...     Portfolio.from_spec(spec), MarketContext(), MarketContext(), "2025-01-01", "2025-01-02", "parallel"
    ... )
    >>> result.total_pnl.amount == 0
    True
    """
    ...

def allocate_weights(spec_json: str) -> str:
    """
    Allocate strategy weights from a JSON specification.

    The specification contains a ``scheme`` (for example
    ``"inverse_volatility"``), ``total_capital``, and a list of strategy
    objects with ``id`` and return/history fields required by the selected
    scheme. The Rust allocator computes normalized weights and money amounts;
    Python only passes the JSON through.

    Parameters
    ----------
    spec_json : str
        JSON-serialized allocation specification.

    Returns
    -------
    str
        JSON allocation result with the selected scheme and per-strategy
        ``id``, ``weight``, and allocated capital fields.

    Raises
    ------
    ValueError
        If the JSON is malformed, required fields are missing, the
        scheme is unsupported, or the selected scheme cannot be evaluated.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import allocate_weights
    >>> spec = '{"scheme":"equal","total_capital":1000.0,"strategies":[{"id":"a"},{"id":"b"}]}'
    >>> [entry["weight"] for entry in json.loads(allocate_weights(spec))["allocations"]]
    [0.5, 0.5]
    """
    ...

def validate_allocation_json(spec_json: str) -> None:
    """
    Validate a strategy allocation JSON specification.

    Performs the same Rust-side parse and semantic validation used by
    :func:`allocate_weights` without computing allocations.

    Parameters
    ----------
    spec_json : str
        JSON-serialized allocation specification.

    Raises
    ------
    ValueError
        If the specification is malformed or invalid.

    Examples
    --------
    >>> from finstack_quant.portfolio import validate_allocation_json
    >>> spec = '{"scheme":"equal","total_capital":1000.0,"strategies":[{"id":"a"},{"id":"b"}]}'
    >>> validate_allocation_json(spec) is None
    True
    """
    ...

def replay_portfolio(
    portfolio: Portfolio | str,
    snapshots_json: str,
    config_json: str,
) -> str:
    """
    Replay a portfolio through dated market snapshots.

    Parameters
    ----------
    portfolio : Portfolio or str
        Typed :class:`Portfolio` or JSON ``PortfolioSpec``.
    snapshots_json : str
        JSON array or envelope of market snapshots.
    config_json : str
        JSON replay configuration controlling dates, valuation
        options, and output detail.

    Returns
    -------
    str
        JSON replay result containing dated valuations and diagnostics.

    Raises
    ------
    PortfolioError
        If the portfolio, snapshots, or replay config are
        invalid, or if a snapshot valuation fails.

    Examples
    --------
    >>> from finstack_quant.portfolio import replay_portfolio
    >>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> try:
    ...     replay_portfolio(spec, "[]", '{"mode":"pv_only"}')
    ... except ValueError as exc:
    ...     print("must be non-empty" in str(exc))
    True
    """
    ...

def parametric_var_decomposition(
    position_ids: list[str],
    weights: list[float],
    covariance: list[list[float]] | npt.NDArray[np.float64],
    confidence: float = 0.95,
    compute_incremental: bool = False,
) -> PositionRiskDecomposition:
    """
    Decompose portfolio parametric VaR across positions.

    Parameters
    ----------
    position_ids : list[str]
        Position identifiers aligned with ``weights``.
    weights : list[float]
        Portfolio weights or exposures.
    covariance : list[list[float]] or numpy.ndarray
        Square covariance matrix aligned with ``position_ids``. C-contiguous
        ``float64`` arrays use the direct buffer path.
    confidence : float, default 0.95
        VaR confidence level in ``(0, 1)``.
    compute_incremental : bool, default False
        Whether to calculate leave-one-out incremental VaR for each position.

    Returns
    -------
    PositionRiskDecomposition
        Typed portfolio VaR/ES totals and per-position contributions.

    Raises
    ------
    ValueError
        If dimensions do not match, covariance is malformed, or the
        confidence level is invalid.

    Examples
    --------
    >>> from finstack_quant.portfolio import parametric_var_decomposition
    >>> result = parametric_var_decomposition(["A", "B"], [1.0, 2.0], [[0.04, 0.0], [0.0, 0.01]])
    >>> round(result.portfolio_var, 6)
    -0.465235
    """
    ...

def parametric_es_decomposition(
    position_ids: list[str],
    weights: list[float],
    covariance: list[list[float]] | npt.NDArray[np.float64],
    confidence: float = 0.95,
) -> PositionRiskDecomposition:
    """
    Decompose portfolio parametric expected shortfall across positions.

    Parameters
    ----------
    position_ids : list[str]
        Position identifiers aligned with ``weights``.
    weights : list[float]
        Portfolio weights or exposures.
    covariance : list[list[float]] or numpy.ndarray
        Square covariance matrix aligned with ``position_ids``. C-contiguous
        ``float64`` arrays use the direct buffer path.
    confidence : float, default 0.95
        ES confidence level in ``(0, 1)``.

    Returns
    -------
    PositionRiskDecomposition
        Typed portfolio VaR/ES totals and per-position contributions.

    Raises
    ------
    ValueError
        If dimensions do not match, covariance is malformed, or the
        confidence level is invalid.

    Examples
    --------
    >>> from finstack_quant.portfolio import parametric_es_decomposition
    >>> result = parametric_es_decomposition(["A", "B"], [1.0, 2.0], [[0.04, 0.0], [0.0, 0.01]])
    >>> round(result.portfolio_es, 6)
    -0.583423
    """
    ...

def historical_var_decomposition(
    position_ids: list[str],
    position_pnls: list[list[float]] | npt.NDArray[np.float64],
    confidence: float = 0.95,
) -> PositionRiskDecomposition:
    """
    Decompose historical VaR from scenario or realized position P&Ls.

    Parameters
    ----------
    position_ids : list[str]
        Position identifiers.
    position_pnls : list[list[float]] or numpy.ndarray
        Position-major matrix of P&Ls shaped
        ``len(position_ids) x n_scenarios``. C-contiguous ``float64`` arrays
        use the direct buffer path.
    confidence : float, default 0.95
        Historical VaR confidence level in ``(0, 1)``.

    Returns
    -------
    PositionRiskDecomposition
        Typed historical VaR/ES totals and per-position contributions.

    Raises
    ------
    ValueError
        If the P&L matrix is empty, ragged, dimensionally
        inconsistent, or the confidence level is invalid.

    Examples
    --------
    >>> from finstack_quant.portfolio import historical_var_decomposition
    >>> pnl = [list(range(-10, 10)), [2 * value for value in range(-10, 10)]]
    >>> historical_var_decomposition(["A", "B"], pnl).portfolio_var
    -30.0
    """
    ...

def evaluate_risk_budget(
    position_ids: list[str],
    actual_var: list[float],
    target_var_pct: list[float],
    portfolio_var: float,
    utilization_threshold: float = 1.20,
) -> RiskBudgetResult:
    """
    Compare actual position VaR against target risk-budget shares.

    Parameters
    ----------
    position_ids : list[str]
        Position identifiers aligned with ``actual_var`` and
        ``target_var_pct``.
    actual_var : list[float]
        Position component VaR amounts.
    target_var_pct : list[float]
        Target share of total portfolio VaR per position.
    portfolio_var : float
        Total portfolio VaR used to convert target percentages
        into target VaR amounts.
    utilization_threshold : float, default 1.20
        Breach threshold for actual / target utilization.

    Returns
    -------
    RiskBudgetResult
        Typed per-position utilization, excess VaR, and breach diagnostics.

    Raises
    ------
    ValueError
        If input lengths differ or risk-budget inputs are invalid.

    Examples
    --------
    >>> from finstack_quant.portfolio import evaluate_risk_budget
    >>> result = evaluate_risk_budget(["A", "B"], [1.0, 2.0], [0.5, 0.5], 3.0)
    >>> (result.has_breach, result.total_overbudget)
    (True, 0.5)
    """
    ...

def roll_effective_spread(returns: list[float]) -> float | None:
    """
    Estimate the Roll effective bid-ask spread from returns.

    Returns ``None`` when there are too few observations or the first-order
    autocovariance does not imply a positive spread.

    Parameters
    ----------
    returns : list[float]
        Ordered simple decimal returns sampled at a consistent observation
        frequency.

    Returns
    -------
    float | None
        Effective spread as a decimal, or ``None`` when the estimate is unavailable.

    Examples
    --------
    >>> from finstack_quant.portfolio import roll_effective_spread
    >>> roll_effective_spread([0.01, -0.01, 0.01, -0.01])
    0.02
    """
    ...

def amihud_illiquidity(returns: list[float], volumes: list[float]) -> float | None:
    """
    Compute Amihud illiquidity from absolute returns and traded volumes.

    Parameters
    ----------
    returns : list[float]
        Period returns.
    volumes : list[float]
        Traded volumes aligned with ``returns``.

    Returns
    -------
    float or None
        Average ``abs(return) / volume`` over positive-volume observations, or
        ``None`` when no valid observations are available.

    Examples
    --------
    >>> from finstack_quant.portfolio import amihud_illiquidity
    >>> amihud_illiquidity([0.01, 0.02], [100.0, 200.0])
    0.0001
    """
    ...

def days_to_liquidate(
    position_value: float,
    avg_daily_volume: float,
    participation_rate: float,
) -> float:
    """
    Estimate liquidation horizon in trading days.

    Parameters
    ----------
    position_value : float
        Position market value to liquidate.
    avg_daily_volume : float
        Average daily market volume in the same notional units.
    participation_rate : float
        Maximum fraction of daily volume the liquidation may consume.

    Returns
    -------
    float
        ``position_value / (avg_daily_volume * participation_rate)``.

    Raises
    ------
    ValueError
        If volume or participation inputs are non-positive.

    Examples
    --------
    >>> from finstack_quant.portfolio import days_to_liquidate
    >>> days_to_liquidate(1_000_000, 250_000, 0.20)
    20.0
    """
    ...

def liquidity_tier(days_to_liquidate: float) -> str:
    """
    Classify liquidation horizon into the Rust liquidity-tier labels.

    Parameters
    ----------
    days_to_liquidate : float
        Estimated trading-day horizon required to fully liquidate the position.

    Returns
    -------
    str
        Bucket label derived from the supplied trading and market inputs.

    Examples
    --------
    >>> from finstack_quant.portfolio import liquidity_tier
    >>> liquidity_tier(3.0)
    'tier2'
    """
    ...

def lvar_bangia(
    var: float,
    spread_mean: float,
    spread_vol: float,
    confidence: float,
    position_value: float,
) -> dict[str, float]:
    """
    Compute Bangia-style liquidity-adjusted VaR.

    Parameters
    ----------
    var : float
        Base market VaR.
    spread_mean : float
        Mean bid-ask spread.
    spread_vol : float
        Spread volatility.
    confidence : float
        Confidence level for the liquidity adjustment.
    position_value : float
        Position value used to scale spread cost.

    Returns
    -------
    dict[str, float]
        Dict containing the base VaR, liquidity add-on, and adjusted LVaR.

    Raises
    ------
    ValueError
        If ``var`` is non-finite or non-positive; ``spread_mean`` or
        ``spread_vol`` is non-finite or negative; ``confidence`` is outside
        ``(0, 1)``; or ``position_value`` is non-finite.

    Examples
    --------
    >>> from finstack_quant.portfolio import lvar_bangia
    >>> result = lvar_bangia(-100.0, 0.01, 0.005, 0.99, 1_000_000)
    >>> round(result["lvar"], 2)
    -10915.87
    """
    ...

def almgren_chriss_impact(
    position_size: float,
    avg_daily_volume: float,
    volatility: float,
    execution_horizon_days: float,
    permanent_impact_coef: float,
    temporary_impact_coef: float,
    reference_price: float | None = None,
) -> dict[str, float]:
    """
    Estimate Almgren-Chriss execution impact components.

    Parameters
    ----------
    position_size : float
        Trade size in shares or notional units.
    avg_daily_volume : float
        Average daily volume in matching units.
    volatility : float
        Asset volatility used for risk scaling.
    execution_horizon_days : float
        Execution horizon in trading days.
    permanent_impact_coef : float
        Permanent impact coefficient.
    temporary_impact_coef : float
        Temporary impact coefficient.
    reference_price : float, optional
        Optional price used to convert share impact to notional impact.

    Returns
    -------
    dict[str, float]
        Dict of permanent, temporary, and total impact estimates.

    Raises
    ------
    ValueError
        If ``position_size`` is non-finite; ``avg_daily_volume``, ``volatility``,
        or ``execution_horizon_days`` is non-finite or non-positive;
        ``permanent_impact_coef`` is non-finite or negative;
        ``temporary_impact_coef`` is non-finite or non-positive; or a supplied
        ``reference_price`` is non-finite or non-positive.

    Examples
    --------
    >>> from finstack_quant.portfolio import almgren_chriss_impact
    >>> result = almgren_chriss_impact(100_000, 1_000_000, 0.20, 5.0, 0.10, 0.20)
    >>> round(result["expected_cost_bp"], 2)
    50282842.71
    """
    ...

def kyle_lambda(volumes: list[float], returns: list[float], reference_price: float) -> float | None:
    """
    Estimate Kyle's lambda from volume and return observations.

    Multiplies the mean absolute decimal return per unit volume by a reference
    price so the result is a price-space impact coefficient.

    Parameters
    ----------
    volumes : list[float]
        Ordered trading-volume observations in consistent notional or share units.
    returns : list[float]
        Ordered simple decimal returns aligned one-for-one with ``volumes``.
    reference_price : float
        Positive price per share or contract used to convert the return-space
        ratio into price-space lambda.

    Returns
    -------
    float | None
        Estimated price-space Kyle lambda, or ``None`` for invalid samples or
        a non-positive or non-finite reference price.

    Examples
    --------
    >>> from finstack_quant.portfolio import kyle_lambda
    >>> kyle_lambda([100.0, 200.0], [0.01, -0.02], 50.0)
    0.005
    """
    ...

def brinson_fachler(sectors_json: str) -> str:
    """
    Compute single-period Brinson-Fachler attribution from sector JSON.

    Parameters
    ----------
    sectors_json : str
        JSON array of ``SectorPeriod`` objects with ``sector``,
        ``portfolio_weight``, ``benchmark_weight``, ``portfolio_return``, and
        ``benchmark_return`` fields. Returns are simple decimal returns for the
        period (e.g. ``0.02`` for +2%).

    Returns
    -------
    str
        JSON-serialized ``BrinsonPeriodResult`` with allocation, selection, and
        interaction effects plus total active return.

    Raises
    ------
    PortfolioError
        If sector weights do not sum to one or returns are invalid.
    ValueError
        If ``sectors_json`` is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#brinson-fachler-1985``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import brinson_fachler
    >>> sectors = [
    ...     {
    ...         "sector": "A",
    ...         "portfolio_weight": 1.0,
    ...         "benchmark_weight": 1.0,
    ...         "portfolio_return": 0.02,
    ...         "benchmark_return": 0.01,
    ...     }
    ... ]
    >>> json.loads(brinson_fachler(json.dumps(sectors)))["total_excess_return"]
    0.01
    """
    ...

def carino_link(periods_json: str) -> str:
    """
    Compute Carino-linked multi-period Brinson attribution from period JSON.

    Parameters
    ----------
    periods_json : str
        JSON array of periods, where each period is an array of ``SectorPeriod``
        objects (same schema as :func:`brinson_fachler`).

    Returns
    -------
    str
        JSON-serialized ``CarinoLinkedAttribution`` with linked allocation,
        selection, and interaction effects across periods.

    Raises
    ------
    PortfolioError
        If any period fails Brinson validation.
    ValueError
        If ``periods_json`` is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#carino-1999``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import carino_link
    >>> period = [
    ...     {
    ...         "sector": "A",
    ...         "portfolio_weight": 1.0,
    ...         "benchmark_weight": 1.0,
    ...         "portfolio_return": 0.02,
    ...         "benchmark_return": 0.01,
    ...     }
    ... ]
    >>> result = json.loads(carino_link(json.dumps([period, period])))
    >>> round(result["linked_selection"], 4)
    0.0203
    """
    ...

def campisi_attribution(portfolio_json: str, benchmark_json: str, config_json: str) -> str:
    """
    Compute single-period Campisi fixed-income benchmark attribution.

    Binds Rust ``finstack_quant_portfolio::campisi_attribution``.

    Parameters
    ----------
    portfolio_json : str
        JSON array of ``FiPositionSnapshot`` objects with ``sector``,
        ``weight``, ``total_return``, ``yield_annual``, ``modified_duration``,
        ``spread_duration``, ``spread``, ``delta_treasury_yield``, and
        ``delta_spread`` fields (decimals; durations in years). Unknown fields
        are rejected and weights must sum to one. ``spread_duration`` must be
        the canonical quote-reproducing Z-spread duration; ``spread`` and
        ``delta_spread`` must use the matching ``z_spread`` basis. OAS,
        G-spread, and discount-margin values are incompatible. The direct JSON
        shape has no metric-ID provenance, so this binding cannot detect a
        mislabeled numeric spread basis.
    benchmark_json : str
        JSON array of ``FiPositionSnapshot`` objects for the benchmark, subject
        to the same quote-reproducing Z-spread basis contract.
    config_json : str
        JSON object whose only field is ``period_years`` (e.g. ``0.25``). It is
        required — there is no default — and unknown keys are rejected. The
        spread convention is not configurable: the Campisi spread effect
        ``-SD * delta_spread`` is identical under the absolute and
        Duration-Times-Spread conventions, so a mode switch could only relabel
        an identical result.

    Returns
    -------
    str
        JSON-serialized ``FiAttributionResult``. ``result["sectors"]`` is a
        flat records array: ``pd.DataFrame(json.loads(result)["sectors"])``
        yields the sector x component table directly.

    Raises
    ------
    PortfolioError
        If weights do not sum to one, inputs are non-finite, ``period_years``
        is not finite and positive, either side is empty, or a sector present
        on either side has ``|net sector weight| <= 1e-6 * gross absolute
        sector weight``. The spread *level* is unconstrained: zero and negative
        Z-spreads are accepted, because the spread effect never divides by the
        level. Spread-basis provenance cannot be validated from the numeric JSON
        shape.
    ValueError
        If any JSON argument is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#campisi-2000``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import campisi_attribution
    >>> portfolio = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.02,
    ...         "yield_annual": 0.04,
    ...         "modified_duration": 5.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> benchmark = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.01,
    ...         "yield_annual": 0.03,
    ...         "modified_duration": 4.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> result = json.loads(campisi_attribution(json.dumps(portfolio), json.dumps(benchmark), '{"period_years":0.25}'))
    >>> result["active_return"]
    0.01
    """
    ...

def campisi_carino_link(periods_json: str) -> str:
    """
    Carino-link already-computed single-period Campisi attribution results.

    Binds Rust ``finstack_quant_portfolio::campisi_carino_link``.

    Each period result already carries its own applied ``period_years``, so
    periods of *different* lengths link correctly here (e.g. act/365 calendar
    months: 31/365, 28/365, 31/365). Use this entry point for real day counts;
    :func:`campisi_carino_link_from_snapshots` applies one shared
    ``period_years`` to every period and is only correct for equal-length
    periods.

    Parameters
    ----------
    periods_json : str
        JSON array of ``FiAttributionResult`` objects in chronological order,
        each the parsed output of :func:`campisi_attribution`. Every period
        must carry the same sector ordering. Unknown fields are rejected.

    Returns
    -------
    str
        JSON-serialized ``FiCarinoLinkedResult`` whose five linked totals sum
        to the geometrically compounded active return.

    Raises
    ------
    PortfolioError
        If ``periods_json`` is empty; sector orderings differ; a consumed
        top-level return/effect, per-sector linked effect, or sector
        ``total_active`` is non-finite; ``active_return`` disagrees with the
        portfolio-minus-benchmark return; a sector ``total_active`` disagrees
        with its five effects; sector effects do not reconcile to their
        declared top-level totals; the five totals do not reconcile to
        ``active_return`` within the overflow-safe scaled-L1 tolerance; a
        reconciliation residual is non-finite; or a return is at or below
        -100% (outside the Carino domain).
    ValueError
        If ``periods_json`` is malformed or does not match the
        ``FiAttributionResult`` schema.

    Sources
    -------
    See ``docs/REFERENCES.md#carino-1999``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import campisi_attribution, campisi_carino_link
    >>> portfolio = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.02,
    ...         "yield_annual": 0.04,
    ...         "modified_duration": 5.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> benchmark = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.01,
    ...         "yield_annual": 0.03,
    ...         "modified_duration": 4.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> period = json.loads(campisi_attribution(json.dumps(portfolio), json.dumps(benchmark), '{"period_years":0.25}'))
    >>> linked = json.loads(campisi_carino_link(json.dumps([period, period])))
    >>> round(linked["linked_selection"], 6)
    0.013195
    """
    ...

def campisi_carino_link_from_snapshots(periods_json: str, config_json: str) -> str:
    """
    Compute Carino-linked multi-period Campisi attribution from period JSON.

    Binds Rust ``finstack_quant_portfolio::campisi_carino_link_from_snapshots``.

    One ``config_json`` — and therefore one ``period_years`` — is applied to
    every period, so this entry point is only correct when all periods have the
    same length. For mixed-length periods (real day counts), compute each
    period with :func:`campisi_attribution` and link the results with
    :func:`campisi_carino_link`.

    Parameters
    ----------
    periods_json : str
        JSON array of period objects, each with ``portfolio`` and
        ``benchmark`` arrays of ``FiPositionSnapshot`` (same schema as
        :func:`campisi_attribution`).
    config_json : str
        JSON ``FiAttributionConfig`` shared across periods; ``period_years`` is
        its only field and is required (no default).

    Returns
    -------
    str
        JSON-serialized ``FiCarinoLinkedResult`` whose five linked totals sum
        to the geometrically compounded active return.

    Raises
    ------
    PortfolioError
        If ``periods_json`` is empty, any period fails Campisi validation, or
        sector orderings differ across periods.
    ValueError
        If any JSON argument is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#carino-1999``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import campisi_carino_link_from_snapshots
    >>> portfolio = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.02,
    ...         "yield_annual": 0.04,
    ...         "modified_duration": 5.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> benchmark = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.01,
    ...         "yield_annual": 0.03,
    ...         "modified_duration": 4.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> periods = json.dumps([{"portfolio": portfolio, "benchmark": benchmark}])
    >>> result = json.loads(campisi_carino_link_from_snapshots(periods, '{"period_years":0.25}'))
    >>> round(result["linked_selection"], 4)
    0.0065
    """
    ...

def campisi_reconciliation_check(result_json: str, tolerance: float) -> str:
    """
    Reconcile the five Campisi effect totals against the active return.

    Binds the Rust method
    ``finstack_quant_portfolio::FiAttributionResult::reconciliation_check``
    (a method in Rust, exposed here as a JSON function on the namespace).

    The Campisi decomposition reconciles by construction because selection is
    the residual, so this is a floating-point sanity gate rather than a model
    check. Without it callers must re-sum ``total_allocation``,
    ``total_active_carry``, ``total_active_treasury``, ``total_active_spread``
    and ``total_selection`` by hand.

    Parameters
    ----------
    result_json : str
        JSON ``FiAttributionResult``, as returned by
        :func:`campisi_attribution`. Unknown fields are rejected.
    tolerance : float
        Absolute tolerance in return units; ``1e-10`` is appropriate for
        return-space values.

    Returns
    -------
    str
        JSON-serialized ``FiReconciliationReport`` with ``total_residual``
        (``active_return`` minus the five totals), ``is_reconciled`` and the
        ``tolerance`` that was applied.

    Raises
    ------
    ValueError
        If ``result_json`` is malformed or carries unknown fields.

    Sources
    -------
    See ``docs/REFERENCES.md#campisi-2000``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import campisi_attribution, campisi_reconciliation_check
    >>> portfolio = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.02,
    ...         "yield_annual": 0.04,
    ...         "modified_duration": 5.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> benchmark = [
    ...     {
    ...         "sector": "GOVT",
    ...         "weight": 1.0,
    ...         "total_return": 0.01,
    ...         "yield_annual": 0.03,
    ...         "modified_duration": 4.0,
    ...         "spread_duration": 0.0,
    ...         "spread": 0.0,
    ...         "delta_treasury_yield": -0.001,
    ...         "delta_spread": 0.0,
    ...     }
    ... ]
    >>> result = campisi_attribution(json.dumps(portfolio), json.dumps(benchmark), '{"period_years":0.25}')
    >>> json.loads(campisi_reconciliation_check(result, 1e-10))["is_reconciled"]
    True
    """
    ...

def cell_returns_from_reference(reference_json: str, base_label: str, config_json: str) -> str:
    """
    Build a duration-cell base-return table from a reference universe.

    Binds Rust ``finstack_quant_portfolio::cell_returns_from_reference``
    (Dynkin, Hyman & Vankudre 1998, Appendix B): buckets ``reference_json``
    into fixed-width duration cells and averages each cell's member total
    returns, interpolating interior gaps and flat-extrapolating
    leading/trailing gaps.

    Parameters
    ----------
    reference_json : str
        JSON array of ``ReferenceReturn`` objects (``duration``,
        ``total_return``, both decimals with duration in years); must be
        non-empty. Unknown fields are rejected.
    base_label : str
        Label identifying the resulting curve (e.g. ``"UST"``), carried
        through to the output's ``base_label`` for policy visibility.
    config_json : str
        JSON ``CellConfig``; ``width`` is its only field (cell width in
        years, finite and positive) and is required — there is no default.

    Returns
    -------
    str
        JSON-serialized ``DurationCellTable``.

    Raises
    ------
    PortfolioError
        If ``reference_json`` is empty, ``config.width`` is not finite and
        positive, any reference entry has a non-finite ``total_return`` or a
        non-finite/negative ``duration``, the width produces two numerically
        distinct cells that collide on their one-decimal label, or the
        largest reference ``duration`` divided by ``config.width`` would
        require more than an internal sanity bound of cells (100,000; a
        units mistake, e.g. days instead of years, rather than a legitimate
        duration grid).
    ValueError
        If any JSON argument is malformed or carries unknown fields.

    Sources
    -------
    See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import cell_returns_from_reference
    >>> reference = [{"duration": 1.0, "total_return": 0.02}]
    >>> table = json.loads(cell_returns_from_reference(json.dumps(reference), "UST", '{"width":2.0}'))
    >>> table["cells"][0]["base_return"]
    0.02
    """
    ...

def cell_returns_from_curves(
    start: DiscountCurve,
    end: DiscountCurve,
    horizon_years: float,
    max_duration: float,
    base_label: str,
    config_json: str,
) -> str:
    """
    Build a duration-cell base-return table from start/end discount curves.

    Binds Rust ``finstack_quant_portfolio::cell_returns_from_curves``: each
    cell's base return is the holding-period return of a hypothetical
    zero-coupon position bought at the cell midpoint off ``start`` and
    revalued off ``end`` after ``horizon_years`` have elapsed. Every
    resulting cell is observed (a curve has a discount factor at every
    queried point), unlike the reference-universe path in
    :func:`cell_returns_from_reference`.

    Parameters
    ----------
    start : DiscountCurve
        Discount curve observed at the start of the holding period.
    end : DiscountCurve
        Discount curve observed ``horizon_years`` later, at period end.
    horizon_years : float
        Length of the holding period, in years; must be finite and positive.
    max_duration : float
        Upper bound of the duration grid, in years; must be finite and
        strictly greater than ``horizon_years``.
    base_label : str
        Label identifying the base curve (e.g. ``"UST"``, ``"USD-SOFR"``),
        stamped into the result purely for policy visibility.
    config_json : str
        JSON ``CellConfig``; ``width`` is its only field and is required.

    Returns
    -------
    str
        JSON-serialized ``DurationCellTable``.

    Raises
    ------
    PortfolioError
        If ``config.width`` or ``horizon_years`` is not finite and positive,
        ``max_duration`` does not strictly exceed ``horizon_years``,
        ``max_duration`` divided by ``config.width`` would require more than
        an internal sanity bound of cells (100,000; a units mistake rather
        than a legitimate duration grid), a cell's midpoint does not exceed
        ``horizon_years`` (unavoidable whenever ``config.width`` is not
        strictly greater than ``2 * horizon_years``, since the first cell's
        midpoint is ``config.width / 2``), either curve produces a
        non-positive/non-finite discount factor at a queried time, or the
        width produces colliding one-decimal cell labels.
    ValueError
        If ``config_json`` is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.

    Examples
    --------
    >>> from datetime import date
    >>> import json
    >>> from finstack_quant.core.market_data import DiscountCurve
    >>> from finstack_quant.portfolio import cell_returns_from_curves
    >>> start = DiscountCurve.flat("start", date(2025, 1, 1), 0.02)
    >>> end = DiscountCurve.flat("end", date(2025, 4, 1), 0.03)
    >>> table = json.loads(cell_returns_from_curves(start, end, 0.25, 2.0, "UST", '{"width":1.0}'))
    >>> len(table["cells"])
    2
    """
    ...

def excess_returns(positions_json: str, table_json: str) -> str:
    """
    Compute duration-matched credit excess returns against a base-return table.

    Binds Rust ``finstack_quant_portfolio::excess_returns`` (Dynkin, Hyman &
    Vankudre 1998, Appendix B): each position's ``duration`` is matched to
    its duration cell in ``table_json`` and the position's excess return is
    ``total_return - cell.base_return``, the credit-specific component of
    performance isolated from the general level/shape move of the base curve.

    Parameters
    ----------
    positions_json : str
        JSON array of ``ExcessReturnPosition`` objects (``id``, ``weight``,
        ``duration``, ``total_return``); weights must sum to ``1.0`` within
        ``1e-6``.
    table_json : str
        JSON ``DurationCellTable``, as returned by
        :func:`cell_returns_from_reference` or :func:`cell_returns_from_curves`.

    Returns
    -------
    str
        JSON-serialized ``ExcessReturnResult`` with per-position and
        portfolio-level total/base/excess returns.

    Raises
    ------
    PortfolioError
        If ``table_json`` has no cells; a cell has an empty or duplicate
        label, non-finite bounds or base return, a negative lower bound, a
        non-positive width, non-ascending lower bounds, or overlaps its
        predecessor; a position has a non-finite
        weight/duration/total_return; a position's duration falls outside
        every cell, including a valid gap (the error names the position); or
        the position weights do not sum to ``1.0`` within tolerance.
    ValueError
        If any JSON argument is malformed.

    Sources
    -------
    See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import cell_returns_from_reference, excess_returns
    >>> reference = [{"duration": 1.0, "total_return": 0.02}]
    >>> table = cell_returns_from_reference(json.dumps(reference), "UST", '{"width":2.0}')
    >>> positions = [{"id": "B1", "weight": 1.0, "duration": 1.0, "total_return": 0.03}]
    >>> result = json.loads(excess_returns(json.dumps(positions), table))
    >>> round(result["portfolio_excess_return"], 4)
    0.01
    """
    ...

def grid_attribution(portfolio_json: str, benchmark_json: str) -> str:
    """
    Compute a single-period hierarchical duration-cell x sector grid attribution.

    Binds Rust ``finstack_quant_portfolio::grid_attribution`` (Dynkin, Hyman
    & Vankudre 1998, Appendix A): decomposes active return into a per-cell
    curve (positioning) effect, a within-cell sector allocation effect, and
    a security-selection residual per (cell, sector).

    Parameters
    ----------
    portfolio_json : str
        JSON array of ``GridPosition`` objects (``cell``, ``sector``,
        ``weight``, ``total_return``) for the portfolio side; weights must
        sum to ``1.0`` within ``1e-6``.
    benchmark_json : str
        JSON array of ``GridPosition`` objects for the benchmark side; same
        weight-sum requirement.

    Returns
    -------
    str
        JSON-serialized ``GridAttributionResult`` whose ``total_curve``,
        ``total_sector`` and ``total_selection`` sum to ``active_return`` to
        floating-point precision for well-conditioned inputs; among
        *accepted* inputs (those that clear the near-zero-net-weight check
        below), the reconciliation residual grows the closer any bucket's
        net weight sits to that check's own rejection boundary — see the
        Rust module docs' measured residuals for magnitudes.

    Raises
    ------
    PortfolioError
        If any weight or return is non-finite, either side's weights don't
        sum to ``1.0`` within tolerance, or a (cell) or (cell, sector)
        bucket has positions on a side but nets to a weight that is zero, or
        numerically near zero (within ``1e-6`` relative to its own gross
        weight), which is undefined-or-explosive to attribute (the error
        names the bucket and the side).
    ValueError
        If either JSON argument is malformed or carries unknown fields.

    Sources
    -------
    See ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import grid_attribution
    >>> portfolio = [{"cell": "0-3", "sector": "GOVT", "weight": 1.0, "total_return": 0.02}]
    >>> benchmark = [{"cell": "0-3", "sector": "GOVT", "weight": 1.0, "total_return": 0.01}]
    >>> result = json.loads(grid_attribution(json.dumps(portfolio), json.dumps(benchmark)))
    >>> result["total_selection"]
    0.01
    """
    ...

def grid_carino_link(periods_json: str) -> str:
    """
    Carino-link multi-period hierarchical grid attribution results.

    Binds Rust ``finstack_quant_portfolio::grid_carino_link`` (Carino 1999):
    applies Carino smoothing to a chronological sequence of single-period
    :func:`grid_attribution` results so the three top-level effects
    (``linked_curve``, ``linked_sector``, ``linked_selection``) sum exactly
    to the geometrically compounded active return. Only the three top-level
    effects are linked; per-cell / per-(cell, sector) multi-period linking
    is out of scope.

    Parameters
    ----------
    periods_json : str
        JSON array of ``GridAttributionResult`` objects, in chronological
        order, each the parsed output of :func:`grid_attribution`.

    Returns
    -------
    str
        JSON-serialized ``GridCarinoLinkedResult``.

    Raises
    ------
    PortfolioError
        If ``periods_json`` is empty; any consumed return or top-level effect
        is non-finite; ``active_return`` disagrees with the portfolio-minus-
        benchmark return; the three effect totals do not reconcile to
        ``active_return`` within an overflow-safe scaled-L1 tolerance; a
        return-identity or reconciliation residual is non-finite; or any
        per-period or compounded return is at or below -100% (outside the
        Carino domain).
    ValueError
        If ``periods_json`` is malformed or does not match the
        ``GridAttributionResult`` schema.

    Sources
    -------
    See ``docs/REFERENCES.md#carino-1999`` and
    ``docs/REFERENCES.md#dynkin-hyman-vankudre-1998``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import grid_attribution, grid_carino_link
    >>> portfolio = [{"cell": "0-3", "sector": "GOVT", "weight": 1.0, "total_return": 0.02}]
    >>> benchmark = [{"cell": "0-3", "sector": "GOVT", "weight": 1.0, "total_return": 0.01}]
    >>> period = json.loads(grid_attribution(json.dumps(portfolio), json.dumps(benchmark)))
    >>> result = json.loads(grid_carino_link(json.dumps([period, period])))
    >>> round(result["linked_selection"], 4)
    0.0203
    """
    ...

def factor_brinson_attribution(input_json: str, factor_returns: list[float]) -> str:
    """
    Compute Jeet-Partani (2023) factor-Brinson unified attribution.

    Binds Rust ``finstack_quant_portfolio::factor_brinson_attribution``:
    generalizes classical Brinson-Fachler allocation/selection to continuous
    factor exposures by replacing the sector partition with a
    factor-exposure matrix and a caller-supplied benchmark factor-return
    vector.

    Parameters
    ----------
    input_json : str
        JSON ``FactorBrinsonInput`` with ``asset_ids``, ``asset_returns``,
        ``exposures`` (row-major ``n_assets x n_factors``), ``factor_names``,
        ``portfolio_weights`` and ``benchmark_weights``. Each weight vector
        must sum to ``1.0`` within ``1e-6``; weights may be negative (short
        positions).
    factor_returns : list[float]
        Caller-supplied benchmark factor returns ``f_b``, length
        ``input.factor_names``. Typically fit with
        :func:`finstack_quant.analytics.constrained_least_squares` (using
        benchmark weights) so the completeness condition below holds.

    Returns
    -------
    str
        JSON-serialized ``FactorBrinsonResult`` with ``allocation``,
        ``selection``, and their per-factor / per-asset breakdowns.

    Raises
    ------
    PortfolioError
        If any array has the wrong length relative to ``n_assets`` /
        ``n_factors``, either ``n_assets`` or ``n_factors`` is zero, any
        input value is non-finite, portfolio or benchmark weights don't sum
        to ``1.0`` within tolerance, or the Jeet-Partani completeness
        residual ``|h_b'eps_b|`` exceeds tolerance — meaning the supplied
        ``factor_returns`` do not fully explain the benchmark return. The
        error directs callers to
        :func:`finstack_quant.analytics.constrained_least_squares`.
    ValueError
        If ``input_json`` is malformed or carries unknown fields.

    Sources
    -------
    See ``docs/REFERENCES.md#jeet-partani-2023``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import factor_brinson_attribution
    >>> inputs = {
    ...     "asset_ids": ["A"],
    ...     "asset_returns": [0.02],
    ...     "exposures": [1.0],
    ...     "factor_names": ["Market"],
    ...     "portfolio_weights": [1.0],
    ...     "benchmark_weights": [1.0],
    ... }
    >>> result = json.loads(factor_brinson_attribution(json.dumps(inputs), [0.02]))
    >>> result["active_return"]
    0.0
    """
    ...

def twrr_modified_dietz(period_json: str) -> float | None:
    """
    Compute a Modified-Dietz TWRR sub-period return from period JSON.

    Parameters
    ----------
    period_json : str
        JSON-encoded ``TwrrPeriod`` with beginning market value, external
        cashflows, and ending market value for the sub-period.

    Returns
    -------
    float or None
        Sub-period time-weighted return as a decimal, or ``None`` when the
        period cannot be computed (e.g. zero denominator).

    Raises
    ------
    ValueError
        If ``period_json`` is malformed.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import twrr_modified_dietz
    >>> period = {"beginning_market_value": 100.0, "ending_market_value": 110.0, "cashflows": []}
    >>> twrr_modified_dietz(json.dumps(period))
    0.1
    """
    ...

def twrr_linked(returns_json: str, horizon_years: float) -> str | None:
    """
    Geometrically link TWRR sub-period returns over a horizon.

    Parameters
    ----------
    returns_json : str
        JSON array of sub-period decimal returns (e.g. from Modified Dietz).
    horizon_years : float
        Reporting horizon in years used to annualize the linked return.

    Returns
    -------
    str or None
        JSON-encoded linked return result, or ``None`` when linking fails (e.g.
        empty return series).

    Raises
    ------
    ValueError
        If ``returns_json`` is malformed.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.portfolio import twrr_linked
    >>> result = json.loads(twrr_linked("[0.05,0.03]", horizon_years=1.0))
    >>> round(result["cumulative"], 4)
    0.0815
    """
    ...

def mwr_xirr(cashflows_json: str) -> float:
    """
    Compute money-weighted return via XIRR from dated cashflow JSON.

    Parameters
    ----------
    cashflows_json : str
        JSON array of ``DatedCashflow`` objects with ISO dates and signed amounts
        (investments negative, distributions positive).

    Returns
    -------
    float
        Internal rate of return as a decimal annualized rate.

    Raises
    ------
    PortfolioError
        If XIRR does not converge or cashflows lack a sign change.
    ValueError
        If ``cashflows_json`` is malformed.

    Examples
    --------
    >>> from finstack_quant.portfolio import mwr_xirr
    >>> cashflows = '[{"date":"2025-01-01","amount":-100.0},{"date":"2026-01-01","amount":110.0}]'
    >>> round(mwr_xirr(cashflows), 6)
    0.1
    """
    ...

# factor_model typed result classes (Slice 8)

class FactorContribution:
    """
    Aggregate contribution of a single factor to portfolio risk.

    Examples
    --------
    >>> from finstack_quant.portfolio import FactorContribution
    >>> item = FactorContribution.from_json(
    ...     '{"factor_id":"Rates","absolute_risk":1.0,"relative_risk":0.5,"marginal_risk":0.2}'
    ... )
    >>> (item.factor_id, item.relative_risk)
    ('Rates', 0.5)
    """

    @classmethod
    def from_json(cls, json_str: str) -> FactorContribution:
        """
        Deserialize a factor contribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized factor contribution, normally produced by
            ``FactorContribution.to_json``.

        Returns
        -------
        FactorContribution
            Validated `FactorContribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import FactorContribution
        >>> item = FactorContribution.from_json(
        ...     '{"factor_id":"Rates","absolute_risk":1.0,"relative_risk":0.5,"marginal_risk":0.2}'
        ... )
        >>> item.absolute_risk
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this factor contribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `FactorContribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def factor_id(self) -> str:
        """
        Factor identifier.
        Returns
        -------
        str
            The factor id exposed by this `FactorContribution`.
        """
        ...

    @property
    def absolute_risk(self) -> float:
        """
        Absolute risk contribution.
        Returns
        -------
        float
            The absolute risk exposed by this `FactorContribution`.
        """
        ...

    @property
    def relative_risk(self) -> float:
        """
        Share of total portfolio risk.
        Returns
        -------
        float
            The relative risk exposed by this `FactorContribution`.
        """
        ...

    @property
    def marginal_risk(self) -> float:
        """
        Marginal risk contribution.
        Returns
        -------
        float
            The marginal risk exposed by this `FactorContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionFactorContribution:
    """
    Per-position contribution to a specific factor bucket.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionFactorContribution
    >>> item = PositionFactorContribution.from_json('{"position_id":"P1","factor_id":"Rates","risk_contribution":1.0}')
    >>> (item.position_id, item.factor_id)
    ('P1', 'Rates')
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionFactorContribution:
        """
        Deserialize a position-factor contribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized position-factor contribution, normally
            produced by ``PositionFactorContribution.to_json``.

        Returns
        -------
        PositionFactorContribution
            Validated `PositionFactorContribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFactorContribution
        >>> item = PositionFactorContribution.from_json(
        ...     '{"position_id":"P1","factor_id":"Rates","risk_contribution":1.0}'
        ... )
        >>> item.risk_contribution
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position-factor contribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionFactorContribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionFactorContribution`.
        """
        ...

    @property
    def factor_id(self) -> str:
        """
        Factor identifier.
        Returns
        -------
        str
            The factor id exposed by this `PositionFactorContribution`.
        """
        ...

    @property
    def risk_contribution(self) -> float:
        """
        Risk contribution for this position-factor pair.
        Returns
        -------
        float
            The risk contribution exposed by this `PositionFactorContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionResidualContribution:
    """
    Annualized residual variance contributed by a single position.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionResidualContribution
    >>> item = PositionResidualContribution.from_json(
    ...     '{"position_id":"P1","residual_variance":0.1,"source":{"kind":"other"}}'
    ... )
    >>> (item.position_id, item.source_kind)
    ('P1', 'other')
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionResidualContribution:
        """
        Deserialize a residual contribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized residual contribution, normally produced by
            ``PositionResidualContribution.to_json``.

        Returns
        -------
        PositionResidualContribution
            Validated `PositionResidualContribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionResidualContribution
        >>> item = PositionResidualContribution.from_json(
        ...     '{"position_id":"P1","residual_variance":0.1,"source":{"kind":"other"}}'
        ... )
        >>> item.residual_variance
        0.1
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this residual contribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionResidualContribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionResidualContribution`.
        """
        ...

    @property
    def residual_variance(self) -> float:
        """
        Residual variance assigned to this position.
        Returns
        -------
        float
            The residual variance exposed by this `PositionResidualContribution`.
        """
        ...

    @property
    def source_kind(self) -> str:
        """
        Source category used to derive residual risk.
        Returns
        -------
        str
            The source kind exposed by this `PositionResidualContribution`.
        """
        ...

    @property
    def source_issuer_id(self) -> str | None:
        """
        Issuer identifier for issuer-sourced residual risk, if present.
        Returns
        -------
        str or None
            The source issuer id exposed by this `PositionResidualContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class RiskDecomposition:
    """
    Portfolio-level risk decomposition across factors and residuals.

    Examples
    --------
    >>> from finstack_quant.portfolio import RiskDecomposition
    >>> doc = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
    >>> decomposition = RiskDecomposition.from_json(doc)
    >>> (decomposition.total_risk, decomposition.residual_risk)
    (1.0, 1.0)
    """

    @classmethod
    def from_json(cls, json_str: str) -> RiskDecomposition:
        """
        Deserialize a risk decomposition from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized factor-and-residual decomposition, normally
            produced by ``RiskDecomposition.to_json``.

        Returns
        -------
        RiskDecomposition
            Validated `RiskDecomposition` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import RiskDecomposition
        >>> doc = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
        >>> RiskDecomposition.from_json(doc).measure_json
        '"variance"'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this risk decomposition to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `RiskDecomposition`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def total_risk(self) -> float:
        """
        Total portfolio risk under the decomposition measure.
        Returns
        -------
        float
            The total risk exposed by this `RiskDecomposition`.
        """
        ...

    @property
    def measure_json(self) -> str:
        """
        Risk measure specification as JSON.
        Returns
        -------
        str
            The measure json exposed by this `RiskDecomposition`.
        """
        ...

    @property
    def residual_risk(self) -> float:
        """
        Residual risk not explained by factor contributions.
        Returns
        -------
        float
            The residual risk exposed by this `RiskDecomposition`.
        """
        ...

    @property
    def factor_contributions(self) -> list[FactorContribution]:
        """
        Factor-level risk contributions.
        Returns
        -------
        list[FactorContribution]
        """
        ...

    @property
    def position_factor_contributions(self) -> list[PositionFactorContribution]:
        """
        Position-by-factor risk contributions.
        Returns
        -------
        list[PositionFactorContribution]
        """
        ...

    @property
    def position_residual_contributions(self) -> list[PositionResidualContribution]:
        """
        Per-position residual risk contributions.
        Returns
        -------
        list[PositionResidualContribution]
            Empty unless the decomposer had per-issuer idiosyncratic vol
            estimates (credit-aware position decomposers).
        """
        ...

    def to_factor_dataframe(self) -> pd.DataFrame:
        """
        Export factor contributions as a pandas DataFrame.

        Columns: ``factor_id``, ``absolute_risk``, ``relative_risk``,
        ``marginal_risk`` — identical to
        :meth:`FactorRiskDecomposition.to_factor_dataframe`, which renders the
        same Rust type reached through the sensitivity engine.

        Returns
        -------
        pd.DataFrame
            One row per entry of :attr:`factor_contributions`.
        """
        ...

    def to_position_factor_dataframe(self) -> pd.DataFrame:
        """
        Export position x factor contributions as a pandas DataFrame.

        Columns: ``position_id``, ``factor_id``, ``risk_contribution`` —
        identical to
        :meth:`FactorRiskDecomposition.to_position_factor_dataframe`.

        Returns
        -------
        pd.DataFrame
            One row per entry of :attr:`position_factor_contributions`.
        """
        ...

    def to_position_residual_dataframe(self) -> pd.DataFrame:
        """
        Export per-position residual variance contributions as a DataFrame.

        Columns: ``position_id``, ``residual_variance`` (annualized variance,
        non-negative), ``source_kind`` (``"from_credit_model"`` or
        ``"other"``), ``source_issuer_id`` (``None`` unless ``source_kind`` is
        ``"from_credit_model"``).

        Returns
        -------
        pd.DataFrame
            One row per entry of :attr:`position_residual_contributions`; zero
            rows — with the columns still present — when the decomposer
            produced no position-level residual allocation.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionVarContribution:
    """
    Per-position component / marginal VaR.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionVarContribution
    >>> item = PositionVarContribution.from_json(
    ...     '{"position_id":"P1","component_var":-1.0,"relative_var":1.0,"marginal_var":-1.0,"incremental_var":null}'
    ... )
    >>> (item.position_id, item.component_var)
    ('P1', -1.0)
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionVarContribution:
        """
        Deserialize a position VaR contribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized component and marginal VaR contribution,
            normally produced by ``PositionVarContribution.to_json``.

        Returns
        -------
        PositionVarContribution
            Validated `PositionVarContribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionVarContribution
        >>> item = PositionVarContribution.from_json(
        ...     '{"position_id":"P1","component_var":-1.0,"relative_var":1.0,"marginal_var":-1.0,"incremental_var":null}'
        ... )
        >>> item.relative_var
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position VaR contribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionVarContribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionVarContribution`.
        """
        ...

    @property
    def component_var(self) -> float:
        """
        Component VaR assigned to this position.
        Returns
        -------
        float
            The component var exposed by this `PositionVarContribution`.
        """
        ...

    @property
    def relative_var(self) -> float:
        """
        Share of total portfolio VaR.
        Returns
        -------
        float
            The relative var exposed by this `PositionVarContribution`.
        """
        ...

    @property
    def marginal_var(self) -> float | None:
        """
        Marginal VaR, if computed.
        Returns
        -------
        float or None
            The marginal var exposed by this `PositionVarContribution`.
        """
        ...

    @property
    def incremental_var(self) -> float | None:
        """
        Incremental VaR, if requested in the decomposition config.
        Returns
        -------
        float or None
            The incremental var exposed by this `PositionVarContribution`.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export this contribution as a single-row pandas DataFrame.

        Columns: ``position_id``, ``component_var``, ``relative_var``,
        ``marginal_var``, ``incremental_var``.

        Returns
        -------
        pd.DataFrame
            Exactly one row, for symmetry with
            :meth:`PositionRiskDecomposition.to_dataframe`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionEsContribution:
    """
    Per-position component / marginal ES.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionEsContribution
    >>> item = PositionEsContribution.from_json(
    ...     '{"position_id":"P1","component_es":-1.2,"relative_es":1.0,"marginal_es":-1.2}'
    ... )
    >>> (item.position_id, item.component_es)
    ('P1', -1.2)
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionEsContribution:
        """
        Deserialize a position ES contribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized component and marginal expected-shortfall
            contribution, normally produced by ``PositionEsContribution.to_json``.

        Returns
        -------
        PositionEsContribution
            Validated `PositionEsContribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionEsContribution
        >>> item = PositionEsContribution.from_json(
        ...     '{"position_id":"P1","component_es":-1.2,"relative_es":1.0,"marginal_es":-1.2}'
        ... )
        >>> item.relative_es
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position ES contribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionEsContribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionEsContribution`.
        """
        ...

    @property
    def component_es(self) -> float:
        """
        Component expected shortfall assigned to this position.
        Returns
        -------
        float
            The component es exposed by this `PositionEsContribution`.
        """
        ...

    @property
    def relative_es(self) -> float:
        """
        Share of total portfolio expected shortfall.
        Returns
        -------
        float
            The relative es exposed by this `PositionEsContribution`.
        """
        ...

    @property
    def marginal_es(self) -> float | None:
        """
        Marginal expected shortfall, if computed.
        Returns
        -------
        float or None
            The marginal es exposed by this `PositionEsContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionRiskDecomposition:
    """
    Complete position-level risk decomposition.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionRiskDecomposition
    >>> doc = '{"portfolio_var":-1.0,"portfolio_es":-1.2,"confidence":0.95,"method":"parametric","var_contributions":[],"es_contributions":[],"n_positions":0,"euler_residual":0.0}'
    >>> result = PositionRiskDecomposition.from_json(doc)
    >>> (result.portfolio_var, result.method)
    (-1.0, 'parametric')
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionRiskDecomposition:
        """
        Deserialize a position risk decomposition from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized VaR/ES decomposition, normally produced by
            ``PositionRiskDecomposition.to_json``.

        Returns
        -------
        PositionRiskDecomposition
            Validated `PositionRiskDecomposition` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionRiskDecomposition
        >>> doc = '{"portfolio_var":-1.0,"portfolio_es":-1.2,"confidence":0.95,"method":"parametric","var_contributions":[],"es_contributions":[],"n_positions":0,"euler_residual":0.0}'
        >>> PositionRiskDecomposition.from_json(doc).confidence
        0.95
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position risk decomposition to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionRiskDecomposition`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def portfolio_var(self) -> float:
        """
        Total portfolio Value-at-Risk.
        Returns
        -------
        float
            Portfolio-currency amount under the workspace loss convention, so
            losses are reported as **negative** numbers.
        """
        ...

    @property
    def portfolio_es(self) -> float:
        """
        Portfolio expected shortfall.
        Returns
        -------
        float
            The portfolio es exposed by this `PositionRiskDecomposition`.
        """
        ...

    @property
    def confidence(self) -> float:
        """
        Confidence level used for VaR/ES.
        Returns
        -------
        float
            The confidence exposed by this `PositionRiskDecomposition`.
        """
        ...

    @property
    def n_positions(self) -> int:
        """
        Number of positions included in the decomposition.
        Returns
        -------
        int
            The n positions exposed by this `PositionRiskDecomposition`.
        """
        ...

    @property
    def method(self) -> str:
        """
        Decomposition method label.
        Returns
        -------
        str
            The method exposed by this `PositionRiskDecomposition`.
        """
        ...

    @property
    def euler_residual(self) -> float | None:
        """
        Euler allocation residual, if reported.
        Returns
        -------
        float or None
            The euler residual exposed by this `PositionRiskDecomposition`.
        """
        ...

    @property
    def var_contributions(self) -> list[PositionVarContribution]:
        """
        Per-position VaR contributions.
        Returns
        -------
        list[PositionVarContribution]
        """
        ...

    @property
    def es_contributions(self) -> list[PositionEsContribution]:
        """
        Per-position expected shortfall contributions.
        Returns
        -------
        list[PositionEsContribution]
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the joined per-position VaR and ES decomposition.

        ``var_contributions`` and ``es_contributions`` are both keyed by
        position, so they are joined on ``position_id`` into one frame. Rows
        follow ``var_contributions`` order; an ES column is ``None`` for a
        position that has no matching ES entry.

        The portfolio-level scalars (:attr:`portfolio_var`,
        :attr:`portfolio_es`, :attr:`confidence`, :attr:`method`) are header
        metadata and are deliberately **not** repeated on every row.

        Columns: ``position_id``, ``component_var``, ``relative_var``,
        ``marginal_var``, ``incremental_var``, ``component_es``,
        ``relative_es``, ``marginal_es``.

        Returns
        -------
        pd.DataFrame
            One row per entry of :attr:`var_contributions`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionBudgetEntry:
    """
    Per-position budget comparison entry.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionBudgetEntry
    >>> item = PositionBudgetEntry.from_json(
    ...     '{"position_id":"P1","actual_component_var":1.0,"target_component_var":0.8,"utilization":1.25,"excess":0.2}'
    ... )
    >>> (item.position_id, item.utilization)
    ('P1', 1.25)
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionBudgetEntry:
        """
        Deserialize a risk-budget entry from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized per-position budget comparison, normally
            produced by ``PositionBudgetEntry.to_json``.

        Returns
        -------
        PositionBudgetEntry
            Validated `PositionBudgetEntry` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionBudgetEntry
        >>> item = PositionBudgetEntry.from_json(
        ...     '{"position_id":"P1","actual_component_var":1.0,"target_component_var":0.8,"utilization":1.25,"excess":0.2}'
        ... )
        >>> item.excess
        0.2
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this risk-budget entry to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionBudgetEntry`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionBudgetEntry`.
        """
        ...

    @property
    def actual_component_var(self) -> float:
        """
        Actual component VaR for this position.
        Returns
        -------
        float
            The actual component var exposed by this `PositionBudgetEntry`.
        """
        ...

    @property
    def target_component_var(self) -> float:
        """
        Target component VaR for this position.
        Returns
        -------
        float
            The target component var exposed by this `PositionBudgetEntry`.
        """
        ...

    @property
    def utilization(self) -> float:
        """
        Actual-to-target utilization ratio.
        Returns
        -------
        float
            The utilization exposed by this `PositionBudgetEntry`.
        """
        ...

    @property
    def excess(self) -> float:
        """
        Actual component VaR less target component VaR.
        Returns
        -------
        float
            The excess exposed by this `PositionBudgetEntry`.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export this budget entry as a single-row pandas DataFrame.

        Columns: ``position_id``, ``actual_component_var``,
        ``target_component_var``, ``utilization``, ``excess``.

        Returns
        -------
        pd.DataFrame
            Exactly one row, with the same schema
            :meth:`RiskBudgetResult.to_dataframe` emits.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class RiskBudgetResult:
    """
    Budget evaluation result across positions.

    Examples
    --------
    >>> from finstack_quant.portfolio import RiskBudgetResult
    >>> result = RiskBudgetResult.from_json('{"positions":[],"total_overbudget":0.0,"has_breach":false}')
    >>> (result.total_overbudget, result.has_breach)
    (0.0, False)
    """

    @classmethod
    def from_json(cls, json_str: str) -> RiskBudgetResult:
        """
        Deserialize a risk-budget result from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized portfolio risk-budget result, normally
            produced by ``RiskBudgetResult.to_json``.

        Returns
        -------
        RiskBudgetResult
            Validated `RiskBudgetResult` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import RiskBudgetResult
        >>> RiskBudgetResult.from_json('{"positions":[],"total_overbudget":0.0,"has_breach":false}').positions
        []
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this risk-budget result to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `RiskBudgetResult`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def total_overbudget(self) -> float:
        """
        Total amount above target risk budgets.
        Returns
        -------
        float
            The total overbudget exposed by this `RiskBudgetResult`.
        """
        ...

    @property
    def has_breach(self) -> bool:
        """
        Whether any position exceeds the utilization threshold.
        Returns
        -------
        bool
            Whether this `RiskBudgetResult` has breach.
        """
        ...

    @property
    def positions(self) -> list[PositionBudgetEntry]:
        """
        Per-position risk-budget entries.
        Returns
        -------
        list[PositionBudgetEntry]
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-position budget comparison as a pandas DataFrame.

        One row per entry of :attr:`positions`. The scalars
        :attr:`total_overbudget` and :attr:`has_breach` are header metadata
        and are not repeated per row.

        Columns: ``position_id``, ``actual_component_var``,
        ``target_component_var``, ``utilization`` (ratio, not percentage),
        ``excess``.

        Returns
        -------
        pd.DataFrame
            One row per budgeted position.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class FactorContributionDelta:
    """
    Per-factor contribution change between a baseline and a scenario.

    Examples
    --------
    >>> from finstack_quant.portfolio import FactorContributionDelta
    >>> delta = FactorContributionDelta.from_json('{"factor_id":"Rates","absolute_change":0.1,"relative_change":0.2}')
    >>> (delta.factor_id, delta.absolute_change)
    ('Rates', 0.1)
    """

    @classmethod
    def from_json(cls, json_str: str) -> FactorContributionDelta:
        """
        Deserialize a factor contribution delta from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized baseline-to-scenario factor delta, normally
            produced by ``FactorContributionDelta.to_json``.

        Returns
        -------
        FactorContributionDelta
            Validated `FactorContributionDelta` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import FactorContributionDelta
        >>> delta = FactorContributionDelta.from_json(
        ...     '{"factor_id":"Rates","absolute_change":0.1,"relative_change":0.2}'
        ... )
        >>> delta.relative_change
        0.2
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this factor contribution delta to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `FactorContributionDelta`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def factor_id(self) -> str:
        """
        Factor identifier.
        Returns
        -------
        str
            The factor id exposed by this `FactorContributionDelta`.
        """
        ...

    @property
    def absolute_change(self) -> float:
        """
        Absolute contribution change.
        Returns
        -------
        float
            The absolute change exposed by this `FactorContributionDelta`.
        """
        ...

    @property
    def relative_change(self) -> float:
        """
        Relative contribution change.
        Returns
        -------
        float
            The relative change exposed by this `FactorContributionDelta`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class WhatIfResult:
    """
    Result of a position what-if scenario.

    Examples
    --------
    >>> from finstack_quant.portfolio import WhatIfResult
    >>> risk = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
    >>> result = WhatIfResult.from_json('{"before":' + risk + ',"after":' + risk + ',"delta":[]}')
    >>> result.delta
    []
    """

    @classmethod
    def from_json(cls, json_str: str) -> WhatIfResult:
        """
        Deserialize a what-if result from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized before-and-after risk result, normally
            produced by ``WhatIfResult.to_json``.

        Returns
        -------
        WhatIfResult
            Validated `WhatIfResult` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import WhatIfResult
        >>> risk = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
        >>> result = WhatIfResult.from_json('{"before":' + risk + ',"after":' + risk + ',"delta":[]}')
        >>> result.before.total_risk
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this what-if result to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `WhatIfResult`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def before(self) -> RiskDecomposition:
        """
        Baseline risk decomposition.
        Returns
        -------
        RiskDecomposition
        """
        ...

    @property
    def after(self) -> RiskDecomposition:
        """
        Post-scenario risk decomposition.
        Returns
        -------
        RiskDecomposition
        """
        ...

    @property
    def delta(self) -> list[FactorContributionDelta]:
        """
        Per-factor contribution changes.
        Returns
        -------
        list[FactorContributionDelta]
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-factor contribution changes as a pandas DataFrame.

        One row per entry of :attr:`delta`. The baseline and scenario
        decompositions are not flattened here — reach them through
        :attr:`before` / :attr:`after` and their own frame accessors.

        Columns: ``factor_id``, ``absolute_change`` (risk-measure units),
        ``relative_change`` (dimensionless fraction, not a percentage).

        Returns
        -------
        pd.DataFrame
            One row per changed factor.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class StressResult:
    """
    Result of a factor-stress scenario.

    Examples
    --------
    >>> from finstack_quant.portfolio import StressResult
    >>> risk = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
    >>> result = StressResult.from_json(
    ...     '{"total_pnl":-10.0,"position_pnl":[["P1",-10.0]],"stressed_decomposition":' + risk + "}"
    ... )
    >>> result.total_pnl
    -10.0
    """

    @classmethod
    def from_json(cls, json_str: str) -> StressResult:
        """
        Deserialize a stress result from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized stressed P&L and decomposition result, normally
            produced by ``StressResult.to_json``.

        Returns
        -------
        StressResult
            Validated `StressResult` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import StressResult
        >>> risk = '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
        >>> result = StressResult.from_json(
        ...     '{"total_pnl":-10.0,"position_pnl":[["P1",-10.0]],"stressed_decomposition":' + risk + "}"
        ... )
        >>> result.position_pnl
        [('P1', -10.0)]
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this stress result to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `StressResult`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def total_pnl(self) -> float:
        """
        Total portfolio P&L under the stress scenario.
        Returns
        -------
        float
            The total pnl exposed by this `StressResult`.
        """
        ...

    @property
    def position_pnl(self) -> list[tuple[str, float]]:
        """
        Per-position P&L pairs.
        Returns
        -------
        list[tuple[str, float]]
        """
        ...

    @property
    def stressed_decomposition(self) -> RiskDecomposition:
        """
        Risk decomposition after applying the stress.
        Returns
        -------
        RiskDecomposition
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-position stressed P&L as a pandas DataFrame.

        One row per entry of :attr:`position_pnl`. The scenario totals stay on
        :attr:`total_pnl` and :attr:`stressed_decomposition`.

        Columns: ``position_id``, ``pnl`` (portfolio-currency amount; a loss
        is negative).

        Returns
        -------
        pd.DataFrame
            One row per position with a stressed P&L contribution.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class StressPositionEntry:
    """
    Single position's contribution to tail stress.

    Examples
    --------
    >>> from finstack_quant.portfolio import StressPositionEntry
    >>> item = StressPositionEntry.from_json(
    ...     '{"position_id":"P1","avg_tail_pnl":-5.0,"pct_of_tail_loss":1.0,"worst_scenario_pnl":-10.0}'
    ... )
    >>> (item.position_id, item.worst_scenario_pnl)
    ('P1', -10.0)
    """

    @classmethod
    def from_json(cls, json_str: str) -> StressPositionEntry:
        """
        Deserialize a stress position entry from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized per-position tail-stress contribution, normally
            produced by ``StressPositionEntry.to_json``.

        Returns
        -------
        StressPositionEntry
            Validated `StressPositionEntry` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import StressPositionEntry
        >>> item = StressPositionEntry.from_json(
        ...     '{"position_id":"P1","avg_tail_pnl":-5.0,"pct_of_tail_loss":1.0,"worst_scenario_pnl":-10.0}'
        ... )
        >>> item.pct_of_tail_loss
        1.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this stress position entry to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `StressPositionEntry`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `StressPositionEntry`.
        """
        ...

    @property
    def avg_tail_pnl(self) -> float:
        """
        Average P&L across tail scenarios.
        Returns
        -------
        float
            The avg tail pnl exposed by this `StressPositionEntry`.
        """
        ...

    @property
    def pct_of_tail_loss(self) -> float:
        """
        Share of aggregate tail loss.
        Returns
        -------
        float
            The pct of tail loss exposed by this `StressPositionEntry`.
        """
        ...

    @property
    def worst_scenario_pnl(self) -> float:
        """
        Worst single-scenario P&L for this position.
        Returns
        -------
        float
            The worst scenario pnl exposed by this `StressPositionEntry`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class TailScenarioBreakdown:
    """
    Breakdown of a single tail scenario.

    Examples
    --------
    >>> from finstack_quant.portfolio import TailScenarioBreakdown
    >>> scenario = TailScenarioBreakdown.from_json('{"scenario_index":0,"portfolio_pnl":-10.0,"position_pnls":[-10.0]}')
    >>> (scenario.scenario_index, scenario.portfolio_pnl)
    (0, -10.0)
    """

    @classmethod
    def from_json(cls, json_str: str) -> TailScenarioBreakdown:
        """
        Deserialize a tail scenario breakdown from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized tail-scenario P&L breakdown, normally
            produced by ``TailScenarioBreakdown.to_json``.

        Returns
        -------
        TailScenarioBreakdown
            Validated `TailScenarioBreakdown` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import TailScenarioBreakdown
        >>> scenario = TailScenarioBreakdown.from_json(
        ...     '{"scenario_index":0,"portfolio_pnl":-10.0,"position_pnls":[-10.0]}'
        ... )
        >>> scenario.position_pnls
        [-10.0]
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this tail scenario breakdown to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `TailScenarioBreakdown`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def scenario_index(self) -> int:
        """
        Scenario index in the source P&L matrix.
        Returns
        -------
        int
            The scenario index exposed by this `TailScenarioBreakdown`.
        """
        ...

    @property
    def portfolio_pnl(self) -> float:
        """
        Portfolio P&L for this tail scenario.
        Returns
        -------
        float
            The portfolio pnl exposed by this `TailScenarioBreakdown`.
        """
        ...

    @property
    def position_pnls(self) -> list[float]:
        """
        Per-position P&L for this scenario, index-aligned to
        ``StressAttribution.position_ids`` (entry ``i`` is the P&L for
        ``position_ids[i]``).
        Returns
        -------
        list[float]
            The position pnls exposed by this `TailScenarioBreakdown`.
        """
        ...

    def to_dataframe(self, position_ids: list[str]) -> pd.DataFrame:
        """
        Export this scenario's per-position P&L as a pandas DataFrame.

        The breakdown carries no identifiers of its own — they live once on
        the parent ``StressAttribution.position_ids`` — so they must be
        supplied here, exactly as for :meth:`FactorPnlProfile.to_dataframe`.

        Columns: ``position_id``, ``pnl`` (portfolio-currency amount; a loss
        is negative).

        Parameters
        ----------
        position_ids : list[str]
            Position identifiers, normally ``attribution.position_ids``. Must
            match the number of entries in :attr:`position_pnls`.

        Returns
        -------
        pd.DataFrame
            One row per position.

        Raises
        ------
        ValueError
            If ``len(position_ids)`` does not match ``len(position_pnls)``.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class StressAttribution:
    """
    Per-position attribution of portfolio losses in tail scenarios.

    Examples
    --------
    >>> from finstack_quant.portfolio import StressAttribution
    >>> doc = '{"var_threshold":-5.0,"n_tail_scenarios":1,"position_ids":["P1"],"position_contributions":[],"tail_scenarios":[]}'
    >>> result = StressAttribution.from_json(doc)
    >>> (result.var_threshold, result.n_tail_scenarios)
    (-5.0, 1)
    """

    @classmethod
    def from_json(cls, json_str: str) -> StressAttribution:
        """
        Deserialize stress attribution from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized tail-loss attribution, normally produced by
            ``StressAttribution.to_json``.

        Returns
        -------
        StressAttribution
            Validated `StressAttribution` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import StressAttribution
        >>> doc = '{"var_threshold":-5.0,"n_tail_scenarios":1,"position_ids":["P1"],"position_contributions":[],"tail_scenarios":[]}'
        >>> StressAttribution.from_json(doc).position_ids
        ['P1']
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this stress attribution to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `StressAttribution`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def var_threshold(self) -> float:
        """
        VaR threshold used to select tail scenarios.
        Returns
        -------
        float
            The var threshold exposed by this `StressAttribution`.
        """
        ...

    @property
    def n_tail_scenarios(self) -> int:
        """
        Number of scenarios classified as tail scenarios.
        Returns
        -------
        int
            The n tail scenarios exposed by this `StressAttribution`.
        """
        ...

    @property
    def position_ids(self) -> list[str]:
        """
        Canonical position ordering shared by every ``tail_scenarios`` entry.
        ``tail_scenarios[k].position_pnls[i]`` is the P&L for ``position_ids[i]``.
        Returns
        -------
        list[str]
            The position ids exposed by this `StressAttribution`.
        """
        ...

    @property
    def position_contributions(self) -> list[StressPositionEntry]:
        """
        Per-position tail-loss contributions.
        Returns
        -------
        list[StressPositionEntry]
        """
        ...

    @property
    def tail_scenarios(self) -> list[TailScenarioBreakdown]:
        """
        Detailed tail scenario breakdowns.
        Returns
        -------
        list[TailScenarioBreakdown]
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-position tail-loss contributions as a DataFrame.

        One row per entry of :attr:`position_contributions`, in the same
        (largest-driver-first) order.

        Columns: ``position_id``, ``avg_tail_pnl`` (portfolio-currency average
        across tail scenarios; a loss is negative), ``pct_of_tail_loss``
        (**fraction**, not percentage, of total portfolio tail loss),
        ``worst_scenario_pnl``.

        Returns
        -------
        pd.DataFrame
            One row per contributing position.
        """
        ...

    def to_scenario_dataframe(self) -> pd.DataFrame:
        """
        Export the tail scenario x position P&L matrix as a DataFrame.

        Rows are tail scenarios indexed by
        ``TailScenarioBreakdown.scenario_index``; columns are the position
        identifiers from :attr:`position_ids`, in that order. Every cell is a
        portfolio-currency P&L (a loss is negative).

        Columns: one per entry of :attr:`position_ids`.

        Returns
        -------
        pd.DataFrame
            One row per tail scenario, indexed by scenario index.

        Raises
        ------
        ValueError
            If any tail scenario's ``position_pnls`` width disagrees with
            ``len(position_ids)`` (only reachable through a hand-built
            :meth:`from_json` payload).
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionAssignment:
    """
    Matched factor assignments for a single portfolio position.

    The full ``(dependency, factor_id)`` pairs are available as JSON via
    :meth:`mappings_json`; matched factor identifiers are accessible directly
    via the :attr:`factor_ids` property.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionAssignment
    >>> assignment = PositionAssignment.from_json('{"position_id":"P1","mappings":[]}')
    >>> (assignment.position_id, assignment.n_mappings)
    ('P1', 0)
    """

    @classmethod
    def from_json(cls, json_str: str) -> PositionAssignment:
        """
        Deserialize a position assignment from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized factor assignment for one position, normally
            produced by ``PositionAssignment.to_json``.

        Returns
        -------
        PositionAssignment
            Validated `PositionAssignment` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionAssignment
        >>> PositionAssignment.from_json('{"position_id":"P1","mappings":[]}').factor_ids
        []
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position assignment to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionAssignment`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionAssignment`.
        """
        ...

    @property
    def n_mappings(self) -> int:
        """
        Number of dependency-to-factor mappings.
        Returns
        -------
        int
            The n mappings exposed by this `PositionAssignment`.
        """
        ...

    def mappings_json(self) -> str:
        """
        Return detailed dependency-to-factor mappings as JSON.
        Returns
        -------
        str
            Compact JSON for matched ``(dependency, factor_id, beta)`` triples.
        """
        ...

    @property
    def factor_ids(self) -> list[str]:
        """
        Matched factor identifiers.
        Returns
        -------
        list[str]
            The factor ids exposed by this `PositionAssignment`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class UnmatchedEntry:
    """
    Single unmatched dependency surfaced during assignment.

    Examples
    --------
    >>> from finstack_quant.portfolio import UnmatchedEntry
    >>> entry = UnmatchedEntry.from_json('{"position_id":"P1","dependency":{"fx_pair":{"base":"USD","quote":"EUR"}}}')
    >>> entry.position_id
    'P1'
    """

    @classmethod
    def from_json(cls, json_str: str) -> UnmatchedEntry:
        """
        Deserialize an unmatched dependency entry from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized unmatched-factor diagnostic, normally produced
            by ``UnmatchedEntry.to_json``.

        Returns
        -------
        UnmatchedEntry
            Validated `UnmatchedEntry` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import UnmatchedEntry
        >>> entry = UnmatchedEntry.from_json(
        ...     '{"position_id":"P1","dependency":{"fx_pair":{"base":"USD","quote":"EUR"}}}'
        ... )
        >>> "fx_pair" in entry.to_json()
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this unmatched entry to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `UnmatchedEntry`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `UnmatchedEntry`.
        """
        ...

    def dependency_json(self) -> str:
        """
        Return the unmatched dependency payload as JSON.
        Returns
        -------
        str
            Compact JSON for the unmatched market-data dependency.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class FactorAssignmentReport:
    """
    Assignment results for a portfolio-level factor mapping pass.

    Examples
    --------
    >>> from finstack_quant.portfolio import FactorAssignmentReport
    >>> report = FactorAssignmentReport.from_json('{"assignments":[{"position_id":"P1","mappings":[]}],"unmatched":[]}')
    >>> len(report.assignments)
    1
    """

    @classmethod
    def from_json(cls, json_str: str) -> FactorAssignmentReport:
        """
        Deserialize a factor assignment report from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized factor-model assignment report, normally
            produced by ``FactorAssignmentReport.to_json``.

        Returns
        -------
        FactorAssignmentReport
            Validated `FactorAssignmentReport` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import FactorAssignmentReport
        >>> report = FactorAssignmentReport.from_json(
        ...     '{"assignments":[{"position_id":"P1","mappings":[]}],"unmatched":[]}'
        ... )
        >>> report.unmatched
        []
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this factor assignment report to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `FactorAssignmentReport`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def assignments(self) -> list[PositionAssignment]:
        """
        Matched assignments by position.
        Returns
        -------
        list[PositionAssignment]
        """
        ...

    @property
    def unmatched(self) -> list[UnmatchedEntry]:
        """
        Dependencies that could not be mapped to factors.
        Returns
        -------
        list[UnmatchedEntry]
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the matched assignments as a long-format DataFrame.

        One row per ``(dependency, factor_id, beta)`` mapping, so a position
        matched by a hierarchical matcher (e.g. credit) contributes one row
        per level. Row count is the sum of ``n_mappings`` across
        :attr:`assignments`, not the number of positions.

        Columns: ``position_id``, ``factor_id``, ``beta`` (factor loading),
        ``dependency_json`` — the ``MarketDependency`` variant tree is
        deliberately kept as a JSON string rather than exploded into columns,
        matching :meth:`PositionAssignment.mappings_json`.

        Returns
        -------
        pd.DataFrame
            One row per matched mapping.

        Raises
        ------
        ValueError
            If a market dependency cannot be serialized to JSON.
        """
        ...

    def to_unmatched_dataframe(self) -> pd.DataFrame:
        """
        Export the unmatched dependencies as a pandas DataFrame.

        Columns: ``position_id``, ``dependency_json`` (the unmatched
        ``MarketDependency`` as a JSON string).

        Returns
        -------
        pd.DataFrame
            One row per entry of :attr:`unmatched`; a zero-row frame (columns
            still present) means every dependency matched a configured factor.

        Raises
        ------
        ValueError
            If a market dependency cannot be serialized to JSON.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class LevelVolContribution:
    """
    Aggregated risk contribution for a single hierarchy level.

    Examples
    --------
    >>> from finstack_quant.portfolio import LevelVolContribution
    >>> try:
    ...     LevelVolContribution()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.LevelVolContribution' instances
    """

    @property
    def level_name(self) -> str:
        """
        Hierarchy level name.
        Returns
        -------
        str
            The level name exposed by this `LevelVolContribution`.
        """
        ...

    @property
    def total(self) -> float:
        """
        Total contribution for this level.
        Returns
        -------
        float
            The total exposed by this `LevelVolContribution`.
        """
        ...

    @property
    def by_bucket(self) -> dict[str, float]:
        """
        Contribution by hierarchy bucket.
        Returns
        -------
        dict[str, float]
            The by bucket exposed by this `LevelVolContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionVolContribution:
    """
    Per-position vol breakdown under :class:`CreditVolReport`.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionVolContribution
    >>> try:
    ...     PositionVolContribution()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.PositionVolContribution' instances
    """

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `PositionVolContribution`.
        """
        ...

    @property
    def factor_total(self) -> float:
        """
        Factor-driven volatility contribution.
        Returns
        -------
        float
            The factor total exposed by this `PositionVolContribution`.
        """
        ...

    @property
    def idiosyncratic(self) -> float:
        """
        Idiosyncratic volatility contribution.
        Returns
        -------
        float
            The idiosyncratic exposed by this `PositionVolContribution`.
        """
        ...

    @property
    def total(self) -> float:
        """
        Total position volatility contribution.
        Returns
        -------
        float
            The total exposed by this `PositionVolContribution`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class CreditVolReport:
    """
    Aggregated vol report grouped by hierarchy level.

    Examples
    --------
    >>> from finstack_quant.portfolio import CreditVolReport
    >>> try:
    ...     CreditVolReport()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.CreditVolReport' instances
    """

    @property
    def total(self) -> float:
        """
        Total portfolio volatility under the report measure.
        Returns
        -------
        float
            The total exposed by this `CreditVolReport`.
        """
        ...

    @property
    def measure_json(self) -> str:
        """
        Risk measure specification as JSON.
        Returns
        -------
        str
            The measure json exposed by this `CreditVolReport`.
        """
        ...

    @property
    def generic(self) -> float:
        """
        Generic factor contribution.
        Returns
        -------
        float
            The generic exposed by this `CreditVolReport`.
        """
        ...

    @property
    def idiosyncratic_total(self) -> float:
        """
        Aggregate idiosyncratic contribution.
        Returns
        -------
        float
            The idiosyncratic total exposed by this `CreditVolReport`.
        """
        ...

    @property
    def by_level(self) -> list[LevelVolContribution]:
        """
        Volatility contribution by hierarchy level.
        Returns
        -------
        list[LevelVolContribution]
        """
        ...

    @property
    def by_position(self) -> list[PositionVolContribution] | None:
        """
        Optional per-position volatility contributions.
        Returns
        -------
        list[PositionVolContribution] or None
        """
        ...

    def to_level_dataframe(self) -> pd.DataFrame:
        """
        Export the per-hierarchy-level rollup as a pandas DataFrame.

        One row per entry of :attr:`by_level`. The per-bucket drill-down stays
        on ``LevelVolContribution.by_bucket``, which is already a plain dict.

        Columns: ``level_name``, ``total`` (level contribution in the units of
        :attr:`measure_json`).

        Returns
        -------
        pd.DataFrame
            One row per hierarchy level.
        """
        ...

    def to_position_dataframe(self) -> pd.DataFrame:
        """
        Export the optional per-position breakdown as a pandas DataFrame.

        One row per entry of :attr:`by_position`. When the report was built
        without ``by_position=True`` the result is a zero-row frame that still
        carries the full column schema, so downstream ``df[...]`` selections
        keep working instead of raising ``KeyError``.

        Columns: ``position_id``, ``factor_total`` (systematic),
        ``idiosyncratic`` (issuer-specific), ``total`` (their sum) — all in
        the units of :attr:`measure_json`.

        Returns
        -------
        pd.DataFrame
            One row per position, or zero rows when :attr:`by_position` is
            ``None``.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class VolHorizon:
    """
    Forecast horizon used to scale a calibrated `Sample` vol estimate.

    Examples
    --------
    >>> from finstack_quant.portfolio import VolHorizon
    >>> VolHorizon.n_steps(5).n
    5
    """

    @classmethod
    def one_step(cls) -> VolHorizon:
        """
        Use the calibrated one-step forecast horizon.

        Returns
        -------
        VolHorizon
            One-calibrated-period forecast horizon.

        Examples
        --------
        >>> from finstack_quant.portfolio import VolHorizon
        >>> VolHorizon.one_step().kind
        'one_step'
        """
        ...

    @classmethod
    def unconditional(cls) -> VolHorizon:
        """
        Use the unconditional long-run forecast horizon.

        Returns
        -------
        VolHorizon
            Unconditional long-run forecast horizon.

        Examples
        --------
        >>> from finstack_quant.portfolio import VolHorizon
        >>> VolHorizon.unconditional().kind
        'unconditional'
        """
        ...

    @classmethod
    def n_steps(cls, n: int) -> VolHorizon:
        """
        Scale the forecast to a fixed number of discrete steps.

        Parameters
        ----------
        n : int
            Positive number of calibrated sampling periods to forecast ahead.

        Returns
        -------
        VolHorizon
            Discrete forecast horizon spanning ``n`` calibrated periods.

        Examples
        --------
        >>> from finstack_quant.portfolio import VolHorizon
        >>> VolHorizon.n_steps(5).n
        5
        """
        ...

    @classmethod
    def years(cls, years: float) -> VolHorizon:
        """
        Scale the forecast to a year fraction.

        Parameters
        ----------
        years : float
            Positive forecast horizon in years, converted using the calibrated
            model's observation frequency.

        Returns
        -------
        VolHorizon
            Forecast horizon spanning the supplied fractional number of years.

        Raises
        ------
        ValueError
            If ``years`` is non-finite or negative.

        Examples
        --------
        >>> from finstack_quant.portfolio import VolHorizon
        >>> VolHorizon.years(2.5).years_value
        2.5
        """
        ...

    @classmethod
    def parse(cls, s: str) -> VolHorizon:
        """
        Parse a horizon string accepted by the Rust factor model.

        Parameters
        ----------
        s : str
            Horizon expression such as ``"one_step"``, ``"unconditional"``,
            a step count, or a year-based form accepted by the model.

        Returns
        -------
        VolHorizon
            Horizon variant represented by the keyword or JSON descriptor in ``s``.

        Raises
        ------
        ValueError
            If ``s`` is not a horizon descriptor accepted by ``VolHorizon``.

        Examples
        --------
        >>> from finstack_quant.portfolio import VolHorizon
        >>> VolHorizon.parse('{"n_steps":5}').n
        5
        """
        ...

    @property
    def kind(self) -> str:
        """
        Horizon variant label.
        Returns
        -------
        str
            The kind exposed by this `VolHorizon`.
        """
        ...

    @property
    def n(self) -> int | None:
        """
        Step count for ``n_steps`` horizons.
        Returns
        -------
        int or None
            The n exposed by this `VolHorizon`.
        """
        ...

    @property
    def years_value(self) -> float | None:
        """
        Year fraction for ``years`` horizons.
        Returns
        -------
        float or None
            The years value exposed by this `VolHorizon`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class DecompositionConfig:
    """
    Configuration for position-level VaR decomposition.

    Examples
    --------
    >>> from finstack_quant.portfolio import DecompositionConfig
    >>> DecompositionConfig.parametric_95().confidence
    0.95
    """

    @classmethod
    def parametric_95(cls) -> DecompositionConfig:
        """
        Default 95% parametric VaR decomposition config.

        Returns
        -------
        DecompositionConfig
            Parametric 95% config with incremental VaR disabled and no explicit seed.

        Examples
        --------
        >>> from finstack_quant.portfolio import DecompositionConfig
        >>> (DecompositionConfig.parametric_95().method, DecompositionConfig.parametric_95().confidence)
        ('parametric', 0.95)
        """
        ...

    @classmethod
    def parametric_99(cls) -> DecompositionConfig:
        """
        Default 99% parametric VaR decomposition config.

        Returns
        -------
        DecompositionConfig
            Parametric 99% config with incremental VaR disabled and no explicit seed.

        Examples
        --------
        >>> from finstack_quant.portfolio import DecompositionConfig
        >>> DecompositionConfig.parametric_99().confidence
        0.99
        """
        ...

    @classmethod
    def historical(cls, confidence: float) -> DecompositionConfig:
        """
        Build a historical VaR decomposition configuration.

        Parameters
        ----------
        confidence : float
            VaR confidence as a decimal probability in ``(0, 1)``, such as
            ``0.95`` for a 95% confidence level.

        Returns
        -------
        DecompositionConfig
            Historical config at the supplied confidence with no incremental VaR or seed.

        Examples
        --------
        >>> from finstack_quant.portfolio import DecompositionConfig
        >>> DecompositionConfig.historical(0.975).method
        'historical'
        """
        ...

    def with_incremental(self) -> DecompositionConfig:
        """
        Return a copy that requests incremental VaR.
        Returns
        -------
        DecompositionConfig
        """
        ...

    def with_seed(self, seed: int) -> DecompositionConfig:
        """
        Return a copy with a deterministic simulation seed.

        Parameters
        ----------
        seed : int
            Integer seed used to reproduce any randomized decomposition steps.

        Returns
        -------
        DecompositionConfig
            Copy with ``seed`` recorded for simulation-path decompositions.
        """
        ...

    @property
    def confidence(self) -> float:
        """
        VaR/ES confidence level.
        Returns
        -------
        float
            The confidence exposed by this `DecompositionConfig`.
        """
        ...

    @property
    def method(self) -> str:
        """
        Decomposition method label.
        Returns
        -------
        str
            The method exposed by this `DecompositionConfig`.
        """
        ...

    @property
    def compute_incremental(self) -> bool:
        """
        Whether incremental VaR is requested.
        Returns
        -------
        bool
            The compute incremental exposed by this `DecompositionConfig`.
        """
        ...

    @property
    def seed(self) -> int | None:
        """
        Optional deterministic seed.
        Returns
        -------
        int or None
            The seed exposed by this `DecompositionConfig`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

def factor_stress(
    portfolio: Portfolio | str,
    market: MarketContext | str,
    factor_model_config_json: str,
    as_of: str,
    stresses: list[tuple[str, float]],
) -> StressResult:
    """
    Run a factor-stress scenario and revalue the portfolio.

    Builds the Rust factor model from ``factor_model_config_json``, analyzes
    the base portfolio, computes sensitivities, applies the requested factor
    shifts, and returns the stressed result.

    Parameters
    ----------
    portfolio : Portfolio or str
        Portfolio instance or JSON portfolio specification accepted
        by the compiled portfolio extractor.
    market : MarketContext or str
        MarketContext instance or JSON market context accepted by the
        compiled market extractor.
    factor_model_config_json : str
        JSON-encoded ``finstack_quant_factor_model::FactorModelConfig``.
    as_of : str
        ISO calculation date, ``YYYY-MM-DD``.
    stresses : list[tuple[str, float]]
        ``(factor_id, shift)`` pairs. Factor IDs must match the
        configured model; shifts use the Rust factor model's units for that
        factor.

    Returns
    -------
    StressResult
        StressResult containing base/stressed portfolio risk and per-factor
        deltas.

    Raises
    ------
    ValueError
        If JSON parsing, date parsing, model construction, market
        lookup, or portfolio valuation fails.
    TypeError
        If ``portfolio`` or ``market`` cannot be converted to the
        expected Rust types.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import factor_stress
    >>> portfolio = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> try:
    ...     factor_stress(portfolio, MarketContext(), "{}", "2025-01-01", [])
    ... except ValueError as exc:
    ...     print("missing field `factors`" in str(exc))
    True
    """
    ...

def position_what_if(
    portfolio: Portfolio | str,
    market: MarketContext | str,
    factor_model_config_json: str,
    as_of: str,
    changes: list[dict[str, Any]],
) -> WhatIfResult:
    """
    Run position remove/resize what-if analysis.

    The Python binding accepts JSON-like dictionaries for remove and resize
    changes, then delegates the sensitivity reallocation and result generation
    to Rust.

    Parameters
    ----------
    portfolio : Portfolio or str
        Portfolio instance or JSON portfolio specification accepted
        by the compiled portfolio extractor.
    market : MarketContext or str
        MarketContext instance or JSON market context accepted by the
        compiled market extractor.
    factor_model_config_json : str
        JSON-encoded ``finstack_quant_factor_model::FactorModelConfig``.
    as_of : str
        ISO calculation date, ``YYYY-MM-DD``.
    changes : list[dict[str, Any]]
        List of dictionaries. Remove changes use
        ``{"kind": "remove", "position_id": "..."}``; resize changes use
        ``{"kind": "resize", "position_id": "...", "new_quantity": 123.0}``.
        Add changes are not supported by this JSON-shaped Python helper
        because adding requires a typed Rust position object.

    Returns
    -------
    WhatIfResult
        WhatIfResult with base and scenario risk decomposition deltas.

    Raises
    ------
    ValueError
        If a change kind is unknown, resize omits
        ``new_quantity``, add is requested, JSON/config parsing fails, or
        Rust factor-model evaluation fails.
    TypeError
        If ``portfolio`` or ``market`` cannot be converted to the
        expected Rust types.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import position_what_if
    >>> portfolio = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> try:
    ...     position_what_if(portfolio, MarketContext(), "{}", "2025-01-01", [])
    ... except ValueError as exc:
    ...     print("missing field `factors`" in str(exc))
    True
    """
    ...

def build_stress_attribution(
    position_ids: list[str],
    position_pnls: list[list[float]] | npt.NDArray[np.float64],
    confidence: float = 0.95,
) -> StressAttribution:
    """
    Build tail-scenario stress attribution from position P&Ls.

    Python input is position-major: one row per position, and each row contains
    that position's P&L across all scenarios. The binding transposes this into
    Rust's scenario-major buffer before selecting tail scenarios.

    Parameters
    ----------
    position_ids : list[str]
        Position identifiers, one per row in ``position_pnls``.
    position_pnls : list[list[float]] or numpy.ndarray
        Matrix shaped ``len(position_ids) x n_scenarios``.
        Every row must have the same number of finite scenario P&Ls.
        C-contiguous ``float64`` arrays use the direct buffer path.
    confidence : float, default 0.95
        Tail confidence level in ``(0.5, 1)``. The Rust engine
        selects ``floor((1 - confidence) * n_scenarios)`` tail scenarios.

    Returns
    -------
    StressAttribution
        StressAttribution containing VaR threshold, tail scenario count,
        per-position tail contributions, and scenario-level P&L breakdowns.

    Raises
    ------
    ValueError
        If dimensions are inconsistent, confidence is outside
        ``(0.5, 1)``, the requested tail has zero scenarios, or any P&L is
        non-finite.

    Examples
    --------
    >>> from finstack_quant.portfolio import build_stress_attribution
    >>> pnl = [list(range(-10, 10)), [2 * value for value in range(-10, 10)]]
    >>> result = build_stress_attribution(["A", "B"], pnl)
    >>> (result.var_threshold, result.n_tail_scenarios)
    (-30.0, 1)
    """
    ...

def build_credit_vol_report(
    decomposition: RiskDecomposition,
    model: CreditFactorModel,
    by_position: bool = False,
) -> CreditVolReport:
    """
    Build a credit volatility report from decomposition outputs.

    Aggregates a Rust ``RiskDecomposition`` against the supplied credit factor
    model hierarchy into generic, level, idiosyncratic, and optional
    per-position volatility contributions.

    Parameters
    ----------
    decomposition : RiskDecomposition
        Factor risk decomposition to summarize.
    model : CreditFactorModel
        Credit factor model whose hierarchy and factor taxonomy label
        the report levels and buckets.
    by_position : bool, default False
        Include per-position contribution rows when ``True``.

    Returns
    -------
    CreditVolReport
        CreditVolReport with total, generic, level, idiosyncratic, and optional
        position-level volatility contribution fields.

    Examples
    --------
    >>> from finstack_quant.portfolio import RiskDecomposition, build_credit_vol_report
    >>> risk = RiskDecomposition.from_json(
    ...     '{"total_risk":1.0,"measure":"variance","factor_contributions":[],"residual_risk":1.0,"position_factor_contributions":[],"position_residual_contributions":[]}'
    ... )
    >>> try:
    ...     build_credit_vol_report(risk, object())
    ... except TypeError as exc:
    ...     print("CreditFactorModel" in str(exc))
    True
    """
    ...

def position_component_var(
    decomp: PositionRiskDecomposition,
    position_id: str,
) -> float:
    """
    Look up a position's component VaR inside a decomposition.

    Parameters
    ----------
    decomp : PositionRiskDecomposition
        Typed risk decomposition containing component VaR by position.
    position_id : str
        Position identifier whose component VaR is required; absent IDs raise
        ``KeyError``.

    Returns
    -------
    float
        Loss-signed component VaR for the requested position and portfolio measure.

    Raises
    ------
    KeyError
        If ``position_id`` is absent from ``decomp``.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionRiskDecomposition, position_component_var
    >>> doc = '{"portfolio_var":-1.0,"portfolio_es":-1.2,"confidence":0.95,"method":"parametric","var_contributions":[{"position_id":"P1","component_var":-1.0,"relative_var":1.0,"marginal_var":-1.0,"incremental_var":null}],"es_contributions":[],"n_positions":1,"euler_residual":0.0}'
    >>> position_component_var(PositionRiskDecomposition.from_json(doc), "P1")
    -1.0
    """
    ...

# optimization spec/result classes (Slice 9)

class WeightingScheme:
    """
    How optimization weights are defined.

    Examples
    --------
    >>> from finstack_quant.portfolio import WeightingScheme
    >>> WeightingScheme.value_weight().label
    'value_weight'
    """

    @classmethod
    def value_weight(cls) -> WeightingScheme:
        """
        Weight positions by market value.

        Returns
        -------
        WeightingScheme
            Weights as shares of gross absolute base-currency portfolio PV.

        Examples
        --------
        >>> from finstack_quant.portfolio import WeightingScheme
        >>> WeightingScheme.value_weight().label
        'value_weight'
        """
        ...

    @classmethod
    def notional_weight(cls) -> WeightingScheme:
        """
        Weight positions by notional.

        Returns
        -------
        WeightingScheme
            Weights as normalized shares of notional exposure.

        Examples
        --------
        >>> from finstack_quant.portfolio import WeightingScheme
        >>> WeightingScheme.notional_weight().label
        'notional_weight'
        """
        ...

    @classmethod
    def unit_scaling(cls) -> WeightingScheme:
        """
        Use unit scaling rather than value/notional scaling.

        Returns
        -------
        WeightingScheme
            Existing weights as quantity multipliers and candidate weights as quantities.

        Examples
        --------
        >>> from finstack_quant.portfolio import WeightingScheme
        >>> WeightingScheme.unit_scaling().label
        'unit_scaling'
        """
        ...

    @property
    def label(self) -> str:
        """
        Rust enum label for this weighting scheme.
        Returns
        -------
        str
            The label exposed by this `WeightingScheme`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class MissingMetricPolicy:
    """
    Policy for handling positions missing required metrics.

    Examples
    --------
    >>> from finstack_quant.portfolio import MissingMetricPolicy
    >>> MissingMetricPolicy.strict().label
    'strict'
    """

    @classmethod
    def zero(cls) -> MissingMetricPolicy:
        """
        Treat missing metric values as zero.

        Returns
        -------
        MissingMetricPolicy
            Policy substituting ``0.0`` for each missing required metric.

        Examples
        --------
        >>> from finstack_quant.portfolio import MissingMetricPolicy
        >>> MissingMetricPolicy.zero().label
        'zero'
        """
        ...

    @classmethod
    def exclude(cls) -> MissingMetricPolicy:
        """
        Exclude positions with missing required metrics.

        Returns
        -------
        MissingMetricPolicy
            Policy excluding missing-metric positions and retaining their current weights.

        Examples
        --------
        >>> from finstack_quant.portfolio import MissingMetricPolicy
        >>> MissingMetricPolicy.exclude().label
        'exclude'
        """
        ...

    @classmethod
    def strict(cls) -> MissingMetricPolicy:
        """
        Reject optimization when required metrics are missing.

        Returns
        -------
        MissingMetricPolicy
            Policy failing optimization when any required metric is missing.

        Examples
        --------
        >>> from finstack_quant.portfolio import MissingMetricPolicy
        >>> MissingMetricPolicy.strict().label
        'strict'
        """
        ...

    @property
    def label(self) -> str:
        """
        Rust enum label for this policy.
        Returns
        -------
        str
            The label exposed by this `MissingMetricPolicy`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class Inequality:
    """
    Inequality / equality operator (`<=`, `>=`, `==`).

    Examples
    --------
    >>> from finstack_quant.portfolio import Inequality
    >>> Inequality.le().label
    'le'
    """

    @classmethod
    def le(cls) -> Inequality:
        """
        Less-than-or-equal inequality.

        Returns
        -------
        Inequality
            Operator encoding ``lhs <= rhs``.

        Examples
        --------
        >>> from finstack_quant.portfolio import Inequality
        >>> Inequality.le().label
        'le'
        """
        ...

    @classmethod
    def ge(cls) -> Inequality:
        """
        Greater-than-or-equal inequality.

        Returns
        -------
        Inequality
            Operator encoding ``lhs >= rhs``.

        Examples
        --------
        >>> from finstack_quant.portfolio import Inequality
        >>> Inequality.ge().label
        'ge'
        """
        ...

    @classmethod
    def eq(cls) -> Inequality:
        """
        Equality constraint.

        Returns
        -------
        Inequality
            Operator encoding ``lhs == rhs``.

        Examples
        --------
        >>> from finstack_quant.portfolio import Inequality
        >>> Inequality.eq().label
        'eq'
        """
        ...

    @property
    def label(self) -> str:
        """
        Return the label for `Inequality`.
        Operator label.
        Returns
        -------
        str
            The label exposed by this `Inequality`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class TradeDirection:
    """
    Trade direction (buy/sell/hold).

    Examples
    --------
    >>> from finstack_quant.portfolio import TradeDirection
    >>> TradeDirection.buy().label
    'buy'
    """

    @classmethod
    def buy(cls) -> TradeDirection:
        """
        Compute buy for `TradeDirection`.
        Buy direction.

        Returns
        -------
        TradeDirection
            Direction for increasing an instrument exposure.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeDirection
        >>> TradeDirection.buy().label
        'buy'
        """
        ...

    @classmethod
    def sell(cls) -> TradeDirection:
        """
        Compute sell for `TradeDirection`.
        Sell direction.

        Returns
        -------
        TradeDirection
            Direction for decreasing an instrument exposure.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeDirection
        >>> TradeDirection.sell().label
        'sell'
        """
        ...

    @classmethod
    def hold(cls) -> TradeDirection:
        """
        Hold/no-trade direction.

        Returns
        -------
        TradeDirection
            Direction representing no change in exposure.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeDirection
        >>> TradeDirection.hold().label
        'hold'
        """
        ...

    @property
    def label(self) -> str:
        """
        Direction label.
        Returns
        -------
        str
            The label exposed by this `TradeDirection`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class TradeType:
    """
    Trade type (existing/new-position/close-out).

    Examples
    --------
    >>> from finstack_quant.portfolio import TradeType
    >>> TradeType.existing().label
    'existing'
    """

    @classmethod
    def existing(cls) -> TradeType:
        """
        Trade an existing position.

        Returns
        -------
        TradeType
            Trade type for adjusting an existing portfolio position.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeType
        >>> TradeType.existing().label
        'existing'
        """
        ...

    @classmethod
    def new_position(cls) -> TradeType:
        """
        Open a new candidate position.

        Returns
        -------
        TradeType
            Trade type for adding a candidate instrument as a new position.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeType
        >>> TradeType.new_position().label
        'new_position'
        """
        ...

    @classmethod
    def close_out(cls) -> TradeType:
        """
        Close an existing position.

        Returns
        -------
        TradeType
            Trade type for fully closing an existing position.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeType
        >>> TradeType.close_out().label
        'close_out'
        """
        ...

    @property
    def label(self) -> str:
        """
        Trade-type label.
        Returns
        -------
        str
            The label exposed by this `TradeType`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PerPositionMetric:
    """
    Per-position metric source for optimization expressions.

    Examples
    --------
    >>> from finstack_quant.portfolio import PerPositionMetric
    >>> PerPositionMetric.pv_base().kind
    'pv_base'
    """

    @classmethod
    def metric(cls, metric_id: str) -> PerPositionMetric:
        """
        Use a valuation metric by fully qualified metric ID.

        Parameters
        ----------
        metric_id : str
            Per-position metric key, such as ``"pv01::usd_ois"`` or
            ``"cs01::BOND_A"``.

        Returns
        -------
        PerPositionMetric
            Metric source reading ``metric_id`` from each position valuation.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.metric("duration").kind
        'metric'
        """
        ...

    @classmethod
    def custom_key(cls, key: str) -> PerPositionMetric:
        """
        Use a custom per-position metric key.

        Parameters
        ----------
        key : str
            Custom metric key emitted by the portfolio valuation pipeline.

        Returns
        -------
        PerPositionMetric
            Metric source reading the custom measure named ``key``.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.custom_key("desk_metric").kind
        'custom_key'
        """
        ...

    @classmethod
    def pv_base(cls) -> PerPositionMetric:
        """
        Use present value converted to portfolio base currency.

        Returns
        -------
        PerPositionMetric
            Metric source using each scaled position PV in portfolio base currency.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.pv_base().kind
        'pv_base'
        """
        ...

    @classmethod
    def pv_native(cls) -> PerPositionMetric:
        """
        Use present value in the instrument native currency.

        Returns
        -------
        PerPositionMetric
            Metric source using each scaled position PV in its native currency.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.pv_native().kind
        'pv_native'
        """
        ...

    @classmethod
    def attribute(cls, key: str) -> PerPositionMetric:
        """
        Use a position attribute as a metric source.

        Parameters
        ----------
        key : str
            Attribute name stored on positions, whose numeric values become the
            metric for selected positions.

        Returns
        -------
        PerPositionMetric
            Metric source reading the numeric position attribute named ``key``.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.attribute("sector").kind
        'attribute'
        """
        ...

    @classmethod
    def attribute_indicator(
        cls,
        key: str,
        op: str,
        text: str | None = None,
        number: float | None = None,
    ) -> PerPositionMetric:
        """
        Use a boolean position-attribute comparison as an indicator metric.

        Parameters
        ----------
        key : str
            Position attribute name to compare.
        op : str
            Comparison operator accepted by Rust, such as ``"eq"`` or ``"gt"``.
        text : str or None
            Optional string comparison value; supply when ``op`` compares text.
        number : float or None
            Optional numeric comparison value; supply when ``op`` compares numbers.

        Returns
        -------
        PerPositionMetric
            Binary metric returning ``1.0`` for a match and ``0.0`` otherwise.

        Raises
        ------
        ValueError
            If ``op`` is unknown, or exactly one of ``text`` and ``number`` is
            not supplied.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.attribute_indicator("sector", "eq", text="tech").kind
        'attribute_indicator'
        """
        ...

    @classmethod
    def constant(cls, value: float) -> PerPositionMetric:
        """
        Use a constant metric value for every selected position.

        Parameters
        ----------
        value : float
            Numeric metric value assigned identically to each selected position.

        Returns
        -------
        PerPositionMetric
            Metric source returning ``value`` for every selected position.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> PerPositionMetric.constant(1.5).kind
        'constant'
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> PerPositionMetric:
        """
        Deserialize a per-position metric expression from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized metric-source expression, normally produced by
            ``PerPositionMetric.to_json``.

        Returns
        -------
        PerPositionMetric
            Validated `PerPositionMetric` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PerPositionMetric
        >>> metric = PerPositionMetric.from_json(PerPositionMetric.pv_base().to_json())
        >>> metric.kind
        'pv_base'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this per-position metric expression to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PerPositionMetric`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def kind(self) -> str:
        """
        Metric-source variant label.
        Returns
        -------
        str
            The kind exposed by this `PerPositionMetric`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PositionFilter:
    """
    Declarative filter for selecting which positions a rule applies to.

    Examples
    --------
    >>> from finstack_quant.portfolio import PositionFilter
    >>> PositionFilter.all().kind
    'all'
    """

    @classmethod
    def all(cls) -> PositionFilter:
        """
        Select all positions.

        Returns
        -------
        PositionFilter
            Filter matching every portfolio position.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.all().kind
        'all'
        """
        ...

    @classmethod
    def by_entity_id(cls, entity_id: str) -> PositionFilter:
        """
        Select positions for one entity ID.

        Parameters
        ----------
        entity_id : str
            Entity identifier assigned to positions that should be selected.

        Returns
        -------
        PositionFilter
            Filter matching positions assigned to ``entity_id``.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.by_entity_id("issuer-1").kind
        'by_entity_id'
        """
        ...

    @classmethod
    def by_attribute(
        cls,
        key: str,
        op: str,
        text: str | None = None,
        number: float | None = None,
    ) -> PositionFilter:
        """
        Select positions by attribute comparison.

        Parameters
        ----------
        key : str
            Position attribute name to compare.
        op : str
            Comparison operator accepted by Rust, such as ``"eq"`` or ``"gte"``.
        text : str or None
            Optional string comparison value for text-valued attributes.
        number : float or None
            Optional numeric comparison value for numeric attributes.

        Returns
        -------
        PositionFilter
            Filter matching positions whose attribute satisfies the comparison.

        Raises
        ------
        ValueError
            If ``op`` is unknown, or exactly one of ``text`` and ``number`` is
            not supplied.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.by_attribute("sector", "eq", text="tech").kind
        'by_attribute'
        """
        ...

    @classmethod
    def by_position_ids(cls, position_ids: list[str]) -> PositionFilter:
        """
        Select positions by explicit position IDs.

        Parameters
        ----------
        position_ids : list[str]
            Explicit portfolio position identifiers to include in the filter.

        Returns
        -------
        PositionFilter
            Filter matching IDs contained in ``position_ids``.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.by_position_ids(["P1", "P2"]).kind
        'by_position_ids'
        """
        ...

    @classmethod
    def not_(cls, inner: PositionFilter) -> PositionFilter:
        """
        Negate another filter.

        Parameters
        ----------
        inner : PositionFilter
            Existing filter whose matching positions should be excluded.

        Returns
        -------
        PositionFilter
            Filter matching positions not matched by ``inner``.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.not_(PositionFilter.all()).kind
        'not'
        """
        ...

    @classmethod
    def and_(cls, filters: list[PositionFilter]) -> PositionFilter:
        """
        Select positions matching all child filters.

        Parameters
        ----------
        filters : list[PositionFilter]
            Child filters that every selected position must satisfy.

        Returns
        -------
        PositionFilter
            Filter matching positions that satisfy every child filter.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.and_([PositionFilter.all()]).kind
        'and'
        """
        ...

    @classmethod
    def or_(cls, filters: list[PositionFilter]) -> PositionFilter:
        """
        Select positions matching any child filter.

        Parameters
        ----------
        filters : list[PositionFilter]
            Child filters of which at least one must match each selected position.

        Returns
        -------
        PositionFilter
            Filter matching positions that satisfy at least one child filter.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> PositionFilter.or_([PositionFilter.all()]).kind
        'or'
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> PositionFilter:
        """
        Deserialize a position filter from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized filter tree, normally produced by
            ``PositionFilter.to_json``.

        Returns
        -------
        PositionFilter
            Validated `PositionFilter` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import PositionFilter
        >>> filter_ = PositionFilter.from_json(PositionFilter.all().to_json())
        >>> filter_.kind
        'all'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this position filter to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PositionFilter`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def kind(self) -> str:
        """
        Filter variant label.
        Returns
        -------
        str
            The kind exposed by this `PositionFilter`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class MetricExpr:
    """
    Portfolio-level metric expression.

    Examples
    --------
    >>> from finstack_quant.portfolio import MetricExpr, PerPositionMetric
    >>> MetricExpr.weighted_sum(PerPositionMetric.pv_base()).kind
    'weighted_sum'
    """

    @classmethod
    def weighted_sum(
        cls,
        metric: PerPositionMetric,
        filter: PositionFilter | None = None,
    ) -> MetricExpr:
        """
        Build a weighted-sum portfolio metric expression.

        Parameters
        ----------
        metric : PerPositionMetric
            Per-position value to multiply by each selected position weight.
        filter : PositionFilter or None
            Optional selector limiting the expression to matching positions.

        Returns
        -------
        MetricExpr
            Expression for ``sum_i(w_i * m_i)``, restricted by ``filter`` when supplied.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, PerPositionMetric
        >>> MetricExpr.weighted_sum(PerPositionMetric.pv_base()).kind
        'weighted_sum'
        """
        ...

    @classmethod
    def value_weighted_average(
        cls,
        metric: PerPositionMetric,
        filter: PositionFilter | None = None,
    ) -> MetricExpr:
        """
        Build a value-weighted-average portfolio metric expression.

        Parameters
        ----------
        metric : PerPositionMetric
            Per-position value to average using portfolio market-value weights.
        filter : PositionFilter or None
            Optional selector limiting the expression to matching positions.

        Returns
        -------
        MetricExpr
            Value-weighted average of the metric, restricted by ``filter`` when supplied.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, PerPositionMetric
        >>> MetricExpr.value_weighted_average(PerPositionMetric.pv_base()).kind
        'value_weighted_average'
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> MetricExpr:
        """
        Deserialize a metric expression from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized portfolio metric expression, normally produced
            by ``MetricExpr.to_json``.

        Returns
        -------
        MetricExpr
            Validated `MetricExpr` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, PerPositionMetric
        >>> original = MetricExpr.weighted_sum(PerPositionMetric.pv_base())
        >>> MetricExpr.from_json(original.to_json()).kind
        'weighted_sum'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this metric expression to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `MetricExpr`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def kind(self) -> str:
        """
        Metric-expression variant label.
        Returns
        -------
        str
            The kind exposed by this `MetricExpr`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class Objective:
    """
    Optimization direction and target.

    Examples
    --------
    >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric
    >>> Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base())).direction
    'maximize'
    """

    @classmethod
    def maximize(cls, expr: MetricExpr) -> Objective:
        """
        Maximize the supplied metric expression.

        Parameters
        ----------
        expr : MetricExpr
            Portfolio-level metric expression the optimizer should maximize.

        Returns
        -------
        Objective
            Objective directing the optimizer to maximize ``expr``.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric
        >>> expr = MetricExpr.weighted_sum(PerPositionMetric.pv_base())
        >>> Objective.maximize(expr).direction
        'maximize'
        """
        ...

    @classmethod
    def minimize(cls, expr: MetricExpr) -> Objective:
        """
        Minimize the supplied metric expression.

        Parameters
        ----------
        expr : MetricExpr
            Portfolio-level metric expression the optimizer should minimize.

        Returns
        -------
        Objective
            Objective directing the optimizer to minimize ``expr``.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric
        >>> expr = MetricExpr.weighted_sum(PerPositionMetric.pv_base())
        >>> Objective.minimize(expr).direction
        'minimize'
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> Objective:
        """
        Deserialize an optimization objective from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized objective direction and metric expression,
            normally produced by ``Objective.to_json``.

        Returns
        -------
        Objective
            Validated `Objective` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric
        >>> original = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
        >>> Objective.from_json(original.to_json()).direction
        'maximize'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this optimization objective to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `Objective`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def direction(self) -> str:
        """
        Optimization direction label.
        Returns
        -------
        str
            The direction exposed by this `Objective`.
        """
        ...

    @property
    def expr(self) -> MetricExpr:
        """
        Metric expression being optimized.
        Returns
        -------
        MetricExpr
            The expr exposed by this `Objective`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class Constraint:
    """
    Declarative constraint specification.

    Examples
    --------
    >>> from finstack_quant.portfolio import Constraint
    >>> Constraint.budget(1.0).kind
    'budget'
    """

    @classmethod
    def metric_bound(
        cls,
        metric: MetricExpr,
        op: Inequality,
        rhs: float,
        label: str | None = None,
    ) -> Constraint:
        """
        Constrain a metric expression against a right-hand side.

        Parameters
        ----------
        metric : MetricExpr
            Portfolio expression whose evaluated value is constrained.
        op : Inequality
            Less-than, greater-than, or equality operator for the bound.
        rhs : float
            Numeric right-hand-side bound in the metric expression's units.
        label : str or None
            Optional stable label for diagnostics, reporting, and dual values.

        Returns
        -------
        Constraint
            Constraint encoding ``metric op rhs`` with the optional diagnostic label.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint, Inequality, MetricExpr, PerPositionMetric
        >>> expr = MetricExpr.weighted_sum(PerPositionMetric.pv_base())
        >>> Constraint.metric_bound(expr, Inequality.le(), 1.0).kind
        'metric_bound'
        """
        ...

    @classmethod
    def weight_bounds(
        cls,
        filter: PositionFilter,
        min: float,
        max: float,
        label: str | None = None,
    ) -> Constraint:
        """
        Constrain selected position weights to a min/max interval.

        Parameters
        ----------
        filter : PositionFilter
            Selector identifying the positions subject to the weight interval.
        min : float
            Inclusive lower bound on each selected position's decimal weight.
        max : float
            Inclusive upper bound on each selected position's decimal weight.
        label : str or None
            Optional stable label for diagnostics, reporting, and dual values.

        Returns
        -------
        Constraint
            Inclusive ``[min, max]`` weight interval for positions matching ``filter``.

        Raises
        ------
        ValueError
            If ``min`` is greater than ``max``.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint, PositionFilter
        >>> Constraint.weight_bounds(PositionFilter.all(), 0.0, 1.0).kind
        'weight_bounds'
        """
        ...

    @classmethod
    def max_turnover(
        cls,
        max_turnover: float,
        label: str | None = None,
    ) -> Constraint:
        """
        Constrain total portfolio turnover.

        Parameters
        ----------
        max_turnover : float
            Maximum permitted aggregate turnover as a decimal weight fraction.
        label : str or None
            Optional stable label for diagnostics, reporting, and dual values.

        Returns
        -------
        Constraint
            Bound enforcing ``sum(abs(w_new - w_current)) <= max_turnover``.

        Raises
        ------
        ValueError
            If ``max_turnover`` is negative.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint
        >>> Constraint.max_turnover(0.10).kind
        'max_turnover'
        """
        ...

    @classmethod
    def budget(cls, rhs: float) -> Constraint:
        """
        Constrain total portfolio budget/weight to ``rhs``.

        Parameters
        ----------
        rhs : float
            Required total portfolio weight, normally ``1.0`` for a fully
            invested long-only budget.

        Returns
        -------
        Constraint
            Normalization constraint enforcing ``sum(w_i) == rhs``.

        Raises
        ------
        ValueError
            If ``rhs`` is non-finite or negative.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint
        >>> (Constraint.budget(1.0).kind, Constraint.budget(1.0).label)
        ('budget', 'budget')
        """
        ...

    @classmethod
    def exposure_limit(
        cls,
        key: str,
        value: str,
        max_share: float,
        label: str | None = None,
    ) -> Constraint:
        """
        Constrain maximum exposure share for an attribute key/value.

        Parameters
        ----------
        key : str
            Position attribute name used to define the exposure bucket.
        value : str
            Attribute value identifying the exposure bucket to cap.
        max_share : float
            Maximum decimal portfolio-weight share permitted in the bucket.
        label : str or None
            Optional stable label for diagnostics, reporting, and dual values.

        Returns
        -------
        Constraint
            Attribute-bucket constraint enforcing weighted exposure ``<= max_share``.

        Raises
        ------
        ValueError
            If ``max_share`` is outside ``[0, 1]`` or is non-finite.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint
        >>> Constraint.exposure_limit("sector", "tech", 0.20).kind
        'metric_bound'
        """
        ...

    @classmethod
    def exposure_minimum(
        cls,
        key: str,
        value: str,
        min_share: float,
        label: str | None = None,
    ) -> Constraint:
        """
        Constrain minimum exposure share for an attribute key/value.

        Parameters
        ----------
        key : str
            Position attribute name used to define the exposure bucket.
        value : str
            Attribute value identifying the exposure bucket to floor.
        min_share : float
            Minimum decimal portfolio-weight share required in the bucket.
        label : str or None
            Optional stable label for diagnostics, reporting, and dual values.

        Returns
        -------
        Constraint
            Attribute-bucket constraint enforcing weighted exposure ``>= min_share``.

        Raises
        ------
        ValueError
            If ``min_share`` is outside ``[0, 1]`` or is non-finite.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint
        >>> Constraint.exposure_minimum("sector", "tech", 0.10).kind
        'metric_bound'
        """
        ...

    def with_label(self, label: str) -> Constraint:
        """
        Return a copy with a human-readable label.

        Parameters
        ----------
        label : str
            Stable reader-facing name used in diagnostics and optimizer reports.

        Returns
        -------
        Constraint
            Copy carrying ``label``; budget constraints retain their fixed label.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> Constraint:
        """
        Deserialize an optimization constraint from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized constraint, normally produced by
            ``Constraint.to_json``.

        Returns
        -------
        Constraint
            Validated `Constraint` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import Constraint
        >>> original = Constraint.budget(1.0)
        >>> Constraint.from_json(original.to_json()).kind
        'budget'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this optimization constraint to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `Constraint`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def kind(self) -> str:
        """
        Constraint variant label.
        Returns
        -------
        str
            The kind exposed by this `Constraint`.
        """
        ...

    @property
    def label(self) -> str | None:
        """
        Optional human-readable label.
        Returns
        -------
        str or None
            The label exposed by this `Constraint`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class CandidatePosition:
    """
    Candidate instrument that could be added to the portfolio.

    Construction from Python is not yet supported (requires the instrument
    binding bridge). Returned by getters on :class:`TradeUniverse`.

    Examples
    --------
    >>> from finstack_quant.portfolio import CandidatePosition
    >>> try:
    ...     CandidatePosition()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.CandidatePosition' instances
    """

    @property
    def id(self) -> str:
        """
        Candidate position identifier.
        Returns
        -------
        str
            The id exposed by this `CandidatePosition`.
        """
        ...

    @property
    def entity_id(self) -> str:
        """
        Candidate entity identifier.
        Returns
        -------
        str
            The entity id exposed by this `CandidatePosition`.
        """
        ...

    @property
    def max_weight(self) -> float:
        """
        Maximum allowed candidate weight.
        Returns
        -------
        float
            The max weight exposed by this `CandidatePosition`.
        """
        ...

    @property
    def min_weight(self) -> float:
        """
        Minimum allowed candidate weight.
        Returns
        -------
        float
            The min weight exposed by this `CandidatePosition`.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Underlying instrument identifier.
        Returns
        -------
        str
            The instrument id exposed by this `CandidatePosition`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class TradeUniverse:
    """
    Universe of tradeable existing positions and candidate additions.

    Examples
    --------
    >>> from finstack_quant.portfolio import TradeUniverse
    >>> TradeUniverse.all_positions().tradeable_filter.kind
    'all'
    """

    @classmethod
    def all_positions(cls) -> TradeUniverse:
        """
        Make every existing position tradeable.

        Returns
        -------
        TradeUniverse
            Universe with all existing positions tradeable, no candidates, and no candidate shorts.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeUniverse
        >>> universe = TradeUniverse.all_positions()
        >>> (universe.tradeable_filter.kind, universe.candidates)
        ('all', [])
        """
        ...

    @property
    def tradeable_filter(self) -> PositionFilter:
        """
        Filter selecting tradeable positions.
        Returns
        -------
        PositionFilter
            The tradeable filter exposed by this `TradeUniverse`.
        """
        ...

    @property
    def held_filter(self) -> PositionFilter | None:
        """
        Optional filter selecting held positions.
        Returns
        -------
        PositionFilter or None
        """
        ...

    @property
    def candidates(self) -> list[CandidatePosition]:
        """
        Candidate new positions.
        Returns
        -------
        list[CandidatePosition]
        """
        ...

    @property
    def allow_short_candidates(self) -> bool:
        """
        Whether candidates may receive negative weights.
        Returns
        -------
        bool
            The allow short candidates exposed by this `TradeUniverse`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class OptimizationStatus:
    """
    Status of an optimization run.

    Examples
    --------
    >>> from finstack_quant.portfolio import OptimizationStatus
    >>> (OptimizationStatus.optimal().kind, OptimizationStatus.optimal().is_feasible)
    ('optimal', True)
    """

    @classmethod
    def optimal(cls) -> OptimizationStatus:
        """
        Successful optimal solve.

        Returns
        -------
        OptimizationStatus
            Feasible status indicating that the solver proved optimality.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> (OptimizationStatus.optimal().kind, OptimizationStatus.optimal().is_feasible)
        ('optimal', True)
        """
        ...

    @classmethod
    def feasible_but_suboptimal(cls) -> OptimizationStatus:
        """
        Feasible solution that did not prove optimality.

        Returns
        -------
        OptimizationStatus
            Usable status indicating feasibility without proven optimality.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> OptimizationStatus.feasible_but_suboptimal().kind
        'feasible_but_suboptimal'
        """
        ...

    @classmethod
    def unbounded(cls) -> OptimizationStatus:
        """
        Optimization problem is unbounded.

        Returns
        -------
        OptimizationStatus
            Non-feasible status indicating an unbounded objective.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> OptimizationStatus.unbounded().is_feasible
        False
        """
        ...

    @classmethod
    def infeasible(cls, conflicting_constraints: list[str]) -> OptimizationStatus:
        """
        Create an infeasible status with the listed conflicting constraints.

        Parameters
        ----------
        conflicting_constraints : list[str]
            Constraint labels implicated in the infeasibility diagnosis.

        Returns
        -------
        OptimizationStatus
            Non-feasible status retaining the conflicting constraint labels.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> status = OptimizationStatus.infeasible(["budget"])
        >>> status.conflicting_constraints
        ['budget']
        """
        ...

    @classmethod
    def error(cls, message: str) -> OptimizationStatus:
        """
        Create a solver or model-building error status.

        Parameters
        ----------
        message : str
            Reader-facing error or diagnostic message returned by the optimizer.

        Returns
        -------
        OptimizationStatus
            Non-feasible solver-error status retaining ``message`` for diagnostics.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> OptimizationStatus.error("solver failed").message
        'solver failed'
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> OptimizationStatus:
        """
        Deserialize an optimization status from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized solver status, normally produced by
            ``OptimizationStatus.to_json``.

        Returns
        -------
        OptimizationStatus
            Validated `OptimizationStatus` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import OptimizationStatus
        >>> original = OptimizationStatus.infeasible(["budget"])
        >>> OptimizationStatus.from_json(original.to_json()).conflicting_constraints
        ['budget']
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this optimization status to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `OptimizationStatus`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def kind(self) -> str:
        """
        Status variant label.
        Returns
        -------
        str
            The kind exposed by this `OptimizationStatus`.
        """
        ...

    @property
    def is_feasible(self) -> bool:
        """
        Whether the status includes a feasible solution.
        Returns
        -------
        bool
            Whether feasible holds for this `OptimizationStatus`.
        """
        ...

    @property
    def conflicting_constraints(self) -> list[str]:
        """
        Constraint labels implicated in infeasibility.
        Returns
        -------
        list[str]
            The conflicting constraints exposed by this `OptimizationStatus`.
        """
        ...

    @property
    def message(self) -> str | None:
        """
        Error or diagnostic message, if present.
        Returns
        -------
        str or None
            The message exposed by this `OptimizationStatus`.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class TradeSpec:
    """
    Trade specification for a single position.

    Examples
    --------
    >>> from finstack_quant.portfolio import TradeSpec
    >>> doc = (
    ...     '{"position_id":"P1","instrument_id":"I1","trade_type":"existing",'
    ...     '"direction":"buy","current_quantity":1.0,"target_quantity":2.0,'
    ...     '"delta_quantity":1.0,"current_weight":0.1,"target_weight":0.2}'
    ... )
    >>> TradeSpec.from_json(doc).delta_quantity
    1.0
    """

    @classmethod
    def from_json(cls, json_str: str) -> TradeSpec:
        """
        Deserialize a trade specification from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized trade recommendation, normally produced by
            ``TradeSpec.to_json``.

        Returns
        -------
        TradeSpec
            Validated `TradeSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import TradeSpec
        >>> doc = (
        ...     '{"position_id":"P1","instrument_id":"I1","trade_type":"existing",'
        ...     '"direction":"buy","current_quantity":1.0,"target_quantity":2.0,'
        ...     '"delta_quantity":1.0,"current_weight":0.1,"target_weight":0.2}'
        ... )
        >>> TradeSpec.from_json(doc).direction.label
        'buy'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this trade specification to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `TradeSpec`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def position_id(self) -> str:
        """
        Portfolio position identifier.
        Returns
        -------
        str
            The position id exposed by this `TradeSpec`.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Underlying instrument identifier.
        Returns
        -------
        str
            The instrument id exposed by this `TradeSpec`.
        """
        ...

    @property
    def trade_type(self) -> TradeType:
        """
        Whether the trade adjusts an existing position, opens a new one, or
        closes one out.
        Returns
        -------
        TradeType
            The trade type exposed by this `TradeSpec`.
        """
        ...

    @property
    def direction(self) -> TradeDirection:
        """
        Trade direction.
        Returns
        -------
        TradeDirection
            The direction exposed by this `TradeSpec`.
        """
        ...

    @property
    def current_quantity(self) -> float:
        """
        Current quantity.
        Returns
        -------
        float
            The current quantity exposed by this `TradeSpec`.
        """
        ...

    @property
    def target_quantity(self) -> float:
        """
        Target quantity.
        Returns
        -------
        float
            The target quantity exposed by this `TradeSpec`.
        """
        ...

    @property
    def delta_quantity(self) -> float:
        """
        Target quantity less current quantity.
        Returns
        -------
        float
            The delta quantity exposed by this `TradeSpec`.
        """
        ...

    @property
    def current_weight(self) -> float:
        """
        Current portfolio weight.
        Returns
        -------
        float
            The current weight exposed by this `TradeSpec`.
        """
        ...

    @property
    def target_weight(self) -> float:
        """
        Target portfolio weight, as a **fraction** rather than a percentage.
        Returns
        -------
        float
            The target weight exposed by this `TradeSpec`.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export this trade as a single-row pandas DataFrame.

        Columns: ``position_id``, ``instrument_id``, ``trade_type``,
        ``current_quantity``, ``target_quantity``, ``delta_quantity``,
        ``direction``, ``current_weight``, ``target_weight``.

        Returns
        -------
        pd.DataFrame
            Exactly one row, with the same schema
            :meth:`PortfolioOptimizationResult.to_trade_dataframe` emits, so
            single-trade frames concatenate cleanly onto a full trade list.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioOptimizationSpec:
    """
    JSON-serializable portfolio optimization specification.

    Examples
    --------
    >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric, PortfolioOptimizationSpec
    >>> objective = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
    >>> spec_json = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> PortfolioOptimizationSpec.new(spec_json, objective).objective.direction
    'maximize'
    """

    @classmethod
    def new(
        cls,
        portfolio_spec_json: str,
        objective: Objective,
    ) -> PortfolioOptimizationSpec:
        """
        Create an optimization specification from portfolio JSON and objective.

        Parameters
        ----------
        portfolio_spec_json : str
            Canonical ``PortfolioSpec`` JSON defining positions and starting weights.
        objective : Objective
            Direction and metric expression the optimizer should solve for.

        Returns
        -------
        PortfolioOptimizationSpec
            Spec with no constraints, value weighting, zero-fill missing metrics, and no label.

        Raises
        ------
        ValueError
            If ``portfolio_spec_json`` is malformed or does not match the
            ``PortfolioSpec`` schema.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric, PortfolioOptimizationSpec
        >>> objective = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
        >>> spec_json = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
        >>> PortfolioOptimizationSpec.new(spec_json, objective).weighting.label
        'value_weight'
        """
        ...

    def with_constraint(self, constraint: Constraint) -> PortfolioOptimizationSpec:
        """
        Return a copy with an additional constraint.

        Parameters
        ----------
        constraint : Constraint
            Constraint to append to the current optimization specification.

        Returns
        -------
        PortfolioOptimizationSpec
            Copy with ``constraint`` appended after the existing constraints.
        """
        ...

    def with_objective(self, objective: Objective) -> PortfolioOptimizationSpec:
        """
        Return a copy with a replacement objective.

        Parameters
        ----------
        objective : Objective
            New direction and metric expression that replaces the existing one.

        Returns
        -------
        PortfolioOptimizationSpec
            Copy with the existing objective replaced by ``objective``.
        """
        ...

    def with_weighting(self, weighting: WeightingScheme) -> PortfolioOptimizationSpec:
        """
        Return a copy with a replacement weighting scheme.

        Parameters
        ----------
        weighting : WeightingScheme
            Market-value or notional convention used to calculate portfolio weights.

        Returns
        -------
        PortfolioOptimizationSpec
            Copy with the existing weighting scheme replaced by ``weighting``.
        """
        ...

    def with_missing_metric_policy(self, policy: MissingMetricPolicy) -> PortfolioOptimizationSpec:
        """
        Return a copy with a replacement missing-metric policy.

        Parameters
        ----------
        policy : MissingMetricPolicy
            Policy defining how positions without required metric data are handled.

        Returns
        -------
        PortfolioOptimizationSpec
            Copy with the existing missing-metric policy replaced by ``policy``.
        """
        ...

    def with_label(self, label: str) -> PortfolioOptimizationSpec:
        """
        Return a copy with a human-readable label.

        Parameters
        ----------
        label : str
            Stable reader-facing name for reports, diagnostics, and persistence.

        Returns
        -------
        PortfolioOptimizationSpec
            Copy with its audit and reporting label set to ``label``.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> PortfolioOptimizationSpec:
        """
        Deserialize an optimization specification from canonical JSON.

        Parameters
        ----------
        json_str : str
            Canonical serialized portfolio optimization specification, normally
            produced by ``PortfolioOptimizationSpec.to_json``.

        Returns
        -------
        PortfolioOptimizationSpec
            Validated `PortfolioOptimizationSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If the payload is malformed or cannot be deserialized into the documented portfolio type.

        Examples
        --------
        >>> from finstack_quant.portfolio import MetricExpr, Objective, PerPositionMetric, PortfolioOptimizationSpec
        >>> objective = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
        >>> spec_json = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
        >>> original = PortfolioOptimizationSpec.new(spec_json, objective)
        >>> PortfolioOptimizationSpec.from_json(original.to_json()).objective.direction
        'maximize'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this optimization specification to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioOptimizationSpec`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def objective(self) -> Objective:
        """
        Optimization objective.
        Returns
        -------
        Objective
            The objective exposed by this `PortfolioOptimizationSpec`.
        """
        ...

    @property
    def constraints(self) -> list[Constraint]:
        """
        Optimization constraints.
        Returns
        -------
        list[Constraint]
            The constraints exposed by this `PortfolioOptimizationSpec`.
        """
        ...

    @property
    def weighting(self) -> WeightingScheme:
        """
        Weighting scheme used by the optimization.
        Returns
        -------
        WeightingScheme
            The weighting exposed by this `PortfolioOptimizationSpec`.
        """
        ...

    @property
    def missing_metric_policy(self) -> MissingMetricPolicy:
        """
        Policy for missing per-position metrics.
        Returns
        -------
        MissingMetricPolicy
        """
        ...

    @property
    def label(self) -> str | None:
        """
        Optional human-readable label.
        Returns
        -------
        str or None
            The label exposed by this `PortfolioOptimizationSpec`.
        """
        ...

    def portfolio_spec_json(self) -> str:
        """
        Return the embedded portfolio specification JSON.
        Returns
        -------
        str
            Compact canonical JSON for the embedded ``PortfolioSpec``.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class PortfolioOptimizationResult:
    """
    Result of an optimization run (Serialize-only; no ``from_json``).

    Examples
    --------
    >>> from finstack_quant.portfolio import PortfolioOptimizationResult
    >>> try:
    ...     PortfolioOptimizationResult()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.PortfolioOptimizationResult' instances
    """

    def to_json(self) -> str:
        """
        Serialize this optimization result to JSON.
        Returns
        -------
        str
            Canonical JSON representation of this `PortfolioOptimizationResult`, suitable for a matching `from_json` call.
        """
        ...

    @property
    def status(self) -> OptimizationStatus:
        """
        Optimization status.
        Returns
        -------
        OptimizationStatus
        """
        ...

    @property
    def is_feasible(self) -> bool:
        """
        Whether the solver returned a feasible portfolio.
        Returns
        -------
        bool
            Whether feasible holds for this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def objective_value(self) -> float:
        """
        Objective value at the solution.
        Returns
        -------
        float
            The objective value exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def current_weights(self) -> dict[str, float]:
        """
        Current weights by position ID.
        Returns
        -------
        dict[str, float]
            The current weights exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def optimal_weights(self) -> dict[str, float]:
        """
        Optimized target weights by position ID.
        Returns
        -------
        dict[str, float]
            The optimal weights exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def weight_deltas(self) -> dict[str, float]:
        """
        Target less current weight by position ID.
        Returns
        -------
        dict[str, float]
            The weight deltas exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def implied_quantities(self) -> dict[str, float]:
        """
        Implied target quantities by position ID.
        Returns
        -------
        dict[str, float]
            The implied quantities exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def metric_values(self) -> dict[str, float]:
        """
        Portfolio metric values at the solution.
        Returns
        -------
        dict[str, float]
            The metric values exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def constraint_slacks(self) -> dict[str, float]:
        """
        Constraint slack values by constraint label.
        Returns
        -------
        dict[str, float]
            The constraint slacks exposed by this `PortfolioOptimizationResult`.
        """
        ...

    @property
    def turnover(self) -> float:
        """
        Total turnover implied by the solution.
        Returns
        -------
        float
            The turnover exposed by this `PortfolioOptimizationResult`.
        """
        ...

    def to_trade_list(self) -> list[TradeSpec]:
        """
        Convert weight deltas into trade specifications.
        Returns
        -------
        list[TradeSpec]
            Material trades sorted by descending absolute quantity change.
        """
        ...

    def new_position_trades(self) -> list[TradeSpec]:
        """
        Return trade specs for new candidate positions only.
        Returns
        -------
        list[TradeSpec]
            New-candidate subset of ``to_trade_list()``, preserving its order.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-position weight and quantity solution, indexed by position.

        ``current_weights``, ``optimal_weights``, ``weight_deltas`` and
        ``implied_quantities`` share one position key space, so they are
        joined into a single frame. The row axis is the union of their keys,
        ordered by first appearance (``current_weights`` first); a value
        missing from one map becomes ``None`` in that column — this is how
        candidate positions with no current weight show up.

        Columns: ``current_weight``, ``optimal_weight``, ``weight_delta`` (all
        fractions of the portfolio, not percentages), ``implied_quantity``
        (units / face / notional per the weighting scheme).

        Returns
        -------
        pd.DataFrame
            One row per position; the index holds the position ids.
        """
        ...

    def to_trade_dataframe(self) -> pd.DataFrame:
        """
        Export :meth:`to_trade_list` as a pandas DataFrame.

        One row per trade, in the same order as :meth:`to_trade_list` (sorted
        by absolute quantity delta, largest first).

        Columns: ``position_id``, ``instrument_id``, ``trade_type``
        (``"existing"``, ``"new_position"``, ``"close_out"``),
        ``current_quantity``, ``target_quantity``, ``delta_quantity``,
        ``direction`` (``"buy"``, ``"sell"``, ``"hold"``), ``current_weight``,
        ``target_weight`` (weights are fractions, not percentages).

        Returns
        -------
        pd.DataFrame
            One row per trade; a solution that requires no trades yields a
            zero-row frame that still carries the column schema.
        """
        ...

    def binding_constraints(self) -> list[tuple[str, float]]:
        """
        Return constraints with near-zero slack as ``(label, slack)`` pairs.
        Returns
        -------
        list[tuple[str, float]]
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

def optimize_portfolio(
    spec: PortfolioOptimizationSpec,
    market: MarketContext | str,
) -> PortfolioOptimizationResult:
    """
    Optimize portfolio weights using the LP-based optimizer.

    Parameters
    ----------
    spec : PortfolioOptimizationSpec
        Typed portfolio definition, objective, constraints, and solver policy.
    market : MarketContext or str
        Market context object or JSON used to value positions and calculate metrics.

    Returns
    -------
    PortfolioOptimizationResult
        Typed optimal weights, trades, constraint values, and diagnostics.

    Raises
    ------
    TypeError
        If ``market`` is neither a ``MarketContext`` nor a JSON string.
    ValueError
        If supplied market JSON is malformed or schema-incompatible.
    PortfolioError
        If the embedded portfolio specification cannot be constructed.
    FinstackValuationError
        If a required position metric cannot be valued.
    FinstackFxError
        If a required base-currency conversion is unavailable.
    FinstackOptimizationError
        If the optimization problem or solver fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import (
    ...     MetricExpr,
    ...     Objective,
    ...     PerPositionMetric,
    ...     PortfolioError,
    ...     PortfolioOptimizationSpec,
    ...     optimize_portfolio,
    ... )
    >>> objective = Objective.maximize(MetricExpr.weighted_sum(PerPositionMetric.pv_base()))
    >>> portfolio = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
    >>> spec = PortfolioOptimizationSpec.new(portfolio, objective)
    >>> try:
    ...     optimize_portfolio(spec, MarketContext())
    ... except PortfolioError as exc:
    ...     print("no decision variables" in str(exc))
    True
    """
    ...

# Factor Sensitivity

class SensitivityMatrix:
    """
    Positions-by-factors sensitivity matrix.

    Each element ``(i, j)`` is the first-order sensitivity of position *i* to
    factor *j*, denominated in the factor's bump units (e.g. PV change per 1 bp
    for a rates factor).

    Construct via :func:`compute_factor_sensitivities`.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import compute_factor_sensitivities
    >>> matrix = compute_factor_sensitivities("[]", "[]", MarketContext(), "2025-01-15")
    >>> (matrix.n_positions, matrix.n_factors)
    (0, 0)
    """

    @property
    def position_ids(self) -> list[str]:
        """
        Ordered position identifiers (row axis).
        Returns
        -------
        list[str]
            The position ids exposed by this `SensitivityMatrix`.
        """
        ...

    @property
    def factor_ids(self) -> list[str]:
        """
        Ordered factor identifiers (column axis).
        Returns
        -------
        list[str]
            The factor ids exposed by this `SensitivityMatrix`.
        """
        ...

    @property
    def n_positions(self) -> int:
        """
        Number of positions (rows).
        Returns
        -------
        int
            The n positions exposed by this `SensitivityMatrix`.
        """
        ...

    @property
    def n_factors(self) -> int:
        """
        Number of factors (columns).
        Returns
        -------
        int
            The n factors exposed by this `SensitivityMatrix`.
        """
        ...

    def delta(self, position_idx: int, factor_idx: int) -> float:
        """
        Read a single sensitivity element.

        Parameters
        ----------
        position_idx : int
            Zero-based row index into ``position_ids`` for the requested position.
        factor_idx : int
            Column index.

        Returns
        -------
        float
            Sensitivity value.

        Raises
        ------
        ValueError
            If ``position_idx`` or ``factor_idx`` is outside the matrix bounds.
        """
        ...

    def position_deltas(self, position_idx: int) -> list[float]:
        """
        Sensitivity row for a single position across all factors.

        Parameters
        ----------
        position_idx : int
            Zero-based row index into ``position_ids`` for the requested position.

        Returns
        -------
        list[float]
            List of delta values, one per factor.

        Raises
        ------
        ValueError
            If ``position_idx`` is outside the matrix bounds.
        """
        ...

    def factor_deltas(self, factor_idx: int) -> list[float]:
        """
        Sensitivity column for a single factor across all positions.

        Parameters
        ----------
        factor_idx : int
            Column index.

        Returns
        -------
        list[float]
            List of delta values, one per position.

        Raises
        ------
        ValueError
            If ``factor_idx`` is outside the matrix bounds.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a pandas DataFrame with positions as rows and factors as columns.

        Returns
        -------
        pd.DataFrame
            DataFrame indexed by position IDs with factor IDs as column names.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

class FactorPnlProfile:
    """
    P&L profile for one factor across a scenario grid.

    Each profile captures the hypothetical P&L for every position at each
    scenario shift, enabling non-linear (gamma, convexity) analysis.

    Construct via :func:`compute_pnl_profiles`.

    Examples
    --------
    >>> from finstack_quant.portfolio import FactorPnlProfile
    >>> try:
    ...     FactorPnlProfile()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.portfolio.FactorPnlProfile' instances
    """

    @property
    def factor_id(self) -> str:
        """
        Factor identifier.
        Returns
        -------
        str
            The factor id exposed by this `FactorPnlProfile`.
        """
        ...

    @property
    def shifts(self) -> list[float]:
        """
        Scenario shift coordinates (bump-size multiples).
        Returns
        -------
        list[float]
            The shifts exposed by this `FactorPnlProfile`.
        """
        ...

    @property
    def position_pnls(self) -> list[list[float]]:
        """
        Per-shift P&L vectors indexed as ``[shift_idx][position_idx]``.
        Returns
        -------
        list[list[float]]
            The position pnls exposed by this `FactorPnlProfile`.
        """
        ...

    def to_dataframe(self, position_ids: list[str]) -> pd.DataFrame:
        """
        Export as a pandas DataFrame with shifts as rows and positions as columns.

        Parameters
        ----------
        position_ids : list[str]
            Position identifiers to use as column names.  Must
            match the number of positions in the profile.

        Returns
        -------
        pd.DataFrame
            DataFrame indexed by shift values with position IDs as column names.

        Raises
        ------
        ValueError
            If ``len(position_ids)`` does not match the profile width.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

def compute_factor_sensitivities(
    positions_json: str,
    factors_json: str,
    market: MarketContext | str,
    as_of: str,
    bump_config_json: str | None = None,
) -> SensitivityMatrix:
    """
        Compute first-order factor sensitivities using central finite differences.

        Parameters
        ----------
        positions_json : str
            JSON array of position objects, each with ``id`` (str),
    ``instrument`` (canonical v1 instrument envelope), and ``weight`` (float).
        factors_json : str
            JSON array of ``FactorDefinition`` objects.
        market : MarketContext or str
            ``MarketContext`` instance or JSON string.
        as_of : str
            Valuation date in ISO 8601 format.
        bump_config_json : str, optional
            Optional JSON-serialized ``BumpSizeConfig``.
            Defaults to 1 bp / 1 % per factor type.

        Returns
        -------
        SensitivityMatrix
            Positions-by-factors delta matrix.

        Raises
        ------
        TypeError
            If ``market`` is neither a ``MarketContext`` nor a JSON string.
        ValueError
            If position, factor, market, bump-configuration, or date input is
            malformed or invalid, or a factor cannot be bumped or priced.
        KeyError
            If repricing requires market data that is absent from ``market``.

        Examples
        --------
        >>> from finstack_quant.core.market_data import MarketContext
        >>> from finstack_quant.portfolio import compute_factor_sensitivities
        >>> matrix = compute_factor_sensitivities("[]", "[]", MarketContext(), "2025-01-15")
        >>> matrix.to_dataframe().shape
        (0, 0)
    """
    ...

def compute_pnl_profiles(
    positions_json: str,
    factors_json: str,
    market: MarketContext | str,
    as_of: str,
    bump_config_json: str | None = None,
    n_scenario_points: int = 5,
) -> list[FactorPnlProfile]:
    """
    Compute scenario P&L profiles via full repricing across a factor grid.

    Parameters
    ----------
    positions_json : str
        JSON array of position objects (same schema as
        :func:`compute_factor_sensitivities`).
    factors_json : str
        JSON array of ``FactorDefinition`` objects.
    market : MarketContext or str
        ``MarketContext`` instance or JSON string.
    as_of : str
        Valuation date in ISO 8601 format.
    bump_config_json : str, optional
        Optional JSON-serialized ``BumpSizeConfig``.
    n_scenario_points : int, default 5
        Number of scenario grid points
        (default 5 produces shifts ``[-2, -1, 0, 1, 2]``).

    Returns
    -------
    list[FactorPnlProfile]
        One profile per factor, each containing scenario P&L for every position.

    Raises
    ------
    TypeError
        If ``market`` is neither a ``MarketContext`` nor a JSON string.
    ValueError
        If position, factor, market, bump-configuration, or date input is
        malformed or invalid; if ``n_scenario_points`` is less than ``3`` or
        even; or if a factor cannot be bumped or priced.
    KeyError
        If repricing requires market data that is absent from ``market``.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import compute_pnl_profiles
    >>> compute_pnl_profiles("[]", "[]", MarketContext(), "2025-01-15")
    []
    """
    ...

# Risk Decomposition

class FactorRiskDecomposition:
    """
    Portfolio-level decomposition of total risk across factors and positions.

    Obtain via :func:`decompose_factor_risk`.  The decomposition expresses
    forecasted portfolio risk (variance, volatility, VaR, or ES) as a sum of
    Euler-allocated factor-level contributions, each drillable to per-position
    detail.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import compute_factor_sensitivities, decompose_factor_risk
    >>> matrix = compute_factor_sensitivities("[]", "[]", MarketContext(), "2025-01-15")
    >>> decompose_factor_risk(matrix, '{"factor_ids":[],"n":0,"data":[]}').total_risk
    0.0
    """

    @property
    def total_risk(self) -> float:
        """
        Total portfolio risk under the selected measure.
        Returns
        -------
        float
            The total risk exposed by this `FactorRiskDecomposition`.
        """
        ...

    @property
    def measure(self) -> str:
        """
        Risk measure used (e.g. ``"Variance"``, ``"Volatility"``).
        Returns
        -------
        str
            The measure exposed by this `FactorRiskDecomposition`.
        """
        ...

    @property
    def residual_risk(self) -> float:
        """
        Residual (idiosyncratic) risk not attributed to any factor.
        Returns
        -------
        float
            The residual risk exposed by this `FactorRiskDecomposition`.
        """
        ...

    def factor_contributions(self) -> list[dict[str, object]]:
        """
        Factor-level contributions as a list of dicts.

        Each dict contains ``factor_id``, ``absolute_risk``, ``relative_risk``,
        and ``marginal_risk``.

        Returns
        -------
        list[dict[str, object]]
            List of per-factor contribution dicts.
        """
        ...

    def position_factor_contributions(self) -> list[dict[str, object]]:
        """
        Position x factor contributions as a list of dicts.

        Each dict contains ``position_id``, ``factor_id``, and
        ``risk_contribution``.

        Returns
        -------
        list[dict[str, object]]
            List of per-position, per-factor contribution dicts.
        """
        ...

    def to_factor_dataframe(self) -> pd.DataFrame:
        """
        Export factor contributions as a pandas DataFrame.

        Columns: ``factor_id``, ``absolute_risk``, ``relative_risk``,
        ``marginal_risk``.

        Returns
        -------
        pd.DataFrame
            DataFrame with one row per factor.
        """
        ...

    def to_position_factor_dataframe(self) -> pd.DataFrame:
        """
        Export position x factor contributions as a pandas DataFrame.

        Columns: ``position_id``, ``factor_id``, ``risk_contribution``.

        Returns
        -------
        pd.DataFrame
            DataFrame with one row per position-factor pair.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

def decompose_factor_risk(
    sensitivities: SensitivityMatrix,
    covariance_json: str,
    risk_measure: str | dict[str, Any] | None = None,
) -> FactorRiskDecomposition:
    """
    Decompose portfolio risk into factor and position contributions.

    Uses the parametric (covariance-based) Euler decomposition to attribute
    forecasted portfolio risk across factors and individual positions.

    Parameters
    ----------
    sensitivities : SensitivityMatrix
        Weighted position x factor sensitivity matrix, as
        returned by :func:`compute_factor_sensitivities`.
    covariance_json : str
        JSON-serialized ``FactorCovarianceMatrix``.  Must use
        the same factor IDs and ordering as the sensitivity matrix.
    risk_measure : str or dict[str, Any] or None, optional
        Risk measure.  Defaults to ``"variance"``.
        Accepts Python strings (``"variance"``, ``"volatility"``) or dicts
        (``{"var": {"confidence": 0.99}}``,
        ``{"expected_shortfall": {"confidence": 0.975}}``).

    Returns
    -------
    FactorRiskDecomposition
        Portfolio-level risk decomposition with factor and position detail.

    Raises
    ------
    ValueError
        If factor axes do not match or the covariance matrix is
        invalid.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import compute_factor_sensitivities, decompose_factor_risk
    >>> matrix = compute_factor_sensitivities("[]", "[]", MarketContext(), "2025-01-15")
    >>> result = decompose_factor_risk(matrix, '{"factor_ids":[],"n":0,"data":[]}', "volatility")
    >>> (result.total_risk, result.to_factor_dataframe().shape)
    (0.0, (0, 4))
    """
    ...
