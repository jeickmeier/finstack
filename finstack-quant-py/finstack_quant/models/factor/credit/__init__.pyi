"""
Credit factor hierarchy: calibration, decomposition, and covariance forecasts.

Bindings for ``finstack-quant-models::factor`` credit hierarchy artifacts. Models
are JSON-first: calibrate or load a :class:`CreditFactorModel`, decompose
observed spreads into level/adder components, link period-to-period changes, and
forecast factor covariance for risk reporting.

Examples
--------
>>> from finstack_quant.models.factor.credit import CreditFactorModel
>>> try:
...     CreditFactorModel.from_json("{}")
... except ValueError as exc:
...     "missing field" in str(exc)
True
"""

from __future__ import annotations

import datetime

import pandas as pd
from typing import Any

class CreditFactorModel:
    """
    Calibrated credit factor hierarchy artifact.

    Produced by :class:`CreditCalibrator` or loaded via :meth:`from_json`. All
    fields are read-only; mutations require re-calibration.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, CreditFactorModel
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> calibrated = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> CreditFactorModel.from_json(calibrated.to_json()).schema
    'finstack_quant.credit_factor_model/1'
    """

    @staticmethod
    def from_json(json: str) -> CreditFactorModel:
        """
        Deserialize a credit factor model from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json` or the offline calibrator.

        Returns
        -------
        CreditFactorModel
            Parsed, validated model instance.

        Raises
        ------
        ValueError
            If the JSON is malformed or fails structural validation.

        Examples
        --------
        >>> from finstack_quant.models.factor.credit import CreditFactorModel
        >>> try:
        ...     CreditFactorModel.from_json("{}")
        ... except ValueError as exc:
        ...     "missing field" in str(exc)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this model to pretty-printed JSON.

        Returns
        -------
        str
            JSON suitable for storage or transmission.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def schema(self) -> str:
        """
        Namespaced schema marker (``"finstack_quant.credit_factor_model/1"``).

        Returns
        -------
        str
            Exact contract marker embedded in the artifact.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def as_of(self) -> str:
        """
        Calibration anchor date as an ISO 8601 string.

        Returns
        -------
        str
            Model as-of date.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n_levels(self) -> int:
        """
        Number of hierarchy levels (broadest to narrowest).

        Returns
        -------
        int
            Count of hierarchy dimensions.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n_issuers(self) -> int:
        """
        Number of issuer beta rows in the artifact.

        Returns
        -------
        int
            Issuer count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n_factors(self) -> int:
        """
        Number of systematic factors in the model configuration.

        Returns
        -------
        int
            Factor count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def level_names(self) -> list[str]:
        """
        Return hierarchy level names.

        Returns
        -------
        list[str]
            Dimension names (e.g. ``["Rating", "Region", "Sector"]``).

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def issuer_ids(self) -> list[str]:
        """
        Return issuer IDs present in the artifact.

        Returns
        -------
        list[str]
            Issuer identifier strings.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def factor_ids(self) -> list[str]:
        """
        Return factor IDs in the model configuration.

        Returns
        -------
        list[str]
            Factor identifier strings.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str: ...

class CreditCalibrator:
    """
    Deterministic calibrator that produces a :class:`CreditFactorModel`.

    Configuration and inputs are JSON strings so callers can work with plain
    dicts (via ``json.dumps``) without typed wrappers for every sub-field.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> CreditCalibrator(config_json).calibrate(inputs_json).n_issuers
    1
    """

    def __init__(self, config_json: str) -> None:
        """
        Construct a calibrator from a JSON configuration.

        Parameters
        ----------
        config_json : str
            JSON-encoded ``CreditCalibrationConfig``.

        Raises
        ------
        ValueError
            If ``config_json`` is not a valid ``CreditCalibrationConfig``.
        """
        ...

    def calibrate(self, inputs_json: str) -> CreditFactorModel:
        """
        Run calibration and return a validated factor model.

        Parameters
        ----------
        inputs_json : str
            JSON-encoded ``CreditCalibrationInputs`` (spreads, issuers, etc.).

        Returns
        -------
        CreditFactorModel
            Calibrated hierarchy artifact.

        Raises
        ------
        ValueError
            If inputs are invalid or calibration fails.
        """
        ...

    def __repr__(self) -> str: ...

class LevelsAtDate:
    """
    Decomposed credit spread levels at a single observation date.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, decompose_levels
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> levels = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
    >>> (levels.date, levels.generic, levels.adder())
    ('2024-03-01', 100.0, {'A': 5.0})
    """

    @staticmethod
    def from_json(json: str) -> LevelsAtDate:
        """Deserialize a snapshot from canonical JSON.

        Parameters
        ----------
        json:
            Canonical ``LevelsAtDate`` JSON object.

        Returns
        -------
        LevelsAtDate
            Parsed typed decomposition snapshot.

        Raises
        ------
        ValueError
            If the payload is malformed or contains non-finite values.

        Examples
        --------
        >>> LevelsAtDate.from_json('{"date":"2024-01-01","generic":0.0,"by_level":[],"adder":{}}').date
        '2024-01-01'
        """
        ...

    def to_json(self) -> str:
        """Serialize this snapshot to compact canonical JSON.

        Returns
        -------
        str
            Canonical compact JSON for this snapshot.

        Raises
        ------
        ValueError
            If a numeric field is non-finite or serialization fails.
        """
        ...

    @property
    def date(self) -> str:
        """
        Observation date as an ISO 8601 string.

        Returns
        -------
        str
            Decomposition date.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def generic(self) -> float:
        """
        Generic (market-wide) spread component in basis points.

        Returns
        -------
        float
            Generic level in bp.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n_levels(self) -> int:
        """
        Number of hierarchy levels in the decomposition.

        Returns
        -------
        int
            Level count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def level_values(self, level_index: int) -> dict[str, float]:
        """
        Return bucket values for one hierarchy level.

        Parameters
        ----------
        level_index : int
            Zero-based level index (0 = broadest).

        Returns
        -------
        dict[str, float]
            Map of bucket label to spread contribution in bp.

        Raises
        ------
        ValueError
            If ``level_index`` is out of range.
        """
        ...

    def adder(self) -> dict[str, float]:
        """
        Return issuer-specific adder spreads keyed by issuer ID.

        Returns
        -------
        dict[str, float]
            Per-issuer adder in bp.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-level bucket values as a pandas DataFrame.

        Columns: ``date``, ``level_index``, ``dimension``, ``bucket``,
        ``value``.

        One row per (level, bucket) pair. ``date`` repeats on every row as an
        ISO string so a row survives ``pd.concat`` across dates. Rows are
        ordered by ``level_index``, then by ``bucket`` — the values are a
        sorted map, so repeated exports are identical.

        The scalar ``generic`` factor and the per-issuer residuals are not
        levels; read them from the ``generic`` property and :meth:`to_series`
        respectively.

        Returns
        -------
        pd.DataFrame
            One row per (level, bucket) pair. A snapshot from a hierarchy with
            no levels yields a zero-row frame that still carries the columns
            above.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_series(self) -> pd.Series:
        """
        Export the per-issuer residual adders as a pandas Series.

        Returns
        -------
        pd.Series
            Named ``adder``, indexed by issuer ID, holding the residual after
            peeling the generic factor and every hierarchy level. Issuers are
            in sorted order, so repeated exports and
            ``pd.concat([...], axis=1)`` across dates align on the index.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class PeriodDecomposition:
    """
    Period-over-period change in decomposed credit spread levels.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, decompose_levels, decompose_period
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> start = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
    >>> end = decompose_levels(model, '{"A": 0.01065}', 0.01015, "2024-03-02")
    >>> period = decompose_period(start, end)
    >>> (period.from_date, period.to_date, period.d_generic)
    ('2024-03-01', '2024-03-02', 1.5)
    """

    @staticmethod
    def from_json(json: str) -> PeriodDecomposition:
        """Deserialize a decomposition from canonical JSON.

        Parameters
        ----------
        json:
            Canonical ``PeriodDecomposition`` JSON object.

        Returns
        -------
        PeriodDecomposition
            Parsed typed period decomposition.

        Raises
        ------
        ValueError
            If the payload is malformed or contains non-finite values.

        Examples
        --------
        >>> payload = '{"from":"2024-01-01","to":"2024-01-02","d_generic":0.0,"by_level":[],"d_adder":{}}'
        >>> PeriodDecomposition.from_json(payload).from_date
        '2024-01-01'
        """
        ...

    def to_json(self) -> str:
        """Serialize this decomposition to compact canonical JSON.

        Returns
        -------
        str
            Canonical compact JSON for this decomposition.

        Raises
        ------
        ValueError
            If a numeric field is non-finite or serialization fails.
        """
        ...

    @property
    def from_date(self) -> str:
        """
        Start date of the decomposition window (ISO 8601).

        Returns
        -------
        str
            Period start.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    @property
    def to_date(self) -> str:
        """
        End date of the decomposition window (ISO 8601).

        Returns
        -------
        str
            End date of the decomposition window (ISO 8601).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def d_generic(self) -> float:
        """
        Change in generic spread over the period (bp).

        Returns
        -------
        float
            Generic delta in bp.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n_levels(self) -> int:
        """
        Number of hierarchy levels.

        Returns
        -------
        int
            Level count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def level_deltas(self, level_index: int) -> dict[str, float]:
        """
        Return bucket deltas for one hierarchy level.

        Parameters
        ----------
        level_index : int
            Zero-based level index.

        Returns
        -------
        dict[str, float]
            Map of bucket label to spread change in bp.

        Raises
        ------
        ValueError
            If ``level_index`` is out of range.
        """
        ...

    def d_adder(self) -> dict[str, float]:
        """
        Return issuer adder changes keyed by issuer ID.

        Returns
        -------
        dict[str, float]
            Per-issuer adder delta in bp.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-level bucket deltas as a pandas DataFrame.

        Columns: ``from_date``, ``to_date``, ``level_index``, ``dimension``,
        ``bucket``, ``delta``.

        This is the default export and the same table as
        :meth:`to_level_dataframe`. The per-issuer adder deltas are a
        separate, differently-keyed table; see :meth:`to_adder_dataframe`. The
        scalar ``d_generic`` is metadata and is not repeated per row.

        Returns
        -------
        pd.DataFrame
            One row per (level, bucket) pair. A decomposition with no
            hierarchy levels yields a zero-row frame that still carries the
            columns above.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_level_dataframe(self) -> pd.DataFrame:
        """
        Export the per-level bucket deltas as a pandas DataFrame.

        Columns: ``from_date``, ``to_date``, ``level_index``, ``dimension``,
        ``bucket``, ``delta``. Identical to :meth:`to_dataframe`.

        One row per (level, bucket) pair — the long format the level deltas
        naturally take, since each level has its own bucket set. The two dates
        repeat on every row as ISO strings so a row survives ``pd.concat``
        across periods. Rows are ordered by ``level_index``, then by
        ``bucket`` — the deltas are a sorted map, so bucket order is the sorted
        key order and repeated exports are identical.

        A decomposition with no hierarchy levels yields a zero-row frame that
        still carries the columns above.

        Returns
        -------
        pd.DataFrame
            Long-format frame of bucket deltas, one row per (level, bucket).

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_adder_dataframe(self) -> pd.DataFrame:
        """
        Export the per-issuer adder deltas as a pandas DataFrame.

        Columns: ``from_date``, ``to_date``, ``issuer_id``, ``d_adder``.

        One row per issuer. The two dates repeat on every row as ISO strings so
        a row survives ``pd.concat`` across periods. Rows are ordered by
        ``issuer_id`` — the adders are a sorted map, so this is the sorted key
        order and repeated exports are identical.

        A decomposition sharing no issuers between snapshots yields a zero-row
        frame that still carries the columns above.

        Returns
        -------
        pd.DataFrame
            One row per issuer present in both snapshots.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str: ...

class FactorCovarianceMatrix:
    """
    Validated factor covariance matrix with deterministic row-major storage.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import FactorCovarianceMatrix
    >>> matrix = FactorCovarianceMatrix.from_json('{"factor_ids":["credit::generic"],"n":1,"data":[0.04]}')
    >>> matrix.variance("credit::generic")
    0.04
    """

    @staticmethod
    def from_json(json: str) -> FactorCovarianceMatrix:
        """Deserialize and validate a covariance matrix from canonical JSON.

        Parameters
        ----------
        json : str
            Object with ordered ``factor_ids``, dimension ``n``, and row-major ``data``.

        Returns
        -------
        FactorCovarianceMatrix
            Validated symmetric positive-semidefinite covariance matrix.

        Raises
        ------
        ValueError
            If JSON is malformed, dimensions disagree, IDs repeat, or the matrix is invalid.

        Examples
        --------
        >>> FactorCovarianceMatrix.from_json('{"factor_ids":[],"n":0,"data":[]}').n_factors
        0
        """
        ...

    def to_json(self) -> str:
        """Serialize this matrix to canonical JSON.

        Returns
        -------
        str
            Compact JSON with ordered axes and row-major covariance data.

        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @property
    def n_factors(self) -> int:
        """Return the number of covariance axes.

        Returns
        -------
        int
            Number of factor IDs, equal to the matrix row and column count.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def factor_ids(self) -> list[str]:
        """Return ordered factor identifiers for rows and columns.

        Returns
        -------
        list[str]
            Independent copy of the canonical covariance-axis order.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def data(self) -> list[float]:
        """Return row-major annualized covariance values.

        Returns
        -------
        list[float]
            ``n_factors * n_factors`` entries in canonical factor order.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    def variance(self, factor_id: str) -> float:
        """Return one factor's annualized variance.

        Parameters
        ----------
        factor_id : str
            Canonical factor identifier to query.

        Returns
        -------
        float
            Diagonal covariance entry, or ``0.0`` when the factor is unknown.

        Raises
        ------
        None
            This method does not raise.
        """
        ...

    def covariance(self, lhs: str, rhs: str) -> float:
        """Return annualized covariance between two factors.

        Parameters
        ----------
        lhs : str
            Row factor identifier.
        rhs : str
            Column factor identifier.

        Returns
        -------
        float
            Covariance entry, or ``0.0`` when either factor is unknown.

        Raises
        ------
        None
            This method does not raise.
        """
        ...

    def correlation(self, lhs: str, rhs: str) -> float:
        """Return correlation between two factors.

        Parameters
        ----------
        lhs : str
            First factor identifier.
        rhs : str
            Second factor identifier.

        Returns
        -------
        float
            Covariance normalized by both standard deviations; ``0.0`` for unknown or zero-variance factors.

        Raises
        ------
        None
            This method does not raise.
        """
        ...

class FactorModelConfig:
    """
    Portfolio factor-model configuration assembled at a forecast horizon.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import FactorModelConfig
    >>> config = FactorModelConfig.from_json(
    ...     '{"factors":[],"covariance":{"factor_ids":[],"n":0,"data":[]},"matching":{"mapping_table":[]},"pricing_mode":"delta_based","risk_measure":"variance"}'
    ... )
    >>> config.n_factors
    0
    """

    @staticmethod
    def from_json(json: str) -> FactorModelConfig:
        """Deserialize and validate a factor-model configuration.

        Parameters
        ----------
        json : str
            Canonical Rust ``FactorModelConfig`` JSON.

        Returns
        -------
        FactorModelConfig
            Typed configuration with matching and covariance invariants checked.

        Raises
        ------
        ValueError
            If JSON is malformed or matching rules reference undeclared factors.

        Examples
        --------
        >>> FactorModelConfig.from_json(
        ...     '{"factors":[],"covariance":{"factor_ids":[],"n":0,"data":[]},"matching":{"mapping_table":[]},"pricing_mode":"delta_based","risk_measure":"variance"}'
        ... ).factor_ids
        []
        """
        ...

    def to_json(self) -> str:
        """Serialize this configuration to canonical JSON.

        Returns
        -------
        str
            Compact JSON accepted by factor-risk workflows.

        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    def validate(self) -> None:
        """Validate that matching rules emit only declared factor IDs.

        Raises
        ------
        ValueError
            If a matcher references an undeclared factor or duplicates issuer rows.
        """
        ...

    @property
    def n_factors(self) -> int:
        """Return the number of configured factors.

        Returns
        -------
        int
            Length of the factor definition list.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def factor_ids(self) -> list[str]:
        """Return factor-definition IDs in canonical order.

        Returns
        -------
        list[str]
            Ordered IDs aligned to covariance axes.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def factors(self) -> list[dict[str, Any]]:
        """Return structured factor definitions.

        Returns
        -------
        list[dict[str, Any]]
            Independent Python representation of canonical factor-definition fields.

        Raises
        ------
        ValueError
            If conversion to Python values fails.
        """
        ...

    @property
    def covariance(self) -> FactorCovarianceMatrix:
        """Return the covariance matrix aligned to ``factor_ids``.

        Returns
        -------
        FactorCovarianceMatrix
            Independent typed covariance wrapper.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def matching(self) -> dict[str, Any]:
        """Return the declarative factor-matching configuration.

        Returns
        -------
        dict[str, Any]
            Structured Python representation of the canonical matching variant.

        Raises
        ------
        ValueError
            If conversion to Python values fails.
        """
        ...

    @property
    def pricing_mode(self) -> str:
        """Return the sensitivity extraction strategy.

        Returns
        -------
        str
            ``"delta_based"`` or ``"full_repricing"``.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def risk_measure(self) -> str | dict[str, Any]:
        """Return the canonical risk-measure value.

        Returns
        -------
        str or dict[str, Any]
            Scalar label for variance/volatility or structured VaR/ES parameters.

        Raises
        ------
        ValueError
            If conversion to Python values fails.
        """
        ...

    @property
    def bump_size(self) -> dict[str, Any] | None:
        """Return optional finite-difference bump overrides.

        Returns
        -------
        dict[str, Any] or None
            Structured bump configuration, or ``None`` when defaults apply.

        Raises
        ------
        ValueError
            If conversion to Python values fails.
        """
        ...

    @property
    def unmatched_policy(self) -> str | None:
        """Return the policy for unmatched dependencies.

        Returns
        -------
        str or None
            ``"strict"``, ``"residual"``, ``"warn"``, or ``None`` for the default.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

class FactorCovarianceForecast:
    """
    Factor covariance and idiosyncratic vol forecasts from a credit factor model.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, FactorCovarianceForecast
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> forecast = FactorCovarianceForecast(model)
    >>> forecast.covariance_at("one_step").factor_ids
    ['credit::generic']
    """

    def __init__(self, model: CreditFactorModel) -> None:
        """
        Bind a covariance forecast engine to a calibrated model.

        Parameters
        ----------
        model : CreditFactorModel
            Calibrated hierarchy artifact used as the risk factor basis.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def covariance_at(self, horizon: str) -> FactorCovarianceMatrix:
        """
        Return a typed factor covariance matrix at a forecast horizon.

        Parameters
        ----------
        horizon : str
            ``"one_step"``, ``"unconditional"``, or JSON ``'{"n_steps": N}'``.

        Returns
        -------
        FactorCovarianceMatrix
            Typed covariance matrix with ordered factor axes and row-major data.

        Raises
        ------
        ValueError
            If ``horizon`` is invalid or the model lacks required inputs.
        """
        ...

    def idiosyncratic_vol(self, issuer_id: str, horizon: str) -> float:
        """
        Return idiosyncratic volatility for an issuer at a horizon.

        Parameters
        ----------
        issuer_id : str
            Issuer identifier present in the model artifact.
        horizon : str
            ``"one_step"``, ``"unconditional"``, or JSON ``'{"n_steps": N}'``.

        Returns
        -------
        float
            Idiosyncratic volatility (decimal annualized).

        Raises
        ------
        ValueError
            If the issuer or horizon is unknown.
        """
        ...

    def factor_model_at(self, horizon: str, risk_measure_json: str) -> FactorModelConfig:
        """
        Return a typed portfolio-ready factor model at a horizon.

        Parameters
        ----------
        horizon : str
            ``"one_step"``, ``"unconditional"``, or JSON ``'{"n_steps": N}'``.
        risk_measure_json : str
            JSON-encoded risk-measure configuration (e.g. VaR horizon, scaling).

        Returns
        -------
        FactorModelConfig
            Typed configuration suitable for portfolio risk decomposition or ``to_json()``.

        Raises
        ------
        ValueError
            If inputs are invalid or the forecast cannot be built.
        """
        ...

    def __repr__(self) -> str: ...

def decompose_levels(
    model: CreditFactorModel,
    observed_spreads_json: str,
    observed_generic: float,
    as_of: datetime.date | str,
    runtime_tags_json: str | None = None,
) -> LevelsAtDate:
    """
    Decompose observed issuer spreads into hierarchy levels and adders.

    Parameters
    ----------
    model : CreditFactorModel
        Calibrated hierarchy artifact.
    observed_spreads_json : str
        JSON map of issuer ID to observed spread in basis points.
    observed_generic : float
        Observed market generic spread in basis points.
    as_of : datetime.date | str
        Observation date, either a date-like object or an ISO 8601 string.
    runtime_tags_json : str, optional
        Optional JSON map of runtime tags for bucket assignment overrides.

    Returns
    -------
    LevelsAtDate
        Decomposed levels at ``as_of``.

    Raises
    ------
    ValueError
        If spreads, dates, or model references are invalid.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, decompose_levels
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> decompose_levels(model, '{"A": 0.0125}', 0.0120, "2025-06-30").generic
    120.0
    """
    ...

def decompose_period(
    from_levels: LevelsAtDate,
    to_levels: LevelsAtDate,
) -> PeriodDecomposition:
    """
    Compute period-over-period deltas between two level decompositions.

    Parameters
    ----------
    from_levels : LevelsAtDate
        Start-of-period decomposition.
    to_levels : LevelsAtDate
        End-of-period decomposition.

    Returns
    -------
    PeriodDecomposition
        Bucket and adder deltas between the two dates.

    Raises
    ------
    ValueError
        If the two decompositions are incompatible (e.g. different models).

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import CreditCalibrator, decompose_levels, decompose_period
    >>> config_json = (
    ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
    ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
    ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
    ... )
    >>> inputs_json = (
    ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
    ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
    ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
    ...     '"idiosyncratic_overrides":{}}'
    ... )
    >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
    >>> start = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
    >>> end = decompose_levels(model, '{"A": 0.01065}', 0.01015, "2024-03-02")
    >>> decompose_period(start, end).d_generic
    1.5
    """
    ...

class VolHorizon:
    """
    Forecast horizon used to scale a calibrated `Sample` vol estimate.

    Examples
    --------
    >>> from finstack_quant.models.factor.credit import VolHorizon
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

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models.factor.credit import VolHorizon
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

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models.factor.credit import VolHorizon
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

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.models.factor.credit import VolHorizon
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
        >>> from finstack_quant.models.factor.credit import VolHorizon
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
        >>> from finstack_quant.models.factor.credit import VolHorizon
        >>> VolHorizon.parse('{"n_steps":5}').n
        5
        """
        ...

    @property
    def kind(self) -> str:
        """
        Discriminator for the volatility-horizon variant.

        Returns
        -------
        str
            Discriminator for the volatility-horizon variant.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def n(self) -> int | None:
        """
        Step count for ``n_steps`` horizons.

        Returns
        -------
            Step count for ``n_steps`` horizons.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def years_value(self) -> float | None:
        """
        Year fraction for ``years`` horizons.

        Returns
        -------
            Year fraction for ``years`` horizons.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug representation.
        Returns
        -------
        str
        """
        ...

__all__ = [
    "CreditFactorModel",
    "CreditCalibrator",
    "LevelsAtDate",
    "PeriodDecomposition",
    "VolHorizon",
    "FactorCovarianceForecast",
    "FactorCovarianceMatrix",
    "FactorModelConfig",
    "decompose_levels",
    "decompose_period",
]
