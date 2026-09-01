"""
Feature engineering: panel-data transformations for signal research.

Bindings for the ``finstack-quant-features`` crate
(``finstack_quant.features``). Provides time-series, cross-sectional, and
pairwise transforms (z-score, rank, rolling mean, neutralization, risk-scaled
weights) plus a general panel dispatcher :func:`transform_panel_json`.

Examples
--------
>>> from finstack_quant.features import transform_cross_sectional
>>> transform_cross_sectional([1.0, 3.0], ["2026-01-01"] * 2, "rank")
[0.0, 1.0]
"""

from __future__ import annotations

from types import ModuleType
from typing import Any

TransformParams = dict[str, Any]

__all__ = [
    "clean_signal",
    "dataframe",
    "neutralize",
    "neutralize_and_zscore",
    "normalize_signal",
    "rank_to_weights",
    "risk_scaled_weights",
    "rolling_regression_residual",
    "transform_cross_sectional",
    "transform_cross_sectional_grouped",
    "transform_panel_json",
    "transform_timeseries",
    "transform_timeseries_pairwise",
]

dataframe: ModuleType

def transform_timeseries(
    values: list[float | None],
    entity: list[str],
    order: list[str],
    op: str,
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Transform a panel value column within each entity over time.

    The input is a flat panel column. Rows are grouped by ``entity``, sorted by
    ``order`` within each group, transformed, and returned in the original input
    order. ``order`` is lexicographic; use ISO-8601 for calendar chronology.
    ``window``, ``periods``, ``half_life``, and EWMA ``span`` count finite
    observations (pandas ``skipna``); missing rows do not decay. ``None`` and
    non-finite numeric values are treated as missing and produce ``None``
    where the requested transform cannot be evaluated.

    Parameters
    ----------
    values : list[float | None]
        Numeric input column. ``None`` represents missing data.
    entity : list[str]
        Entity key for each row; length must match ``values``.
    order : list[str]
        Sort key for each row within an entity; length must match
        ``values``. Ties preserve input order.
    op : str
        Operation name. Supported values are ``"returns"``,
        ``"log_returns"``, ``"diff"``, ``"lag"``,
        ``"rolling_mean"``, ``"rolling_sum"``, ``"rolling_std"``,
        ``"rolling_min"``, ``"rolling_max"``, ``"rolling_zscore"``,
        ``"rolling_rank"``, ``"rolling_quantile"``, ``"rolling_skew"``,
        ``"rolling_kurtosis"``, ``"rolling_slope"``,
        ``"rolling_sharpe"``, ``"rolling_winsorize"``, ``"drawdown"``,
        ``"hampel_filter"``, ``"exponential_decay_weights"``,
        ``"ewma_mean"``, ``"ewma_vol"``, and ``"ewma_zscore"``.
    params : TransformParams or None
        Optional operation parameters:
        ``periods`` for ``returns``, ``log_returns``, ``diff``, and
        ``lag`` (default ``1``); ``window`` and ``min_periods`` for
        rolling operations (defaults ``1`` and ``window``); optional
        ``risk_free`` for ``rolling_sharpe`` (default ``0.0``, same units
        as the return series, no annualization); required
        positive finite pandas ``span`` for EWMA operations (not a
        RiskMetrics ``lambda``); required positive finite ``half_life``
        for ``exponential_decay_weights``.

    Returns
    -------
    list[float | None]
        Output column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, ``op`` is unsupported, or params are
        malformed. Integer params must be positive. EWMA operations require
        a positive finite ``span``.

    Notes
    -----
    ``returns`` and ``log_returns`` return ``None`` when the prior value is
    missing or has magnitude at or below ``1e-12``. ``rolling_std`` and
    ``rolling_zscore`` use sample standard deviation and require at least
    two finite observations. ``ewma_mean``, ``ewma_vol``, and
    ``ewma_zscore`` expect a **return** series and share one pandas
    ``adjust=False`` centered-variance recursion. The first finite
    observation has vol ``None`` and z-score ``0.0``. Missing rows skip
    without decaying.     ``rolling_sharpe`` is a period feature
    ``(mean - risk_free) / sample_std`` on returns, not the annualized
    ``analytics`` / GIPS Sharpe. ``risk_free`` defaults to ``0.0`` in the
    same units as the return series. ``drawdown`` takes a **level**
    series; the ``analytics`` drawdown takes **returns**.

    Examples
    --------
    >>> from finstack_quant.features import transform_timeseries
    >>> transform_timeseries([1.0, 3.0, 6.0], ["A"] * 3, ["1", "2", "3"], "diff")
    [None, 2.0, 3.0]
    """
    ...

def transform_cross_sectional(
    values: list[float | None],
    time_key: list[str],
    op: str,
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Transform a panel value column across entities at each timestamp.

    Rows are partitioned by ``time_key`` and the selected operation is applied
    independently within each partition. Results are returned in the original
    input order. ``None`` and non-finite numeric values are skipped.

    Parameters
    ----------
    values : list[float | None]
        Numeric input column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    op : str
        Operation name. Supported values are ``"zscore"``, ``"rank"``,
        ``"percentile_rank"``, ``"quantile_bucket"``, ``"demean"``,
        ``"robust_zscore"``, ``"minmax_scale"``, ``"clip"``,
        ``"clip_by_sigma"``,
        ``"normal_score_transform"``, ``"long_short_weights"``,
        ``"cap_weights"``,
        ``"fill_missing"``, ``"is_finite"``, ``"nan_mask"``, and
        ``"winsorize"``.
    params : TransformParams or None
        Optional operation parameters. ``quantile_bucket`` accepts
        ``buckets``; ``clip`` accepts explicit ``lower`` and ``upper``;
        ``clip_by_sigma`` accepts ``sigma``; ``winsorize`` accepts ``lower``
        and ``upper`` quantile
        probabilities; ``cap_weights`` accepts ``max_abs``;
        ``fill_missing`` accepts ``value``.

    Returns
    -------
    list[float | None]
        Output column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, ``op`` is unsupported, params are
        malformed, explicit clip bounds are inverted, ``sigma`` is
        negative, or quantile bounds do not satisfy
        ``0 <= lower <= upper <= 1``.

    Notes
    -----
    ``zscore`` uses population standard deviation and returns ``0.0`` for
    finite rows when partition standard deviation is at or below ``1e-12``.
    ``rank`` returns percentile ranks in ``[0, 1]``; ties share the lowest
    tied rank and a single finite row maps to ``0.0``. ``percentile_rank``
    returns open-interval ranks using average tied positions.

    Examples
    --------
    >>> from finstack_quant.features import transform_cross_sectional
    >>> transform_cross_sectional([1.0, 2.0, 3.0], ["2026-01-01"] * 3, "rank")
    [0.0, 0.5, 1.0]
    """
    ...

def transform_cross_sectional_grouped(
    values: list[float | None],
    time_key: list[str],
    groups: list[str],
    op: str,
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Transform a panel value column within each timestamp/group sub-partition.

    Rows are partitioned by the ``(time_key, groups)`` pair and the selected
    cross-sectional operation is applied independently within each
    sub-partition. Results are returned in the original input order.

    Parameters
    ----------
    values : list[float | None]
        Numeric input column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    groups : list[str]
        Secondary partition key combined with ``time_key``; length must
        match ``values``.
    op : str
        Cross-sectional operation name. Accepts the same operations as
        :func:`transform_cross_sectional`.
    params : TransformParams or None
        Optional operation parameters, forwarded to the chosen ``op``.

    Returns
    -------
    list[float | None]
        Output column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, ``op`` is unsupported, or params are
        malformed.

    Examples
    --------
    >>> from finstack_quant.features import transform_cross_sectional_grouped
    >>> transform_cross_sectional_grouped(
    ...     [1.0, 3.0, 10.0, 14.0],
    ...     ["2026-01-01"] * 4,
    ...     ["tech", "tech", "finance", "finance"],
    ...     "zscore",
    ... )
    [-1.0, 1.0, -1.0, 1.0]
    """
    ...

def neutralize(
    values: list[float | None],
    time_key: list[str],
    exposures: list[list[float | None]],
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Return cross-sectional OLS residuals after regressing on exposures.

    Within each ``time_key`` partition, ``values`` is regressed on the exposure
    columns by ordinary least squares and the residuals are returned. Rows whose
    ``values`` or any exposure is missing are excluded from the fit and map to
    ``None`` in the output.

    Parameters
    ----------
    values : list[float | None]
        Signal column to neutralize. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    exposures : list[list[float | None]]
        Exposure columns, each aligned to ``values`` (same length and
        row order).
    params : TransformParams or None
        Optional parameters. ``fit_intercept`` (default ``True``) adds an
        intercept term to the regression.

    Returns
    -------
    list[float | None]
        Residual column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, an exposure column has the wrong length,
        params are malformed, or a ``time_key`` partition is singular
        or underdetermined (the error names that ``time_key``).

    Examples
    --------
    >>> from finstack_quant.features import neutralize
    >>> neutralize([1.0, 2.0, 2.0, 4.0], ["2026-01-01"] * 4, [[0.0, 1.0, 0.0, 1.0]])
    [-0.5, -1.0, 0.5, 1.0]
    """
    ...

def transform_timeseries_pairwise(
    values: list[float | None],
    other: list[float | None],
    entity: list[str],
    order: list[str],
    op: str,
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Transform two panel columns per entity with a rolling pairwise operation.

    Rows are grouped by ``entity`` and sorted by ``order`` within each group.
    ``order`` is lexicographic; use ISO-8601 for calendar chronology. Each
    output row is computed from the trailing ``window`` of paired finite
    ``(values, other)`` observations. ``window`` counts finite pairs, not
    calendar days (pandas ``skipna``).

    Parameters
    ----------
    values : list[float | None]
        First numeric column. ``None`` represents missing data.
    other : list[float | None]
        Second numeric column aligned to ``values``; length must match.
    entity : list[str]
        Entity key for each row; length must match ``values``.
    order : list[str]
        Sort key for each row within an entity; length must match
        ``values``. Ties preserve input order.
    op : str
        Operation name. Supported values are ``"rolling_cov"``,
        ``"rolling_corr"``, and ``"rolling_beta"``.
    params : TransformParams or None
        Optional parameters. ``window`` (default ``1``) and
        ``min_periods`` (default ``window``) bound the trailing window.

    Returns
    -------
    list[float | None]
        Output column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, ``op`` is unsupported, or params are
        malformed.

    Notes
    -----
    At least two paired finite observations are always required, regardless
    of ``min_periods``.

    Examples
    --------
    >>> from finstack_quant.features import transform_timeseries_pairwise
    >>> beta = transform_timeseries_pairwise(
    ...     [1.0, 2.0, 3.0],
    ...     [1.0, 2.0, 4.0],
    ...     ["A"] * 3,
    ...     ["1", "2", "3"],
    ...     "rolling_beta",
    ...     {"window": 3, "min_periods": 3},
    ... )
    >>> [None if value is None else round(value, 3) for value in beta]
    [None, None, 0.643]
    """
    ...

def rolling_regression_residual(
    values: list[float | None],
    exposures: list[list[float | None]],
    entity: list[str],
    order: list[str],
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Return rolling per-entity OLS residuals against exposure columns.

    Rows are grouped by ``entity`` and sorted by ``order``. For each row, an OLS
    fit of ``values`` on the exposure columns is computed over the trailing
    ``window`` of complete rows, and the row's residual from that fit is
    returned.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    exposures : list[list[float | None]]
        Exposure columns, each aligned to ``values`` (same length and
        row order).
    entity : list[str]
        Entity key for each row; length must match ``values``.
    order : list[str]
        Sort key for each row within an entity; length must match
        ``values``. Ties preserve input order.
    params : TransformParams or None
        Optional parameters. ``window`` (default ``1``); ``min_periods``
        (default ``window``) is the minimum number of complete rows required
        to fit; ``fit_intercept`` (default ``True``).

    Returns
    -------
    list[float | None]
        Residual column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ, an exposure column has the wrong length,
        or params are malformed.

    Notes
    -----
    Rank-deficient windows emit ``None`` for that row. That is intentional
    and unlike :func:`neutralize`, which fails the call.

    Examples
    --------
    >>> from finstack_quant.features import rolling_regression_residual
    >>> residual = rolling_regression_residual(
    ...     [1.0, 2.0, 5.0],
    ...     [[0.0, 1.0, 2.0]],
    ...     ["A"] * 3,
    ...     ["1", "2", "3"],
    ...     {"window": 3, "min_periods": 3},
    ... )
    >>> [None if value is None else round(value, 3) for value in residual]
    [None, None, 0.333]
    """
    ...

def risk_scaled_weights(
    values: list[float | None],
    time_key: list[str],
    volatility: list[float | None],
) -> list[float | None]:
    """
    Convert a signal to dollar-neutral inverse-risk-scaled weights.

    Within each ``time_key`` partition, finite rows with ``|vol| > 1e-12``
    become ``raw = signal / vol``, then ``centered = raw - mean(raw)``,
    then ``weight = centered / sum(|centered|)``.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    volatility : list[float | None]
        Risk estimate per row, aligned to ``values``. A magnitude at
        or below ``1e-12`` is treated as missing.
    Returns
    -------
    list[float | None]
        Weight column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ.

    Notes
    -----
    Rows with missing ``values`` or ``|volatility| <= 1e-12`` map to
    ``None``. A partition whose centered gross is at or below ``1e-12``
    emits ``0.0`` for those finite rows.

    Examples
    --------
    >>> from finstack_quant.features import risk_scaled_weights
    >>> risk_scaled_weights([1.0, 2.0, 2.0, 4.0], ["2026-01-01"] * 4, [1.0, 2.0, 1.0, 2.0])
    [-0.25, -0.25, 0.25, 0.25]
    """
    ...

def clean_signal(
    values: list[float | None],
    time_key: list[str],
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Apply the default cross-sectional signal-cleaning pass.

    Delegates to :func:`transform_cross_sectional` with the
    ``"winsorize"`` operation, clamping each timestamp partition to its
    ``lower``/``upper`` sample quantiles.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    params : TransformParams or None
        Optional quantile bounds ``lower`` (default ``0.01``) and
        ``upper`` (default ``0.99``).

    Returns
    -------
    list[float | None]
        Cleaned column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ or quantile bounds do not satisfy
        ``0 <= lower <= upper <= 1``.

    Examples
    --------
    >>> from finstack_quant.features import clean_signal
    >>> clean_signal([1.0, 2.0, 100.0], ["2026-01-01"] * 3, {"lower": 0.0, "upper": 0.5})
    [1.0, 2.0, 2.0]
    """
    ...

def normalize_signal(
    values: list[float | None],
    time_key: list[str],
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Normalize a signal cross-sectionally with a selected method.

    Applies a single-column cross-sectional operation independently within each
    ``time_key`` partition.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    params : TransformParams or None
        Optional parameters. ``method`` selects any single-column
        operation accepted by :func:`transform_cross_sectional` and defaults
        to ``"zscore"``; remaining params are forwarded to that operation.

    Returns
    -------
    list[float | None]
        Normalized column aligned to ``values``. The output length always
        matches the input length.

    Raises
    ------
    ValueError
        If lengths differ, ``method`` is unsupported, or params are
        malformed.

    Examples
    --------
    >>> from finstack_quant.features import normalize_signal
    >>> normalize_signal([1.0, 2.0, 100.0], ["2026-01-01"] * 3, {"method": "rank"})
    [0.0, 0.5, 1.0]
    """
    ...

def rank_to_weights(
    values: list[float | None],
    time_key: list[str],
) -> list[float | None]:
    """
    Convert cross-sectional ranks into gross-normalized long/short weights.

    Within each ``time_key`` partition, values are ranked, demeaned, and scaled
    so the sum of absolute weights is ``1``, yielding a dollar-neutral long/short
    profile.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    Returns
    -------
    list[float | None]
        Weight column aligned to ``values``. The output length always matches
        the input length.

    Raises
    ------
    ValueError
        If lengths differ.

    Examples
    --------
    >>> from finstack_quant.features import rank_to_weights
    >>> rank_to_weights([1.0, 2.0, 100.0], ["2026-01-01"] * 3)
    [-0.5, 0.0, 0.5]
    """
    ...

def neutralize_and_zscore(
    values: list[float | None],
    time_key: list[str],
    exposures: list[list[float | None]],
    params: TransformParams | None = None,
) -> list[float | None]:
    """
    Neutralize a signal against exposures, then cross-sectional z-score.

    Runs :func:`neutralize` to residualize ``values`` on the exposure columns
    within each ``time_key`` partition, then applies a ``"zscore"`` transform to
    the residuals.

    Parameters
    ----------
    values : list[float | None]
        Signal column. ``None`` represents missing data.
    time_key : list[str]
        Cross-sectional partition key for each row; length must match
        ``values``.
    exposures : list[list[float | None]]
        Exposure columns, each aligned to ``values`` (same length and
        row order).
    params : TransformParams or None
        Optional parameters forwarded to :func:`neutralize`;
        ``fit_intercept`` (default ``True``).

    Returns
    -------
    list[float | None]
        Z-scored residual column aligned to ``values``. The output length always
        matches the input length.

    Raises
    ------
    ValueError
        If lengths differ, an exposure column has the wrong length,
        params are malformed, or a ``time_key`` partition is singular
        or underdetermined (the error names that ``time_key``).

    Examples
    --------
    >>> from finstack_quant.features import neutralize_and_zscore
    >>> scores = neutralize_and_zscore([1.0, 2.0, 2.0, 4.0], ["2026-01-01"] * 4, [[0.0, 1.0, 0.0, 1.0]])
    >>> [round(value, 3) for value in scores]
    [-0.632, -1.265, 0.632, 1.265]
    """
    ...

def transform_panel_json(spec_json: str) -> str:
    """
    Apply a JSON panel transform pipeline and return JSON result columns.

    ``spec_json`` is a JSON object with:

    - ``values``: list of numbers or ``null``.
    - ``entity`` and ``order``: required when any operation has
      ``"family": "timeseries"``.
    - ``time_key``: required when any operation has
      ``"family": "cross_sectional"``.
    - ``operations``: list of named operations. Each operation has ``name``,
      ``family`` (``"timeseries"`` or ``"cross_sectional"``), ``op``,
      optional ``params``, and optional ``input``.

    Operations run sequentially. ``input`` selects the source column: omit it
    (default) to read the previous operation output, or the raw ``values``
    column for the first operation. Set ``input`` to ``"values"`` to branch
    from the raw column, or to an already evaluated operation name. Forward
    references are rejected.

    Operation names must be unique, non-empty, and must not be the reserved
    name ``values``. Unknown fields are rejected by the Rust serde model.

    Parameters
    ----------
    spec_json : str
        JSON-serialized panel transform specification.

    Returns
    -------
    str
        JSON string shaped as
        ``{"columns": [{"name": name, "values": values}]}``, preserving
        operation order with every output column aligned to the input values.

    Raises
    ------
    ValueError
        If the JSON is malformed, required keys are missing,
        operation names are duplicated, empty, or reserved (``values``),
        ``input`` names an unknown column, or an operation fails
        validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.features import transform_panel_json
    >>> spec = {
    ...     "values": [10.0, 12.0, 20.0, 21.0],
    ...     "time_key": ["1", "2", "1", "2"],
    ...     "operations": [{"name": "rank", "family": "cross_sectional", "op": "rank"}],
    ... }
    >>> json.loads(transform_panel_json(json.dumps(spec)))["columns"][0]["values"]
    [0.0, 0.0, 1.0, 1.0]
    """
    ...
