"""
Performance analytics: returns, drawdowns, risk metrics, and benchmarks.

The sole entry point is :class:`Performance`. Construct from a price panel
(``Performance(prices_df)`` / ``Performance.from_arrays(...)``) or from a
return panel (``Performance.from_returns(returns_df)`` /
``Performance.from_returns_arrays(...)``); every analytic — return / risk
scalars, drawdown statistics, rolling windows, periodic returns
(MTD / QTD / YTD / FYTD), benchmark alpha/beta, basic factor models — is a
method on the resulting instance.

The remaining classes are value-object outputs returned by `Performance`
methods (`LookbackReturns`, `PeriodStats`, etc.).

Examples
--------
>>> from datetime import date
>>> from finstack_quant.analytics import Performance
>>> perf = Performance.from_returns_arrays([date(2024, 1, 1), date(2024, 1, 2)], [[0.01, 0.02]], ["FUND"])
>>> perf.ticker_names
['FUND']
"""

from __future__ import annotations

import datetime
from typing import Sequence

import numpy as np
import numpy.typing as npt
import pandas as pd
from finstack_quant.core.dates import DayCount

__all__ = [
    "AnalyticsError",
    "Performance",
    "LookbackReturns",
    "PeriodStats",
    "BetaResult",
    "GreeksResult",
    "RollingGreeks",
    "MultiFactorResult",
    "DrawdownEpisode",
    "DatedSeries",
    "constrained_least_squares",
]

# Errors

class AnalyticsError(ValueError):
    """
    Analytics validation or calculation failure.

    Examples
    --------
    >>> from finstack_quant.analytics import AnalyticsError
    >>> str(AnalyticsError("invalid analytics input"))
    'invalid analytics input'
    """

# Value-object results

class PeriodStats:
    """
    Aggregated statistics for grouped periodic returns.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
    >>> perf = Performance.from_returns_arrays(
    ...     dates, [[0.10, -0.05, 0.02, 0.0, 0.03, -0.01]], ["FUND"], frequency="monthly"
    ... )
    >>> round(perf.period_stats(0, aggregation_frequency="monthly").win_rate, 1)
    1.0
    """

    @staticmethod
    def from_json(json: str) -> PeriodStats:
        """
        Deserialize a ``PeriodStats`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        PeriodStats
            Parsed ``PeriodStats`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.analytics import PeriodStats, Performance
        >>> dates = [date(2024, month, 1) for month in range(1, 7)]
        >>> perf = Performance.from_returns_arrays(
        ...     dates, [[0.10, -0.05, 0.02, 0.0, 0.03, -0.01]], ["FUND"], frequency="monthly"
        ... )
        >>> stats = perf.period_stats(0, aggregation_frequency="monthly")
        >>> PeriodStats.from_json(stats.to_json()).win_rate
        0.5
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def best(self) -> float:
        """
        Highest single-period return in the sample, as a decimal.

        Returns
        -------
        float
            Highest single-period return.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def worst(self) -> float:
        """
        Lowest single-period return in the sample, as a decimal.

        Returns
        -------
        float
            Lowest single-period return.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def consecutive_wins(self) -> int:
        """
        Longest consecutive winning streak.

        Returns
        -------
        int
            Maximum number of consecutive positive-return periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def consecutive_losses(self) -> int:
        """
        Longest consecutive losing streak.

        Returns
        -------
        int
            Maximum number of consecutive negative-return periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def win_rate(self) -> float:
        """
        Fraction of positive-return periods.

        Returns
        -------
        float
            Win rate in ``[0, 1]``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def avg_return(self) -> float:
        """
        Average return across all periods.

        Returns
        -------
        float
            Mean periodic return.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def avg_win(self) -> float:
        """
        Average return of positive periods.

        Returns
        -------
        float
            Mean return across winning periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def avg_loss(self) -> float:
        """
        Average return of negative periods.

        Returns
        -------
        float
            Mean return across losing periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def payoff_ratio(self) -> float:
        """
        Payoff ratio (avg win / |avg loss|).

        Returns
        -------
        float
            Ratio of average win to absolute average loss.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def profit_factor(self) -> float:
        """
        Profit factor (gross profits / gross losses).

        Returns
        -------
        float
            Sum of wins divided by sum of absolute losses.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def cpc_ratio(self) -> float:
        """
        CPC index (profit_factor x win_rate x payoff_ratio).

        Returns
        -------
        float
            Composite measure of profitability consistency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def kelly_criterion(self) -> float:
        """
        Kelly criterion optimal fraction.

        Returns
        -------
        float
            Optimal bet fraction for maximizing long-run growth.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def __repr__(self) -> str: ...

class BetaResult:
    """
    Regression beta with confidence interval.

    The 95% interval uses Student-t critical values for finite samples and an
    asymptotic normal approximation once ``n - 2 >= 240``.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
    >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
    >>> perf = Performance.from_returns_arrays(
    ...     dates,
    ...     [[2.0 * value for value in benchmark], benchmark],
    ...     ["FUND", "BENCH"],
    ...     benchmark_ticker="BENCH",
    ...     frequency="monthly",
    ... )
    >>> perf.beta()[0].beta
    2.0
    """

    @staticmethod
    def from_json(json: str) -> BetaResult:
        """
        Deserialize a ``BetaResult`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        BetaResult
            Parsed ``BetaResult`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import BetaResult, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
        >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
        >>> perf = Performance.from_returns_arrays(
        ...     dates,
        ...     [[2.0 * value for value in benchmark], benchmark],
        ...     ["FUND", "BENCH"],
        ...     benchmark_ticker="BENCH",
        ...     frequency="monthly",
        ... )
        >>> BetaResult.from_json(perf.beta()[0].to_json()).beta
        2.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def beta(self) -> float:
        """
        OLS slope of asset returns versus the benchmark.

        Returns
        -------
        float
            OLS regression slope vs benchmark.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def std_err(self) -> float:
        """
        Standard error of the beta estimate.

        Returns
        -------
        float
            Standard error from the OLS fit.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def ci_lower(self) -> float:
        """
        Lower 95% confidence bound.

        Returns
        -------
        float
            Lower bound of the 95% CI for beta.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def ci_upper(self) -> float:
        """
        Upper 95% confidence bound.

        Returns
        -------
        float
            Upper bound of the 95% CI for beta.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a single-row pandas DataFrame.

        Columns: ``beta``, ``std_err``, ``ci_lower``, ``ci_upper``.

        One flat record describes one regression, so a one-row frame is the
        right shape: ``pd.concat([r.to_dataframe() for r in results])`` stacks
        every ticker's beta into one comparison table without reshaping.

        A degenerate regression (fewer than three observations) yields
        non-finite estimates, which arrive as ``None`` and make the affected
        column ``object`` dtype; coerce with ``pd.to_numeric`` before
        aggregating.

        Returns
        -------
        pd.DataFrame
            Single-row frame with the beta point estimate and its interval.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class GreeksResult:
    """
    Alpha, beta, and goodness-of-fit from a single-index regression.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
    >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
    >>> perf = Performance.from_returns_arrays(
    ...     dates,
    ...     [[2.0 * value for value in benchmark], benchmark],
    ...     ["FUND", "BENCH"],
    ...     benchmark_ticker="BENCH",
    ...     frequency="monthly",
    ... )
    >>> round(perf.greeks()[0].alpha, 12)
    0.0
    """

    @staticmethod
    def from_json(json: str) -> GreeksResult:
        """
        Deserialize a ``GreeksResult`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        GreeksResult
            Parsed ``GreeksResult`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import GreeksResult, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
        >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
        >>> perf = Performance.from_returns_arrays(
        ...     dates,
        ...     [[2.0 * value for value in benchmark], benchmark],
        ...     ["FUND", "BENCH"],
        ...     benchmark_ticker="BENCH",
        ...     frequency="monthly",
        ... )
        >>> round(GreeksResult.from_json(perf.greeks()[0].to_json()).alpha, 12)
        0.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def alpha(self) -> float:
        """
        Annualized Jensen alpha.

        Returns
        -------
        float
            Annualized intercept from the single-index regression.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def beta(self) -> float:
        """
        OLS slope of asset returns versus the benchmark.

        Returns
        -------
        float
            OLS regression slope vs benchmark.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def r_squared(self) -> float:
        """
        Coefficient of determination of the fitted model.

        Returns
        -------
        float
            Coefficient of determination.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def adjusted_r_squared(self) -> float:
        """
        Degrees-of-freedom-adjusted coefficient of determination.

        Returns
        -------
        float
            Degrees-of-freedom-adjusted R².

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a single-row pandas DataFrame.

        Columns: ``alpha``, ``beta``, ``r_squared``, ``adjusted_r_squared``.

        One flat record describes one regression, so a one-row frame is the
        right shape: ``pd.concat([r.to_dataframe() for r in results])`` stacks
        every ticker's greeks into one comparison table without reshaping.

        Non-finite estimates from a degenerate fit arrive as ``None`` and make
        the affected column ``object`` dtype; coerce with ``pd.to_numeric``
        before aggregating.

        Returns
        -------
        pd.DataFrame
            Single-row frame with alpha, beta, and goodness-of-fit.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class RollingGreeks:
    """
    Rolling alpha and beta time series.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
    >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
    >>> perf = Performance.from_returns_arrays(
    ...     dates,
    ...     [[2.0 * value for value in benchmark], benchmark],
    ...     ["FUND", "BENCH"],
    ...     benchmark_ticker="BENCH",
    ...     frequency="monthly",
    ... )
    >>> rolling = perf.rolling_greeks(0, window=3)
    >>> len(rolling.dates) == len(rolling.betas)
    True
    """

    @staticmethod
    def from_json(json: str) -> RollingGreeks:
        """
        Deserialize a ``RollingGreeks`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        RollingGreeks
            Parsed ``RollingGreeks`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import RollingGreeks, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
        >>> benchmark = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
        >>> perf = Performance.from_returns_arrays(
        ...     dates,
        ...     [[2.0 * value for value in benchmark], benchmark],
        ...     ["FUND", "BENCH"],
        ...     benchmark_ticker="BENCH",
        ...     frequency="monthly",
        ... )
        >>> rolling = RollingGreeks.from_json(perf.rolling_greeks(0, window=3).to_json())
        >>> len(rolling.dates) == len(rolling.betas)
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def dates(self) -> list[datetime.date]:
        """
        Date labels for each rolling window.

        Returns
        -------
        list[datetime.date]
            Window-end dates aligned 1:1 with :attr:`alphas` and :attr:`betas`.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    @property
    def alphas(self) -> npt.NDArray[np.float64]:
        """
        Rolling annualized Jensen alpha for each window.

        Returns
        -------
        npt.NDArray[np.float64]
            Annualized Jensen alpha per window.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def betas(self) -> npt.NDArray[np.float64]:
        """
        Rolling OLS beta versus the benchmark for each window.

        Returns
        -------
        npt.NDArray[np.float64]
            OLS beta per window.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self) -> pd.DataFrame:
        """
        Convert to a pandas DataFrame with date index and alpha/beta columns.

        Returns
        -------
        pd.DataFrame
            DataFrame with columns ``alpha`` and ``beta``, indexed by date.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class MultiFactorResult:
    """
    Multi-factor regression result.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
    >>> factor = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
    >>> perf = Performance.from_returns_arrays(
    ...     dates, [[2.0 * value for value in factor]], ["FUND"], frequency="monthly"
    ... )
    >>> round(float(perf.multi_factor_greeks(0, [factor]).betas[0]), 1)
    2.0
    """

    @staticmethod
    def from_json(json: str) -> MultiFactorResult:
        """
        Deserialize a ``MultiFactorResult`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        MultiFactorResult
            Parsed ``MultiFactorResult`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import MultiFactorResult, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(6)]
        >>> factor = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03]
        >>> perf = Performance.from_returns_arrays(
        ...     dates, [[2.0 * value for value in factor]], ["FUND"], frequency="monthly"
        ... )
        >>> fitted = MultiFactorResult.from_json(perf.multi_factor_greeks(0, [factor]).to_json())
        >>> round(float(fitted.betas[0]), 1)
        2.0
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def alpha(self) -> float:
        """
        Annualized OLS intercept of the (possibly rf-adjusted) dependent series.

        For ``return_kind="excess"`` this is the intercept of already-excess
        ``y``. For ``return_kind="total"`` it is Jensen-style after subtracting
        the decompounded period risk-free rate from ``y`` only.

        Returns
        -------
        float
            Annualized regression intercept.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def betas(self) -> npt.NDArray[np.float64]:
        """
        One beta per factor, in factor order.

        Returns
        -------
        npt.NDArray[np.float64]
            One beta per factor, in factor order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def r_squared(self) -> float:
        """
        Coefficient of determination of the fitted model.

        Returns
        -------
        float
            Coefficient of determination.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def adjusted_r_squared(self) -> float:
        """
        Degrees-of-freedom-adjusted coefficient of determination.

        Returns
        -------
        float
            Degrees-of-freedom-adjusted R².

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def residual_vol(self) -> float:
        """
        Residual (idiosyncratic) volatility of the multi-factor fit.

        Returns
        -------
        float
            Standard deviation of regression residuals.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self, factor_names: list[str] | None = None) -> pd.DataFrame:
        """
        Export the factor loadings as a pandas DataFrame, one row per factor.

        Columns: ``factor``, ``beta``, ``alpha``, ``r_squared``,
        ``adjusted_r_squared``, ``residual_vol``.

        The loadings are the per-row payload; the four regression-level
        statistics repeat on every row so a single row carries its own fit
        context after ``pd.concat`` across tickers or ``groupby("factor")``.

        Rows follow the order of the ``factor_returns`` passed to
        :meth:`Performance.multi_factor_greeks`, which is also the order of
        :attr:`betas`. There is always at least one row: the regression rejects
        an empty factor set.

        Parameters
        ----------
        factor_names : list[str], optional
            Labels for the ``factor`` column, positionally aligned with
            :attr:`betas`. Defaults to ``factor_0``, ``factor_1``, ... because
            the regression itself carries no names.

        Returns
        -------
        pd.DataFrame
            One row per fitted factor.

        Raises
        ------
        ValueError
            If ``factor_names`` is supplied and its length differs from the
            number of fitted betas.
        """
        ...

    def __repr__(self) -> str: ...

class DrawdownEpisode:
    """
    A single drawdown episode with timing and depth information.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
    >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
    >>> round(perf.drawdown_details(0, n=1)[0].max_drawdown, 1)
    -0.2
    """

    @staticmethod
    def from_json(json: str) -> DrawdownEpisode:
        """
        Deserialize a ``DrawdownEpisode`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        DrawdownEpisode
            Parsed ``DrawdownEpisode`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import DrawdownEpisode, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
        >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
        >>> episode = perf.drawdown_details(0, n=1)[0]
        >>> round(DrawdownEpisode.from_json(episode.to_json()).max_drawdown, 1)
        -0.2
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def start(self) -> datetime.date:
        """
        Start date of the drawdown.

        Returns
        -------
        datetime.date
            Date when the drawdown began.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    @property
    def valley(self) -> datetime.date:
        """
        Date of the maximum drawdown within this episode.

        Returns
        -------
        datetime.date
            Date of the deepest point.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    @property
    def end(self) -> datetime.date | None:
        """
        Recovery date (``None`` if still in drawdown).

        Returns
        -------
        datetime.date or None
            Recovery date, or ``None`` if the episode is ongoing.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    @property
    def duration_days(self) -> int:
        """
        Duration in calendar days.

        Returns
        -------
        int
            Number of calendar days from start to end (or valley if ongoing).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def max_drawdown(self) -> float:
        """
        Maximum drawdown depth (negative).

        Returns
        -------
        float
            Peak-to-trough drawdown as a negative decimal.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def near_recovery_threshold(self) -> float:
        """
        Near-recovery threshold.

        Returns
        -------
        float
            Price level that would signal near-recovery.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def truncated_at_start(self) -> bool:
        """
        True when the episode began before the first observation (left-censored).

        Returns
        -------
        bool
            ``True`` if the drawdown started before the available data window.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def __repr__(self) -> str: ...

class LookbackReturns:
    """
    Period-to-date returns for each ticker.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
    >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
    >>> len(perf.lookback_returns(date(2024, 1, 5)).fytd)
    1
    """

    @staticmethod
    def from_json(json: str) -> LookbackReturns:
        """
        Deserialize a ``LookbackReturns`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`.

        Returns
        -------
        LookbackReturns
            Parsed ``LookbackReturns`` instance.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import LookbackReturns, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
        >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
        >>> lookback = perf.lookback_returns(date(2024, 1, 5))
        >>> len(LookbackReturns.from_json(lookback.to_json()).mtd)
        1
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def mtd(self) -> npt.NDArray[np.float64]:
        """
        Month-to-date returns per ticker.

        Returns
        -------
        npt.NDArray[np.float64]
            Array of MTD returns, one per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def qtd(self) -> npt.NDArray[np.float64]:
        """
        Quarter-to-date returns per ticker.

        Returns
        -------
        npt.NDArray[np.float64]
            Array of QTD returns, one per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def ytd(self) -> npt.NDArray[np.float64]:
        """
        Year-to-date returns per ticker.

        Returns
        -------
        npt.NDArray[np.float64]
            Array of YTD returns, one per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def fytd(self) -> npt.NDArray[np.float64]:
        """
        Fiscal-year-to-date returns per ticker.

        Returns
        -------
        npt.NDArray[np.float64]
            Array of FYTD returns, one per ticker. The fiscal calendar defaults
            to a January-1 start when no fiscal configuration is supplied.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self, ticker_names: list[str]) -> pd.DataFrame:
        """
        Convert to a pandas DataFrame with ticker names as index.

        Columns: ``mtd``, ``qtd``, ``ytd``, and ``fytd``.

        Parameters
        ----------
        ticker_names : list[str]
            Ticker labels to use as the DataFrame index.

        Returns
        -------
        pd.DataFrame
            DataFrame indexed by ticker name with the four lookback return
            columns, including ``fytd``.

        Raises
        ------
        ValueError
            If ``ticker_names`` does not contain one label for each ticker in
            the lookback vectors.
        """
        ...

    def __repr__(self) -> str: ...

class DatedSeries:
    """
    Date-indexed numeric series returned by the rolling-window analytics.

    Rolling-window methods return this shared carrier with a metric-specific
    DataFrame column name.

    Examples
    --------
    >>> from datetime import date, timedelta
    >>> from finstack_quant.analytics import Performance
    >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
    >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
    >>> series = perf.rolling_returns(0, window=2)
    >>> (len(series.values) == len(series.dates), series.value_column)
    (True, 'return')
    """

    @staticmethod
    def from_json(json: str) -> DatedSeries:
        """
        Deserialize a ``DatedSeries`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by :meth:`to_json`, carrying ``values``,
            ``dates`` and ``value_column``.

        Returns
        -------
        DatedSeries
            Parsed ``DatedSeries`` instance, including its metric label.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not satisfy the serialized schema.

        Examples
        --------
        >>> from datetime import date, timedelta
        >>> from finstack_quant.analytics import DatedSeries, Performance
        >>> dates = [date(2024, 1, 1) + timedelta(days=i) for i in range(5)]
        >>> perf = Performance.from_arrays(dates, [[100.0, 90.0, 95.0, 80.0, 100.0]], ["FUND"])
        >>> series = DatedSeries.from_json(perf.rolling_returns(0, window=2).to_json())
        >>> series.value_column
        'return'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Emits ``values`` and ``dates`` plus the ``value_column`` label, so the
        metric name survives a round-trip.

        Returns
        -------
        str
            Compact JSON string.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def values(self) -> npt.NDArray[np.float64]:
        """
        Rolling values, one per window.

        Returns
        -------
        npt.NDArray[np.float64]
            Metric values aligned with :attr:`dates`.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def dates(self) -> list[datetime.date]:
        """
        Window-end dates aligned 1:1 with :attr:`values`.

        Returns
        -------
        list[datetime.date]
            Date labels for each rolling window.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    @property
    def value_column(self) -> str:
        """
        Column name used by :meth:`to_dataframe`.

        Returns
        -------
        str
            Metric-specific column name (e.g. ``sharpe``, ``volatility``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def to_dataframe(self) -> pd.DataFrame:
        """
        Convert to a pandas DataFrame with date index and a value column.

        The column is named after :attr:`value_column` (e.g. ``sharpe``,
        ``sortino``, ``volatility``, or ``return``).

        Returns
        -------
        pd.DataFrame
            DataFrame with a date index and one column named
            :attr:`value_column`.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

# Performance engine

class Performance:
    """
    Stateful performance analytics engine over a panel of ticker series.

    Construct from a pandas DataFrame of prices (``Performance(df)``), a
    DataFrame of returns (``Performance.from_returns(df)``), or from raw
    arrays via :meth:`from_arrays` / :meth:`from_returns_arrays`.

    Scalar-per-ticker metrics return a ``pandas.Series`` indexed by ticker name
    and named after the metric, so results are selected by label rather than by
    column position, and ``pd.concat([perf.sharpe(), perf.sortino()], axis=1)``
    yields correctly-named columns.

    Examples
    --------
    >>> import pandas as pd
    >>> from finstack_quant.analytics import Performance
    >>> prices = pd.DataFrame(
    ...     {"FUND": [100.0, 101.0, 103.0]},
    ...     index=pd.to_datetime(["2024-01-01", "2024-01-02", "2024-01-03"]),
    ... )
    >>> Performance(prices).ticker_names
    ['FUND']
    """

    def __init__(
        self,
        prices: pd.DataFrame,
        benchmark_ticker: str | None = None,
        frequency: str = "daily",
    ) -> None:
        """
        Build from a pandas DataFrame of prices.

        Parameters
        ----------
        prices : pandas.DataFrame
            Price panel with a date-like index (``datetime.date`` or
            ``pd.Timestamp``) and one column per ticker.
        benchmark_ticker : str, optional
            Benchmark column name. Defaults to the first column when ``None``.
        frequency : str, optional
            Return aggregation frequency. One of ``"daily"``, ``"weekly"``,
            ``"monthly"``, ``"quarterly"``, ``"semiannual"``, or ``"annual"``.
            Default ``"daily"``.

        Raises
        ------
        AnalyticsError
            If ``prices`` is not a DataFrame, dates are invalid, or the panel is
            empty.
        TypeError
            If ``prices`` is not a pandas ``DataFrame`` (use
            :meth:`from_arrays` for raw lists).

        """

    @staticmethod
    def from_arrays(
        dates: Sequence[object],
        prices: list[list[float]],
        ticker_names: list[str],
        benchmark_ticker: str | None = None,
        frequency: str = "daily",
    ) -> Performance:
        """
        Construct from raw arrays (dates, prices matrix, ticker names).

        Parameters
        ----------
        dates : sequence
            Observation dates as ``datetime.date``, ``pd.Timestamp``, or ISO
            strings parseable by the binding layer.
        prices : list[list[float]]
            Column-major price matrix; ``prices[i]`` is the series for ticker
            ``ticker_names[i]``.
        ticker_names : list[str]
            Column labels, one per price series.
        benchmark_ticker : str, optional
            Benchmark ticker name. Defaults to the first column when ``None``.
        frequency : str, optional
            One of ``"daily"``, ``"weekly"``, ``"monthly"``, ``"quarterly"``,
            ``"semiannual"``, or ``"annual"``. Default ``"daily"``.

        Returns
        -------
        Performance
            Analytics engine over the supplied panel.

        Raises
        ------
        AnalyticsError
            If dimensions are inconsistent, dates are invalid, or ``frequency`` is
            unrecognized.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.analytics import Performance
        >>> perf = Performance.from_arrays(
        ...     [date(2024, 1, 1), date(2024, 1, 2), date(2024, 1, 3)],
        ...     [[100.0, 101.0, 103.0]],
        ...     ["FUND"],
        ... )
        >>> perf.ticker_names
        ['FUND']
        """

    @staticmethod
    def from_returns(
        returns: pd.DataFrame,
        benchmark_ticker: str | None = None,
        frequency: str = "daily",
    ) -> Performance:
        """
        Build from a pandas DataFrame of simple returns.

        Parameters
        ----------
        returns : pandas.DataFrame
            Simple-return panel aligned with a date-like index and one column per
            ticker (decimal returns, e.g. ``0.01`` for +1%).
        benchmark_ticker : str, optional
            Benchmark column name. Defaults to the first column when ``None``.
        frequency : str, optional
            One of ``"daily"``, ``"weekly"``, ``"monthly"``, ``"quarterly"``,
            ``"semiannual"``, or ``"annual"``. Default ``"daily"``.

        Raises
        ------
        AnalyticsError
            If ``returns`` is invalid or empty.
        TypeError
            If ``returns`` is not a pandas ``DataFrame`` (use
            :meth:`from_returns_arrays` for raw lists).

        Returns
        -------
        Performance
            Analytics engine over the supplied return panel.

        Examples
        --------
        >>> import pandas as pd
        >>> from finstack_quant.analytics import Performance
        >>> returns = pd.DataFrame(
        ...     {"FUND": [0.01, 0.02]},
        ...     index=pd.to_datetime(["2024-01-01", "2024-01-02"]),
        ... )
        >>> Performance.from_returns(returns).ticker_names
        ['FUND']
        """

    @staticmethod
    def from_returns_arrays(
        dates: Sequence[object],
        returns: list[list[float]],
        ticker_names: list[str],
        benchmark_ticker: str | None = None,
        frequency: str = "daily",
    ) -> Performance:
        """
        Construct from raw return arrays (dates, returns matrix, ticker names).

        Parameters
        ----------
        dates : sequence
            Return observation dates.
        returns : list[list[float]]
            Column-major simple-return matrix; ``returns[i]`` is the series for
            ``ticker_names[i]``.
        ticker_names : list[str]
            Column labels.
        benchmark_ticker : str, optional
            Benchmark ticker name.
        frequency : str, optional
            One of ``"daily"``, ``"weekly"``, ``"monthly"``, ``"quarterly"``,
            ``"semiannual"``, or ``"annual"``. Default ``"daily"``.

        Returns
        -------
        Performance
            Analytics engine over the supplied return panel.

        Raises
        ------
        AnalyticsError
            If dimensions are inconsistent or ``frequency`` is unrecognized.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.analytics import Performance
        >>> perf = Performance.from_returns_arrays([date(2024, 1, 1), date(2024, 1, 2)], [[0.01, 0.02]], ["FUND"])
        >>> perf.ticker_names
        ['FUND']
        """

    # -- Mutators --

    def reset_date_range(self, start: object, end: object) -> None:
        """
        Restrict analytics to ``[start, end]``.

        Parameters
        ----------
        start : object
            Start date as an object exposing integer ``year``, ``month``, and
            ``day`` attributes, such as ``datetime.date`` or ``pd.Timestamp``.
        end : object
            End date as an object exposing integer ``year``, ``month``, and
            ``day`` attributes, such as ``datetime.date`` or ``pd.Timestamp``.

        Raises
        ------
        TypeError
            If ``start`` or ``end`` is not a date-like object exposing integer
            ``year``, ``month``, and ``day`` attributes.
        ValueError
            If either object's components do not form a valid calendar date.
        """

    def reset_bench_ticker(self, ticker: str) -> None:
        """
        Change the benchmark ticker.

        Parameters
        ----------
        ticker : str
            New benchmark column name.

        Raises
        ------
        AnalyticsError
            If ``ticker`` does not match a loaded ticker name.
        """

    # -- Getters --

    @property
    def ticker_names(self) -> list[str]:
        """
        Ticker names in column order.

        Returns
        -------
        list[str]
            Column labels from the input panel.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def benchmark_idx(self) -> int:
        """
        Zero-based column index of the benchmark series.

        Returns
        -------
        int
            Zero-based column index of the benchmark series.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    @property
    def frequency(self) -> str:
        """
        Observation frequency as the canonical lowercase token.

        Returns
        -------
        str
            One of ``"daily"``, ``"weekly"``, ``"monthly"``, etc.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """

    def dates(self) -> list[datetime.date]:
        """
        Full return-aligned date grid (independent of any active window).

        Returns
        -------
        list[datetime.date]
            All observation dates in the panel.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def active_dates(self) -> list[datetime.date]:
        """
        Observation dates of the currently active analysis window.

        Returns
        -------
        list[datetime.date]
            Dates within the active ``[start, end]`` range.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def active_dates_for_ticker(self, ticker_idx: int) -> list[datetime.date]:
        """
        Observation dates for one ticker's active return series.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.

        Returns
        -------
        list[datetime.date]
            Dates where the specified ticker has valid returns.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    # -- Scalar-per-ticker methods --

    def cagr(
        self,
        day_count: str | DayCount | None = None,
        calendar_id: str | None = None,
    ) -> pd.Series:
        """
        Compound annual growth rate for each ticker.

        The default convention is Act/365.25. Pass ``"act365_25"`` for the
        same default, a core DayCount name such as ``"act_365f"`` or
        ``"bus_252"``, or a :class:`~finstack_quant.core.dates.DayCount`.
        ``bus_252`` requires ``calendar_id``.

        Parameters
        ----------
        day_count : str or DayCount, optional
            ``None`` / ``"act365_25"`` for Act/365.25, or a core DayCount
            name / instance (``"act_365f"``, ``DayCount.ACT_365F``,
            ``"bus_252"``, …).
        calendar_id : str, optional
            Holiday-calendar id required when ``day_count`` is Bus/252.

        Returns
        -------
        pd.Series
            Compound annual growth rate indexed by ticker name.

        Raises
        ------
        ValueError
            If ``day_count`` is not a recognized convention.
        KeyError
            If ``calendar_id`` is set but cannot be resolved.
        AnalyticsError
            If the active date window cannot be annualized, or Bus/252 is
            requested without a calendar.
        """

    def mean_return(self, annualize: bool = True) -> pd.Series:
        """
        Mean return for each ticker.

        Parameters
        ----------
        annualize : bool, default True
            Whether to annualize the mean return.

        Returns
        -------
        pd.Series
            Mean return indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.
        """

    def volatility(self, annualize: bool = True) -> pd.Series:
        """
        Volatility for each ticker.

        Parameters
        ----------
        annualize : bool, default True
            Whether to annualize the volatility.

        Returns
        -------
        pd.Series
            Standard deviation of returns indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.
        """

    def sharpe(self, risk_free_rate: float = 0.0) -> pd.Series:
        """
        Sharpe ratio for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        pd.Series
            Sharpe ratios over the active return window, indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Examples
        --------
        >>> from datetime import date
        >>> from finstack_quant.analytics import Performance
        >>> perf = Performance.from_returns_arrays([date(2024, 1, 1), date(2024, 1, 2)], [[0.01, 0.02]], ["FUND"])
        >>> sharpe = perf.sharpe()
        >>> (sharpe.name, list(sharpe.index))
        ('sharpe', ['FUND'])

        Sources
        -------
        - Sharpe (1966): see docs/REFERENCES.md#sharpe1966
        """

    def sortino(self, mar: float = 0.0) -> pd.Series:
        """
        Sortino ratio for each ticker.

        Parameters
        ----------
        mar : float, default 0.0
            Minimum acceptable return per period (not annualized).

        Returns
        -------
        pd.Series
            Sortino ratios over the active return window, indexed by ticker name.

        Notes
        -----
        ``mar`` is per-period; Sharpe ``risk_free_rate`` inputs are annualized.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Sortino and van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
        """

    def calmar(self) -> pd.Series:
        """
        Calmar ratio for each ticker over the active window.

        This is CAGR / |max drawdown| on the loaded (or reset) date range,
        not Young's 36-month CTA definition.

        Returns
        -------
        pd.Series
            CAGR divided by absolute max drawdown indexed by ticker name.

        Raises
        ------
        ValueError
            If the active date window cannot be annualized.

        Sources
        -------
        - Young (1991): see docs/REFERENCES.md#youngCalmar1991
        """

    def max_drawdown(self) -> pd.Series:
        """
        Max drawdown for each ticker.

        Returns
        -------
        pd.Series
            Peak-to-trough drawdown (negative), indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def mean_drawdown(self) -> pd.Series:
        """
        Mean drawdown (path-weighted average) for each ticker.

        Returns
        -------
        pd.Series
            Average drawdown (negative), indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def value_at_risk(self, confidence: float = 0.95) -> pd.Series:
        """
        Historical VaR for each ticker.

        Parameters
        ----------
        confidence : float, default 0.95
            Confidence level (e.g. ``0.95`` for 95% VaR).

        Returns
        -------
        pd.Series
            Historical VaR (negative), indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - J.P. Morgan RiskMetrics (1996): see docs/REFERENCES.md#jpmorgan1996RiskMetrics
        """

    def expected_shortfall(self, confidence: float = 0.95) -> pd.Series:
        """
        Expected Shortfall for each ticker.

        Parameters
        ----------
        confidence : float, default 0.95
            Confidence level.

        Returns
        -------
        pd.Series
            Expected shortfall (negative), indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Artzner, Delbaen, Eber, and Heath (1999): see
          docs/REFERENCES.md#artzner1999CoherentRisk
        """

    def tracking_error(self) -> pd.Series:
        """
        Tracking error for each ticker vs benchmark.

        Returns
        -------
        pd.Series
            Annualized standard deviation of excess returns indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Grinold and Kahn (1999): see docs/REFERENCES.md#grinoldKahn1999ActivePortfolio
        """

    def information_ratio(self) -> pd.Series:
        """
        Information ratio for each ticker vs benchmark.

        Returns
        -------
        pd.Series
            Annualized excess return divided by tracking error indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Grinold and Kahn (1999): see docs/REFERENCES.md#grinoldKahn1999ActivePortfolio
        """

    def skewness(self) -> pd.Series:
        """
        Skewness for each ticker.

        Returns
        -------
        pd.Series
            Third moment of returns indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Joanes and Gill (1998): see docs/REFERENCES.md#joanesGill1998
        """

    def kurtosis(self) -> pd.Series:
        """
        Kurtosis for each ticker.

        Returns
        -------
        pd.Series
            Fourth moment of returns indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Joanes and Gill (1998): see docs/REFERENCES.md#joanesGill1998
        """

    def geometric_mean(self) -> pd.Series:
        """
        Geometric mean for each ticker.

        Returns
        -------
        pd.Series
            Geometric mean return indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def downside_deviation(self, mar: float = 0.0) -> pd.Series:
        """
        Downside deviation for each ticker.

        ``mar`` is per-period; Sharpe risk-free inputs are annualized.

        Parameters
        ----------
        mar : float, default 0.0
            Minimum acceptable return per period.

        Returns
        -------
        pd.Series
            Downside deviation indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.
        """

    def max_drawdown_duration(self) -> pd.Series:
        """
        Max drawdown duration (calendar days) for each ticker.

        Returns
        -------
        pd.Series
            Longest drawdown duration in calendar days, indexed by ticker name
            and kept at an integer dtype.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def up_capture(self) -> pd.Series:
        """
        Empyrical-style annualized geometric up-capture vs benchmark.

        Returns
        -------
        pd.Series
            Up-capture ratio indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def down_capture(self) -> pd.Series:
        """
        Empyrical-style annualized geometric down-capture vs benchmark.

        Returns
        -------
        pd.Series
            Down-capture ratio indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def capture_ratio(self) -> pd.Series:
        """
        Empyrical-style annualized geometric capture ratio vs benchmark.

        Returns
        -------
        pd.Series
            Up-capture divided by down-capture indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def omega_ratio(self, threshold: float = 0.0) -> pd.Series:
        """
        Omega ratio for each ticker.

        Parameters
        ----------
        threshold : float, default 0.0
            Return threshold for the gain/loss split.

        Returns
        -------
        pd.Series
            Omega ratio indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Keating and Shadwick (2002): see docs/REFERENCES.md#keatingShadwick2002
        """

    def treynor(self, risk_free_rate: float = 0.0) -> pd.Series:
        """
        Treynor ratio for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        pd.Series
            Excess return per unit of beta indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Treynor (1965): see docs/REFERENCES.md#treynor1965
        """

    def gain_to_pain(self) -> pd.Series:
        """
        Gain-to-pain ratio for each ticker.

        Returns
        -------
        pd.Series
            Sum of gains divided by sum of absolute losses indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Schwager (2012): see docs/REFERENCES.md#schwager2012
        """

    def ulcer_index(self) -> pd.Series:
        """
        Ulcer index for each ticker.

        Returns
        -------
        pd.Series
            Root-mean-square of drawdown depths indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Martin (1987): see docs/REFERENCES.md#martinUlcer1987
        """

    def martin_ratio(self) -> pd.Series:
        """
        Martin ratio for each ticker.

        Returns
        -------
        pd.Series
            Excess return per unit of ulcer index indexed by ticker name.

        Raises
        ------
        ValueError
            If the active date window cannot be annualized.

        Sources
        -------
        - Martin (1987): see docs/REFERENCES.md#martinUlcer1987
        """

    def recovery_factor(self) -> pd.Series:
        """
        Recovery factor for each ticker.

        Returns
        -------
        pd.Series
            Total return divided by max drawdown indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def pain_index(self) -> pd.Series:
        """
        Pain index for each ticker.

        Returns
        -------
        pd.Series
            Average drawdown depth indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.

        Sources
        -------
        - Schwager (2012): see docs/REFERENCES.md#schwager2012
        """

    def pain_ratio(self, risk_free_rate: float = 0.0) -> pd.Series:
        """
        Pain ratio for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        pd.Series
            Excess return per unit of pain index indexed by ticker name.

        Raises
        ------
        ValueError
            If the active date window cannot be annualized.

        Sources
        -------
        - Schwager (2012): see docs/REFERENCES.md#schwager2012
        """

    def tail_ratio(self, confidence: float = 0.95) -> pd.Series:
        """
        Tail ratio for each ticker.

        Parameters
        ----------
        confidence : float, default 0.95
            Confidence level for the tail quantile.

        Returns
        -------
        pd.Series
            Right-tail gain divided by left-tail loss indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Notes
        -----
        The upper tail uses the ``confidence`` quantile and the lower tail uses
        ``1 - confidence``. A zero lower tail with a positive upper tail returns
        ``inf``; both tails zero returns ``NaN``.
        """

    def r_squared(self) -> pd.Series:
        """
        R-squared for each ticker vs benchmark.

        Returns
        -------
        pd.Series
            Coefficient of determination indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def batting_average(self) -> pd.Series:
        """
        Batting average for each ticker vs benchmark.

        Returns
        -------
        pd.Series
            Fraction of periods where the ticker outperformed the benchmark,
            indexed by ticker name.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """

    def parametric_var(
        self,
        confidence: float = 0.95,
        horizon_periods: float | None = None,
    ) -> pd.Series:
        """
        Equal-weight Gaussian VaR for each ticker.

        ``horizon_periods=None`` is one-period VaR. A positive ``h`` scales
        the mean by ``h`` and volatility by ``sqrt(h)``. Empty or invalid
        series return ``NaN``.

        Parameters
        ----------
        confidence : float, default 0.95
            Tail confidence as a decimal probability.
        horizon_periods : float, optional
            Horizon in observation periods. ``None`` is one period.

        Returns
        -------
        pd.Series
            Parametric VaR (negative), indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.
        """

    def cornish_fisher_var(
        self,
        confidence: float = 0.95,
        horizon_periods: float | None = None,
    ) -> pd.Series:
        """
        Cornish-Fisher VaR for each ticker.

        ``horizon_periods=None`` is one-period VaR. A positive ``h`` scales
        the Cornish–Fisher moments to that horizon. Empty or invalid series
        return ``NaN``.

        Parameters
        ----------
        confidence : float, default 0.95
            Tail confidence as a decimal probability.
        horizon_periods : float, optional
            Horizon in observation periods. ``None`` is one period.

        Returns
        -------
        pd.Series
            Cornish-Fisher modified VaR (negative), indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Cornish and Fisher (1937): see docs/REFERENCES.md#cornishFisher1937
        """

    def cdar(self, confidence: float = 0.95) -> pd.Series:
        """
        Conditional drawdown at risk for each ticker.

        Parameters
        ----------
        confidence : float, default 0.95
            Confidence level.

        Returns
        -------
        pd.Series
            Conditional drawdown-at-risk (negative), indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Chekhlov, Uryasev, and Zabarankin (2005): see docs/REFERENCES.md#chekhlov2005
        """

    def m_squared(self, risk_free_rate: float = 0.0) -> pd.Series:
        """
        M-squared for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        pd.Series
            M-squared measure indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Modigliani and Modigliani (1997): see docs/REFERENCES.md#modigliani1997
        """

    def modified_sharpe(
        self,
        risk_free_rate: float = 0.0,
        confidence: float = 0.95,
    ) -> pd.Series:
        """
        Modified Sharpe ratio for each ticker.

        The numerator is annualized excess return and the denominator is
        Cornish-Fisher VaR at the corresponding annual horizon. The panel
        frequency supplies the periods-per-year scaling for both terms,
        including the horizon decay of skewness and excess kurtosis; this
        method does not divide an annualized numerator by one-period VaR.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal, decompounded to the panel
            frequency before constructing annualized excess return.
        confidence : float, default 0.95
            Confidence level for annual-horizon Cornish-Fisher VaR.

        Returns
        -------
        pd.Series
            Modified Sharpe ratio indexed by ticker name.

        Raises
        ------
        ValueError
            If the computed values cannot be wrapped as a labelled pandas object.

        Sources
        -------
        - Gregoriou and Gueyie (2003): see docs/REFERENCES.md#gregoriou2003
        """

    def sterling_ratio(self, risk_free_rate: float = 0.0, n: int = 5) -> pd.Series:
        """
        Sterling ratio for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.
        n : int, default 5
            Number of largest drawdowns to average.

        Returns
        -------
        pd.Series
            Sterling ratio indexed by ticker name.

        Raises
        ------
        ValueError
            If the active date window cannot be annualized.

        Sources
        -------
        - Kestner (1996): see docs/REFERENCES.md#kestner1996
        """

    def burke_ratio(self, risk_free_rate: float = 0.0, n: int = 5) -> pd.Series:
        """
        Burke ratio for each ticker.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.
        n : int, default 5
            Number of largest drawdowns to use.

        Returns
        -------
        pd.Series
            Burke ratio indexed by ticker name.

        Raises
        ------
        ValueError
            If the active date window cannot be annualized.

        Sources
        -------
        - Burke (1994): see docs/REFERENCES.md#burke1994
        """

    # -- Vector-per-ticker methods --

    def returns(self) -> list[list[float]]:
        """
        Per-period simple returns for each ticker.

        Canonical accessor for the raw return panel over the active window.
        Prefer this over :meth:`excess_returns` with an all-zero risk-free
        series or un-compounding :meth:`cumulative_returns`. Series are
        span-aware and therefore ragged across tickers on edge-ragged panels.

        Returns
        -------
        list[list[float]]
            Per-ticker simple return series as decimal fractions
            (``0.01`` for ``+1%``), in date order.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def returns_for_ticker(self, ticker_idx: int) -> list[float]:
        """
        Per-period simple returns for a single ticker.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.

        Returns
        -------
        list[float]
            Simple return series as decimal fractions (``0.01`` for ``+1%``),
            in date order, spanning that ticker's active dates.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def cumulative_returns(self) -> list[list[float]]:
        """
        Cumulative returns for each ticker.

        Returns
        -------
        list[list[float]]
            Per-ticker cumulative return time series.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def drawdown_series(self) -> list[list[float]]:
        """
        Drawdown series for each ticker.

        Returns
        -------
        list[list[float]]
            Per-ticker drawdown time series (negative or zero).

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def correlation_matrix(self) -> list[list[float]]:
        """
        Correlation matrix across all tickers.

        Uses the complete-case common window when every ticker has at least
        two overlapping points; otherwise pairwise intersecting spans. The
        matrix is Higham-repaired to the nearest correlation matrix.

        Returns
        -------
        list[list[float]]
            Symmetric correlation matrix indexed by ticker column order.

        Raises
        ------
        AnalyticsError
            If a pair is degenerate (zero variance or non-finite) or Higham
            repair fails.
        """

    def cumulative_returns_outperformance(self) -> list[list[float]]:
        """
        Cumulative returns outperformance vs benchmark.

        Returns
        -------
        list[list[float]]
            Per-ticker cumulative excess return time series.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def drawdown_difference(self) -> list[list[float]]:
        """
        Drawdown difference vs benchmark.

        Returns
        -------
        list[list[float]]
            Per-ticker drawdown difference time series.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def excess_returns(
        self,
        rf: list[float],
        nperiods: float | None = None,
    ) -> list[list[float]]:
        """
        Excess returns over a risk-free series aligned to the panel grid.

        Each ticker subtracts ``rf[panel_index]`` on its active span.
        ``nperiods=None`` geometrically decompounds an annual series using
        the engine frequency; pass ``1.0`` when ``rf`` is already periodic.

        Parameters
        ----------
        rf : list[float]
            Risk-free series with one value per active panel date.
        nperiods : float, optional
            Periods per year used to decompound annual ``rf``. ``None`` uses
            the engine frequency; ``1.0`` treats ``rf`` as already periodic.

        Returns
        -------
        list[list[float]]
            Per-ticker excess return series.

        Raises
        ------
        AnalyticsError
            If ``len(rf)`` differs from the number of active panel dates.
        """

    # -- Per-ticker structured methods --

    def beta(self) -> list[BetaResult]:
        """
        Beta for each ticker vs benchmark.

        Returns
        -------
        list[BetaResult]
            Per-ticker :class:`BetaResult` with CI.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def greeks(self, risk_free_rate: float = 0.0) -> list[GreeksResult]:
        """
        Greeks (annualized Jensen alpha, beta, R²) for each ticker vs benchmark.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        list[GreeksResult]
            Per-ticker :class:`GreeksResult`.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """

    def rolling_greeks(
        self,
        ticker_idx: int,
        window: int = 63,
        risk_free_rate: float = 0.0,
    ) -> RollingGreeks:
        """
        Rolling greeks for a specific ticker.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        window : int, default 63
            Rolling window size in observations.
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        RollingGreeks
            :class:`RollingGreeks` with dates, alphas, and betas.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def rolling_volatility(self, ticker_idx: int, window: int = 63) -> DatedSeries:
        """
        Rolling volatility for a specific ticker (column name ``volatility``).

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        window : int, default 63
            Rolling window size in observations.

        Returns
        -------
        DatedSeries
            :class:`DatedSeries` with ``value_column="volatility"``.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def rolling_sortino(self, ticker_idx: int, window: int = 63, mar: float = 0.0) -> DatedSeries:
        """
        Rolling Sortino for a specific ticker (column name ``sortino``).

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        window : int, default 63
            Rolling window size in observations.
        mar : float, default 0.0
            Minimum acceptable return per period.

        Returns
        -------
        DatedSeries
            :class:`DatedSeries` with ``value_column="sortino"``.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def rolling_sharpe(
        self,
        ticker_idx: int,
        window: int = 63,
        risk_free_rate: float = 0.0,
    ) -> DatedSeries:
        """
        Rolling Sharpe for a specific ticker (column name ``sharpe``).

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        window : int, default 63
            Rolling window size in observations.
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.

        Returns
        -------
        DatedSeries
            :class:`DatedSeries` with ``value_column="sharpe"``.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def rolling_returns(self, ticker_idx: int, window: int) -> DatedSeries:
        """
        Rolling N-period compounded total return (column name ``return``).

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        window : int
            Rolling window size in observations.

        Returns
        -------
        DatedSeries
            :class:`DatedSeries` with ``value_column="return"``.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def drawdown_details(self, ticker_idx: int, n: int = 5) -> list[DrawdownEpisode]:
        """
        Top-N drawdown episodes for a specific ticker.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        n : int, default 5
            Number of largest drawdown episodes to return.

        Returns
        -------
        list[DrawdownEpisode]
            List of :class:`DrawdownEpisode` objects, deepest first.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """

    def multi_factor_greeks(
        self,
        ticker_idx: int,
        factor_returns: list[list[float]],
        return_kind: str = "excess",
        risk_free_rate: float = 0.0,
    ) -> MultiFactorResult:
        """
        Multi-factor regression for a specific ticker.

        Factor series are already-excess (Fama–French style).
        ``return_kind="excess"`` leaves the ticker series unchanged.
        ``return_kind="total"`` subtracts the geometrically decompounded
        period risk-free rate from the ticker series only.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        factor_returns : list[list[float]]
            Already-excess factor return matrix; ``factor_returns[i]`` is the
            return series for factor ``i``.
        return_kind : str, default ``"excess"``
            ``"excess"`` or ``"total"``.
        risk_free_rate : float, default 0.0
            Annualized decimal risk-free rate used when ``return_kind`` is
            ``"total"``.

        Returns
        -------
        MultiFactorResult
            :class:`MultiFactorResult` with alpha, betas, and fit statistics.

        Raises
        ------
        ValueError
            If ``return_kind`` is not ``"excess"`` or ``"total"``.
        AnalyticsError
            If ``ticker_idx`` is out of range, no factors are supplied, factor
            lengths differ from the ticker return series, returns are
            non-finite, observations are insufficient, or the regression is
            numerically singular.
        """

    def lookback_returns(
        self,
        ref_date: object,
        fiscal_year_start_month: int | None = None,
        fiscal_year_start_day: int | None = None,
    ) -> LookbackReturns:
        """
        Period-to-date lookback returns.

        Defaults to a January-1 fiscal-year start. FYTD is the first
        observation on or after that fiscal calendar start through
        ``ref_date``. Holidays are not skipped. The first included simple
        return still spans the prior close.

        Parameters
        ----------
        ref_date : object
            Reference date (``datetime.date``, ``pd.Timestamp``, or ISO string).
        fiscal_year_start_month : int, optional
            Fiscal year start month in ``1..=12``.
        fiscal_year_start_day : int, optional
            Fiscal year start day in ``1..=31``.

        Returns
        -------
        LookbackReturns
            :class:`LookbackReturns` with mandatory MTD, QTD, YTD, and FYTD
            vectors, each containing one value per ticker.

        Raises
        ------
        ValueError
            If *fiscal_year_start_month* is not in ``1..=12`` or
            *fiscal_year_start_day* is not in ``1..=31``.
        """

    def period_stats(
        self,
        ticker_idx: int,
        aggregation_frequency: str = "monthly",
        fiscal_year_start_month: int | None = None,
        fiscal_year_start_day: int | None = None,
    ) -> PeriodStats:
        """
        Period statistics for one ticker at a given aggregation frequency.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        aggregation_frequency : str, default "monthly"
            Aggregation frequency (``"daily"``, ``"weekly"``, ``"monthly"``,
            ``"quarterly"``, ``"annual"``).
        fiscal_year_start_month : int, optional
            Fiscal year start month in ``1..=12``.
        fiscal_year_start_day : int, optional
            Fiscal year start day in ``1..=31``.

        Returns
        -------
        PeriodStats
            :class:`PeriodStats` with win/loss streaks, ratios, etc.

        Raises
        ------
        ValueError
            If *fiscal_year_start_month* is not in ``1..=12`` or
            *fiscal_year_start_day* is not in ``1..=31``.
        """

    def periodic_returns(
        self,
        frequency: str = "monthly",
    ) -> list[list[tuple[datetime.date, float]]]:
        """
        Calendar-bucketed compounded returns for all tickers.

        Parameters
        ----------
        frequency : str, default "monthly"
            Calendar-bucketing frequency: one of ``"daily"``, ``"weekly"``,
            ``"monthly"``, ``"quarterly"``, ``"semiannual"``, or
            ``"annual"``.

        Returns
        -------
        list[list[tuple[datetime.date, float]]]
            Ticker-major panel in :attr:`ticker_names` order. Each inner list
            contains chronological ``(period_end_date, compounded_return)``
            points. Returns are simple decimal fractions (``0.01`` means 1%);
            chaining one ticker's points reconciles with the final value from
            :meth:`cumulative_returns`.

        Raises
        ------
        ValueError
            If ``frequency`` is not a supported token.

        Examples
        --------
        >>> from datetime import date
        >>> perf = Performance.from_returns_arrays(
        ...     [date(2024, 1, 1), date(2024, 1, 2)],
        ...     [[0.01, 0.02]],
        ...     ["FUND"],
        ... )
        >>> panel = perf.periodic_returns()
        >>> (len(panel), panel[0][0][0], round(panel[0][0][1], 4))
        (1, datetime.date(2024, 1, 2), 0.0302)
        """

    # -- DataFrame export methods --

    def to_dataframe(self) -> pd.DataFrame:
        """
        The primary pandas view: the summary statistics table.

        One row per ticker, one column per scalar metric. Alias for
        `to_summary_dataframe` with default arguments, so every result type
        answers to a plain `to_dataframe()`. The other `*_to_dataframe`
        methods are the secondary views.

        Returns
        -------
        pd.DataFrame
            Summary statistics, one row per ticker.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_summary_dataframe(
        self,
        risk_free_rate: float = 0.0,
        confidence: float = 0.95,
    ) -> pd.DataFrame:
        """
        Summary statistics for all tickers as a pandas DataFrame.

        *risk_free_rate* affects only the ``sharpe`` column; the MAR-based
        metrics (``sortino``, ``downside_deviation``) and the ``omega_ratio``
        threshold are fixed at ``0.0``. *confidence* applies to
        ``value_at_risk``, ``expected_shortfall``, and ``tail_ratio``.

        Parameters
        ----------
        risk_free_rate : float, default 0.0
            Annualized risk-free rate as a decimal.
        confidence : float, default 0.95
            Confidence level for VaR, ES, and tail ratio.

        Returns
        -------
        pd.DataFrame
            Summary statistics indexed by ticker name.

        Raises
        ------
        AnalyticsError
            If a ticker's active range has no positive holding period and
            therefore cannot be annualized.
        """
        ...

    def to_returns_dataframe(self) -> pd.DataFrame:
        """
        Per-period simple returns for all tickers as a pandas DataFrame.

        Ragged per-ticker series are padded with ``NaN`` onto the active date
        grid. Prefer this over :meth:`excess_returns` with an all-zero
        risk-free series or un-compounding
        :meth:`to_cumulative_returns_dataframe`.

        Returns
        -------
        pd.DataFrame
            Simple returns indexed by date, one column per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_cumulative_returns_dataframe(self) -> pd.DataFrame:
        """
        Cumulative returns for all tickers as a pandas DataFrame.

        Returns
        -------
        pd.DataFrame
            Cumulative returns indexed by date, one column per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_periodic_returns_dataframe(self, frequency: str = "monthly") -> pd.DataFrame:
        """
        Calendar-bucketed compounded returns for all tickers.

        Parameters
        ----------
        frequency : str, default "monthly"
            Bucketing frequency: one of ``"daily"``, ``"weekly"``,
            ``"monthly"``, ``"quarterly"``, ``"semiannual"``, ``"annual"``.

        Returns
        -------
        pd.DataFrame
            Compounded period returns indexed by period-end date, one column
            per ticker. Buckets reconcile with
            :meth:`to_cumulative_returns_dataframe`. This convenience exit is
            built from the same canonical result as :meth:`periodic_returns`.

        Raises
        ------
        ValueError
            If ``frequency`` is not a recognized frequency.
        """
        ...

    def to_drawdown_series_dataframe(self) -> pd.DataFrame:
        """
        Drawdown series for all tickers as a pandas DataFrame.

        Returns
        -------
        pd.DataFrame
            Drawdown series indexed by date, one column per ticker.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def to_correlation_dataframe(self) -> pd.DataFrame:
        """
        Correlation matrix as a pandas DataFrame indexed by ticker name.

        Returns
        -------
        pd.DataFrame
            Symmetric correlation matrix with ticker names on both axes.

        Raises
        ------
        AnalyticsError
            If a pair is degenerate or Higham repair fails.
        """
        ...

    def to_drawdown_details_dataframe(
        self,
        ticker_idx: int,
        n: int = 5,
    ) -> pd.DataFrame:
        """
        Top-N drawdown episodes for a ticker as a pandas DataFrame.

        Columns: ``start``, ``valley``, ``end``, ``duration_days``,
        ``max_drawdown``, ``near_recovery_threshold``, ``truncated_at_start``.

        Parameters
        ----------
        ticker_idx : int
            Zero-based ticker column index.
        n : int, default 5
            Number of largest drawdown episodes to return.

        Returns
        -------
        pd.DataFrame
            Drawdown episodes, one row per episode, deepest first.

        Raises
        ------
        AnalyticsError
            If ``ticker_idx`` is outside the loaded ticker columns.
        """
        ...

    def to_lookback_returns_dataframe(
        self,
        ref_date: object,
        fiscal_year_start_month: int | None = None,
        fiscal_year_start_day: int | None = None,
    ) -> pd.DataFrame:
        """
        Period-to-date lookback returns as a pandas DataFrame.

        Indexed by ticker name with columns ``mtd``, ``qtd``, ``ytd``,
        and ``fytd``. See :meth:`lookback_returns` for the FYTD fiscal-start
        semantics.

        Parameters
        ----------
        ref_date : object
            Reference date.
        fiscal_year_start_month : int, optional
            Fiscal year start month in ``1..=12``.
        fiscal_year_start_day : int, optional
            Fiscal year start day in ``1..=31``.

        Returns
        -------
        pd.DataFrame
            Lookback returns indexed by ticker name.

        Raises
        ------
        ValueError
            If *fiscal_year_start_month* is not in ``1..=12`` or
            *fiscal_year_start_day* is not in ``1..=31``.
        """
        ...

def constrained_least_squares(
    exposures: list[float],
    n_factors: int,
    returns: list[float],
    weights: list[float],
) -> list[float]:
    """
    Fit factor returns satisfying the equality constraint ``w'Xf = w'r``.

    Binds Rust
    ``finstack_quant_analytics::regression::constrained_least_squares``: adds
    the minimal Lagrangian correction to an unconstrained OLS fit so the
    corrected factor returns exactly reproduce the weighted realized return
    ``w'r``. Typically used to fit the benchmark factor returns consumed by
    :func:`finstack_quant.portfolio.factor_brinson_attribution`, which
    requires factor returns satisfying that same completeness condition.

    Parameters
    ----------
    exposures : list[float]
        Row-major factor exposure matrix, ``n_assets x n_factors``: asset
        ``i``'s exposure to factor ``j`` is ``exposures[i * n_factors + j]``.
    n_factors : int
        Number of factor columns in ``exposures``; must be a positive integer
        representable as the platform's unsigned pointer-sized integer.
    returns : list[float]
        Realized asset returns, length ``n_assets`` (defines ``n_assets``).
    weights : list[float]
        Holding weights whose weighted return ``w'r`` must be fully
        reproduced by ``w'Xf`` (e.g. benchmark weights for a
        benchmark-return attribution); length ``n_assets``.

    Returns
    -------
    list[float]
        Constrained factor returns ``f``, one per factor, satisfying
        ``w'Xf = w'r`` to numerical precision.

    Raises
    ------
    AnalyticsError
        If ``n_factors`` is zero, ``returns`` is empty, ``exposures`` or
        ``weights`` has the wrong length, ``n_assets * n_factors`` overflows,
        any input value is non-finite, the design matrix is rank-deficient,
        coefficient rescaling or the constraint correction produces a
        non-finite value, or ``w`` and the constraint direction are
        numerically orthogonal while OLS does not already satisfy the
        constraint, so no scalar correction can restore it.
    TypeError
        If ``n_factors`` is not an integer.
    OverflowError
        If ``n_factors`` is negative or exceeds the platform's unsigned
        pointer-sized integer range.

    Sources
    -------
    See ``docs/REFERENCES.md#jeet-partani-2023``.

    Examples
    --------
    >>> from finstack_quant.analytics import constrained_least_squares
    >>> exposures = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0]
    >>> returns = [0.05, 0.02, 0.01]
    >>> weights = [0.6, 0.3, 0.1]
    >>> f = constrained_least_squares(exposures, 2, returns, weights)
    >>> len(f)
    2
    """
    ...
