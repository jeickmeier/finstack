"""
Financial statement modeling: builders, evaluators, forecasts, DSL, adjustments.

Python bindings for the ``finstack-quant-statements`` Rust crate: model specifications,
``ModelBuilder``, ``Evaluator``, formula parsing/validation, and EBITDA-style
normalization helpers.

Examples
--------
>>> from finstack_quant.statements import NodeId
>>> NodeId("revenue").as_str()
'revenue'

"""

from __future__ import annotations

from datetime import date
from typing import Literal

import pandas as pd

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import BusinessDayConvention, DayCount, Tenor
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.table import ArrowTable
from finstack_quant.statements import schema as schema

__all__ = [
    "schema",
    "ForecastMethod",
    "ForecastSpec",
    "NodeType",
    "NodeId",
    "NumericMode",
    "FinancialModelSpec",
    "ModelBuilder",
    "MixedNodeBuilder",
    "MetricRegistry",
    "MonteCarloConfig",
    "MonteCarloResults",
    "StatementResult",
    "Evaluator",
    "parse_formula",
    "run_monte_carlo",
    "validate_formula",
    "NormalizationConfig",
    "normalize",
    "CheckSuiteSpec",
    "CheckReport",
    "EcfSweepSpec",
    "PikToggleSpec",
    "WaterfallSpec",
]

class MonteCarloConfig:
    """
    Set reproducible Monte Carlo sampling and retained-output options.

    Parameters
    ----------
    n_paths : int
        Number of stochastic paths to simulate; larger values improve sampling
        precision at greater runtime and memory cost.
    seed : int
        Deterministic random-number seed used to reproduce the simulation.
    percentiles : list[float] or None
        Requested percentile levels as decimal probabilities, such as ``0.95``;
        ``None`` uses the engine defaults.
    include_path_data : bool
        Whether to retain individual path data in addition to summary outputs.

    Examples
    --------
    >>> from finstack_quant.statements import MonteCarloConfig
    >>> config = MonteCarloConfig(4, 7, [0.5])
    >>> (config.n_paths, config.seed, config.percentiles)
    (4, 7, [0.5])

    """
    def __init__(
        self,
        n_paths: int,
        seed: int,
        percentiles: list[float] | None = ...,
        include_path_data: bool = ...,
    ) -> None:
        """
        Configure a Monte Carlo run over a statement model's forecast periods.

        Parameters
        ----------
        n_paths : int
            Number of simulated paths. Percentile standard error scales as
            ``1 / sqrt(n_paths)``, so tails need proportionally more paths:
            roughly 1 000–2 000 for means/medians, 5 000–10 000 for the 5th
            and 95th percentiles, and 10 000+ for 1st/99th percentiles, CVaR,
            or breach probabilities. No minimum is imposed.
        seed : int
            Base seed from which per-path and per-node seeds are derived. The
            same seed reproduces the run exactly, in serial and in parallel.
        percentiles : list[float] or None, default None
            Percentiles to compute, each a **decimal fraction in [0, 1]** —
            ``0.05`` is the 5th percentile, not ``5``. Values are sorted and
            deduplicated; out-of-range values raise. ``None`` uses the engine
            default ``[0.05, 0.5, 0.95]``.
        include_path_data : bool, default False
            Whether to retain the full per-path long table. Off by default
            because it grows as ``n_paths * metrics * forecast periods``.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @staticmethod
    def from_json(json: str) -> MonteCarloConfig:
        """
        Deserialize Monte Carlo sampling settings from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload describing paths, seed, percentile levels, and output
            retention.

        Returns
        -------
        MonteCarloConfig
            Validated configuration reconstructed from canonical JSON.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates the serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements import MonteCarloConfig
        >>> config = MonteCarloConfig(4, 7, [0.5])
        >>> MonteCarloConfig.from_json(config.to_json()).n_paths
        4

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this configuration to canonical JSON.

        Returns
        -------
        str
            Canonical JSON accepted by :meth:`from_json` and
            :func:`run_monte_carlo`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def n_paths(self) -> int:
        """
        Number of paths the simulation will draw.

        Returns
        -------
        int
            Configured stochastic path count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def seed(self) -> int:
        """
        Base seed from which per-path and per-node seeds are derived.

        Reusing a seed reproduces the run bit-for-bit; serial and parallel
        execution agree.

        Returns
        -------
        int
            Seed used to reproduce path sampling.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def percentiles(self) -> list[float]:
        """
        Percentiles to compute, as decimal fractions in [0, 1].

        Returns
        -------
        list[float]
            Sorted, deduplicated quantile levels — ``0.05`` is the 5th
            percentile, not ``5``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def include_path_data(self) -> bool:
        """
        Whether the run retains the full per-path table alongside percentiles.

        Returns
        -------
        bool
            ``True`` when the result includes path-level data, reachable via
            :meth:`MonteCarloResults.to_paths_dataframe`.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class MonteCarloResults:
    """
    Hold percentile summaries from statement-model Monte Carlo evaluation.

    Examples
    --------
    >>> from finstack_quant.statements import MonteCarloResults
    >>> payload = '{"percentile_results":{},"n_paths":2,"percentiles":[0.5],"forecast_periods":[],"path_data":null}'
    >>> MonteCarloResults.from_json(payload).n_paths
    2

    """

    @staticmethod
    def from_json(json: str) -> MonteCarloResults:
        """
        Deserialize Monte Carlo output from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload returned by :func:`run_monte_carlo` or a matching
            serialized result.

        Returns
        -------
        MonteCarloResults
            Validated result reconstructed from canonical JSON.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates the serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements import MonteCarloResults
        >>> payload = '{"percentile_results":{},"n_paths":2,"percentiles":[0.5],"forecast_periods":[],"path_data":null}'
        >>> MonteCarloResults.from_json(payload).forecast_periods
        []

        """
        ...

    def to_json(self) -> str:
        """
        Serialize these results to canonical JSON.

        Returns
        -------
        str
            Canonical JSON accepted by :meth:`from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def n_paths(self) -> int:
        """
        Number of paths actually simulated.

        The aggregator fails the run unless this equals the configured
        ``n_paths``, so a partial simulation never reaches a result.

        Returns
        -------
        int
            Path count represented by the summaries.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def percentiles(self) -> list[float]:
        """
        Percentiles computed for every metric and period.

        Returns
        -------
        list[float]
            Sorted, deduplicated quantile levels as decimal fractions in
            [0, 1] — ``0.5`` is the median.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def forecast_periods(self) -> list[str]:
        """
        Forecast periods covered by the simulation, in evaluation order.

        Only non-actual periods are simulated, so historical periods are absent
        from this list and from every percentile series.

        Returns
        -------
        list[str]
            Period identifier strings (e.g. ``"2025Q1"``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def percentile_by_period(self, metric: str, percentile: float) -> dict[str, float] | None:
        """
        Look up one percentile of one metric as a period-keyed dict.

        Parameters
        ----------
        metric : str
            Node identifier whose simulated distribution to read.
        percentile : float
            Quantile as a decimal fraction in [0, 1] (``0.95`` for the 95th
            percentile). It must match a configured percentile to within
            1e-12; nothing is interpolated between configured levels.

        Returns
        -------
        dict[str, float] or None
            Period identifier → percentile value, in forecast-period order.
            ``None`` when the metric is unknown or the percentile was not
            configured for this run.

        Notes
        -----
        This method does not raise; unknown metrics or unconfigured percentiles
        return ``None``.
        """
        ...

    def to_dataframe(self, metric: str) -> pd.DataFrame:
        """
        Export one metric's percentile fan as a pandas ``DataFrame``.

        Rows are the metric's forecast periods (index name ``period``, in
        evaluation order); each column holds one configured percentile of the
        simulated distribution, in the metric's own units (currency amounts for
        monetary nodes, unitless otherwise).

        Columns: one per configured percentile, named ``p<q>`` where ``q`` is
        the quantile as a decimal fraction — ``p0.05``, ``p0.5``, ``p0.95`` for
        the default levels. A cell is ``NaN`` when a period is missing that
        percentile.

        Parameters
        ----------
        metric : str
            Node identifier whose fan chart to build.

        Returns
        -------
        pd.DataFrame
            Percentile-by-period frame indexed by period identifier.

        Raises
        ------
        KeyError
            If ``metric`` was not simulated.

        """
        ...

    def to_paths_dataframe(self) -> pd.DataFrame:
        """
        Export the retained per-path values as a long-format ``DataFrame``.

        Populated only when the run was configured with
        ``include_path_data=True``; otherwise the frame is empty but still
        carries the documented columns. Rows are ordered canonically by
        ``(path_id, metric, period)`` so serial and parallel runs match.

        Columns: ``path_id`` (0-based path index), ``period`` (period
        identifier string), ``metric`` (node identifier), ``value`` (float64,
        in the metric's own units).

        Returns
        -------
        pd.DataFrame
            Long-format per-path frame.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

def run_monte_carlo(
    model: FinancialModelSpec | str,
    config: MonteCarloConfig | str,
) -> MonteCarloResults:
    """
    Run Monte Carlo simulation on a financial model.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Typed model or its canonical JSON serialization.
    config : MonteCarloConfig or str
        Typed sampling configuration or its canonical JSON serialization.

    Returns
    -------
    MonteCarloResults
        Percentile summaries and optional retained path data.

    Raises
    ------
    ValueError
        If either JSON payload is malformed or evaluation rejects the model or
        configuration.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder, MonteCarloConfig, run_monte_carlo
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q2", "2025Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0), ("2025Q2", 110.0)])
    >>> run_monte_carlo(builder.build(), MonteCarloConfig(2, 7, [0.5])).n_paths
    2

    """
    ...

class ForecastMethod:
    """
    Available forecast methods for projecting node values.

    Immutable, hashable enum-style type. Construct variants via static
    factory methods (e.g. ``growth_pct()``).

    Examples
    --------
    >>> from finstack_quant.statements import ForecastMethod
    >>> ForecastMethod.forward_fill() == ForecastMethod.forward_fill()
    True

    """

    @staticmethod
    def forward_fill() -> ForecastMethod:
        """
        Carry the last observed value forward unchanged: ``v[t] = v[t-1]``.

        Takes no parameters and never changes the level, so it is the safe
        default for stock-like nodes (balances) that should hold flat when no
        explicit assumption is supplied.

        Returns
        -------
        ForecastMethod
            Forward-fill forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.forward_fill() == ForecastMethod.forward_fill()
        True
        """
        ...

    @staticmethod
    def growth_pct() -> ForecastMethod:
        """
        Compound growth at a single rate: ``v[t] = v[t-1] * (1 + rate)``.

        ``rate`` is a **decimal fraction per period**, not a percentage — 0.05
        means 5% growth per period, and the period is whatever cadence the
        model's period list uses (quarters, months, years). Rates above 100%
        per period raise an evaluation warning.

        Returns
        -------
        ForecastMethod
            Growth-percentage forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.growth_pct() == ForecastMethod.growth_pct()
        True
        """
        ...

    @staticmethod
    def curve_pct() -> ForecastMethod:
        """
        Period-specific compound growth: ``v[t] = v[t-1] * (1 + curve[t])``.

        ``curve`` supplies one growth rate per forecast period, each a decimal
        fraction per period (0.05 = 5%), consumed in forecast-period order.

        Returns
        -------
        ForecastMethod
            Curve-percentage forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.curve_pct() == ForecastMethod.curve_pct()
        True
        """
        ...

    @staticmethod
    def normal() -> ForecastMethod:
        """
        Additive normal random walk: ``v[t] = v[t-1] + mean + std_dev * z[t]``.

        ``mean`` and ``std_dev`` are per-period **level** quantities in the
        node's own units (currency amounts for ``Money`` nodes, unitless for
        scalar nodes), not rates. ``z[t]`` is a deterministic standard-normal
        draw derived from the configured seed, so runs are reproducible.
        ``std_dev`` must be non-negative.

        Returns
        -------
        ForecastMethod
            Normal distribution forecast method.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.normal() == ForecastMethod.normal()
        True
        """
        ...

    @staticmethod
    def log_normal() -> ForecastMethod:
        """
        Multiplicative log-normal path:
        ``v[t] = v[t-1] * exp(mean - 0.5 * std_dev**2 + std_dev * z[t])``.

        ``mean`` and ``std_dev`` are the per-period log-return drift and
        volatility as decimal fractions (0.02 = 2% log drift per period). The
        ``-0.5 * std_dev**2`` term is the standard log-normal drift adjustment,
        so ``mean`` is the expected *log*-return, not the expected simple
        return. When the base value is zero the path falls back to independent
        ``exp(mean + std_dev * z[t])`` draws.

        Returns
        -------
        ForecastMethod
            Log-normal forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.log_normal() == ForecastMethod.log_normal()
        True
        """
        ...

    @staticmethod
    def override_method() -> ForecastMethod:
        """
        Explicit per-period overrides, forward-filling the periods in between.

        The method reads an ``overrides`` parameter mapping period-identifier
        strings (e.g. ``"2025Q2"``) to values; any forecast period without an
        entry inherits the most recent value. Named ``override_method`` on the
        Python side because the Rust variant is ``Override``.

        Returns
        -------
        ForecastMethod
            Override forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.override_method() == ForecastMethod.override_method()
        True
        """
        ...

    @staticmethod
    def time_series() -> ForecastMethod:
        """
        Project an externally supplied historical series forward with trend
        detection.

        The history is passed through the spec's parameters rather than read
        from the model graph, so the same series can drive several nodes.

        Returns
        -------
        ForecastMethod
            External time-series forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.time_series() == ForecastMethod.time_series()
        True
        """
        ...

    @staticmethod
    def seasonal() -> ForecastMethod:
        """
        Seasonal decomposition forecast (additive or multiplicative).

        Requires a ``season_length`` parameter counted in **periods** (4 for a
        quarterly model, 12 for monthly) and at least two full seasonal cycles
        of history. Use additive mode when seasonal swings are constant in
        absolute terms and multiplicative when they scale with the level;
        prefer additive for series that cross zero.

        Returns
        -------
        ForecastMethod
            Seasonal forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.seasonal() == ForecastMethod.seasonal()
        True
        """
        ...

    @staticmethod
    def fade_to_target() -> ForecastMethod:
        """
        Glide from the base value to a ``target`` level over the forecast
        horizon.

        Shapes (``shape`` parameter, default ``"linear"``): ``"linear"``
        reaches the target exactly at the final period; ``"geometric"`` is
        constant compound growth (CAGR-to-terminal, requires base and target
        non-zero with the same sign); ``"exponential"`` decays the remaining
        gap by half every ``half_life`` periods and never quite reaches the
        target. The workhorse for fading margins/ratios to a long-run
        assumption (NIM normalization, cost-income convergence, ROE fade).

        Returns
        -------
        ForecastMethod
            Fade-to-target forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.fade_to_target() == ForecastMethod.fade_to_target()
        True
        """
        ...

    @staticmethod
    def mean_reverting() -> ForecastMethod:
        """
        Mean-reverting AR(1) path:
        ``v[t] = v[t-1] + reversion_speed * (long_run_mean - v[t-1]) + std_dev * z[t]``.

        ``long_run_mean`` and ``std_dev`` are in the node's own units;
        ``reversion_speed`` is the fraction of the gap closed each period, in
        ``(0, 1]``. ``z[t]`` is a deterministic standard-normal draw derived
        from the configured seed. Use for autocorrelated series that revert
        toward a through-the-cycle level (spreads, charge-off rates, NIM).

        Returns
        -------
        ForecastMethod
            Mean-reverting forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.mean_reverting() == ForecastMethod.mean_reverting()
        True
        """
        ...

    @staticmethod
    def bootstrap() -> ForecastMethod:
        """
        Historical bootstrap: resample observed per-period changes with
        replacement (deterministic with seed).

        ``mode = "growth"`` (default) resamples period-over-period growth
        rates from a strictly positive ``historical`` series and compounds
        them; ``mode = "diff"`` resamples additive level changes and works
        for series that cross zero. Reproduces the empirical distribution of
        changes — including fat tails — without a normality assumption.

        Returns
        -------
        ForecastMethod
            Bootstrap forecast method.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.bootstrap() == ForecastMethod.bootstrap()
        True
        """
        ...

    def __eq__(self, other: object) -> bool:
        """Return whether two forecast method tokens are equal."""
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this forecast method.
        Returns
        -------
        str
        """
        ...

class ForecastSpec:
    """
    Forecast configuration for a statement node.

    Examples
    --------
    >>> from finstack_quant.statements import ForecastSpec
    >>> spec = ForecastSpec.growth(0.05)
    >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
    True

    """

    def __init__(self, method: ForecastMethod, params_json: str | None = None) -> None:
        """
        Build a forecast spec from a method plus its method-specific parameters.

        Parameters
        ----------
        method:
            A :class:`ForecastMethod` describing the projection rule applied
            to forecast periods.
        params_json:
            JSON object of method-specific parameters (``rate``, ``curve``,
            ``mean``, ``std_dev``, ``seed``, ``overrides``, ``season_length``,
            ``target``, ``shape``, ``half_life``, ``long_run_mean``,
            ``reversion_speed``, ``historical``, ``mode``, ``phi``, ...).
            Rates are decimal fractions per period. Every method also accepts
            optional ``min`` / ``max`` bounds that clamp generated values to a
            band (in the node's own units). ``None`` means no parameters,
            which is only valid for ``forward_fill``.

        Raises
        ------
        ValueError
            If params_json is not valid JSON for the method parameter mapping.

        """
        ...

    @staticmethod
    def forward_fill() -> ForecastSpec:
        """
        Forward-fill spec: hold the last observed value flat, no parameters.

        Returns
        -------
        ForecastSpec
            A forward-fill forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.forward_fill()
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def growth(rate: float) -> ForecastSpec:
        """
        Constant compound-growth spec: ``v[t] = v[t-1] * (1 + rate)``.

        Parameters
        ----------
        rate:
            Growth rate as a **decimal fraction per period** (0.05 = 5% per
            period, on the model's own period cadence). Negative values decay
            the series.

        Returns
        -------
        ForecastSpec
            A constant-growth forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.growth(0.05)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def curve(curve: list[float]) -> ForecastSpec:
        """
        Period-by-period growth spec: ``v[t] = v[t-1] * (1 + curve[t])``.

        Parameters
        ----------
        curve:
            One growth rate per forecast period, each a decimal fraction per
            period (0.05 = 5%), consumed in forecast-period order.

        Returns
        -------
        ForecastSpec
            A curve-based forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.curve([0.03, 0.04])
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def normal(mean: float, std_dev: float, seed: int) -> ForecastSpec:
        """
        Additive normal random-walk spec:
        ``v[t] = v[t-1] + mean + std_dev * z[t]``.

        Parameters
        ----------
        mean:
            Per-period additive drift, in the node's own units (currency
            amount for ``Money`` nodes, unitless for scalar nodes).
        std_dev:
            Per-period additive volatility in the same units; must be
            non-negative. Zero gives a deterministic drift path.
        seed:
            Seed for the deterministic standard-normal draws. The evaluator
            mixes a stable hash of the node id into it, so two nodes sharing a
            seed still receive independent shocks.

        Returns
        -------
        ForecastSpec
            A normal-draw forecast specification.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.normal(0.0, 0.1, 7)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def lognormal(mean: float, std_dev: float, seed: int) -> ForecastSpec:
        """
        Multiplicative log-normal spec:
        ``v[t] = v[t-1] * exp(mean - 0.5 * std_dev**2 + std_dev * z[t])``.

        Parameters
        ----------
        mean:
            Per-period **log-return** drift as a decimal fraction (0.02 = 2%
            log drift per period). The ``-0.5 * std_dev**2`` convexity term is
            applied by the engine, so this is not the expected simple return.
        std_dev:
            Per-period log-return volatility as a decimal fraction; must be
            non-negative.
        seed:
            Seed for the deterministic standard-normal draws.

        Returns
        -------
        ForecastSpec
            A log-normal-draw forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.lognormal(0.0, 0.1, 7)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def fade_to_target(target: float) -> ForecastSpec:
        """
        Linear fade-to-target spec:
        ``v[t] = base + (target - base) * t / N``.

        Values glide from the last observed value to ``target`` in equal
        steps, reaching it exactly at the final forecast period. For the
        ``"geometric"`` (CAGR-to-terminal) or ``"exponential"`` (half-life)
        shapes, construct ``ForecastSpec(ForecastMethod.fade_to_target(),
        params_json)`` with ``shape`` and, for exponential, ``half_life``.

        Parameters
        ----------
        target:
            Terminal level in the node's own units (a currency amount for
            monetary nodes, a decimal ratio for scalar nodes such as margins).

        Returns
        -------
        ForecastSpec
            A linear fade-to-target forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.fade_to_target(0.02)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def mean_reverting(long_run_mean: float, reversion_speed: float, std_dev: float, seed: int) -> ForecastSpec:
        """
        Mean-reverting AR(1) spec:
        ``v[t] = v[t-1] + reversion_speed * (long_run_mean - v[t-1]) + std_dev * z[t]``.

        Parameters
        ----------
        long_run_mean:
            Level the series reverts toward, in the node's own units
            (currency amount for monetary nodes, decimal ratio for scalars).
        reversion_speed:
            Fraction of the gap closed each period, in ``(0, 1]``; 1.0
            reverts fully every period, values near 0 revert slowly.
        std_dev:
            Per-period additive shock volatility in the node's own units;
            must be non-negative. Zero gives deterministic geometric decay of
            the gap.
        seed:
            Seed for the deterministic standard-normal draws. The evaluator
            mixes a stable hash of the node id into it, so two nodes sharing
            a seed still receive independent shocks.

        Returns
        -------
        ForecastSpec
            A mean-reverting forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.mean_reverting(0.05, 0.25, 0.01, 7)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def bootstrap(historical: list[float], seed: int) -> ForecastSpec:
        """
        Growth-mode bootstrap spec: resample historical growth rates and
        compound them from the base value.

        The history must be strictly positive in growth mode. For additive
        resampling of level changes (series that cross zero), construct
        ``ForecastSpec(ForecastMethod.bootstrap(), params_json)`` with
        ``mode = "diff"``.

        Parameters
        ----------
        historical:
            At least 2 historical values in the node's own units, oldest
            first; consecutive pairs define the resampled growth rates.
        seed:
            Seed for the deterministic resampling draws.

        Returns
        -------
        ForecastSpec
            A growth-mode bootstrap forecast specification.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.bootstrap([100.0, 105.0, 110.0], 7)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True
        """
        ...

    @staticmethod
    def from_json(json: str) -> ForecastSpec:
        """
        Deserialize a forecast spec from JSON.

        Parameters
        ----------
        json:
            JSON document matching the forecast spec schema.

        Returns
        -------
        ForecastSpec
            Parsed forecast specification.

        Raises
        ------
        ValueError
            If JSON parsing or schema validation fails.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.growth(0.05)
        >>> ForecastSpec.from_json(spec.to_json()).to_json() == spec.to_json()
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this forecast spec to JSON.

        Returns
        -------
        str
            Canonical JSON representation of this forecast specification.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this forecast spec.
        Returns
        -------
        str
        """
        ...

class NodeType:
    """
    How a node combines explicit values, forecasts, and formulas.

    Examples
    --------
    >>> from finstack_quant.statements import NodeType
    >>> NodeType.calculated() == NodeType.calculated()
    True

    """

    @staticmethod
    def value() -> NodeType:
        """
        Value node: explicit per-period values only (actuals, assumptions).

        The node has no formula and no forecast, so periods without an
        explicit value stay unset.

        Returns
        -------
        NodeType
            Value-only node type.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> from finstack_quant.statements import NodeType
        >>> NodeType.value() == NodeType.value()
        True
        """
        ...

    @staticmethod
    def calculated() -> NodeType:
        """
        Calculated node: derived from its formula in every period.

        Returns
        -------
        NodeType
            Calculated node type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import NodeType
        >>> NodeType.calculated() == NodeType.calculated()
        True
        """
        ...

    @staticmethod
    def mixed() -> NodeType:
        """
        Mixed node: explicit value, else forecast, else formula.

        This is the crate's core precedence invariant — **Value > Forecast >
        Formula** — resolved independently for each period, so a node can be
        an actual in historical periods, a forecast in projected periods, and
        a formula wherever neither is supplied.

        Returns
        -------
        NodeType
            Mixed node type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import NodeType
        >>> NodeType.mixed() == NodeType.mixed()
        True
        """
        ...

    @property
    def kind(self) -> str:
        """
        Canonical snake-case wire discriminant for this node type.

        Returns
        -------
        str
            ``"value"``, ``"calculated"``, or ``"mixed"``, matching the JSON
            schema rename for :class:`NodeType`.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __eq__(self, other: object) -> bool:
        """Return whether two node type tokens are equal."""
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this node type.
        Returns
        -------
        str
        """
        ...

class NodeId:
    """
    Type-safe identifier for a node in a financial model.

    Examples
    --------
    >>> from finstack_quant.statements import NodeId
    >>> str(NodeId("revenue"))
    'revenue'

    """

    def __init__(self, id: str) -> None:
        """
        Wrap a raw node identifier string.

        Parameters
        ----------
        id:
            Node identifier as it appears in the model graph and in formulas
            (for example ``"revenue"``). The value is stored verbatim — no
            case folding or trimming — so identifiers are matched exactly.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.

        Examples
        --------
        >>> from finstack_quant.statements import NodeId
        >>> NodeId("ebitda").as_str()
        'ebitda'
        """
        ...

    def as_str(self) -> str:
        """
        Return the underlying identifier string.

        Returns
        -------
        str
            The identifier exactly as supplied at construction.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> from finstack_quant.statements import NodeId
        >>> NodeId("cogs").as_str()
        'cogs'
        """
        ...

    def __repr__(self) -> str:
        """Return a Python-literal style representation.
        Returns
        -------
        str
        """
        ...

    def __str__(self) -> str:
        """Return the identifier as a plain string.
        Returns
        -------
        str
        """
        ...

class NumericMode:
    """
    Numeric evaluation mode for statement evaluation.

    Examples
    --------
    >>> from finstack_quant.statements import NumericMode
    >>> NumericMode.float64() == NumericMode.float64()
    True

    """

    @staticmethod
    def float64() -> NumericMode:
        """
        Use 64-bit floating point arithmetic.

        Returns
        -------
        NumericMode
            IEEE-754 double-precision mode.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.statements import NumericMode
        >>> NumericMode.float64() == NumericMode.float64()
        True
        """
        ...

    def __eq__(self, other: object) -> bool:
        """Return whether two numeric mode tokens are equal."""
        ...

    def __repr__(self) -> str:
        """Return a debug representation of this numeric mode.
        Returns
        -------
        str
        """
        ...

class FinancialModelSpec:
    """
    Top-level financial model specification (wire format).

    Typically built with ``ModelBuilder`` or loaded from JSON.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> builder.build().id
    'demo'

    """

    @staticmethod
    def builder(id: str) -> ModelBuilder:
        """
        Start a staged model build.

        The canonical entry point, mirroring Rust's
        ``FinancialModelSpec::builder(id)`` and the ``Type.builder()`` form
        every other builder-backed type uses. Constructing
        :class:`ModelBuilder` directly is equivalent.

        Parameters
        ----------
        id : str
            Stable model identifier.

        Returns
        -------
        ModelBuilder
            A fresh builder awaiting ``periods(...)``.

        Examples
        --------
        >>> from finstack_quant.statements import FinancialModelSpec
        >>> builder = FinancialModelSpec.builder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> builder.build().id
        'demo'

        Notes
        -----
        This constructor does not raise; validation happens in ``build()``.
        """
        ...

    @staticmethod
    def from_json(json: str) -> FinancialModelSpec:
        """
        Deserialize a model specification from JSON text.

        Parameters
        ----------
        json:
            JSON document matching the statements model schema.

        Returns
        -------
        FinancialModelSpec
            Parsed specification.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON or fails schema validation.

        Examples
        --------
        >>> from finstack_quant.statements import FinancialModelSpec
        >>> payload = (
        ...     '{"id":"demo","periods":[{"id":"2025Q1","start":"2025-01-01",'
        ...     '"end":"2025-04-01","is_actual":false}],"nodes":{},"schema_version":1}'
        ... )
        >>> (FinancialModelSpec.from_json(payload).id, FinancialModelSpec.from_json(payload).node_count)
        ('demo', 0)

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this specification to compact JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `FinancialModelSpec`, suitable for a matching `from_json` call.
        Raises
        ------
        ValueError
            If serialization fails.

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> '"id":"demo"' in builder.build().to_json()
        True

        """
        ...

    @property
    def id(self) -> str:
        """
        Model identifier string.
        Returns
        -------
        str
            Unique model identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def period_count(self) -> int:
        """
        Number of periods in the model timeline.

        Returns
        -------
        int
            Count in **periods** on the model's own cadence (quarters, months,
            years), not months. Their declared order *is* the evaluation
            timeline.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def node_count(self) -> int:
        """
        Number of nodes defined on the model.
        Returns
        -------
        int
            Number of nodes (line items / metrics) declared in the model.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def node_ids(self) -> list[str]:
        """
        List node identifiers in declaration order.

        Returns
        -------
        list[str]
            Ordered node id strings.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> builder.build().node_ids()
        []
        """
        ...

    def has_node(self, node_id: str) -> bool:
        """
        Return whether a node with the given id exists.

        Parameters
        ----------
        node_id:
            Node identifier to test.

        Returns
        -------
        bool
            ``True`` if present.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> builder.build().has_node("revenue")
        False
        """
        ...

    @property
    def schema_version(self) -> int:
        """
        Wire-format schema version of this model spec.

        Returns
        -------
        int
            Only version ``1`` is accepted today; the field exists so persisted
            models can be migrated rather than silently misread.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise summary including id, period count, and node count.
        Returns
        -------
        str
        """
        ...

class ModelBuilder:
    """
    Builder for a ``FinancialModelSpec``.

    Call ``periods`` once, then add nodes with ``value`` / ``compute``, and
    finish with ``build``.

    Note
    ----
    Configuration methods mutate the builder in place *and* return it, so they
    can be chained or called as separate statements. ``build`` and ``mixed``
    are terminal: they consume the builder, and any later configuration call
    raises ``ValueError``.

    Examples
    --------
    Chained:

    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("demo").periods("2025Q1..Q1")
    >>> model = builder.value("revenue", [("2025Q1", 100.0)]).build()
    >>> model.node_ids()
    ['revenue']

    Statement per line:

    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> builder.build().node_ids()
    ['revenue']

    """

    def __init__(self, id: str) -> None:
        """
        Start a new builder for a model with the given id.

        Parameters
        ----------
        id:
            Model identifier assigned to the built ``FinancialModelSpec``.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> builder.build().id
        'demo'

        Notes
        -----
        This constructor does not raise; validation happens in ``build()``.
        """
        ...

    def periods(self, range: str, actuals_until: str | None = None) -> ModelBuilder:
        """
        Define the model's period lattice from a range expression.

        Parameters
        ----------
        range:
            Period range expression such as ``"2025Q1..Q4"``.
        actuals_until:
            Optional last actual period label; ``None`` if not used.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods are already set, the range is invalid, or the builder was consumed.

        """
        ...

    def value(self, node_id: str, values: list[tuple[str, float]]) -> ModelBuilder:
        """
        Add a value node with explicit per-period scalars.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, value)`` pairs, for example ``[("2025Q1", 1.0)]``.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def value_scalar(self, node_id: str, values: list[tuple[str, float]]) -> ModelBuilder:
        """
        Add a scalar value node with explicit per-period values.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, value)`` pairs, for example ``[("2025Q1", 1.0)]``.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def value_money(self, node_id: str, values: list[tuple[str, Money]]) -> ModelBuilder:
        """
        Add a monetary value node with explicit per-period values.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, Money)`` pairs, for example ``[("2025Q1", Money(100.0, "USD"))]``.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def compute(self, node_id: str, formula: str) -> ModelBuilder:
        """
        Add a calculated node from a DSL formula.

        Parameters
        ----------
        node_id:
            Identifier for the computed node.
        formula:
            Expression in the statements DSL (for example ``"revenue - cogs"``).

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the formula fails to compile or the builder state is invalid.

        """
        ...

    def mixed(self, node_id: str) -> MixedNodeBuilder:
        """
        Start configuring a mixed node and consume this builder until ``build`` returns.

        Parameters
        ----------
        node_id:
            Identifier for the new mixed node.

        Returns
        -------
        MixedNodeBuilder
            A builder for the mixed node.  Call ``build`` on the returned
            builder to attach the node and resume this builder.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.

        """
        ...

    def forecast(self, node_id: str, forecast_spec: ForecastSpec) -> ModelBuilder:
        """
        Attach a forecast to an existing node or create a forecast-only mixed node.

        Parameters
        ----------
        node_id:
            Identifier for the node to forecast.
        forecast_spec:
            A :class:`ForecastSpec` describing the projection method and parameters.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.

        """
        ...

    def where_clause(self, where_clause: str) -> ModelBuilder:
        """
        Attach a conditional expression to the last added node.

        Parameters
        ----------
        where_clause:
            DSL expression evaluated per period to gate the node's value.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.

        """
        ...

    def with_meta(self, key: str, value_json: str) -> ModelBuilder:
        """
        Add model-level metadata from a JSON payload.

        Parameters
        ----------
        key:
            Namespaced model-metadata key used to identify the supplied JSON
            value in serialized model output.
        value_json:
            JSON-serialized metadata value.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If value_json is malformed, periods are not configured, or the builder
            has already been consumed.

        """
        ...

    def with_builtin_metrics(self) -> ModelBuilder:
        """
        Add all built-in statement metrics to the model.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def add_metric_from_registry(self, qualified_id: str, registry: MetricRegistry) -> ModelBuilder:
        """
        Add one metric and its dependencies from a metric registry.

        Parameters
        ----------
        qualified_id:
            Fully qualified metric identifier.
        registry:
            A :class:`MetricRegistry` containing the metric definition.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.
        KeyError
            If qualified_id or one of its dependencies cannot be resolved in registry.

        """
        ...

    def add_bond(
        self,
        id: str,
        notional: Money,
        coupon_rate: float,
        issue_date: date,
        maturity_date: date,
        discount_curve_id: str,
    ) -> ModelBuilder:
        """
        Add a fixed-rate bond to the capital structure (US 30/360 semi-annual).

        For non-USD conventions, use :meth:`add_debt` with a pre-built
        ``Bond`` JSON specification.

        Parameters
        ----------
        id:
            Bond identifier.
        notional:
            Face value as a :class:`Money` amount.
        coupon_rate:
            Annual coupon rate as a decimal (e.g. ``0.05`` for 5%).
        issue_date:
            Bond issue date.
        maturity_date:
            Bond maturity date.
        discount_curve_id:
            Curve ID for discounting (e.g. ``"USD-OIS"``).

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If a date is invalid or the builder has already been consumed.
        RuntimeError
            If the bond cannot be added to the model capital structure.

        """
        ...

    def add_swap(
        self,
        id: str,
        notional: Money,
        fixed_rate: float,
        start_date: date,
        maturity_date: date,
        discount_curve_id: str,
        forward_curve_id: str,
    ) -> ModelBuilder:
        """
        Add an interest rate swap to the capital structure (US conventions).

        Parameters
        ----------
        id:
            Swap identifier.
        notional:
            Notional amount as a :class:`Money` value.
        fixed_rate:
            Fixed leg rate as a decimal (e.g. ``0.04`` for 4%).
        start_date:
            Swap start date.
        maturity_date:
            Swap maturity date.
        discount_curve_id:
            Curve ID for discounting.
        forward_curve_id:
            Curve ID for forward rates.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If a date is invalid or the builder has already been consumed.
        RuntimeError
            If the swap cannot be added to the model capital structure.

        """
        ...

    def add_bond_with_convention(
        self,
        id: str,
        notional: Money,
        coupon_rate: float,
        issue_date: date,
        maturity_date: date,
        convention: str,
        discount_curve_id: str,
    ) -> ModelBuilder:
        """
        Add a fixed-rate bond with a market convention preset.

        Applies regional day-count, coupon-frequency, and calendar
        conventions automatically; :meth:`add_bond` uses US corporate
        conventions instead.

        Parameters
        ----------
        id:
            Bond identifier.
        notional:
            Face value as a :class:`Money` amount.
        coupon_rate:
            Annual coupon rate as a decimal (e.g. ``0.03`` for 3%).
        issue_date:
            Bond issue date.
        maturity_date:
            Bond maturity date.
        convention:
            Regional preset as the canonical snake_case identifier:
            ``"us_treasury"``, ``"us_agency"``, ``"german_bund"``,
            ``"uk_gilt"``, ``"french_oat"``, ``"jgb"``, or ``"corporate"``.
        discount_curve_id:
            Curve ID for discounting (e.g. ``"EUR-OIS"``).

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the convention is unknown, a date is invalid, or the builder
            has already been consumed.
        RuntimeError
            If the bond cannot be added to the model capital structure.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("cs-model")
        >>> _ = builder.add_bond_with_convention(
        ...     "BUND-001",
        ...     Money(10_000_000.0, Currency("EUR")),
        ...     0.03,
        ...     date(2025, 1, 15),
        ...     date(2030, 1, 15),
        ...     "german_bund",
        ...     "EUR-OIS",
        ... )
        """
        ...

    def add_swap_with_conventions(
        self,
        id: str,
        notional: Money,
        fixed_rate: float,
        start_date: date,
        maturity_date: date,
        discount_curve_id: str,
        forward_curve_id: str,
        fixed_frequency: Tenor | str,
        fixed_day_count: DayCount,
        float_frequency: Tenor | str,
        float_day_count: DayCount,
        business_day_convention: BusinessDayConvention | str | None = None,
    ) -> ModelBuilder:
        """
        Add an interest rate swap with custom leg conventions.

        Exposes day-count, frequency, and business-day-convention parameters
        for non-USD swaps (e.g. EUR annual ACT/360 fixed legs);
        :meth:`add_swap` uses US conventions instead.

        Parameters
        ----------
        id:
            Swap identifier.
        notional:
            Notional amount as a :class:`Money` value.
        fixed_rate:
            Fixed leg rate as a decimal (e.g. ``0.04`` for 4%).
        start_date:
            Swap start date.
        maturity_date:
            Swap maturity date.
        discount_curve_id:
            Curve ID for discounting.
        forward_curve_id:
            Curve ID for floating-leg forward rates.
        fixed_frequency:
            Payment frequency of the fixed leg (e.g. ``"1Y"``).
        fixed_day_count:
            Day-count convention applied to the fixed leg.
        float_frequency:
            Payment / fixing frequency of the floating leg (e.g. ``"3M"``).
        float_day_count:
            Day-count convention applied to the floating leg.
        business_day_convention:
            Schedule-date rolling convention (default Modified Following).

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If a tenor, day count, convention, or date is invalid, or the
            builder has already been consumed.
        RuntimeError
            If the swap cannot be added to the model capital structure.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import DayCount
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("cs-model")
        >>> _ = builder.add_swap_with_conventions(
        ...     "SWAP-EUR",
        ...     Money(5_000_000.0, Currency("EUR")),
        ...     0.03,
        ...     date(2025, 1, 15),
        ...     date(2030, 1, 15),
        ...     "EUR-OIS",
        ...     "EUR-EURIBOR-6M",
        ...     "1Y",
        ...     DayCount.ACT_360,
        ...     "6M",
        ...     DayCount.ACT_360,
        ...     "modified_following",
        ... )
        """
        ...

    def add_debt(self, id: str, spec_json: str) -> ModelBuilder:
        """
        Add a debt instrument via its canonical v1 instrument envelope.

        Supported instrument types are bonds, convertible bonds, revolving
        credit facilities, term loans, interest-rate swaps, caps/floors, and
        swaptions.

                Parameters
                ----------
                id:
                    Instrument identifier.
                spec_json:
                    ``finstack_quant.instrument/1`` envelope containing the debt instrument.

                Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
                ------
                ValueError
                    If the envelope is invalid or its instrument type is not supported by
                    financial statement capital structures.

        """
        ...

    def reporting_currency(self, currency: Currency) -> ModelBuilder:
        """
        Set the reporting currency used for capital-structure totals.

        Parameters
        ----------
        currency:
            A :class:`Currency` instance. A bare ISO-4217 string is not
            accepted; construct ``Currency("USD")`` first.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the builder has already been consumed.

        """
        ...

    def fx_policy(self, policy: str) -> ModelBuilder:
        """
        Set the FX policy (``cashflow_date``/``period_end``/``period_average``/``custom``).

        Parameters
        ----------
        policy:
            FX conversion policy label.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If policy is unknown or the builder has already been consumed.

        """
        ...

    def waterfall(self, waterfall_spec: WaterfallSpec) -> ModelBuilder:
        """
        Attach a waterfall specification (PIK toggle + ECF sweep + priorities).

        Parameters
        ----------
        waterfall_spec:
            A :class:`WaterfallSpec` defining cash distribution priorities.

        Returns
        -------
        ModelBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the builder has already been consumed.

        """
        ...

    def build(self) -> FinancialModelSpec:
        """
        Materialize the ``FinancialModelSpec`` and consume the builder.

        Returns
        -------
        FinancialModelSpec
            Completed specification.

        Raises
        ------
        ValueError
            If the builder is not ready or was already consumed.

        """
        ...

class MixedNodeBuilder:
    """
    Builder for a mixed statement node.

    A mixed node combines explicit values, a forecast spec, and/or a fallback
    formula.  Obtain an instance via :meth:`ModelBuilder.mixed`.

    Note
    ----
    Methods on this class mutate the builder in place and return ``None``.
    Call them sequentially rather than chaining.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> mixed = builder.mixed("profit")
    >>> _ = mixed.values([("2025Q1", 40.0)])
    >>> _ = mixed.formula("revenue - cost")
    >>> mixed.build().build().has_node("profit")
    True

    """

    def values(self, values: list[tuple[str, float]]) -> MixedNodeBuilder:
        """
        Set scalar explicit values.

        Parameters
        ----------
        values:
            ``(period_id, value)`` pairs for periods where an explicit scalar
            overrides the formula or forecast.

        Returns
        -------
        MixedNodeBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If a period label is invalid or the mixed-node builder has been consumed.

        """
        ...

    def values_money(self, values: list[tuple[str, Money]]) -> MixedNodeBuilder:
        """
        Set monetary explicit values.

        Parameters
        ----------
        values:
            ``(period_id, Money)`` pairs for periods where an explicit monetary
            value overrides the formula or forecast.

        Returns
        -------
        MixedNodeBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If a period label is invalid or the mixed-node builder has been consumed.

        """
        ...

    def forecast(self, forecast_spec: ForecastSpec) -> MixedNodeBuilder:
        """
        Set how this mixed node is projected in forecast periods.

        Parameters
        ----------
        forecast_spec:
            A :class:`ForecastSpec` describing the projection method.

        Returns
        -------
        MixedNodeBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the mixed-node builder has already been consumed.

        """
        ...

    def formula(self, formula: str) -> MixedNodeBuilder:
        """
        Set the fallback formula.

        Parameters
        ----------
        formula:
            DSL expression used when no explicit value or forecast is available.

        Returns
        -------
        MixedNodeBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If formula is empty or invalid, or the mixed-node builder has been consumed.

        """
        ...

    def name(self, name: str) -> MixedNodeBuilder:
        """
        Set the statement node's display name in reports.

        Parameters
        ----------
        name:
            Human-readable node name.

        Returns
        -------
        MixedNodeBuilder
            This builder, for chaining.

        Raises
        ------
        ValueError
            If the mixed-node builder has already been consumed.

        """
        ...

    def build(self) -> ModelBuilder:
        """
        Attach the mixed node and return a ready model builder.

        Returns
        -------
        ModelBuilder
            The parent :class:`ModelBuilder` with the mixed node attached.

        Raises
        ------
        ValueError
            If required builder fields are missing or fail validation.
        """
        ...

class MetricRegistry:
    """
    Reusable statement metric registry.

    Examples
    --------
    >>> from finstack_quant.statements import MetricRegistry
    >>> registry = MetricRegistry.with_builtins()
    >>> len(registry) > 0
    True

    """

    def __init__(self) -> None:
        """
        Create an empty metric registry with no built-in metrics loaded.

        Call :meth:`load_builtins` or use :meth:`with_builtins` to populate it.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @staticmethod
    def with_builtins() -> MetricRegistry:
        """
        Create a registry preloaded with built-in metrics.

        Returns
        -------
        MetricRegistry
            A registry containing all built-in statement metrics.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> from finstack_quant.statements import MetricRegistry
        >>> len(MetricRegistry.with_builtins()) > 0
        True
        """
        ...

    def load_builtins(self) -> None:
        """
        Load the library's built-in statement metrics into this registry.

        Existing entries with the same names are replaced by the built-in
        definitions.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def load_from_json_str(self, json: str) -> None:
        """
        Load metrics from a JSON document.

        Parameters
        ----------
        json:
            JSON string containing metric definitions.

        Raises
        ------
        ValueError
            If json is malformed or contains an invalid metric definition.
        KeyError
            If a referenced registry metric cannot be resolved.

        """
        ...

    def load_from_json(self, path: str) -> None:
        """
        Load metrics from a JSON file path.

        Parameters
        ----------
        path:
            Filesystem path to a JSON document containing metric definitions.

        Raises
        ------
        RuntimeError
            If path cannot be opened or read.
        ValueError
            If the file contains malformed JSON or an invalid metric definition.
        KeyError
            If a referenced registry metric cannot be resolved.

        """
        ...

    def has(self, qualified_id: str) -> bool:
        """
        Return whether a fully qualified metric exists.

        Parameters
        ----------
        qualified_id:
            Fully qualified metric identifier.

        Returns
        -------
        bool
            ``True`` if the metric is registered.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

    def __len__(self) -> int:
        """Return the number of metrics.
        Returns
        -------
        int
        """
        ...

class StatementResult:
    """
    Per-node, per-period numeric results from evaluating a model.

    Examples
    --------
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> Evaluator().evaluate(builder.build()).get("revenue", "2025Q1")
    100.0

    """

    @staticmethod
    def from_json(json: str) -> StatementResult:
        """
        Deserialize evaluation results from JSON.

        Parameters
        ----------
        json:
            JSON document for ``StatementResult``.

        Returns
        -------
        StatementResult
            Parsed results.

        Raises
        ------
        ValueError
            If JSON parsing fails.

        Examples
        --------
        >>> from finstack_quant.statements import Evaluator, ModelBuilder, StatementResult
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
        >>> result = Evaluator().evaluate(builder.build())
        >>> StatementResult.from_json(result.to_json()).get("revenue", "2025Q1")
        100.0

        """
        ...

    def to_json(self) -> str:
        """
        Serialize these results to compact JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `StatementResult`, suitable for a matching `from_json` call.
        Raises
        ------
        ValueError
            If serialization fails.

        """
        ...

    def get(self, node_id: str, period: str) -> float | None:
        """
        Get the value for a node at a specific period.

        Returns the f64 view of whichever source won this period under the
        crate's **Value > Forecast > Formula** precedence rule.

        Parameters
        ----------
        node_id:
            Node identifier.
        period:
            Period label such as ``"2025Q1"``.

        Returns
        -------
        float | None
            The value in the node's own units — a currency amount for
            monetary nodes (currency not carried; use :meth:`get_money` for
            that) and a unitless scalar otherwise. ``None`` when the node or
            period is unknown.

        Raises
        ------
        ValueError
            If ``period`` cannot be parsed as a period id.

        """
        ...

    def get_money(self, node_id: str, period: str) -> Money | None:
        """
        Return the currency-tagged ``Money`` value for a monetary node.

        Preserves fixed-point precision and currency. Returns ``None`` when
        the node is not monetary or has no value for this period.

        Parameters
        ----------
        node_id:
            Node identifier.
        period:
            Period label such as ``"2025Q1"``.

        Returns
        -------
        Money | None
            Monetary value when found, otherwise ``None``.

        Raises
        ------
        ValueError
            If ``period`` cannot be parsed as a period id.
        """
        ...

    def get_scalar(self, node_id: str, period: str) -> float | None:
        """
        Return the scalar value for a non-monetary node.

        Returns ``None`` when the node is monetary or has no value for this
        period.

        Parameters
        ----------
        node_id:
            Node identifier.
        period:
            Period label such as ``"2025Q1"``.

        Returns
        -------
        float | None
            Scalar value when found, otherwise ``None``.

        Raises
        ------
        ValueError
            If ``period`` cannot be parsed as a period id.
        """
        ...

    def get_or(self, node_id: str, period: str, default: float) -> float:
        """
        Return the value for a node at a period, or a default when missing.

        Parameters
        ----------
        node_id:
            Node identifier.
        period:
            Period label such as ``"2025Q1"``.
        default:
            Value returned when the node or period is unknown.

        Returns
        -------
        float
            The evaluated value in the node's own units, or ``default``.

        Raises
        ------
        ValueError
            If ``period`` cannot be parsed as a period id.

        Examples
        --------
        >>> from finstack_quant.statements import Evaluator, ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
        >>> result = Evaluator().evaluate(builder.build())
        >>> result.get_or("missing", "2025Q1", 0.0)
        0.0
        """
        ...

    def all_periods(self, node_id: str) -> list[tuple[str, float]]:
        """
        Get every evaluated period for one node as ordered pairs.

        Parameters
        ----------
        node_id:
            Node identifier.

        Returns
        -------
        list[tuple[str, float]]
            ``(period, value)`` pairs in evaluation order, in the node's own
            units. Empty when the node is not in the result.

        Notes
        -----
        This method does not raise; an unknown *node_id* yields an empty list.

        Examples
        --------
        >>> from finstack_quant.statements import Evaluator, ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
        >>> Evaluator().evaluate(builder.build()).all_periods("revenue")
        [('2025Q1', 100.0)]
        """
        ...

    @property
    def check_report(self) -> CheckReport | None:
        """
        Check report attached by an evaluator configured with checks.

        Returns
        -------
        CheckReport | None
            The report produced by :meth:`Evaluator.with_checks`, or
            ``None`` when no check suite ran during evaluation.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def get_node(self, node_id: str) -> dict[str, float] | None:
        """
        Get every evaluated period for one node as a dict.

        Parameters
        ----------
        node_id:
            Node identifier.

        Returns
        -------
        dict[str, float] | None
            Period identifier string → value, in evaluation order, in the
            node's own units (currency amount for monetary nodes, unitless
            otherwise). ``None`` when the node is not in the result.

        Notes
        -----
        This method does not raise; an unknown *node_id* returns ``None``.
        """
        ...

    def node_ids(self) -> list[str]:
        """
        Return every node id present in this result set, in evaluation order.

        Returns
        -------
        list[str]
            Node ids as declared in the model graph.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def node_count(self) -> int:
        """
        Number of nodes in the result.

        Returns
        -------
        int
            Count of evaluated nodes.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def num_periods(self) -> int:
        """
        Number of periods evaluated.

        Returns
        -------
        int
            Count in **periods** on the model's own cadence (quarters, months,
            years), not months.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def eval_time_ms(self) -> int | None:
        """
        Wall-clock evaluation time in milliseconds, if recorded.

        Diagnostic only — it is not part of the deterministic result, so two
        identical runs may report different values.

        Returns
        -------
        int or None
            Elapsed milliseconds, or ``None`` when the producing run did not
            record it.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def warning_count(self) -> int:
        """
        Number of evaluation warnings recorded.

        Warnings never fail an evaluation; see :attr:`warnings` for what was
        flagged.

        Returns
        -------
        int
            Warning count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def warnings(self) -> list[str]:
        """
        Evaluation warnings as human-readable strings.

        Each entry is the debug form of an ``EvalWarning`` (division by zero,
        non-finite value, skipped non-finite aggregate input, ignored
        capital-structure cashflow, ...), so audit tooling can see *what* was
        flagged rather than only a count.

        Returns
        -------
        list[str]
            One string per recorded warning.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def numeric_mode(self) -> NumericMode:
        """
        Numeric mode stamped into the result envelope (policy visibility).

        Returns
        -------
        NumericMode
            Always ``NumericMode.float64()`` today; the reserved decimal mode
            exists only so persisted payloads can evolve.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def parallel(self) -> bool:
        """
        Whether the evaluation ran in parallel (policy visibility).

        Returns
        -------
        bool
            ``True`` when the DAG was traversed in parallel. Parallel and
            serial runs produce identical values, so this is an audit stamp,
            not a behavioural flag.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self, orient: Literal["long", "wide"] = "long") -> pd.DataFrame:
        """
        Export results as a pandas DataFrame.

        Parameters
        ----------
        orient : {"long", "wide"}, default "long"
            ``"long"`` yields one row per (node, period) with columns
            ``node_id``, ``period``, ``value``, ``value_money``, ``currency``,
            ``value_type``. ``"wide"`` yields node identifiers as rows and
            period identifiers as columns.

        Notes
        -----
        In long form the monetary columns are populated for nodes carrying
        currency information and are otherwise null. ``value_money`` is a
        float64 mirror of the monetary amount (f64, not fixed-point Decimal,
        precision); use ``to_json()`` or ``get_money()`` when full fixed-point
        precision is required.

        Returns
        -------
        pd.DataFrame
            Long- or wide-format frame.

        Raises
        ------
        ValueError
            If ``orient`` is not ``"long"`` or ``"wide"``.
        """
        ...

    def to_arrow_long(self) -> ArrowTable:
        """
        Export the long-format table via Arrow (zero-copy for consumers).

        Returns an `ArrowTable` implementing ``__arrow_c_stream__``; pass it
        to ``pyarrow.table(...)``, ``polars.DataFrame(...)``, or DuckDB.
        Column values and monetary-mirror semantics match ``to_dataframe("long")``,
        plus column roles and table metadata are preserved as Arrow
        field/schema metadata. One column name differs: the period column
        here is ``period_id`` (the table envelope's native name), whereas
        ``to_dataframe("long")`` renames it to ``period``.

        Returns
        -------
        ArrowTable
            Long-format Arrow table with one row per (node, period) pair.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_arrow_wide(self) -> ArrowTable:
        """
        Export the wide-format table via Arrow (zero-copy for consumers).

        Rows are periods (column ``period_id``), one ``float64`` column per
        node, matching ``to_dataframe("wide")`` before its transpose.

        Returns
        -------
        ArrowTable
            Wide-format Arrow table with one row per period.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise summary with node and period counts.
        Returns
        -------
        str
        """
        ...

class Evaluator:
    """
    Evaluates a ``FinancialModelSpec`` into a ``StatementResult``.

    Examples
    --------
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> Evaluator().evaluate(builder.build()).node_count
    1

    """

    def __init__(self) -> None:
        """
        Create a fresh evaluator with default configuration.

        Returns
        -------
        None

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def with_checks(self, suite_spec: CheckSuiteSpec) -> Evaluator:
        """
        Attach a check suite to run automatically after each evaluation.

        The suite spec is resolved (built-in and formula checks) and the
        resulting report is attached to
        :attr:`StatementResult.check_report` on every subsequent
        ``evaluate`` / ``evaluate_with_market`` call.

        Parameters
        ----------
        suite_spec:
            The check-suite specification to resolve and attach.

        Returns
        -------
        Evaluator
            This evaluator, for chaining.

        Raises
        ------
        ValueError
            If the suite spec cannot be resolved into runnable checks.

        Examples
        --------
        >>> from finstack_quant.statements import CheckSuiteSpec, Evaluator, ModelBuilder
        >>> spec = CheckSuiteSpec.from_json('{"name": "s", "builtin_checks": [], "formula_checks": []}')
        >>> builder = ModelBuilder("demo")
        >>> _ = builder.periods("2025Q1..Q1")
        >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
        >>> evaluator = Evaluator().with_checks(spec)
        >>> evaluator.evaluate(builder.build()).check_report.passed
        True
        """
        ...

    def evaluate(self, model: FinancialModelSpec) -> StatementResult:
        """
        Evaluate ``model`` and return numeric results.

        Parameters
        ----------
        model:
            Specification produced by ``ModelBuilder.build`` or ``from_json``.

        Returns
        -------
        StatementResult
            Populated result object.

        Raises
        ------
        ValueError
            If evaluation fails (for example cyclic dependencies or bad formulas).

        """
        ...

    def evaluate_with_market(
        self,
        model: FinancialModelSpec,
        market: MarketContext,
        as_of: date,
    ) -> StatementResult:
        """
        Evaluate ``model`` with market data and an as-of date.

        Use this for capital-structure-aware models and as-of filtering of
        future actual periods.

        Parameters
        ----------
        model:
            Specification produced by ``ModelBuilder.build`` or ``from_json``.
        market:
            A :class:`MarketContext` with curves, FX, and vol surfaces.
        as_of:
            Valuation date for discounting and period filtering.

        Returns
        -------
        StatementResult
            Populated result object with market-aware valuations.

        Raises
        ------
        ValueError
            If evaluation fails or required market data is missing.

        """
        ...

def parse_formula(formula: str) -> str:
    """
    Parse a DSL formula and return a debug string for its AST.

    Parameters
    ----------
    formula:
        Source expression in the statements DSL.

    Returns
    -------
    str
        Debug representation of the parsed abstract syntax tree.

    Raises
    ------
    ValueError
        If parsing fails.

    Examples
    --------
    >>> from finstack_quant.statements import parse_formula
    >>> "revenue" in parse_formula("revenue - cogs")
    True

    """
    ...

def validate_formula(formula: str) -> None:
    """
    Validate that ``formula`` parses and compiles successfully.

    Parameters
    ----------
    formula:
        DSL expression to validate.

    Returns
    -------
    None
        Returns nothing on success. Validation is reported by raising, so
        ``if validate_formula(f):`` is not a validity check.

    Raises
    ------
    ValueError
        If parsing or compilation fails.

    Examples
    --------
    >>> from finstack_quant.statements import validate_formula
    >>> validate_formula("revenue - cogs") is None
    True

    """
    ...

class NormalizationConfig:
    """
    Configuration for normalizing a target metric (for example EBITDA).

    Examples
    --------
    >>> from finstack_quant.statements import NormalizationConfig
    >>> config = NormalizationConfig("ebitda")
    >>> (config.target_node, config.adjustment_count)
    ('ebitda', 0)

    """

    def __init__(self, target_node: str) -> None:
        """
        Create an empty configuration for ``target_node``.

        Parameters
        ----------
        target_node:
            Node id whose values will be adjusted.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.

        Examples
        --------
        >>> from finstack_quant.statements import NormalizationConfig
        >>> config = NormalizationConfig("adjusted_ebitda")
        >>> config.adjustment_count
        0
        """
        ...

    @staticmethod
    def from_json(json: str) -> NormalizationConfig:
        """
        Load normalization rules from JSON.

        Parameters
        ----------
        json:
            JSON document for ``NormalizationConfig``.

        Returns
        -------
        NormalizationConfig
            Parsed configuration.

        Raises
        ------
        ValueError
            If JSON is invalid.

        Examples
        --------
        >>> from finstack_quant.statements import NormalizationConfig
        >>> config = NormalizationConfig("ebitda")
        >>> NormalizationConfig.from_json(config.to_json()).target_node
        'ebitda'

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this configuration to pretty-printed JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `NormalizationConfig`, suitable for a matching `from_json` call.
        Raises
        ------
        ValueError
            If serialization fails.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.statements import NormalizationConfig
        >>> json.loads(NormalizationConfig("ebitda").to_json())["target_node"]
        'ebitda'

        """
        ...

    @property
    def target_node(self) -> str:
        """
        Node id being normalized.
        Returns
        -------
        str
            Node identifier of the metric being normalized (e.g. ``"ebitda"``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def adjustment_count(self) -> int:
        """
        Number of adjustment line items configured.
        Returns
        -------
        int
            Number of add-back / deduction adjustments configured.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise summary including target node and adjustment count.
        Returns
        -------
        str
        """
        ...

def normalize(results: StatementResult, config: NormalizationConfig) -> str:
    """
    Run normalization and return a JSON list of ``NormalizationResult`` objects.

    Parameters
    ----------
    results:
        Evaluated statement output.
    config:
        Target node and adjustment definitions.

    Returns
    -------
    str
        JSON array encoding normalization results.

    Raises
    ------
    ValueError
        If the engine fails.

    Examples
    --------
    >>> from finstack_quant.statements import Evaluator, ModelBuilder, NormalizationConfig, normalize
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("ebitda", [("2025Q1", 25.0)])
    >>> result = Evaluator().evaluate(builder.build())
    >>> import json
    >>> json.loads(normalize(result, NormalizationConfig("ebitda")))[0]["final_value"]
    25.0

    """
    ...

class CheckSuiteSpec:
    """
    A serializable suite specification describing which checks to run.

    Load from JSON (e.g. a team-wide check policy file) and inspect its
    composition (``builtin_check_count`` / ``formula_check_count``). Note:
    running a suite is not yet exposed through the Python bindings; this type is
    currently for loading and inspecting a policy definition only.

    Examples
    --------
    >>> from finstack_quant.statements import CheckSuiteSpec
    >>> suite = CheckSuiteSpec.from_json('{"name":"basic","builtin_checks":[],"formula_checks":[]}')
    >>> (suite.name, suite.builtin_check_count, suite.formula_check_count)
    ('basic', 0, 0)

    """

    @staticmethod
    def from_json(json: str) -> CheckSuiteSpec:
        """
        Deserialize a suite specification from JSON text.

        Parameters
        ----------
        json:
            JSON document matching the ``CheckSuiteSpec`` schema.

        Returns
        -------
        CheckSuiteSpec
            Parsed specification.

        Raises
        ------
        ValueError
            If ``json`` is not valid or fails schema validation.

        Examples
        --------
        >>> from finstack_quant.statements import CheckSuiteSpec
        >>> suite = CheckSuiteSpec.from_json('{"name":"basic","builtin_checks":[],"formula_checks":[]}')
        >>> suite.name
        'basic'

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this specification to pretty-printed JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `CheckSuiteSpec`, suitable for a matching `from_json` call.
        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @property
    def name(self) -> str:
        """
        Suite name, used for display and logging.

        Returns
        -------
        str
            The suite's name.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def builtin_check_count(self) -> int:
        """
        Number of built-in checks the spec will materialize.

        Built-ins are the crate-provided accounting-identity, reconciliation,
        and data-quality checks, selected by name in the spec.

        Returns
        -------
        int
            Count of built-in check entries.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def formula_check_count(self) -> int:
        """
        Number of user-defined formula checks the spec will materialize.

        Formula checks are DSL expressions evaluated per period; their syntax
        is validated when the suite runs, not when the spec is loaded.

        Returns
        -------
        int
            Count of formula check entries.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise summary of the suite spec.
        Returns
        -------
        str
        """
        ...

class CheckReport:
    """
    Validation check report aggregating results and summary statistics.

    Loaded from JSON (``from_json``) produced by the Rust checks framework,
    then inspected via properties or rendered to text/HTML.

    Examples
    --------
    >>> from finstack_quant.statements import CheckReport
    >>> report = CheckReport.from_json(
    ...     '{"results":[],"summary":{"total_checks":0,"passed":0,"failed":0,"errors":0,"warnings":0,"infos":0}}'
    ... )
    >>> (report.passed, report.total_findings)
    (True, 0)

    """

    @staticmethod
    def from_json(json: str) -> CheckReport:
        """
        Deserialize a check report from JSON text.

        Parameters
        ----------
        json:
            JSON document matching the ``CheckReport`` schema.

        Returns
        -------
        CheckReport
            Parsed report.

        Raises
        ------
        ValueError
            If ``json`` is not valid or fails schema validation.

        Examples
        --------
        >>> from finstack_quant.statements import CheckReport
        >>> payload = (
        ...     '{"results":[],"summary":{"total_checks":0,"passed":0,"failed":0,"errors":0,"warnings":0,"infos":0}}'
        ... )
        >>> CheckReport.from_json(payload).total_checks
        0

        """
        ...

    def to_json(self) -> str:
        """
        Serialize this report to pretty-printed JSON.

        Returns
        -------
        str
            JSON text.

            Canonical JSON representation of this `CheckReport`, suitable for a matching `from_json` call.
        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @property
    def passed(self) -> bool:
        """
        Whether the whole report passed: no error-severity finding was retained
        by any check.

        Warnings and info findings do not fail a report. A suite's materiality
        threshold never suppresses error findings, so it cannot flip this to
        ``True``.

        Returns
        -------
        bool
            ``True`` when zero error-severity findings were retained.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def total_checks(self) -> int:
        """
        Number of checks that ran, one per row of :meth:`to_dataframe`.

        Reads the canonical Rust ``CheckSummary.total_checks`` counter.

        Returns
        -------
        int
            Count of check results in the report.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def has_errors(self) -> bool:
        """
        Whether the report contains at least one error-severity finding.

        Delegates to the canonical Rust ``CheckReport::has_errors``.

        Returns
        -------
        bool
            ``True`` when any retained finding has error severity.

        Notes
        -----
        This method does not raise; it reads the stored summary.

        Examples
        --------
        >>> from finstack_quant.statements import CheckReport
        >>> report = CheckReport.from_json(
        ...     '{"results": [], "summary": {"total_checks": 0, "passed": 0,'
        ...     ' "failed": 0, "errors": 0, "warnings": 0, "infos": 0}}'
        ... )
        >>> report.has_errors()
        False
        """
        ...

    def has_warnings(self) -> bool:
        """
        Whether the report contains at least one warning-severity finding.

        Delegates to the canonical Rust ``CheckReport::has_warnings``.

        Returns
        -------
        bool
            ``True`` when any retained finding has warning severity.

        Notes
        -----
        This method does not raise; it reads the stored summary.

        Examples
        --------
        >>> from finstack_quant.statements import CheckReport
        >>> report = CheckReport.from_json(
        ...     '{"results": [], "summary": {"total_checks": 0, "passed": 0,'
        ...     ' "failed": 0, "errors": 0, "warnings": 0, "infos": 0}}'
        ... )
        >>> report.has_warnings()
        False
        """
        ...

    def findings_by_severity(self, severity: str) -> list[dict[str, object]]:
        """
        Return all retained findings of one severity as serde dicts.

        Delegates to the canonical Rust ``CheckReport::findings_by_severity``.

        Parameters
        ----------
        severity:
            One of ``"info"``, ``"warning"``, ``"error"`` (the canonical
            snake_case severity discriminants).

        Returns
        -------
        list[dict[str, object]]
            One ``CheckFinding`` serde dict per matching retained finding.

        Raises
        ------
        ValueError
            If ``severity`` is not a canonical severity discriminant.

        Examples
        --------
        >>> from finstack_quant.statements import CheckReport
        >>> report = CheckReport.from_json(
        ...     '{"results": [], "summary": {"total_checks": 0, "passed": 0,'
        ...     ' "failed": 0, "errors": 0, "warnings": 0, "infos": 0}}'
        ... )
        >>> report.findings_by_severity("error")
        []
        """
        ...

    @property
    def total_findings(self) -> int:
        """
        Number of retained findings across all checks.

        Counts what survived the suite's ``min_severity`` and
        ``materiality_threshold`` reporting filters, not every raw diagnostic
        the checks produced — it matches the row count of
        :meth:`to_findings_dataframe`.

        Returns
        -------
        int
            Retained error, warning and info findings combined.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def total_errors(self) -> int:
        """
        Number of retained error-severity findings across all checks.

        Returns
        -------
        int
            Retained error-severity finding count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def total_warnings(self) -> int:
        """
        Number of retained warning-severity findings across all checks.

        Returns
        -------
        int
            Retained warning-severity finding count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export one row per executed check as a pandas ``DataFrame``.

        Columns: ``check_id`` (stable identifier), ``check_name``
        (human-readable name), ``category`` (snake_case group, one of
        ``accounting_identity``, ``cross_statement_reconciliation``,
        ``internal_consistency``, ``credit_reasonableness``, ``data_quality``),
        ``passed`` (``True`` when the check retained no error-severity
        finding), ``finding_count`` (retained findings for this check, after
        the suite's reporting filters).

        One row per check result, so the row count equals
        :attr:`total_checks`. A report with no checks still yields the
        documented columns with zero rows. Per-finding detail — severity,
        message, and the observed-vs-reference materiality — lives in
        :meth:`to_findings_dataframe`.

        Returns
        -------
        pd.DataFrame
            One row per check result.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_findings_dataframe(self) -> pd.DataFrame:
        """
        Export one row per retained finding as a pandas ``DataFrame``.

        Columns: ``check_id``, ``check_name``, ``category`` (as in
        :meth:`to_dataframe`), ``severity`` (``info``, ``warning`` or
        ``error``), ``message`` (human-readable description), ``period``
        (period identifier string such as ``"2025Q1"``, ``None`` when the
        finding is not period-specific), ``nodes`` (comma-joined node
        identifiers involved, empty string when none), ``materiality_absolute``
        (size of the discrepancy in the compared nodes' own units — currency
        amounts for monetary nodes), ``materiality_relative_pct`` (that
        discrepancy as a **percentage** of the reference value, i.e. already
        multiplied by 100, unlike the decimal-fraction rates used elsewhere in
        this module), ``materiality_reference_value`` (the denominator used)
        and ``materiality_reference_label`` (what that denominator is, e.g.
        ``total_assets``).

        The four materiality columns are ``None`` for findings that carry no
        materiality context. The row count equals :attr:`total_findings`; a
        report with no findings still yields the documented columns with zero
        rows.

        Returns
        -------
        pd.DataFrame
            One row per retained finding.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise summary of the check report.
        Returns
        -------
        str
        """
        ...

class EcfSweepSpec:
    """
    Excess Cash Flow sweep specification.

    Configures how ECF is computed (EBITDA minus taxes/capex/WC/cash interest)
    and what fraction sweeps to debt paydown.

    Examples
    --------
    >>> from finstack_quant.statements import EcfSweepSpec
    >>> sweep = EcfSweepSpec("ebitda", 0.5)
    >>> (sweep.ebitda_node, sweep.sweep_percentage)
    ('ebitda', 0.5)

    """

    def __init__(
        self,
        ebitda_node: str,
        sweep_percentage: float,
        taxes_node: str | None = None,
        capex_node: str | None = None,
        working_capital_node: str | None = None,
        cash_interest_node: str | None = None,
        target_instrument_id: str | None = None,
    ) -> None:
        """
        Configure an excess-cash-flow debt sweep.

        Parameters
        ----------
        ebitda_node : str
            Model node identifier supplying EBITDA before ECF deductions.
        sweep_percentage : float
            Decimal fraction of computed ECF swept to debt, such as ``0.50``.
        taxes_node : str or None, default None
            Optional node identifier for cash taxes deducted from EBITDA.
        capex_node : str or None, default None
            Optional node identifier for capital expenditures deducted from EBITDA.
        working_capital_node : str or None, default None
            Optional node identifier for working-capital cash use or release.
        cash_interest_node : str or None, default None
            Optional node identifier for cash interest deducted before the sweep.
        target_instrument_id : str or None, default None
            Optional debt instrument receiving the ECF paydown; ``None`` uses
            the waterfall's eligible debt allocation.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...
    @staticmethod
    def from_json(json: str) -> EcfSweepSpec:
        """
        Parse an ECF sweep specification from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing the sweep node identifiers and percentage.

        Returns
        -------
        EcfSweepSpec
            Validated `EcfSweepSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or cannot be deserialized as an excess-cash-flow sweep specification.

        Examples
        --------
        >>> from finstack_quant.statements import EcfSweepSpec
        >>> sweep = EcfSweepSpec("ebitda", 0.5)
        >>> EcfSweepSpec.from_json(sweep.to_json()).sweep_percentage
        0.5

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `EcfSweepSpec` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `EcfSweepSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def ebitda_node(self) -> str:
        """
        Node reference or DSL formula supplying EBITDA, the ECF starting point.

        Returns
        -------
        str
            A node id (``"ebitda"``) or an expression over nodes
            (``"revenue - cogs - opex"``), evaluated per period as a monetary
            amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def sweep_percentage(self) -> float:
        """
        Fraction of excess cash flow swept to debt paydown.

        Returns
        -------
        float
            A **decimal fraction in [0, 1]**, not a percentage — 0.5 means a
            50% sweep. Values outside the unit interval are rejected by
            :meth:`WaterfallSpec.validate`.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def target_instrument_id(self) -> str | None:
        """
        Debt instrument the sweep pays down, if the sweep is targeted.

        Returns
        -------
        str | None
            Instrument id, or ``None`` when the sweep applies to all term
            loans in the capital structure.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str: ...

class PikToggleSpec:
    """
    PIK toggle specification.

    Controls when interest accrues as PIK versus cash based on a liquidity
    signal crossing ``threshold``, with optional hysteresis.

    Examples
    --------
    >>> from finstack_quant.statements import PikToggleSpec
    >>> toggle = PikToggleSpec("cash", 100.0)
    >>> (toggle.liquidity_metric, toggle.min_periods_in_pik)
    ('cash', 0)

    """

    def __init__(
        self,
        liquidity_metric: str,
        threshold: float,
        target_instrument_ids: list[str] | None = None,
        min_periods_in_pik: int = 0,
    ) -> None:
        """
        Configure a liquidity-triggered payment-in-kind interest toggle.

        Parameters
        ----------
        liquidity_metric : str
            Model metric or node identifier compared with the trigger threshold.
        threshold : float
            Liquidity threshold in the metric's units that activates PIK logic.
        target_instrument_ids : list[str] or None, default None
            Optional debt instruments subject to the toggle; ``None`` targets
            all eligible instruments in the waterfall.
        min_periods_in_pik : int, default 0
            Hysteresis floor counted in **periods** on the model's own cadence
            (not months): once triggered, PIK stays on for at least this many
            periods. Default 0 lets PIK toggle every period.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...
    @staticmethod
    def from_json(json: str) -> PikToggleSpec:
        """
        Parse a PIK-toggle specification from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing the liquidity trigger and target instruments.

        Returns
        -------
        PikToggleSpec
            Validated `PikToggleSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or cannot be deserialized as a PIK-toggle specification.

        Examples
        --------
        >>> from finstack_quant.statements import PikToggleSpec
        >>> toggle = PikToggleSpec("cash", 100.0)
        >>> PikToggleSpec.from_json(toggle.to_json()).threshold
        100.0

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `PikToggleSpec` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `PikToggleSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def liquidity_metric(self) -> str:
        """
        Node reference or DSL formula producing the liquidity signal.

        Returns
        -------
        str
            A node id (``"cash_balance"``) or an expression
            (``"ebitda / interest_expense"``). Whether the value is monetary
            or a unitless ratio depends on the expression — the threshold must
            use the same units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def threshold(self) -> float:
        """
        Level below which interest accrues as PIK instead of cash.

        Returns
        -------
        float
            Threshold in the **same units as the liquidity metric** — a
            currency amount when the metric is a balance, a unitless ratio
            when it is a coverage ratio. PIK triggers when
            ``metric < threshold``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def min_periods_in_pik(self) -> int:
        """
        Hysteresis floor: minimum time PIK stays on once triggered.

        Returns
        -------
        int
            Count in **periods** on the model's own cadence (quarters for a
            quarterly model), not months. ``0`` disables hysteresis, letting
            PIK toggle every period.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str: ...

class WaterfallSpec:
    """
    Waterfall specification for dynamic cash flow allocation.

    Combines priority-of-payments with optional ECF sweep and PIK toggle.
    Call :meth:`validate` before passing to a builder to surface inconsistent
    configurations (for example ``Sweep`` ordered after ``Equity``).

    Examples
    --------
    >>> from finstack_quant.statements import WaterfallSpec
    >>> waterfall = WaterfallSpec()
    >>> waterfall.priority_of_payments[-1]
    'equity'

    """

    def __init__(
        self,
        priority_of_payments: list[str] | None = None,
        available_cash_node: str | None = None,
        ecf_sweep: EcfSweepSpec | None = None,
        pik_toggle: PikToggleSpec | None = None,
    ) -> None:
        """
        Configure dynamic cash allocation for a financial-model waterfall.

        Parameters
        ----------
        priority_of_payments : list[str] or None, default None
            Ordered payment labels, from highest to lowest priority; ``None``
            applies the builder's default debt-before-equity sequence.
        available_cash_node : str or None, default None
            Optional model node containing cash available for waterfall allocation.
        ecf_sweep : EcfSweepSpec or None, default None
            Optional excess-cash-flow sweep applied within the waterfall.
        pik_toggle : PikToggleSpec or None, default None
            Optional liquidity-driven PIK versus cash-interest configuration.

        Raises
        ------
        ValueError
            If priority_of_payments contains an unknown priority name.

        """
        ...
    @staticmethod
    def from_json(json: str) -> WaterfallSpec:
        """
        Parse a waterfall specification from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing priority, cash source, and optional features.

        Returns
        -------
        WaterfallSpec
            Validated `WaterfallSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or cannot be deserialized as a waterfall specification.

        Examples
        --------
        >>> from finstack_quant.statements import WaterfallSpec
        >>> waterfall = WaterfallSpec()
        >>> WaterfallSpec.from_json(waterfall.to_json()).has_ecf_sweep
        False

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `WaterfallSpec` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `WaterfallSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def validate(self) -> None:
        """
        Validate the spec against internal consistency rules.

        Rejects duplicate priorities, a non-terminal ``equity`` entry, a
        sweep percentage outside [0, 1], an empty PIK target list, a positive
        ECF sweep with no prepayment priority, and — when
        ``available_cash_node`` is set — a stack missing ``fees``,
        ``interest`` or ``amortization``. It does not confirm that referenced
        nodes or instruments exist; that needs the enclosing model.

        Raises
        ------
        ValueError
            If the configuration is economically inconsistent.
        """
        ...

    @property
    def priority_of_payments(self) -> list[str]:
        """
        Payment priority order, highest priority first.

        Returns
        -------
        list[str]
            Snake-case priority names in allocation order — cash is applied to
            the first entry before any of the next. Allocation *within* a
            category is single-class pro-rata across instruments; there is no
            tranche seniority, so a shortfall is shared proportionally.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def available_cash_node(self) -> str | None:
        """
        Node reference or DSL formula for the cash pool the waterfall may spend.

        Returns
        -------
        str | None
            A node id or expression evaluating to a monetary amount per
            period. ``None`` keeps the fully-funded behaviour, in which
            scheduled cashflows are paid in full without being capped against
            available cash.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def has_ecf_sweep(self) -> bool:
        """
        Whether an excess-cash-flow sweep is configured.

        Returns
        -------
        bool
            ``True`` when an :class:`EcfSweepSpec` is attached.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def has_pik_toggle(self) -> bool:
        """
        Whether a PIK toggle is configured.

        Returns
        -------
        bool
            ``True`` when a :class:`PikToggleSpec` is attached.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str: ...
