"""
Statement analysis: sensitivity, variance, scenarios, backtesting, goal seek, DCF, corporate, reports, introspection.

Examples
--------
>>> from finstack_quant.statements_analytics import backtest_forecast
>>> backtest_forecast([1.0, 2.0], [1.0, 2.5])["n"]
2

"""

from __future__ import annotations

import datetime
from typing import Any

import pandas as pd

from finstack_quant.statements import CheckReport, FinancialModelSpec, StatementResult
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.table import ArrowTable

__all__ = [
    "SensitivityConfig",
    "VarianceConfig",
    "ScenarioSet",
    "SensitivityResult",
    "TornadoEntry",
    "VarianceRow",
    "VarianceReport",
    "ScenarioResults",
    "ScenarioDiff",
    "BridgeStep",
    "BridgeChart",
    "run_sensitivity",
    "generate_tornado_entries",
    "run_variance",
    "evaluate_scenario_set",
    "scenario_diff",
    "variance_bridge",
    "backtest_forecast",
    "goal_seek",
    "evaluate_dcf",
    "dcf_sensitivity",
    "evaluate_lbo",
    "wacc",
    "run_corporate_analysis",
    "pl_summary_report",
    "credit_assessment_report",
    "credit_assessment",
    "DependencyTracer",
    "direct_dependencies",
    "all_dependencies",
    "dependents",
    "explain_formula",
    "explain_formula_text",
    "run_checks",
    "run_three_statement_checks",
    "run_credit_underwriting_checks",
    "render_check_report_text",
    "render_check_report_html",
    "Exposure",
    "classify_stage",
    "compute_ecl",
    "compute_ecl_weighted",
    "percentile_rank",
    "z_score",
    "peer_stats",
    "regression_fair_value",
    "compute_multiple",
    "score_relative_value",
    # Credit scorecard extension
    "ScorecardMetric",
    "ScorecardConfig",
    "ScorecardReport",
    "CreditScorecardExtension",
    "validate_scorecard_config",
    # Corkscrew extension
    "AccountType",
    "CorkscrewAccount",
    "CorkscrewConfig",
    "CorkscrewReport",
    "CorkscrewExtension",
    # Vintage template
    "add_vintage_buildup",
    # Roll-forward template
    "add_roll_forward",
    "add_roll_forward_with_opening",
    # Real-estate template
    "SimpleLeaseSpec",
    "RentStepSpec",
    "FreeRentWindowSpec",
    "RenewalSpec",
    "LeaseGrowthConvention",
    "LeaseSpec",
    "RentRollOutputNodes",
    "ManagementFeeBase",
    "ManagementFeeSpec",
    "PropertyTemplateNodes",
    "add_noi_buildup",
    "add_ncf_buildup",
    "add_rent_roll",
    "add_rent_roll_rental_revenue",
    "add_property_operating_statement",
]

class SensitivityConfig:
    """
    Configure deterministic sensitivity scenarios for a statement model.

    Parameters
    ----------
    mode : str
        Scenario-construction mode accepted by the Rust sensitivity engine.
    parameters : list[tuple[str, str, float, list[float]]]
        Node-and-period shock specifications, including the base value and
        ordered values to evaluate; defaults to no parameter shocks.
    target_metrics : list[str]
        Output node IDs to collect for every generated scenario; defaults to
        an empty result selection.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import SensitivityConfig
    >>> config = SensitivityConfig("Diagonal", [], ["profit"])
    >>> (config.parameter_count, config.target_metrics)
    (0, ['profit'])

    """
    def __init__(
        self,
        mode: str,
        parameters: list[tuple[str, str, float, list[float]]] = ...,
        target_metrics: list[str] = ...,
    ) -> None:
        """
        Define parameter shocks and statement outputs for a sensitivity run.

        Parameters
        ----------
        mode : str
            Scenario-construction mode accepted by the Rust sensitivity engine,
            such as ``"Diagonal"``.
        parameters : list[tuple[str, str, float, list[float]]]
            Tuples of node ID, model period, base value, and ordered shocked
            values; an empty list produces no parameter shocks.
        target_metrics : list[str]
            Statement node IDs collected for every generated scenario.

        Raises
        ------
        ValueError
            If mode is unknown or a sensitivity-parameter period cannot be parsed.

        """
        ...

    @staticmethod
    def from_json(json: str) -> SensitivityConfig:
        """
        Deserialize a sensitivity configuration from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload produced by ``to_json`` or following the
            ``SensitivityConfig`` schema.

        Returns
        -------
        SensitivityConfig
            Validated `SensitivityConfig` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import SensitivityConfig
        >>> config = SensitivityConfig("Diagonal", [], ["profit"])
        >>> SensitivityConfig.from_json(config.to_json()).target_metrics
        ['profit']

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `SensitivityConfig` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `SensitivityConfig`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def mode(self) -> str:
        """
        Analysis mode: ``"Diagonal"``, ``"FullGrid"``, or ``"Tornado"``.

        Returns
        -------
        str
            Analysis mode name.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def target_metrics(self) -> list[str]:
        """
        Node identifiers of the statement metrics tracked across scenarios.

        Returns
        -------
        list[str]
            Node identifiers of the tracked target metrics.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def parameter_count(self) -> int:
        """
        Number of configured parameters (one ``ParameterSpec`` per entry).

        Returns
        -------
        int
            Count of configured parameters.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class VarianceConfig:
    """
    Define the labels, metrics, and periods for a variance comparison.

    Parameters
    ----------
    baseline_label : str
        Reader-facing label for the baseline statement result.
    comparison_label : str
        Reader-facing label for the statement result compared with baseline.
    metrics : list[str]
        Statement node IDs whose absolute and percentage variances are shown.
    periods : list[str]
        Model period labels to include in the variance report, in report order.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import VarianceConfig
    >>> config = VarianceConfig("base", "case", ["profit"], ["2025Q1"])
    >>> (config.baseline_label, config.periods)
    ('base', ['2025Q1'])

    """
    def __init__(
        self,
        baseline_label: str,
        comparison_label: str,
        metrics: list[str],
        periods: list[str],
    ) -> None:
        """
        Select the labels, metrics, and periods for a variance report.

        Parameters
        ----------
        baseline_label : str
            Reader-facing name for the baseline statement result.
        comparison_label : str
            Reader-facing name for the result compared with the baseline.
        metrics : list[str]
            Statement node IDs whose absolute and percentage variances are reported.
        periods : list[str]
            Parseable model-period labels included in report order.

        Raises
        ------
        ValueError
            If any requested period label cannot be parsed.

        """
        ...

    @staticmethod
    def from_json(json: str) -> VarianceConfig:
        """
        Deserialize a variance configuration from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload describing the baseline, comparison, metrics, and
            periods to report.

        Returns
        -------
        VarianceConfig
            Validated `VarianceConfig` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import VarianceConfig
        >>> config = VarianceConfig("base", "case", ["profit"], ["2025Q1"])
        >>> VarianceConfig.from_json(config.to_json()).comparison_label
        'case'

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `VarianceConfig` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `VarianceConfig`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def baseline_label(self) -> str:
        """
        Label for the baseline scenario (e.g. ``"management_case"``).

        Returns
        -------
        str
            Baseline scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison_label(self) -> str:
        """
        Label for the comparison scenario (e.g. ``"bank_case"``).

        Returns
        -------
        str
            Comparison scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def metrics(self) -> list[str]:
        """
        Node identifiers of the metrics compared between the two scenarios.

        Returns
        -------
        list[str]
            Node identifiers of the compared metrics.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def periods(self) -> list[str]:
        """
        Periods to compare, as period-id strings (e.g. ``"2025Q1"``).

        Returns
        -------
        list[str]
            Period-id strings covered by the comparison.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class ScenarioSet:
    """
    Name statement-model scenarios and optional parent/model relationships.

    Parameters
    ----------
    scenarios : dict[str, dict[str, float | Money]]
        Mapping from scenario name to typed node overrides. Monetary nodes
        require ``Money`` in the node currency; scalar nodes require ``float``.
    parents : dict[str, str] or None
        Optional mapping from scenario to inherited parent scenario; omitted
        scenarios have no parent.
    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScenarioSet
    >>> ScenarioSet({"base": {}, "downside": {"revenue": 90.0}}).names
    ['base', 'downside']

    """
    def __init__(
        self,
        scenarios: dict[str, dict[str, float | Money]],
        parents: dict[str, str] | None = ...,
    ) -> None:
        """
        Define named overrides and optional scenario inheritance relationships.

        Parameters
        ----------
        scenarios : dict[str, dict[str, float | Money]]
            Typed scalar or monetary node overrides.
        parents : dict[str, str] or None, default None
            Optional mapping from each child scenario to its inherited parent.

        Raises
        ------
        TypeError
            If *scenarios* is not a mapping of names to node-override dicts,
            or *parents* is not a mapping of child names to parent names.
        """
        ...

    @staticmethod
    def from_json(json: str) -> ScenarioSet:
        """
        Deserialize a named scenario set from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing scenario overrides and optional hierarchy.

        Returns
        -------
        ScenarioSet
            Validated `ScenarioSet` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ScenarioSet
        >>> scenarios = ScenarioSet({"base": {}})
        >>> ScenarioSet.from_json(scenarios.to_json()).names
        ['base']

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `ScenarioSet` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ScenarioSet`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def trace(self, scenario: str) -> list[str]:
        """
        Resolve a scenario's inheritance lineage, root-first.

        Parameters
        ----------
        scenario : str
            Name of the scenario to trace.

        Returns
        -------
        list[str]
            Scenario names from the root ancestor through to *scenario*.

        Raises
        ------
        ValueError
            If the scenario is unknown or its parent chain contains a cycle.

        """
        ...

    @property
    def names(self) -> list[str]:
        """
        Scenario names in definition (insertion) order.

        Returns
        -------
        list[str]
            Scenario names in definition order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class TornadoEntry:
    """
    One parameter's downside and upside impact in a tornado chart.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import TornadoEntry
    >>> entry = TornadoEntry.from_json('{"parameter_id":"revenue","downside":-5.0,"upside":7.0}')
    >>> entry.swing
    12.0
    """

    @staticmethod
    def from_json(json: str) -> TornadoEntry:
        """Deserialize one tornado entry from canonical JSON.

        Parameters
        ----------
        json : str
            JSON object containing ``parameter_id``, ``downside``, and ``upside``.

        Returns
        -------
        TornadoEntry
            Typed tornado entry reconstructed from the wire representation.

        Raises
        ------
        ValueError
            If ``json`` is malformed or has an incompatible shape.

        Examples
        --------
        >>> TornadoEntry.from_json('{"parameter_id":"cost","downside":-2.0,"upside":3.0}').parameter_id
        'cost'
        """
        ...

    def to_json(self) -> str:
        """Serialize this entry to canonical JSON.

        Returns
        -------
        str
            Compact JSON containing the canonical Rust fields.

        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @property
    def parameter_id(self) -> str:
        """Return the parameter node identifier.

        Returns
        -------
        str
            Node identifier represented by this entry.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def downside(self) -> float:
        """Return the metric change at the minimum perturbation.

        Returns
        -------
        float
            Downside delta in the target metric's units.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def upside(self) -> float:
        """Return the metric change at the maximum perturbation.

        Returns
        -------
        float
            Upside delta in the target metric's units.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

    @property
    def swing(self) -> float:
        """Return ``upside - downside`` in target-metric units.

        Returns
        -------
        float
            Total tornado swing; values may be negative for nonstandard inputs.

        Raises
        ------
        None
            This property does not raise.
        """
        ...

class SensitivityResult:
    """
    Sensitivity-run result holding the config and generated scenario payloads.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import SensitivityConfig, SensitivityResult
    >>> payload = '{"config":' + SensitivityConfig("Diagonal").to_json() + ',"scenarios":[]}'
    >>> len(SensitivityResult.from_json(payload))
    0

    """

    @staticmethod
    def from_json(json: str) -> SensitivityResult:
        """
        Deserialize a sensitivity-analysis result from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload returned by ``run_sensitivity`` or an equivalent
            serialized Rust result.

        Returns
        -------
        SensitivityResult
            Validated `SensitivityResult` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import SensitivityConfig, SensitivityResult
        >>> payload = '{"config":' + SensitivityConfig("Diagonal").to_json() + ',"scenarios":[]}'
        >>> SensitivityResult.from_json(payload).target_metrics
        []

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `SensitivityResult` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `SensitivityResult`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def __len__(self) -> int: ...
    @property
    def target_metrics(self) -> list[str]:
        """
        Node identifiers of the metrics tracked by the originating config.

        Returns
        -------
        list[str]
            Node identifiers of the tracked target metrics.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def get_parameter_value(self, scenario_index: int, parameter: str) -> float | None:
        """
        Return one shocked parameter value for a generated scenario.

        Parameters
        ----------
        scenario_index : int
            Zero-based position of the generated scenario in result order.
        parameter : str
            Parameter identifier configured in the sensitivity specification.

        Returns
        -------
        float | None
            Shocked value of ``parameter`` in the selected scenario, or
            ``None`` when that parameter was not recorded.

        Raises
        ------
        IndexError
            If scenario_index is outside the available scenario range.

        """
        ...
    def get_value(self, scenario_index: int, node_id: str, period: str) -> float | None:
        """
        Return one scenario output value when it is available.

        Parameters
        ----------
        scenario_index : int
            Zero-based position of the generated scenario in result order.
        node_id : str
            Statement node ID whose simulated value is requested.
        period : str
            Model period label for the requested node value.

        Returns
        -------
        float | None
            Simulated value for ``node_id`` and ``period`` in the selected
            scenario, or ``None`` when no value was recorded.

        Raises
        ------
        IndexError
            If scenario_index is outside the available scenario range.
        ValueError
            If period cannot be parsed.

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the per-scenario parameter values as a pandas ``DataFrame``.

        Columns: ``scenario`` (0-based scenario index), plus one column per
        perturbed parameter, named exactly as the Rust result keys it
        (``node_id@period``). One row per generated scenario; a parameter a
        given scenario does not perturb is ``NaN``. An empty result still
        carries the ``scenario`` column.

        Scenario *outputs* are not included - read them per node and period
        with ``get_value``.

        Returns
        -------
        pd.DataFrame
            One row per generated scenario.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class VarianceRow:
    """
    One period/metric variance row comparing baseline and comparison values.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import VarianceReport
    >>> payload = (
    ...     '{"baseline_label":"base","comparison_label":"case","rows":['
    ...     '{"period":"2025Q1","metric":"revenue","baseline":100.0,'
    ...     '"comparison":110.0,"abs_var":10.0,"pct_var":0.1}]}'
    ... )
    >>> (VarianceReport.from_json(payload).rows[0].metric, VarianceReport.from_json(payload).rows[0].abs_var)
    ('revenue', 10.0)

    """

    @property
    def period(self) -> str:
        """
        Period this row covers, as a period-id string (e.g. ``"2025Q1"``).

        Returns
        -------
        str
            Period-id string for this row.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def metric(self) -> str:
        """
        Node identifier of the compared metric.

        Returns
        -------
        str
            Node identifier of the compared metric.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def baseline(self) -> float:
        """
        Metric value in the baseline scenario, in the metric's own units.

        Returns
        -------
        float
            Baseline value in the metric's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison(self) -> float:
        """
        Metric value in the comparison scenario, in the metric's own units.

        Returns
        -------
        float
            Comparison value in the metric's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def abs_var(self) -> float:
        """
        Absolute variance ``comparison - baseline``, in the metric's units.

        Returns
        -------
        float
            Absolute variance in the metric's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def pct_var(self) -> float | None:
        """
        Percentage variance ``abs_var / baseline`` as a decimal fraction (``0.1`` =
        +10%).

        ``None`` when the baseline is effectively zero, where a ratio would be undefined
        rather than zero; fall back to ``abs_var`` in that case.

        Returns
        -------
        float | None
            Percentage variance as a decimal fraction, or ``None`` on a near-zero
            baseline.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class VarianceReport:
    """
    Variance report holding labeled baseline/comparison rows for one run.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import VarianceReport
    >>> report = VarianceReport.from_json('{"baseline_label":"base","comparison_label":"case","rows":[]}')
    >>> (report.baseline_label, report.comparison_label, report.rows)
    ('base', 'case', [])

    """

    @staticmethod
    def from_json(json: str) -> VarianceReport:
        """
        Deserialize a variance report from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload returned by ``run_variance`` or a serialized report.

        Returns
        -------
        VarianceReport
            Validated `VarianceReport` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import VarianceReport
        >>> report = VarianceReport.from_json('{"baseline_label":"base","comparison_label":"case","rows":[]}')
        >>> report.baseline_label
        'base'

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `VarianceReport` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `VarianceReport`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def baseline_label(self) -> str:
        """
        Label for the baseline scenario (e.g. ``"management_case"``).

        Returns
        -------
        str
            Baseline scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison_label(self) -> str:
        """
        Label for the comparison scenario (e.g. ``"bank_case"``).

        Returns
        -------
        str
            Comparison scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rows(self) -> list[VarianceRow]:
        """
        Per-metric, per-period variance rows, in report order.

        Returns
        -------
        list[VarianceRow]
            Variance rows in report order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the variance rows as a pandas ``DataFrame``.

        Columns: ``period``, ``metric``, ``baseline``, ``comparison``,
        ``abs_var``, ``pct_var``. One row per (metric, period) pair, in report
        order; an empty report still carries the full column schema.

        ``baseline``, ``comparison`` and ``abs_var`` are in the metric's own
        units; ``pct_var`` is a decimal fraction (``0.1`` = +10%) and is
        ``NaN`` where the baseline is effectively zero. The scenario labels
        are report metadata (``baseline_label`` / ``comparison_label``) and
        are not repeated per row.

        Returns
        -------
        pd.DataFrame
            One row per (metric, period) variance row.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class ScenarioResults:
    """
    Named scenario-evaluation result set with per-scenario statement outputs.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScenarioResults
    >>> ScenarioResults.from_json("{}").names
    []

    """

    @staticmethod
    def from_json(json: str) -> ScenarioResults:
        """
        Deserialize evaluated scenario results from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload mapping scenario names to their statement results.

        Returns
        -------
        ScenarioResults
            Validated `ScenarioResults` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ScenarioResults
        >>> ScenarioResults.from_json("{}").get("missing") is None
        True

        """
        ...
    def to_json(self) -> str:
        """
        Serialize `ScenarioResults` to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ScenarioResults`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def names(self) -> list[str]:
        """
        Evaluated scenario names, in the order the scenario set defined them.

        Returns
        -------
        list[str]
            Evaluated scenario names in definition order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def get(self, name: str) -> StatementResult | None:
        """
        Return the statement result for one named scenario.

        Parameters
        ----------
        name : str
            Scenario name as defined in the input ``ScenarioSet``.

        Returns
        -------
        StatementResult | None
            Evaluated statement result for ``name``, or ``None`` when the
            result set has no scenario with that name.

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.
        """
        ...

    def to_comparison_table(self, metrics: list[str]) -> ArrowTable:
        """
        Build a side-by-side comparison table across every evaluated scenario.

        Parameters
        ----------
        metrics : list[str]
            Node identifiers to include as rows.

        Returns
        -------
        ArrowTable
            One column per scenario, one row per (metric, period).

        Raises
        ------
        ValueError
            If the result set or *metrics* is empty.

        """
        ...

    def to_dataframe(self, metrics: list[str]) -> pd.DataFrame:
        """
        Export the scenario comparison as a pandas ``DataFrame``.

        Columns: ``period``, ``metric``, one column per scenario name holding
        that scenario's metric value, and one ``{scenario}_vs_{baseline}_pct``
        column per non-baseline scenario holding the relative change as a
        decimal fraction (``0.1`` = +10%, ``NaN`` on a near-zero baseline).
        One row per (metric, period) pair.

        This is the same table as ``to_comparison_table`` - both call one Rust
        implementation, so the two exports cannot drift apart. The baseline is
        the scenario named ``"base"`` when present, otherwise the first
        scenario.

        Parameters
        ----------
        metrics : list[str]
            Node identifiers to include as rows.

        Returns
        -------
        pd.DataFrame
            One row per (metric, period) pair.

        Raises
        ------
        ValueError
            If the result set or *metrics* is empty.
        """
        ...

def run_sensitivity(
    model: FinancialModelSpec | str,
    config: SensitivityConfig | str,
) -> SensitivityResult:
    """
    Run sensitivity analysis on a financial model.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    config : SensitivityConfig or str
        Typed configuration or JSON string.

    Returns
    -------
    SensitivityResult
        Typed sensitivity result.

    Raises
    ------
    ValueError
        If model or config JSON is malformed, or the sensitivity setup cannot be evaluated.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import SensitivityConfig, run_sensitivity
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.compute("profit", "revenue * 0.5")
    >>> config = SensitivityConfig("Diagonal", [("revenue", "2025Q1", 100.0, [90.0, 110.0])], ["profit"])
    >>> len(run_sensitivity(builder.build(), config))
    2

    """
    ...

def generate_tornado_entries(
    result: SensitivityResult | str,
    metric_node: str,
    period: str | None = None,
) -> list[TornadoEntry]:
    """
    Build typed tornado chart entries from a sensitivity result.

    Parameters
    ----------
    result : SensitivityResult or str
        Typed sensitivity result or JSON string.
    metric_node : str
        Node ID to extract tornado entries for.
    period : str or None
        Optional period string to pin the tornado to.

    Returns
    -------
    list[TornadoEntry]
        Typed entries sorted by descending absolute swing.

    Raises
    ------
    ValueError
        If result JSON is malformed or period cannot be parsed.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import SensitivityConfig, generate_tornado_entries, run_sensitivity
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.compute("profit", "revenue * 0.5")
    >>> config = SensitivityConfig("Tornado", [("revenue", "2025Q1", 100.0, [90.0, 110.0])], ["profit"])
    >>> entries = generate_tornado_entries(run_sensitivity(builder.build(), config), "profit", "2025Q1")
    >>> (len(entries), entries[0].parameter_id)
    (1, 'revenue')

    """
    ...

def run_variance(
    base: StatementResult | str,
    comparison: StatementResult | str,
    config: VarianceConfig | str,
) -> VarianceReport:
    """
    Run variance analysis comparing two statement results.

    Parameters
    ----------
    base : StatementResult or str
        Baseline ``StatementResult`` object or JSON string.
    comparison : StatementResult or str
        Comparison ``StatementResult`` object or JSON string.
    config : VarianceConfig or str
        Typed configuration or JSON string.

    Returns
    -------
    VarianceReport
        Typed variance report.

    Raises
    ------
    ValueError
        If an input JSON payload is malformed or the requested comparison is invalid.

    Examples
    --------
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> from finstack_quant.statements_analytics import VarianceConfig, run_variance
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("profit", [("2025Q1", 25.0)])
    >>> result = Evaluator().evaluate(builder.build())
    >>> report = run_variance(result, result, VarianceConfig("base", "case", ["profit"], ["2025Q1"]))
    >>> report.rows[0].abs_var
    0.0

    """
    ...

def evaluate_scenario_set(
    model: FinancialModelSpec | str,
    scenario_set: ScenarioSet | str,
) -> ScenarioResults:
    """
    Evaluate every scenario in a scenario set against a model.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    scenario_set : ScenarioSet or str
        Typed scenario set or JSON string.

    Returns
    -------
    ScenarioResults
        Typed mapping from scenario names to statement results.

    Raises
    ------
    ValueError
        If model or scenario JSON is malformed, or the scenario graph cannot be evaluated.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import ScenarioSet, evaluate_scenario_set
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> evaluate_scenario_set(builder.build(), ScenarioSet({"base": {}})).names
    ['base']

    """
    ...

def backtest_forecast(actual: list[float], forecast: list[float]) -> dict[str, float | int]:
    """
    Compute forecast accuracy metrics (MAE, MAPE, RMSE).

    Parameters
    ----------
    actual : list[float]
        Observed values.
    forecast : list[float]
        Predicted values (same length as ``actual``).

    Returns
    -------
    dict[str, float | int]
        Dict with keys ``mae``, ``mape``, ``rmse``, and ``n``.

    Raises
    ------
    ValueError
        If actual and forecast have different lengths or are empty.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import backtest_forecast
    >>> backtest_forecast([1.0, 2.0], [1.1, 1.9])["n"]
    2

    """
    ...

def goal_seek(
    model: FinancialModelSpec | str,
    target_node: str,
    target_period: str,
    target_value: float,
    driver_node: str,
    driver_period: str,
    update_model: bool = True,
    bounds: tuple[float, float] | None = None,
) -> tuple[float, str | None]:
    """
    Find the driver value that makes a target node hit a target value.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    target_node : str
        Node optimized toward ``target_value``.
    target_period : str
        Period string for the target (e.g. ``"2025Q4"``).
    target_value : float
        Desired value for the target node.
    driver_node : str
        Node adjusted to reach the target.
    driver_period : str
        Period string for the driver.
    update_model : bool
        If ``True``, write the solved value back into the returned model JSON.
    bounds : tuple[float, float] or None
        Optional ``(lo, hi)`` search bounds for bisection.

    Returns
    -------
    tuple[float, str | None]
        ``(solved_driver_value, updated_model_json)``. The updated model JSON
        is ``None`` when ``update_model`` is ``False``.

    Raises
    ------
    ValueError
        If a period, model node, or bound is invalid, or the solver cannot find a solution.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import goal_seek
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.compute("profit", "revenue * 0.5")
    >>> solved, updated = goal_seek(builder.build(), "profit", "2025Q1", 60.0, "revenue", "2025Q1", False)
    >>> (round(solved, 6), updated)
    (120.0, None)

    """
    ...

def evaluate_dcf(
    model: FinancialModelSpec | str,
    wacc: float,
    terminal_value_json: str,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    mid_year_convention: bool = False,
    max_stable_growth_rate: float | None = None,
    shares_outstanding: float | None = None,
    equity_bridge_json: str | None = None,
    valuation_discounts_json: str | None = None,
    market: MarketContext | str | None = None,
    as_of: datetime.date | str | None = None,
    exit_multiple_metric_node: str | None = None,
) -> dict[str, float | str]:
    """
    Evaluate DCF valuation on a financial model.

    ``as_of`` anchors DCF discounting and, when market data is present,
    statement visibility and curve lookups. Discounting remains WACC-only.
    Year-end discounting is the default.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string (metadata must include ``currency``).
    wacc : float
        Weighted average cost of capital as a decimal (``0.10`` = 10%).
    terminal_value_json : str
        JSON ``TerminalValueSpec`` (tagged enum).
    ufcf_node : str
        Node ID for unlevered free cash flow.
    net_debt_override : float or None
        Optional flat net debt.
    mid_year_convention : bool
        Use mid-year discounting when ``True``. Default ``False`` (year-end).
    max_stable_growth_rate : float or None
        Maximum perpetual stable growth rate. ``None`` uses 5%.
    shares_outstanding : float or None
        Optional basic shares for per-share equity value.
    equity_bridge_json : str or None
        Optional JSON ``EquityBridge``.
    valuation_discounts_json : str or None
        Optional JSON ``ValuationDiscounts`` (DLOM, DLOC).
    market : MarketContext or str or None
        Optional ``MarketContext`` object or JSON string for statement
        evaluation. Not used as the DCF discounting basis. When set,
        ``as_of`` is required.
    as_of : datetime.date or str or None
        DCF valuation date and statement visibility date. Required with
        ``market``; otherwise defaults to the first forecast boundary.
    exit_multiple_metric_node : str or None
        Statement node whose last-forecast-period value supplies the
        exit-multiple terminal metric. When set, that value replaces
        ``terminal_metric`` on an ``ExitMultiple`` spec. Ignored for
        Gordon Growth and H-Model terminals.

    Returns
    -------
    dict[str, float | str]
        Dict with ``equity_value``, ``equity_currency``, ``enterprise_value``, ``net_debt``,
        ``terminal_value_pv``, ``equity_value_per_share``, ``diluted_shares``.

    Raises
    ------
    ValueError
        If ``market`` is set without ``as_of``, a JSON payload is malformed,
        or the model, cash-flow node, exit-multiple metric node, or DCF
        inputs are invalid.

    Examples
    --------
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.statements_analytics import evaluate_dcf
    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("dcf")
    >>> _ = builder.periods("2025..2026")
    >>> _ = builder.value_money("ufcf", [("2025", Money(100.0, "USD")), ("2026", Money(110.0, "USD"))])
    >>> _ = builder.with_meta("currency", '"USD"')
    >>> terminal = '{"type":"gordon_growth","growth_rate":0.02}'
    >>> evaluate_dcf(builder.build(), 0.10, terminal, net_debt_override=0.0)["enterprise_value"] > 0.0
    True

    """
    ...

def dcf_sensitivity(
    model: FinancialModelSpec | str,
    wacc: float,
    terminal_value_json: str,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    wacc_sensitivity_bump: float | None = None,
    wacc_denominator_epsilon: float | None = None,
    max_stable_growth_rate: float | None = None,
    exit_multiple_bump: float | None = None,
    mid_year_convention: bool = False,
    market: MarketContext | str | None = None,
    exit_multiple_metric_node: str | None = None,
) -> dict[str, object]:
    """
    Rank the headline DCF assumptions by enterprise-value impact.

    The statement model is evaluated once and each shocked point re-runs only
    the DCF. Tornado entries are deltas versus the baseline enterprise value,
    sorted by descending absolute swing. Shocks that would collapse the
    ``1/(wacc - g)`` terminal denominator are clamped and the clamp is reported.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string; metadata must include ``currency``.
    wacc : float
        Baseline weighted average cost of capital as a decimal (``0.10`` = 10%).
    terminal_value_json : str
        JSON ``TerminalValueSpec``; selects whether terminal growth or the exit
        multiple is the second shocked parameter.
    ufcf_node : str
        Node ID holding unlevered free cash flow for the forecast periods.
    net_debt_override : float or None
        Flat net-debt amount used instead of the model-derived balance-sheet bridge.
    wacc_sensitivity_bump : float or None
        Absolute shock applied to WACC and to terminal growth, as a decimal
        (``0.01`` = +/-100 bp). ``None`` uses the canonical Rust
        ``DcfOptions`` default.
    wacc_denominator_epsilon : float or None
        Minimum spread preserved between WACC and terminal growth so the terminal
        denominator stays defined, as a decimal (``0.005`` = 50 bp). ``None``
        uses the canonical Rust ``DcfOptions`` default.
    max_stable_growth_rate : float or None
        Maximum perpetual stable growth rate. ``None`` uses 5%.
    exit_multiple_bump : float or None
        Absolute shock applied to an exit multiple, in turns (``1.0`` =
        +/-1.0x). ``None`` uses the canonical Rust ``DcfOptions`` default.
    mid_year_convention : bool
        Use mid-year discounting on every re-run when ``True``. Default
        ``False`` (year-end).
    market : MarketContext or str or None
        ``MarketContext`` object or JSON string used for statement
        evaluation, not WACC discounting.
    exit_multiple_metric_node : str or None
        Statement node whose last-forecast-period value supplies the
        exit-multiple terminal metric when the spec is ``ExitMultiple``.
        ``None`` keeps the spec's explicit ``terminal_metric``.

    Returns
    -------
    dict[str, object]
        Dict with ``baseline_enterprise_value``, ``currency``, ``entries``
        (list of ``{"parameter_id", "downside", "upside"}``), ``wacc_down``,
        ``wacc_down_clamped``, ``terminal_growth_up``, ``terminal_growth_up_clamped``.

    Raises
    ------
    ValueError
        If terminal-value JSON is malformed or the model and sensitivity inputs are invalid.

    Examples
    --------
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.statements_analytics import dcf_sensitivity
    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("dcf")
    >>> _ = builder.periods("2025..2026")
    >>> _ = builder.value_money("ufcf", [("2025", Money(100.0, "USD")), ("2026", Money(110.0, "USD"))])
    >>> _ = builder.with_meta("currency", '"USD"')
    >>> terminal = '{"type":"gordon_growth","growth_rate":0.02}'
    >>> len(dcf_sensitivity(builder.build(), 0.10, terminal, net_debt_override=0.0)["entries"])
    2

    """
    ...

def evaluate_lbo(
    model: FinancialModelSpec | str,
    entry_multiple: float,
    entry_metric_node: str,
    exit_multiple: float,
    exit_metric_node: str,
    exit_net_debt_node: str,
    exit_period: str,
    sources: list[tuple[str, float]],
    transaction_fees: float = 0.0,
) -> dict[str, float | bool | str]:
    """
    Evaluate a leveraged-buyout transaction against a statement model.

    Entry enterprise value is priced at the model's first period, the sponsor
    equity check is solved as the sources-and-uses residual, and exit proceeds
    are the exit enterprise value less the modelled net debt at ``exit_period``.
    IRR is out of scope: pair ``exit_equity_proceeds`` with the equity outflow at
    close and call :func:`finstack_quant.portfolio.mwr_xirr`.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string; metadata must include ``currency``.
    entry_multiple : float
        Entry valuation multiple applied to the entry metric (``8.5`` = 8.5x).
    entry_metric_node : str
        Node ID supplying the entry valuation metric, read at the model's first
        period (typically ``"ebitda"``).
    exit_multiple : float
        Exit valuation multiple applied to the exit metric (``9.5`` = 9.5x).
    exit_metric_node : str
        Node ID supplying the exit valuation metric, read at ``exit_period``.
    exit_net_debt_node : str
        Node ID supplying net debt outstanding at ``exit_period``, where a modelled
        tranche amortisation schedule lands.
    exit_period : str
        Period label at which the sponsor exits, e.g. ``"2029"`` or ``"2029Q4"``.
    sources : list[tuple[str, float]]
        Funded debt tranches at close as ``(name, amount)`` pairs in the model
        currency; amounts must be finite and non-negative.
    transaction_fees : float
        Transaction fees and expenses funded at close, in the model currency.

    Returns
    -------
    dict[str, float | bool | str]
        Dict with ``entry_enterprise_value``, ``entry_metric``, ``debt_total``,
        ``equity_check``, ``sources_total``, ``uses_total``, ``sources_uses_balanced``,
        ``exit_enterprise_value``, ``exit_metric``, ``exit_net_debt``,
        ``exit_equity_proceeds``, ``moic``, and ``currency``.

    Raises
    ------
    ValueError
        If exit_period, a model node, or the sources-and-uses inputs are invalid.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import evaluate_lbo
    >>> builder = ModelBuilder("lbo")
    >>> _ = builder.periods("2025..2026")
    >>> _ = builder.value("ebitda", [("2025", 22.0), ("2026", 26.4)])
    >>> _ = builder.value("total_debt", [("2025", 115.0), ("2026", 35.0)])
    >>> _ = builder.with_meta("currency", '"USD"')
    >>> result = evaluate_lbo(
    ...     builder.build(), 8.5, "ebitda", 9.5, "ebitda", "total_debt", "2026", [("debt", 115.0)], 3.0
    ... )
    >>> (result["entry_enterprise_value"], result["sources_uses_balanced"])
    (187.0, True)

    """
    ...

def wacc(
    equity_weight: float,
    cost_of_equity: float,
    debt_weight: float,
    cost_of_debt: float,
    tax_rate: float,
) -> float:
    """
    Weighted-average cost of capital (WACC).

    Blends the required return on equity with the after-tax cost of debt:
    ``WACC = w_E * r_E + w_D * r_D * (1 - T)``. The ``(1 - T)`` factor is the
    interest tax shield (Modigliani & Miller, 1963); equity carries no shield.

    Parameters
    ----------
    equity_weight : float
        Equity share of total capital as a decimal fraction (``0.6`` = 60%
        equity-funded); must be non-negative.
    cost_of_equity : float
        Required return on equity as a decimal, typically from CAPM (``0.115`` = 11.5%).
    debt_weight : float
        Debt share of total capital as a decimal fraction (``0.4`` = 40% debt-funded);
        must be non-negative and sum with ``equity_weight`` to ``1.0``.
    cost_of_debt : float
        Pre-tax marginal borrowing yield as a decimal, before the interest tax
        shield (``0.06`` = 6%).
    tax_rate : float
        Marginal corporate tax rate as a decimal fraction in ``[0, 1]`` (``0.25`` = 25%).

    Returns
    -------
    float
        Blended discount rate as a decimal fraction.

    Raises
    ------
    ValueError
        If an input is non-finite, a weight is negative, weights do not sum to one,
        or tax_rate is outside the inclusive range from zero to one.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import wacc
    >>> round(wacc(0.6, 0.10, 0.4, 0.05, 0.25), 3)
    0.075

    """
    ...

def run_corporate_analysis(
    model: FinancialModelSpec | str,
    wacc: float | None = None,
    terminal_value_json: str | None = None,
    net_debt_override: float | None = None,
    cfads_node: str | None = None,
    interest_coverage_node: str = "ebitda",
    check_suite_json: str | None = None,
    market: MarketContext | str | None = None,
    as_of: datetime.date | str | None = None,
    ltv_value_node: str | None = None,
) -> dict[str, Any]:
    """
    Run statements plus optional DCF equity and credit context.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    wacc : float or None
        If set, enables DCF at this discount rate (decimal).
    terminal_value_json : str or None
        Required JSON ``TerminalValueSpec`` when ``wacc`` is set.
    net_debt_override : float or None
        Optional flat net debt for the equity bridge.
    cfads_node : str or None
        Required CFADS numerator for capital-structure credit analysis.
    interest_coverage_node : str
        Earnings numerator used only for interest coverage.
    check_suite_json : str or None
        JSON ``CheckSuiteSpec`` required for DCF or credit analysis; it must
        include ``NonFiniteCheck``.
    market : MarketContext or str or None
        Optional ``MarketContext`` object or JSON string used for
        statement evaluation, not WACC discounting.
    as_of : datetime.date | str | None
        Optional valuation date, either a date-like object or an ISO 8601
        string. Required when ``market`` is set.
    ltv_value_node : str or None
        Optional statement node supplying a per-period LTV denominator.
        When set, each period's node value is used when present; a missing
        period skips LTV for that period only. When omitted, a positive DCF
        enterprise value is broadcast as a constant-denominator path
        (current valuation versus forward debt, not a rolled EV).

    Returns
    -------
    dict[str, Any]
        Dict with ``statement_json``, optional ``equity`` scalars, ``credit``
        (instrument_id → credit metrics JSON including ``dscr_incl_fees`` /
        ``dscr_incl_fees_min``), and ``ev_suppressed_non_positive``. The
        credit metrics include ``skipped_periods`` for periods dropped from
        min/max stats.

    Raises
    ------
    ValueError
        If model, market, terminal-value, check-suite, or as_of data is invalid,
        or required DCF/credit configuration is omitted.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import run_corporate_analysis
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("ebitda", [("2025Q1", 25.0)])
    >>> sorted(run_corporate_analysis(builder.build()))
    ['credit', 'ev_suppressed_non_positive', 'statement_json']

    """
    ...

def pl_summary_report(
    results: StatementResult | str,
    line_items: list[str],
    periods: list[str],
) -> str:
    """
    Render a P&L summary report as formatted text.

    Parameters
    ----------
    results : StatementResult or str
        ``StatementResult`` object or JSON string.
    line_items : list[str]
        Node IDs to include as rows.
    periods : list[str]
        Period strings for columns (e.g. ``["2025Q1", "2025Q2"]``).

    Returns
    -------
    str
        Formatted report text.

    Raises
    ------
    ValueError
        If results JSON is malformed or a requested period cannot be parsed.

    Examples
    --------
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> from finstack_quant.statements_analytics import pl_summary_report
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> result = Evaluator().evaluate(builder.build())
    >>> "revenue" in pl_summary_report(result, ["revenue"], ["2025Q1"])
    True

    """
    ...

def credit_assessment_report(results: StatementResult | str, as_of: str) -> str:
    """
    Render a credit assessment report as formatted text.

    Parameters
    ----------
    results : StatementResult or str
        ``StatementResult`` object or JSON string.
    as_of : str
        Period identifier (e.g. ``"2025Q1"``, ``"2025M03"``, ``"FY2025"``).
        Unlike the ``as_of`` valuation dates elsewhere in the bindings this is a
        period, not a date: ``datetime.date`` and ISO 8601 strings are rejected.

    Returns
    -------
    str
        Formatted credit report text.

    Raises
    ------
    ValueError
        If results JSON is malformed or as_of cannot be parsed as a period.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import credit_assessment_report
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> periods = ["2025Q1", "2025Q2", "2025Q3", "2025Q4"]
    >>> builder = ModelBuilder("credit")
    >>> _ = builder.periods("2025Q1..Q4")
    >>> _ = builder.value("ebitda", list(zip(periods, [10.0, 20.0, 30.0, 40.0], strict=True)))
    >>> _ = builder.value("interest_expense", list(zip(periods, [1.0, 2.0, 3.0, 4.0], strict=True)))
    >>> _ = builder.value("total_debt", list(zip(periods, [300.0] * 4, strict=True)))
    >>> results = Evaluator().evaluate(builder.build())
    >>> len(credit_assessment_report(results, "2025Q4").splitlines()) > 1
    True

    """
    ...

def credit_assessment(results: StatementResult | str, as_of: str) -> dict[str, Any]:
    """
    Compute a structured credit assessment (leverage, coverage, FCF).

    Parameters
    ----------
    results : StatementResult or str
        ``StatementResult`` object or JSON string.
    as_of : str
        Period identifier (e.g. ``"2025Q4"``, ``"2025M03"``, ``"FY2025"``).
        Unlike the ``as_of`` valuation dates elsewhere in the bindings this is a
        period, not a date: ``datetime.date`` and ISO 8601 strings are rejected.

    Returns
    -------
    dict[str, Any]
        Dict with ``as_of`` (str), ``leverage_ratio``, ``interest_coverage``,
        ``free_cash_flow`` (float | None), and ``series`` (list of per-period
        dicts with the same metric keys plus ``period``).

    Raises
    ------
    ValueError
        If results JSON is malformed or as_of cannot be parsed as a period.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import credit_assessment
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> periods = ["2025Q1", "2025Q2", "2025Q3", "2025Q4"]
    >>> builder = ModelBuilder("credit")
    >>> _ = builder.periods("2025Q1..Q4")
    >>> _ = builder.value("ebitda", list(zip(periods, [10.0, 20.0, 30.0, 40.0], strict=True)))
    >>> _ = builder.value("interest_expense", list(zip(periods, [1.0, 2.0, 3.0, 4.0], strict=True)))
    >>> _ = builder.value("total_debt", list(zip(periods, [300.0] * 4, strict=True)))
    >>> results = Evaluator().evaluate(builder.build())
    >>> credit_assessment(results, "2025Q4")["leverage_ratio"]
    3.0

    """
    ...

class DependencyTracer:
    """
    Reusable dependency tracer for a financial model.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import DependencyTracer
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> sorted(DependencyTracer(builder.build()).direct_dependencies("profit"))
    ['cost', 'revenue']

    """

    def __init__(self, model: FinancialModelSpec | str) -> None:
        """
        Create a dependency tracer for the given model.

        Parameters
        ----------
        model : FinancialModelSpec or str
            ``FinancialModelSpec`` object or JSON string.

        Raises
        ------
        ValueError
            If model JSON is malformed or its dependency graph is invalid.

        """
        ...
    def dependency_tree(self, node_id: str) -> str:
        """
        Return an ASCII dependency tree for ``node_id``.

        Parameters
        ----------
        node_id : str
            Root node to trace.

        Returns
        -------
        str
            Multi-line ASCII tree rooted at ``node_id`` and containing its
            complete upstream dependency hierarchy.

        Raises
        ------
        ValueError
            If node_id is unknown or its dependency graph is invalid.

        """
        ...

    def dependency_tree_detailed(self, results: StatementResult | str, node_id: str, period: str) -> str:
        """
        Return an ASCII dependency tree annotated with values for one period.

        Parameters
        ----------
        results : StatementResult or str
            Statement results to annotate with.
        node_id : str
            Root node to trace.
        period : str
            Period to annotate.

        Returns
        -------
        str
            Multi-line ASCII dependency tree whose nodes are annotated with
            values from ``results`` for ``period``.

        Raises
        ------
        ValueError
            If results JSON or period is invalid, or node_id cannot be traced.

        """
        ...

    def direct_dependencies(self, node_id: str) -> list[str]:
        """
        List immediate dependencies of ``node_id``.

        Parameters
        ----------
        node_id : str
            Statement node ID whose directly referenced inputs are requested.

        Returns
        -------
        list[str]
            Statement node IDs referenced directly by ``node_id``.

        Raises
        ------
        ValueError
            If node_id is unknown or its dependency graph is invalid.

        """
        ...
    def all_dependencies(self, node_id: str) -> list[str]:
        """
        List all transitive dependencies of ``node_id``.

        Parameters
        ----------
        node_id : str
            Statement node ID whose complete upstream dependency set is requested.

        Returns
        -------
        list[str]
            Direct and transitive upstream node IDs in dependency order, with
            dependencies preceding their dependents.

        Raises
        ------
        ValueError
            If node_id is unknown or the dependency traversal fails.

        """
        ...
    def dependents(self, node_id: str) -> list[str]:
        """
        List nodes that depend on ``node_id``.

        Parameters
        ----------
        node_id : str
            Statement node ID whose downstream dependents are requested.

        Returns
        -------
        list[str]
            Statement node IDs that directly depend on ``node_id``.

        Raises
        ------
        ValueError
            If node_id is unknown or its dependency graph is invalid.

        """
        ...

def direct_dependencies(model: FinancialModelSpec | str, node_id: str) -> list[str]:
    """
    List immediate dependencies of a node.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    node_id : str
        Node whose direct dependencies are listed.

    Returns
    -------
    list[str]
        Direct dependency node IDs.

    Raises
    ------
    ValueError
        If model JSON is malformed or node_id cannot be resolved.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import direct_dependencies
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> model = builder.build()
    >>> sorted(direct_dependencies(model, "profit"))
    ['cost', 'revenue']

    """
    ...

def all_dependencies(model: FinancialModelSpec | str, node_id: str) -> list[str]:
    """
    List all transitive dependencies of a node in dependency order.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    node_id : str
        Root node for the dependency walk.

    Returns
    -------
    list[str]
        Transitive dependency node IDs.

    Raises
    ------
    ValueError
        If model JSON is malformed or the dependency traversal fails for node_id.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import all_dependencies
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> model = builder.build()
    >>> sorted(all_dependencies(model, "profit"))
    ['cost', 'revenue']

    """
    ...

def dependents(model: FinancialModelSpec | str, node_id: str) -> list[str]:
    """
    List nodes that depend on the given node (reverse dependencies).

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    node_id : str
        Node whose dependents are listed.

    Returns
    -------
    list[str]
        Dependent node IDs.

    Raises
    ------
    ValueError
        If model JSON is malformed or node_id cannot be resolved.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import dependents
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> model = builder.build()
    >>> dependents(model, "revenue")
    ['profit']

    """
    ...

def explain_formula(
    model: FinancialModelSpec | str,
    results: StatementResult | str,
    node_id: str,
    period: str,
) -> dict[str, Any]:
    """
    Structured formula explanation for a node and period.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    results : StatementResult or str
        ``StatementResult`` object or JSON string.
    node_id : str
        Node to explain.
    period : str
        Period string.

    Returns
    -------
    dict[str, Any]
        The canonical serde form of the Rust ``Explanation``: ``node_id``,
        ``period_id``, ``final_value``, ``node_type`` (snake_case
        discriminant, e.g. ``"calculated"``), ``formula_text``, and
        ``breakdown`` (list of ``ExplanationStep`` dicts: ``component``,
        ``value``, and ``operation`` — the ``operation`` key is omitted when
        absent). Matches the WASM ``explainFormula`` output exactly.

    Raises
    ------
    ValueError
        If model or results JSON is malformed, period is invalid, or node_id cannot be explained.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import explain_formula
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> model = builder.build()
    >>> results = Evaluator().evaluate(model)
    >>> explanation = explain_formula(model, results, "profit", "2025Q1")
    >>> (explanation["node_id"], explanation["final_value"])
    ('profit', 40.0)

    """
    ...

def explain_formula_text(
    model: FinancialModelSpec | str,
    results: StatementResult | str,
    node_id: str,
    period: str,
) -> str:
    """
    Human-readable multi-line formula explanation.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    results : StatementResult or str
        ``StatementResult`` object or JSON string.
    node_id : str
        Node to explain.
    period : str
        Period string.

    Returns
    -------
    str
        Detailed text explanation.

    Raises
    ------
    ValueError
        If model or results JSON is malformed, period is invalid, or node_id cannot be explained.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import explain_formula_text
    >>> from finstack_quant.statements import Evaluator, ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> model = builder.build()
    >>> results = Evaluator().evaluate(model)
    >>> "profit" in explain_formula_text(model, results, "profit", "2025Q1")
    True

    """
    ...

def run_checks(
    model: FinancialModelSpec | str,
    suite_spec_json: str,
    results: StatementResult | str | None = None,
) -> CheckReport:
    """
    Run checks from a suite spec against a model.

    Resolves both built-in and formula checks, evaluates the model,
    and returns a full check report.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    suite_spec_json : str
        JSON-serialized ``CheckSuiteSpec``.
    results : StatementResult or str or None
        Optional pre-computed ``StatementResult`` (object or JSON);
        skips re-evaluation when provided.

    Returns
    -------
    CheckReport
        Typed report with summary, findings, JSON, and DataFrame accessors.

    Raises
    ------
    ValueError
        If model, results, or suite-spec JSON is malformed, or check evaluation fails.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import run_checks
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> suite = '{"name":"basic","builtin_checks":[],"formula_checks":[]}'
    >>> run_checks(builder.build(), suite).total_checks
    0

    """
    ...

def run_three_statement_checks(
    model: FinancialModelSpec | str,
    mapping_json: str,
    results: StatementResult | str | None = None,
) -> CheckReport:
    """
    Run three-statement checks using a JSON node mapping.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    mapping_json : str
        JSON-serialized ``ThreeStatementMapping``.
    results : StatementResult or str or None
        Optional pre-computed ``StatementResult`` (object or JSON);
        skips re-evaluation when provided.

    Returns
    -------
    CheckReport
        Typed report with summary, findings, JSON, and DataFrame accessors.

    Raises
    ------
    ValueError
        If model, results, or mapping JSON is malformed, or check evaluation fails.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import run_three_statement_checks
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> for node in ["assets", "liabilities", "equity", "cash", "retained_earnings", "net_income"]:
    ...     _ = builder.value(node, [("2025Q1", 0.0)])
    >>> mapping = '{"assets_nodes":["assets"],"liabilities_nodes":["liabilities"],"equity_nodes":["equity"],"cash_node":"cash","retained_earnings_node":"retained_earnings","ppe_node":null,"net_income_node":"net_income","depreciation_node":null,"interest_expense_node":null,"tax_expense_node":null,"pretax_income_node":null,"cfo_node":null,"cfi_node":null,"cff_node":null,"total_cf_node":null,"capex_node":null,"dividends_node":null}'
    >>> run_three_statement_checks(builder.build(), mapping).total_checks > 0
    True

    """
    ...

def run_credit_underwriting_checks(
    model: FinancialModelSpec | str,
    mapping_json: str,
    results: StatementResult | str | None = None,
) -> CheckReport:
    """
    Run credit underwriting checks using a JSON node mapping.

    Parameters
    ----------
    model : FinancialModelSpec or str
        ``FinancialModelSpec`` object or JSON string.
    mapping_json : str
        JSON-serialized ``CreditMapping``.
    results : StatementResult or str or None
        Optional pre-computed ``StatementResult`` (object or JSON);
        skips re-evaluation when provided.

    Returns
    -------
    CheckReport
        Typed report with summary, findings, JSON, and DataFrame accessors.

    Raises
    ------
    ValueError
        If model, results, or mapping JSON is malformed, or check evaluation fails.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import run_credit_underwriting_checks
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> for node, value in [("total_debt", 100.0), ("ebitda", 50.0), ("interest_expense", 5.0)]:
    ...     _ = builder.value(node, [("2025Q1", value)])
    >>> mapping = '{"debt_node":"total_debt","ebitda_node":"ebitda","interest_expense_node":"interest_expense","fcf_node":null,"cash_node":null,"cash_burn_node":null,"leverage_warn":null,"coverage_min_warn":null}'
    >>> run_credit_underwriting_checks(builder.build(), mapping).total_checks > 0
    True

    """
    ...

def render_check_report_text(report_json: str) -> str:
    """
    Render a check report as plain text.

    Parameters
    ----------
    report_json : str
        JSON-serialized ``CheckReport``.

    Returns
    -------
    str
        Human-readable plain-text report.

    Raises
    ------
    ValueError
        If report_json is not a valid serialized check report.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import render_check_report_text
    >>> report = '{"results":[],"summary":{"total_checks":0,"passed":0,"failed":0,"errors":0,"warnings":0,"infos":0}}'
    >>> "Check Report" in render_check_report_text(report)
    True

    """
    ...

def render_check_report_html(report_json: str) -> str:
    """
    Render a check report as HTML with inline styles.

    Parameters
    ----------
    report_json : str
        JSON-serialized ``CheckReport``.

    Returns
    -------
    str
        HTML-formatted report suitable for Jupyter notebooks.

    Raises
    ------
    ValueError
        If report_json is not a valid serialized check report.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import render_check_report_html
    >>> report = '{"results":[],"summary":{"total_checks":0,"passed":0,"failed":0,"errors":0,"warnings":0,"infos":0}}'
    >>> "<h2" in render_check_report_html(report)
    True

    """
    ...

class Exposure:
    """
    A single credit exposure for ECL / IFRS 9 / CECL computation.

    All monetary fields are in the exposure's base currency; all rates and
    probabilities are expressed as decimals (``0.05`` = 5%). Priced EAD is
    ``drawn + undrawn × ccf`` via core ``ead_revolver``.

    Parameters
    ----------
    id : str
        Exposure identifier.
    ead : float
        Drawn outstanding balance at the reporting date.
    lgd : float
        Loss given default (decimal).
    eir : float
        Effective interest rate (decimal).
    remaining_maturity : float
        Remaining maturity in years.
    current_pd : float
        Current probability of default (decimal).
    origination_pd : float
        Probability of default at origination (decimal).
    dpd : int or None
        Days past due (optional).
    undrawn : float
        Undrawn commitment in the same currency as ``ead``. Default ``0.0``.
    ccf : float
        Credit-conversion factor applied to ``undrawn``, as a decimal in
        ``[0, 1]``. Default ``0.75`` (Basel IRB revolver).

    Examples
    --------
    >>> from finstack_quant.statements_analytics import Exposure
    >>> exposure = Exposure("loan", 1_000_000.0, 0.4, 0.05, 3.0, 0.02, 0.01)
    >>> (exposure.id, exposure.ead, exposure.undrawn, exposure.ccf)
    ('loan', 1000000.0, 0.0, 0.75)

    """

    id: str
    ead: float
    undrawn: float
    ccf: float
    lgd: float
    eir: float
    remaining_maturity: float
    current_pd: float
    origination_pd: float
    dpd: int

    def __init__(
        self,
        id: str,
        ead: float,
        lgd: float,
        eir: float,
        remaining_maturity: float,
        current_pd: float,
        origination_pd: float,
        dpd: int | None = None,
        undrawn: float = 0.0,
        ccf: float = 0.75,
    ) -> None:
        """
        Create one exposure with IFRS 9/CECL credit and maturity assumptions.

        Parameters
        ----------
        id : str
            Stable exposure identifier used in ECL results.
        ead : float
            Drawn outstanding balance in the exposure's base-currency units.
        lgd : float
            Loss given default as a decimal fraction.
        eir : float
            Effective interest rate as a decimal annual rate.
        remaining_maturity : float
            Remaining contractual maturity in years.
        current_pd : float
            Current probability of default as a decimal fraction.
        origination_pd : float
            Origination probability of default as a decimal fraction.
        dpd : int or None, default None
            Days past due used by staging backstops; ``None`` applies the
            canonical performing-exposure default of zero days.
        undrawn : float, default 0.0
            Undrawn commitment in the same currency as ``ead``. ``0.0`` is a
            fully drawn term loan.
        ccf : float, default 0.75
            Credit-conversion factor applied to ``undrawn``, as a decimal in
            ``[0, 1]``. Unused when ``undrawn`` is zero.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the exposure as a single-row pandas ``DataFrame``.

        Columns: ``id``, ``ead``, ``undrawn``, ``ccf``, ``lgd``, ``eir``,
        ``remaining_maturity``, ``current_pd``, ``origination_pd``, ``dpd``.

        ``ead`` and ``undrawn`` are in the exposure's base currency; ``ccf``,
        ``lgd``, ``current_pd`` and ``origination_pd`` are decimal fractions
        in ``[0, 1]``; ``eir`` is a decimal annual rate;
        ``remaining_maturity`` is in years; ``dpd`` is a whole number of days
        past due.

        Returns
        -------
        pd.DataFrame
            One row describing the exposure.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

def classify_stage(
    exposure: Exposure,
    pd_delta_stage2: float | None = None,
    dpd_30_trigger: bool | None = None,
    dpd_90_trigger: bool | None = None,
) -> tuple[str, list[str]]:
    """
    Classify an exposure into an IFRS 9 stage.

    Parameters
    ----------
    exposure : Exposure
        Credit exposure.
    pd_delta_stage2 : float or None
        Absolute PD increase threshold (decimal) for SICR.
    dpd_30_trigger : bool or None
        Apply the Stage 2 backstop when ``days_past_due >= 30``. Display
        contract: ``dpd_stage2 (dpd=30 >= 30)``.
    dpd_90_trigger : bool or None
        Apply the Stage 3 backstop when ``days_past_due >= 90``. Display
        contract: ``dpd_stage3 (dpd=90 >= 90)``.

    Returns
    -------
    tuple[str, list[str]]
        ``(stage, trigger_reasons)`` where stage is ``"Stage 1"``,
        ``"Stage 2"``, or ``"Stage 3"`` and ``trigger_reasons`` is the full
        ordered audit trail of fired triggers (``["no_trigger"]`` for a
        clean Stage 1), rendered by the canonical Rust ``StagingTrigger``
        display format.

    Raises
    ------
    ValueError
        If exposure values or the staging thresholds violate the ECL policy constraints.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import Exposure, classify_stage
    >>> exposure = Exposure("loan", 1_000_000.0, 0.4, 0.05, 3.0, 0.02, 0.01)
    >>> classify_stage(exposure)
    ('Stage 1', ['no_trigger'])
    >>> classify_stage(Exposure("loan", 1_000_000.0, 0.4, 0.05, 3.0, 0.02, 0.01, dpd=90))
    ('Stage 3', ['dpd_stage3 (dpd=90 >= 90)'])

    """
    ...

def compute_ecl(
    ead: float,
    pd_schedule: list[tuple[float, float]],
    lgd: float,
    eir: float,
    max_horizon_years: float,
    bucket_width_years: float | None = None,
    stage: str = "stage1",
    ead_schedule: list[tuple[float, float]] | None = None,
    stage3_time_to_recovery_years: float | None = None,
) -> float:
    """
    Compute single-scenario ECL for one exposure.

    Parameters
    ----------
    ead : float
        Priced exposure at default (``drawn + undrawn × ccf``). Term loans
        pass the drawn balance; revolvers should pre-apply
        ``finstack_quant.models.credit.lgd.ead_revolver``.
    pd_schedule : list[tuple[float, float]]
        ``[(time_years, cumulative_pd), ...]`` knots. A
        ``(0.0, 0.0)`` knot is inserted automatically if not present.
    lgd : float
        Loss given default (decimal).
    eir : float
        Effective interest rate (decimal).
    max_horizon_years : float
        Remaining maturity cap.
    bucket_width_years : float or None
        Time-bucket width (default ``0.25`` for quarterly).
    stage : str
        ``"stage1"``, ``"stage2"``, or ``"stage3"``.
    ead_schedule : list[tuple[float, float]] or None
        Optional EAD amortization profile as
        ``[(time_years, ead), ...]`` knots.
    stage3_time_to_recovery_years : float or None
        Stage 3 discounting horizon to expected recovery, in years.

    Returns
    -------
    float
        ECL amount in the exposure's base currency.

    Raises
    ------
    ValueError
        If stage is unknown, a PD or EAD schedule is invalid, or an ECL input is outside
        its accepted range.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import compute_ecl
    >>> compute_ecl(1_000.0, [(1.0, 0.02)], 0.4, 0.05, 1.0) > 0.0
    True

    """
    ...

def compute_ecl_weighted(
    ead: float,
    scenarios: list[tuple[float, list[tuple[float, float]]]],
    lgd: float,
    eir: float,
    max_horizon_years: float,
    bucket_width_years: float | None = None,
    stage: str = "stage1",
    ead_schedule: list[tuple[float, float]] | None = None,
    stage3_time_to_recovery_years: float | None = None,
) -> float:
    """
    Compute probability-weighted ECL across macro scenarios.

    Parameters
    ----------
    ead : float
        Priced exposure at default (``drawn + undrawn × ccf``). Term loans
        pass the drawn balance; revolvers should pre-apply
        ``finstack_quant.models.credit.lgd.ead_revolver``.
    scenarios : list[tuple[float, list[tuple[float, float]]]]
        List of ``(weight, pd_schedule)``. Weights must sum to 1.0.
        A ``(0.0, 0.0)`` knot is inserted automatically into each schedule
        if not present (same convention as ``compute_ecl``).
    lgd : float
        Loss given default (decimal).
    eir : float
        Effective interest rate (decimal).
    max_horizon_years : float
        Remaining maturity cap for the integration, in years.
    bucket_width_years : float or None
        Time-bucket width (default ``0.25`` for quarterly), matching
        :func:`compute_ecl`.
    stage : str
        ``"stage1"``, ``"stage2"``, or ``"stage3"``.
    ead_schedule : list[tuple[float, float]] or None
        Optional EAD amortization profile as
        ``[(time_years, ead), ...]`` knots.
    stage3_time_to_recovery_years : float or None
        Stage 3 discounting horizon to expected recovery, in years.

    Returns
    -------
    float
        Probability-weighted ECL amount in the exposure's base currency.

    Raises
    ------
    ValueError
        If stage is unknown, scenario weights or schedules are invalid, or an ECL input
        is outside its accepted range.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import compute_ecl_weighted
    >>> scenarios = [(0.5, [(1.0, 0.01)]), (0.5, [(1.0, 0.03)])]
    >>> compute_ecl_weighted(1_000.0, scenarios, 0.4, 0.05, 1.0) > 0.0
    True

    """
    ...

# Comparable-company analysis

def percentile_rank(value: float, peer_values: list[float]) -> float | None:
    """
    Percentile rank of ``value`` within ``peer_values`` on a 0-1 scale.

    Parameters
    ----------
    value : float
        Value to rank.
    peer_values : list[float]
        Peer distribution.

    Returns
    -------
    float or None
        Percentile rank in ``[0, 1]``, or ``None`` for empty peers.

    Notes
    -----
    This helper does not raise; empty or degenerate peers return ``None``.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import percentile_rank
    >>> percentile_rank(3.0, [1.0, 2.0, 3.0, 4.0, 5.0])
    0.6

    """
    ...

def z_score(value: float, peer_values: list[float]) -> float | None:
    """
    Standard score of ``value`` within the peer distribution.

    Parameters
    ----------
    value : float
        Value to score.
    peer_values : list[float]
        Peer distribution.

    Returns
    -------
    float or None
        Z-score, or ``None`` for empty peers or zero variance.

    Notes
    -----
    This helper does not raise; empty or degenerate peers return ``None``.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import z_score
    >>> round(z_score(3.0, [1.0, 2.0, 3.0, 4.0, 5.0]), 10)
    0.0

    """
    ...

def peer_stats(peer_values: list[float]) -> dict[str, float] | None:
    """
    Descriptive statistics for a peer distribution.

    Returns a dict with keys ``mean``, ``median``, ``q1``, ``q3``, ``iqr``,
    ``std_dev``, ``min``, ``max``, ``count`` (the Rust ``PeerStats`` serde
    form), or ``None`` when no statistics can be computed — matching the
    WASM twin's ``undefined`` and the sibling ``percentile_rank`` /
    ``z_score`` no-result convention.

    Parameters
    ----------
    peer_values : list[float]
        Peer distribution.

    Returns
    -------
    dict[str, float] or None
        Descriptive statistics, or ``None`` when ``peer_values`` is empty.

    Raises
    ------
    ValueError
        If the result cannot be serialized to a Python dict.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import peer_stats
    >>> peer_stats([1.0, 2.0, 3.0, 4.0, 5.0])["median"]
    3.0

    """
    ...

def regression_fair_value(
    x_values: list[float],
    y_values: list[float],
    subject_x: float,
    subject_y: float,
) -> dict[str, float] | None:
    """
    Single-factor OLS regression fair value with canonical residual semantics.

    Parameters
    ----------
    x_values : list[float]
        Independent variable values for the peer set.
    y_values : list[float]
        Dependent variable values for the peer set.
    subject_x : float
        Independent variable value for the subject company.
    subject_y : float
        Observed dependent variable value for the subject company.

    Returns
    -------
    dict[str, float] or None
        Regression fair value metrics (the Rust ``RegressionResult`` serde
        form), or ``None`` when fewer than three observations are available
        or the fit is unidentifiable — matching the WASM twin's
        ``undefined``.

    Raises
    ------
    ValueError
        If the result cannot be serialized to a Python dict.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import regression_fair_value
    >>> result = regression_fair_value([1.0, 2.0, 3.0], [2.0, 4.0, 6.0], 2.5, 5.0)
    >>> (round(result["fitted_value"], 6), round(result["residual"], 6))
    (5.0, 0.0)

    """
    ...

def compute_multiple(
    company_metrics: dict[str, float],
    multiple: str,
) -> float | None:
    """
    Canonical multiple computation for one company.

    Parameters
    ----------
    company_metrics : dict[str, float]
        Metric values for the company.
    multiple : str
        Multiple identifier (e.g. ``"ev_ebitda"``).

    Returns
    -------
    float or None
        Computed multiple, or ``None`` when inputs are missing.

    Raises
    ------
    ValueError
        If multiple is unknown or company_metrics contains an invalid metric value.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import compute_multiple
    >>> compute_multiple({"enterprise_value": 100.0, "ebitda": 20.0}, "ev_ebitda")
    5.0

    """
    ...

def score_relative_value(
    peer_set: dict[str, Any] | str,
    dimensions: list[dict[str, Any]] | str,
) -> dict[str, Any]:
    """
    Composite relative-value score of a subject against its peer set.

    ``peer_set`` is the canonical serde ``PeerSet``: ``subject`` and ``peers``
    are ``CompanyMetrics`` objects (identifier, ``attributes``, named metric
    fields, and ``custom``), plus ``period_basis`` (``"ltm"``, ``"ntm"``, or a
    custom label). Each dimension supplies ``label``, ``y_extractor``
    (``{"named": field}``, ``{"multiple": id}``, or ``{"custom": key}``),
    ``x_extractors``, ``weight``, and optional ``direction``
    (``"higher_is_cheap"`` or ``"higher_is_rich"``). Positive composite = cheap.

    Parameters
    ----------
    peer_set : dict or str
        Canonical ``PeerSet`` dict or JSON string.
    dimensions : list[dict] or str
        Canonical ``ScoringDimension`` list or JSON string.

    Returns
    -------
    dict[str, Any]
        ``company_id``, ``composite_score``, ``dimensions``, ``confidence``,
        and ``peer_count``.

    Raises
    ------
    ValueError
        If the payload cannot be parsed, dimensions is empty, or the subject
        and peer metrics are unusable.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import score_relative_value
    >>> blank = dict.fromkeys([
    ...     "enterprise_value",
    ...     "market_cap",
    ...     "share_price",
    ...     "oas_bp",
    ...     "yield_pct",
    ...     "ebitda",
    ...     "revenue",
    ...     "ebit",
    ...     "ufcf",
    ...     "lfcf",
    ...     "net_income",
    ...     "book_value",
    ...     "tangible_book_value",
    ...     "dividends_per_share",
    ...     "leverage",
    ...     "interest_coverage",
    ...     "revenue_growth",
    ...     "ebitda_margin",
    ... ])
    >>> def company(cid, leverage):
    ...     return {"id": cid, "attributes": {}, "custom": {}, **blank, "leverage": leverage}
    >>> result = score_relative_value(
    ...     {
    ...         "subject": company("SUBJECT", 2.0),
    ...         "peers": [company("P1", 1.0), company("P2", 3.0)],
    ...         "period_basis": "ltm",
    ...     },
    ...     [{"label": "Lev", "y_extractor": {"named": "leverage"}, "x_extractors": [], "weight": 1.0}],
    ... )
    >>> (result["company_id"], result["peer_count"])
    ('SUBJECT', 2)

    """
    ...

# Credit scorecard extension

class ScorecardMetric:
    """
    Define one weighted metric in a credit-rating scorecard.

    Parameters
    ----------
    name : str
        Stable metric label used in scorecard reports and validation errors.
    formula : str
        Statement-model formula or node expression used to calculate the metric.
    weight : float
        Non-negative contribution weight for the composite rating; defaults to
        ``1.0`` before normalization across usable metrics.
    thresholds_json : str
        JSON mapping that defines rating thresholds for the calculated metric;
        defaults to an empty mapping.
    description : str or None
        Optional reader-facing explanation of the metric and its credit meaning.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScorecardMetric
    >>> metric = ScorecardMetric("leverage", "total_debt / ebitda", weight=0.5)
    >>> (metric.name, metric.weight)
    ('leverage', 0.5)

    """

    def __init__(
        self,
        name: str,
        formula: str,
        weight: float = 1.0,
        thresholds_json: str = "{}",
        description: str | None = None,
    ) -> None:
        """
        Define one weighted formula and its rating thresholds.

        Parameters
        ----------
        name : str
            Stable metric label used in reports and validation diagnostics.
        formula : str
            Statement formula or node expression evaluated for the metric.
        weight : float, default 1.0
            Non-negative contribution weight before normalization across usable metrics.
        thresholds_json : str, default "{}"
            JSON object mapping rating labels to lower and upper threshold pairs.
        description : str or None, default None
            Optional reader-facing explanation of the metric's credit meaning.

        Raises
        ------
        ValueError
            If thresholds_json is malformed or does not map ratings to numeric ranges.

        """
        ...

    @property
    def name(self) -> str:
        """
        Metric name, used as the key in the report's metric scores.

        Returns
        -------
        str
            Metric name.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def formula(self) -> str:
        """
        DSL formula evaluated to produce the metric value.

        Returns
        -------
        str
            DSL formula for the metric.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def weight(self) -> float:
        """
        Weight of this metric in the overall score.

        Weights are relative and need not sum to 1; the report divides the included
        weight by the configured weight to report ``weight_coverage``.

        Returns
        -------
        float
            Relative weight of the metric in the overall score.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def description(self) -> str | None:
        """
        Optional human-readable description of the metric.

        Returns
        -------
        str or None
            Metric description, or ``None`` when unset.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def thresholds_json(self) -> str:
        """
        Serialize rating-label thresholds to JSON.

        Returns
        -------
        str
            JSON object mapping rating labels to two-element lower and upper
            threshold arrays.

        Raises
        ------
        ValueError
            If the thresholds cannot be serialized to JSON.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ScorecardMetric`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> ScorecardMetric:
        """
        Deserialize one scorecard metric from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing metric formula, weight, and thresholds.

        Returns
        -------
        ScorecardMetric
            Validated `ScorecardMetric` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ScorecardMetric
        >>> metric = ScorecardMetric("leverage", "total_debt / ebitda", weight=0.5)
        >>> ScorecardMetric.from_json(metric.to_json()).formula
        'total_debt / ebitda'

        """
        ...

class ScorecardConfig:
    """
    Configuration for credit scorecard analysis.

    ``period`` optionally pins the rated period (e.g. ``"2025Q4"``); when
    ``None`` the scorecard rates the last actual period in the model if any
    exists, otherwise the last model period.

    Parameters
    ----------
    rating_scale : str
        Rating-scale identifier used to interpret metric thresholds; defaults
        to the ``"S&P"`` scale.
    metrics : list[ScorecardMetric]
        Weighted metric definitions used to calculate the composite rating.
    min_rating : str or None
        Optional minimum acceptable rating used by downstream validation.
    period : str or None
        Optional model period to rate; ``None`` chooses the latest available
        actual or model period.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScorecardConfig
    >>> config = ScorecardConfig(period="2025Q1")
    >>> (config.rating_scale, config.period, config.metrics)
    ('S&P', '2025Q1', [])

    """

    def __init__(
        self,
        rating_scale: str = "S&P",
        metrics: list[ScorecardMetric] = ...,
        min_rating: str | None = None,
        period: str | None = None,
    ) -> None:
        """
        Configure rating scale, weighted metrics, and rated period selection.

        Parameters
        ----------
        rating_scale : str, default "S&P"
            Registered rating-scale identifier used to interpret thresholds.
        metrics : list[ScorecardMetric]
            Weighted metric definitions included in the composite rating.
        min_rating : str or None, default None
            Optional minimum acceptable rating for downstream validation.
        period : str or None, default None
            Model period to rate; ``None`` selects the latest actual period,
            falling back to the last model period.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def rating_scale(self) -> str:
        """
        Rating scale identifier (e.g. ``"S&P"``, ``"Moody's"``, ``"Fitch"``).

        Returns
        -------
        str
            Rating scale identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def min_rating(self) -> str | None:
        """
        Minimum acceptable rating on ``rating_scale``, or ``None`` when the scorecard
        imposes no floor.

        Returns
        -------
        str or None
            Minimum acceptable rating, or ``None``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def period(self) -> str | None:
        """
        Period to rate, as a period-id string (e.g. ``"2025Q4"``).

        ``None`` means the last actual period in the model if any exists, otherwise the
        last model period.

        Returns
        -------
        str or None
            Period-id string to rate, or ``None`` for the default period.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def metrics(self) -> list[ScorecardMetric]:
        """
        Metric definitions evaluated by the scorecard, in configured order.

        Returns
        -------
        list[ScorecardMetric]
            Metric definitions in configured order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def validate(self) -> None:
        """
        Validate this object's invariants without executing a report.

        Raises
        ------
        ValueError
            If required fields are missing, out of range, or internally inconsistent.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ScorecardConfig`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> ScorecardConfig:
        """
        Deserialize a credit-scorecard configuration from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing scale, metrics, period selection, and
            optional minimum-rating policy.

        Returns
        -------
        ScorecardConfig
            Validated `ScorecardConfig` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ScorecardConfig
        >>> config = ScorecardConfig(period="2025Q1")
        >>> ScorecardConfig.from_json(config.to_json()).period
        '2025Q1'

        """
        ...

class ScorecardReport:
    """
    Report produced by ``CreditScorecardExtension.execute``.

    ``data_json()`` includes the rated ``period``, the ``partial`` flag, and
    ``weight_coverage`` alongside the per-metric scores and rating.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScorecardReport
    >>> report = ScorecardReport.from_json('{"status":"success","message":"Complete"}')
    >>> (report.status, report.message, report.errors)
    ('success', 'Complete', [])

    """

    @property
    def status(self) -> str:
        """
        Overall scorecard status after metric evaluation.

        Returns
        -------
        str
            Overall scorecard status after metric evaluation.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def message(self) -> str:
        """
        Human-readable summary of the run.

        Returns
        -------
        str
            Summary message for the run.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def warnings(self) -> list[str]:
        """
        Non-fatal warnings raised while scoring (e.g. an excluded metric).

        Returns
        -------
        list[str]
            Non-fatal warnings raised while scoring.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def errors(self) -> list[str]:
        """
        Per-metric failures. A non-empty list means ``status`` is ``"failed"``.

        Returns
        -------
        list[str]
            Per-metric failure messages.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def data_json(self) -> str:
        """
        Serialize report payload fields to JSON.

        Returns
        -------
        str
            JSON serialization of the structured scorecard payload, including
            the rated period, metric scores, rating, partial flag, and weight coverage.

        Raises
        ------
        ValueError
            If the payload cannot be serialized to JSON.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ScorecardReport`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> ScorecardReport:
        """
        Deserialize a credit-scorecard report from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload returned by a scorecard extension execution.

        Returns
        -------
        ScorecardReport
            Validated `ScorecardReport` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ScorecardReport
        >>> ScorecardReport.from_json('{"status":"success","message":"Complete"}').status
        'success'

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the report header as a single-row pandas ``DataFrame``.

        Columns: ``status``, ``message``, ``rating``, ``rating_scale``,
        ``period``, ``total_score``, ``partial``, ``weight_coverage``,
        ``warning_count``, ``error_count``.

        ``period`` is the rated period-id string. ``weight_coverage`` is a
        decimal fraction in ``[0, 1]``: the included metric weight over the
        configured metric weight, so ``0.8`` means a fifth of the configured
        weight was excluded. ``partial`` is ``True`` when any metric was
        excluded or errored. Fields absent from the report payload are
        ``None``. Per-metric detail lives in ``to_metric_scores_dataframe``.

        Returns
        -------
        pd.DataFrame
            One row describing the scorecard run.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_metric_scores_dataframe(self) -> pd.DataFrame:
        """
        Export the per-metric scores as a pandas ``DataFrame``.

        Columns: ``metric``, ``value``, ``score``, ``weight``,
        ``weighted_score``. One row per scored metric, in configured order; a
        report with no scored metrics still carries the full column schema.

        ``value`` is the metric's evaluated value in its own units, ``score``
        its mapped rating score, ``weight`` the configured weight, and
        ``weighted_score`` is ``score * weight``. Metrics that errored or were
        excluded do not appear here - see ``errors`` and ``weight_coverage``.

        Returns
        -------
        pd.DataFrame
            One row per scored metric.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

class CreditScorecardExtension:
    """
    Credit scorecard extension for rating assignment and stress testing.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import CreditScorecardExtension
    >>> CreditScorecardExtension().config() is None
    True

    """

    def __init__(self) -> None:
        """
        Create an extension with no configuration loaded.

        Returns
        -------
        None

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...
    @staticmethod
    def with_config(config: ScorecardConfig) -> CreditScorecardExtension:
        """
        Create a scorecard extension with a validated configuration.

        Parameters
        ----------
        config : ScorecardConfig
            Rating scale, weighted metrics, and period-selection policy to use.

        Returns
        -------
        CreditScorecardExtension
            New extension preloaded with ``config``.

        Notes
        -----
        This builder returns a copy with the field set and does not raise.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import CreditScorecardExtension, ScorecardConfig
        >>> extension = CreditScorecardExtension.with_config(ScorecardConfig())
        >>> extension.config().rating_scale
        'S&P'

        """
        ...
    def set_config(self, config: ScorecardConfig) -> None:
        """
        Replace the extension's scorecard configuration.

        Parameters
        ----------
        config : ScorecardConfig
            New rating scale, metric set, and period-selection policy to apply.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...
    def config(self) -> ScorecardConfig | None:
        """
        Return the currently loaded configuration, or ``None``.

        Returns
        -------
        ScorecardConfig or None

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.
        """
        ...
    def execute(self, model: FinancialModelSpec | str, results: StatementResult | str) -> ScorecardReport:
        """
        Calculate a credit scorecard against evaluated statement results.

        Parameters
        ----------
        model : FinancialModelSpec or str
            Model specification object or equivalent JSON used to resolve nodes.
        results : StatementResult or str
            Evaluated statement result object or equivalent JSON to rate.

        Returns
        -------
        ScorecardReport
            Scorecard status, diagnostics, rating, and per-metric results for
            the supplied model evaluation.

        Raises
        ------
        ValueError
            If the scorecard configuration, model, or statement results are invalid.

        """
        ...

def validate_scorecard_config(config: ScorecardConfig) -> None:
    """
    Validate a scorecard configuration without executing it.

    Parameters
    ----------
    config : ScorecardConfig
        Rating scale, metrics, thresholds, and period policy to validate.

    Raises
    ------
    ValueError
        If the scorecard configuration is internally inconsistent.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ScorecardConfig, validate_scorecard_config
    >>> validate_scorecard_config(ScorecardConfig())

    """
    ...

# Corkscrew (balance-sheet roll-forward) extension

class AccountType:
    """
    Balance-sheet account classifier: asset / liability / equity.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import AccountType
    >>> AccountType.Asset.value()
    'asset'

    """

    Asset: AccountType
    Liability: AccountType
    Equity: AccountType

    @staticmethod
    def from_str(value: str) -> AccountType:
        """
        Parse a balance-sheet account classification.

        Parameters
        ----------
        value : str
            Case-insensitive ``"asset"``, ``"liability"``, or ``"equity"`` value.

        Returns
        -------
        AccountType
            Enum member corresponding to the exact snake-case account type.

        Raises
        ------
        ValueError
            If value is not asset, liability, or equity.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import AccountType
        >>> AccountType.from_str("liability").value()
        'liability'

        """
        ...
    def value(self) -> str:
        """
        Return the canonical snake-case identifier for this variant.

        Returns
        -------
        str
            Exact JSON identifier: ``"asset"``, ``"liability"``, or ``"equity"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class CorkscrewAccount:
    """
    Map one balance-sheet account to its corkscrew input nodes.

    Identity: ``expected = prev + Σ changes − Σ decreases`` (or
    ``beginning + Σ changes − Σ decreases`` when
    ``beginning_balance_node`` is set). Pair a roll-forward template with
    ``changes`` as increases and ``decreases`` as positive disposals.

    Parameters
    ----------
    node_id : str
        Statement node ID receiving the period-end account balance.
    account_type : AccountType
        Asset, liability, or equity classification controlling change signs.
    changes : list[str]
        Statement node IDs added to the opening balance (increases or signed
        net changes).
    decreases : list[str]
        Statement node IDs subtracted from the opening balance (positive
        repayments, outflows, disposals). Default empty.
    beginning_balance_node : str or None
        Optional node ID supplying the opening balance instead of an inferred
        first-period balance.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import AccountType, CorkscrewAccount
    >>> account = CorkscrewAccount("inventory_end", AccountType.Asset, ["purchases"], ["disposals"])
    >>> (account.node_id, account.changes, account.decreases)
    ('inventory_end', ['purchases'], ['disposals'])

    """

    def __init__(
        self,
        node_id: str,
        account_type: AccountType,
        changes: list[str] = ...,
        decreases: list[str] = ...,
        beginning_balance_node: str | None = None,
    ) -> None:
        """
        Map a balance-sheet account to its roll-forward input nodes.

        Parameters
        ----------
        node_id : str
            Statement node receiving the period-end account balance.
        account_type : AccountType
            Asset, liability, or equity classification controlling change signs.
        changes : list[str]
            Statement node IDs added to the opening balance.
        decreases : list[str]
            Statement node IDs subtracted from the opening balance. Default empty.
        beginning_balance_node : str or None, default None
            Optional explicit opening-balance node; ``None`` uses the account's
            prior-period balance.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def node_id(self) -> str:
        """
        Node id of the balance account being rolled forward.

        Returns
        -------
        str
            Node id of the balance account.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def account_type(self) -> AccountType:
        """
        Balance-sheet classifier: asset, liability, or equity.

        Returns
        -------
        AccountType
            Balance-sheet classifier for the account.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def changes(self) -> list[str]:
        """
        Node ids of the period increases (or signed net changes) added to the
        balance.

        Identity: ``expected = prev + Σ changes − Σ decreases``. Prefer
        :attr:`decreases` for positive outflows so roll-forward decrease
        nodes do not need to be negated.

        Returns
        -------
        list[str]
            Node ids of the change series added to the balance.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def decreases(self) -> list[str]:
        """
        Node ids of positive decreases (repayments, outflows, disposals)
        subtracted from the balance.

        Returns
        -------
        list[str]
            Node ids of the decrease series subtracted from the balance.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def beginning_balance_node(self) -> str | None:
        """
        Node id overriding the beginning balance, or ``None`` to use the account's own
        prior-period closing balance.

        Returns
        -------
        str or None
            Beginning-balance override node id, or ``None``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `CorkscrewAccount`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> CorkscrewAccount:
        """
        Deserialize an account mapping from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload identifying the balance node, type, and change nodes.

        Returns
        -------
        CorkscrewAccount
            Validated `CorkscrewAccount` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import AccountType, CorkscrewAccount
        >>> account = CorkscrewAccount("cash", AccountType.Asset, ["cash_change"])
        >>> CorkscrewAccount.from_json(account.to_json()).account_type.value()
        'asset'

        """
        ...

class CorkscrewConfig:
    """
    Configure corkscrew roll-forward validation across balance accounts.

    Parameters
    ----------
    accounts : list[CorkscrewAccount]
        Account mappings to reconcile; defaults to an empty validation set.
    tolerance : float
        Absolute currency-unit tolerance allowed for reconciliation differences;
        defaults to ``0.01``.
    fail_on_error : bool
        Whether reconciliation errors abort execution instead of being reported.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import CorkscrewConfig
    >>> config = CorkscrewConfig(tolerance=0.05)
    >>> (config.accounts, config.tolerance, config.fail_on_error)
    ([], 0.05, False)

    """

    def __init__(
        self,
        accounts: list[CorkscrewAccount] = ...,
        tolerance: float = 0.01,
        fail_on_error: bool = False,
    ) -> None:
        """
        Configure account roll-forwards and reconciliation failure policy.

        Parameters
        ----------
        accounts : list[CorkscrewAccount]
            Balance-sheet accounts and their change-node mappings.
        tolerance : float, default 0.01
            Maximum absolute reconciliation difference in model currency units.
        fail_on_error : bool, default False
            Whether any failed account reconciliation makes execution fail.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def accounts(self) -> list[CorkscrewAccount]:
        """
        Balance accounts validated by this configuration, in configured order.

        Returns
        -------
        list[CorkscrewAccount]
            Validated balance accounts in configured order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def tolerance(self) -> float:
        """
        Absolute roll-forward tolerance, in the balance node's own units.

        A period is flagged when
        ``abs(closing - (opening + sum(changes) - sum(decreases))) >
        tolerance``.

        Returns
        -------
        float
            Absolute roll-forward tolerance in the balance node's units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def fail_on_error(self) -> bool:
        """
        When ``True``, any roll-forward violation is fatal (reported as an error) rather
        than a warning.

        Returns
        -------
        bool
            Whether roll-forward violations are treated as fatal.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `CorkscrewConfig`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> CorkscrewConfig:
        """
        Deserialize corkscrew validation settings from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing account mappings and reconciliation policy.

        Returns
        -------
        CorkscrewConfig
            Validated `CorkscrewConfig` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import CorkscrewConfig
        >>> config = CorkscrewConfig(tolerance=0.05)
        >>> CorkscrewConfig.from_json(config.to_json()).tolerance
        0.05

        """
        ...

class CorkscrewReport:
    """
    Report produced by ``CorkscrewExtension.execute``.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import CorkscrewReport
    >>> report = CorkscrewReport.from_json('{"status":"success","message":"Balanced"}')
    >>> (report.status, report.message, report.warnings)
    ('success', 'Balanced', [])

    """

    @property
    def status(self) -> str:
        """
        Corkscrew run status: ``success`` when the walk completed, else ``failed``.

        Returns
        -------
        str
            Overall execution status.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def message(self) -> str:
        """
        Human-readable summary of the validation run.

        Returns
        -------
        str
            Summary message for the validation run.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def warnings(self) -> list[str]:
        """
        Non-fatal warnings, including roll-forward breaks reported when
        ``fail_on_error`` is ``False``.

        Returns
        -------
        list[str]
            Non-fatal warnings raised during validation.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def errors(self) -> list[str]:
        """
        Roll-forward violations treated as fatal (``fail_on_error=True``) plus any
        structural failure. A non-empty list means ``status`` is ``"failed"``.

        Returns
        -------
        list[str]
            Fatal validation failures.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def data_json(self) -> str:
        """
        Serialize report payload fields to JSON.

        Returns
        -------
        str
            JSON serialization of the structured reconciliation payload,
            including account-level balance checks and differences.

        Raises
        ------
        ValueError
            If the payload cannot be serialized to JSON.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `CorkscrewReport`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> CorkscrewReport:
        """
        Deserialize a corkscrew validation report from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload returned by a corkscrew extension execution.

        Returns
        -------
        CorkscrewReport
            Validated `CorkscrewReport` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import CorkscrewReport
        >>> CorkscrewReport.from_json('{"status":"success","message":"Balanced"}').status
        'success'

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the report header as a single-row pandas ``DataFrame``.

        Columns: ``status``, ``message``, ``account_count``,
        ``warning_count``, ``error_count``.

        ``account_count`` is the number of validated accounts. Per-account
        detail lives in ``to_validations_dataframe``.

        Returns
        -------
        pd.DataFrame
            One row describing the validation run.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_validations_dataframe(self) -> pd.DataFrame:
        """
        Export the per-account roll-forward validations as a pandas
        ``DataFrame``.

        Columns: ``account``, ``type``, ``periods_validated``, ``max_error``,
        ``is_valid``. One row per validated account, in configured order; a
        report with no validations still carries the full column schema.

        ``type`` is the account classifier (``"asset"``, ``"liability"``,
        ``"equity"``), ``periods_validated`` is a count of model periods, and
        ``max_error`` is the largest absolute roll-forward break across those
        periods, in the balance node's own units. ``is_valid`` is ``False``
        when ``max_error`` exceeded the configured tolerance.

        Returns
        -------
        pd.DataFrame
            One row per validated account.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

class CorkscrewExtension:
    """
    Corkscrew extension for balance-sheet roll-forward validation.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import CorkscrewExtension
    >>> CorkscrewExtension().config() is None
    True

    """

    def __init__(self) -> None:
        """
        Create an extension with no configuration loaded.

        Returns
        -------
        None

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...
    @staticmethod
    def with_config(config: CorkscrewConfig) -> CorkscrewExtension:
        """
        Create a corkscrew extension with reconciliation settings.

        Parameters
        ----------
        config : CorkscrewConfig
            Accounts, tolerance, and error policy used during reconciliation.

        Returns
        -------
        CorkscrewExtension
            New extension preloaded with ``config``.

        Notes
        -----
        This builder returns a copy with the field set and does not raise.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import CorkscrewConfig, CorkscrewExtension
        >>> extension = CorkscrewExtension.with_config(CorkscrewConfig())
        >>> extension.config().tolerance
        0.01

        """
        ...
    def set_config(self, config: CorkscrewConfig) -> None:
        """
        Replace the extension's reconciliation configuration.

        Parameters
        ----------
        config : CorkscrewConfig
            Accounts, tolerance, and error policy to apply on the next run.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...
    def config(self) -> CorkscrewConfig | None:
        """
        Return the currently loaded configuration, or ``None``.

        Returns
        -------
        CorkscrewConfig or None

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.
        """
        ...
    def execute(self, model: FinancialModelSpec | str, results: StatementResult | str) -> CorkscrewReport:
        """
        Validate account roll-forwards against evaluated statement results.

        Parameters
        ----------
        model : FinancialModelSpec or str
            Model specification object or JSON used to resolve configured nodes.
        results : StatementResult or str
            Evaluated statement results object or JSON to reconcile.

        Returns
        -------
        CorkscrewReport
            Reconciliation status, diagnostics, and account-level roll-forward
            results for the supplied model evaluation.

        Raises
        ------
        ValueError
            If the corkscrew configuration, model, or statement results are invalid.

        """
        ...

# Vintage template

def add_vintage_buildup(
    model: FinancialModelSpec | str,
    name: str,
    new_volume_node: str,
    decay_curve: list[float],
) -> FinancialModelSpec:
    """
    Apply the vintage (cohort) buildup template to a model spec.

    Returns a typed ``FinancialModelSpec`` with the convolution
    node added.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with the cohort schedule.
    name : str
        Prefix used to name the generated vintage buildup nodes.
    new_volume_node : str
        Existing node ID that supplies new volume for each cohort period.
    decay_curve : list[float]
        Ordered cohort-retention factors by elapsed period, expressed as decimal
        multipliers of original volume.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing the generated vintage-convolution node.

    Raises
    ------
    ValueError
        If model JSON, a node identifier, or decay_curve is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import add_vintage_buildup
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> _ = builder.value("new_volume", [("2025Q1", 10.0), ("2025Q2", 12.0)])
    >>> model = builder.build()
    >>> updated = add_vintage_buildup(model, "customers", "new_volume", [1.0, 0.8])
    >>> updated.node_count > model.node_count
    True

    """
    ...

# Roll-forward template

def add_roll_forward(
    model: FinancialModelSpec | str,
    name: str,
    increases: list[str],
    decreases: list[str],
) -> FinancialModelSpec:
    """
    Apply the roll-forward template (Beginning + Increases - Decreases = Ending) to a model spec.

    Returns a typed ``FinancialModelSpec`` with ``{name}_beg`` and
    ``{name}_end`` nodes added. The first period opens at zero; use
    ``add_roll_forward_with_opening`` for an explicit opening balance.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with roll-forward nodes.
    name : str
        Prefix used to name the generated beginning and ending balance nodes.
    increases : list[str]
        Existing node IDs whose period values increase the ending balance.
    decreases : list[str]
        Existing node IDs whose period values decrease the ending balance.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing zero-opening beginning- and ending-balance nodes.

    Raises
    ------
    ValueError
        If model JSON or a roll-forward node identifier is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import add_roll_forward
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> _ = builder.value("adds", [("2025Q1", 10.0), ("2025Q2", 12.0)])
    >>> model = builder.build()
    >>> updated = add_roll_forward(model, "balance", ["adds"], [])
    >>> updated.has_node("balance_end")
    True

    """
    ...

def add_roll_forward_with_opening(
    model: FinancialModelSpec | str,
    name: str,
    increases: list[str],
    decreases: list[str],
    opening: float,
) -> FinancialModelSpec:
    """
    Apply the roll-forward template with an explicit first-period opening balance.

    Same as ``add_roll_forward`` except the first period's beginning balance
    is ``opening`` instead of zero. Returns a typed ``FinancialModelSpec``.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with roll-forward nodes.
    name : str
        Prefix used to name the generated beginning and ending balance nodes.
    increases : list[str]
        Existing node IDs whose period values increase the ending balance.
    decreases : list[str]
        Existing node IDs whose period values decrease the ending balance.
    opening : float
        Beginning balance assigned to the first modeled period in model units.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing the seeded beginning- and ending-balance nodes.

    Raises
    ------
    ValueError
        If model JSON, opening, or a roll-forward node identifier is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import add_roll_forward_with_opening
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> _ = builder.value("adds", [("2025Q1", 10.0), ("2025Q2", 12.0)])
    >>> model = builder.build()
    >>> updated = add_roll_forward_with_opening(model, "balance", ["adds"], [], 100.0)
    >>> updated.has_node("balance_beg")
    True

    """
    ...

# Real-estate template

class SimpleLeaseSpec:
    """
    Describe a simple per-lease rent schedule for a property model.

    Parameters
    ----------
    node_id : str
        Statement node ID receiving the lease's rental-revenue series.
    start : str
        First included model period label for the lease term.
    base_rent : float
        Contractual rent per modeled period before growth, concessions, and
        occupancy scaling, in the model's currency units.
    end : str or None
        Optional final included model period; ``None`` extends through the
        model horizon.
    growth_rate : float
        Periodic decimal rent-growth rate, such as ``0.03`` for 3%; defaults
        to zero growth.
    free_rent_periods : int
        Number of initial included periods with rent set to zero; defaults to
        no concession.
    occupancy : float
        Decimal occupancy multiplier applied to scheduled rent; defaults to
        fully occupied ``1.0``.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import SimpleLeaseSpec
    >>> lease = SimpleLeaseSpec("lease_a", "2025Q1", 100.0, end="2025Q4")
    >>> (lease.node_id, lease.start, lease.end)
    ('lease_a', '2025Q1', '2025Q4')

    """

    def __init__(
        self,
        node_id: str,
        start: str,
        base_rent: float,
        end: str | None = None,
        growth_rate: float = 0.0,
        free_rent_periods: int = 0,
        occupancy: float = 1.0,
    ) -> None:
        """
        Define a basic lease term, rent, growth, concessions, and occupancy.

        Parameters
        ----------
        node_id : str
            Statement node receiving the lease's rental-revenue series.
        start : str
            First included model-period label for the lease term.
        base_rent : float
            Contractual periodic rent before growth, concessions, and occupancy
            scaling, in the model's currency units.
        end : str or None, default None
            Optional final included model period; ``None`` extends through the
            model horizon.
        growth_rate : float, default 0.0
            Periodic decimal rent-growth rate, such as ``0.03`` for 3%.
        free_rent_periods : int, default 0
            Number of initial included periods with rent set to zero.
        occupancy : float, default 1.0
            Decimal occupancy multiplier applied to scheduled rent.

        Raises
        ------
        ValueError
            If start or end is not a valid model period.

        """
        ...

    @property
    def node_id(self) -> str:
        """
        Node id storing this lease's rent revenue series.

        Returns
        -------
        str
            Node id for the lease's rent revenue series.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def start(self) -> str:
        """
        First period (inclusive) the lease is active, as a period-id string (e.g.
        ``"2025Q1"``).

        Returns
        -------
        str
            First active period as a period-id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def end(self) -> str | None:
        """
        Last period (inclusive) the lease is active, or ``None`` to run through the
        model end.

        Returns
        -------
        str or None
            Last active period as a period-id string, or ``None``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def base_rent(self) -> float:
        """
        Base rent for one model period at ``start``, in model currency units.

        A quarterly model means rent per quarter, not per year.

        Returns
        -------
        float
            Base rent per model period at ``start``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def growth_rate(self) -> float:
        """
        Growth rate compounded every model period after ``start``, as a decimal fraction
        (``0.03`` = +3% per period).

        Returns
        -------
        float
            Per-period growth rate as a decimal fraction.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def free_rent_periods(self) -> int:
        """
        Number of model periods of free rent counted from ``start``.

        Returns
        -------
        int
            Count of free-rent model periods from ``start``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def occupancy(self) -> float:
        """
        Occupancy factor in ``[0, 1]`` applied to rent.

        Returns
        -------
        float
            Occupancy factor in ``[0, 1]``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def validate(self) -> None:
        """
        Validate this object's invariants without executing a report.

        Raises
        ------
        ValueError
            If required fields are missing, out of range, or internally inconsistent.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `SimpleLeaseSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> SimpleLeaseSpec:
        """
        Deserialize a simple lease schedule from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing lease term, rent, growth, and occupancy.

        Returns
        -------
        SimpleLeaseSpec
            Validated `SimpleLeaseSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import SimpleLeaseSpec
        >>> lease = SimpleLeaseSpec("lease_a", "2025Q1", 100.0)
        >>> SimpleLeaseSpec.from_json(lease.to_json()).base_rent
        100.0

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the lease spec as a single-row pandas ``DataFrame``.

        Columns: ``node_id``, ``start``, ``end``, ``base_rent``,
        ``growth_rate``, ``free_rent_periods``, ``occupancy``.

        ``start`` and ``end`` are period-id strings (``end`` is ``None`` for a
        lease running to the model end). ``base_rent`` is per model period,
        ``growth_rate`` and ``occupancy`` are decimal fractions, and
        ``free_rent_periods`` is a count of model periods.

        Returns
        -------
        pd.DataFrame
            One row describing the lease.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class RentStepSpec:
    """
    Reset a lease's base rent from one model period onward.

    Parameters
    ----------
    start : str
        First model period label at which the stepped rent applies.
    rent : float
        Replacement periodic rent in the model's currency units.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import RentStepSpec
    >>> step = RentStepSpec("2026Q1", 125.0)
    >>> (step.start, step.rent)
    ('2026Q1', 125.0)

    """

    def __init__(self, start: str, rent: float) -> None:
        """
        Reset a lease's periodic rent from a specified model period onward.

        Parameters
        ----------
        start : str
            First model-period label at which the replacement rent applies.
        rent : float
            Replacement periodic rent in the model's currency units.

        Raises
        ------
        ValueError
            If start is not a valid model period.

        """
        ...

    @property
    def start(self) -> str:
        """
        Period (inclusive) this rent level takes effect, as a period-id string.

        Returns
        -------
        str
            Effective period as a period-id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def rent(self) -> float:
        """
        Rent for one model period from ``start``, in model currency units.

        This replaces the prevailing rent level rather than adding to it, and restarts
        growth compounding from ``start``.

        Returns
        -------
        float
            Rent per model period from ``start``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `RentStepSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> RentStepSpec:
        """
        Deserialize a rent-step specification from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing the step start period and replacement rent.

        Returns
        -------
        RentStepSpec
            Validated `RentStepSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import RentStepSpec
        >>> step = RentStepSpec("2026Q1", 125.0)
        >>> RentStepSpec.from_json(step.to_json()).rent
        125.0

        """
        ...

class FreeRentWindowSpec:
    """
    Define a finite concession window that sets lease rent to zero.

    Parameters
    ----------
    start : str
        First model period label affected by the free-rent concession.
    periods : int
        Number of consecutive modeled periods with rent set to zero.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import FreeRentWindowSpec
    >>> window = FreeRentWindowSpec("2025Q1", 2)
    >>> (window.start, window.periods)
    ('2025Q1', 2)

    """

    def __init__(self, start: str, periods: int) -> None:
        """
        Define a dated window of consecutive rent-free model periods.

        Parameters
        ----------
        start : str
            First model-period label affected by the concession.
        periods : int
            Number of consecutive modeled periods with rent set to zero.

        Raises
        ------
        ValueError
            If start is not a valid model period.

        """
        ...

    @property
    def start(self) -> str:
        """
        Period (inclusive) free rent starts, as a period-id string.

        Returns
        -------
        str
            First free-rent period as a period-id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def periods(self) -> int:
        """
        Length of the concession as a count of model periods.

        Returns
        -------
        int
            Concession length in model periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `FreeRentWindowSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> FreeRentWindowSpec:
        """
        Deserialize a free-rent concession window from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing the concession start period and duration.

        Returns
        -------
        FreeRentWindowSpec
            Validated `FreeRentWindowSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import FreeRentWindowSpec
        >>> window = FreeRentWindowSpec("2025Q1", 2)
        >>> FreeRentWindowSpec.from_json(window.to_json()).periods
        2

        """
        ...

class RenewalSpec:
    """
    Model a lease renewal in expected-value terms after the base term.

    Parameters
    ----------
    term_periods : int
        Number of modeled periods in the renewal term when the tenant renews.
    probability : float
        Decimal probability of renewal from zero through one.
    downtime_periods : int
        Vacancy periods between the original lease and renewal; defaults to zero.
    rent_factor : float
        Multiplier applied to the ending scheduled rent for the renewal term;
        defaults to ``1.0``.
    free_rent_periods : int
        Initial renewal periods with rent set to zero; defaults to no concession.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import RenewalSpec
    >>> renewal = RenewalSpec(4, 0.75, downtime_periods=1)
    >>> (renewal.term_periods, renewal.probability, renewal.downtime_periods)
    (4, 0.75, 1)

    """

    def __init__(
        self,
        term_periods: int,
        probability: float,
        downtime_periods: int = 0,
        rent_factor: float = 1.0,
        free_rent_periods: int = 0,
    ) -> None:
        """
        Define expected-value assumptions for a lease renewal term.

        Parameters
        ----------
        term_periods : int
            Number of modeled periods in the renewal term when renewal occurs.
        probability : float
            Decimal renewal probability from zero through one.
        downtime_periods : int, default 0
            Vacancy periods between the original lease and renewal.
        rent_factor : float, default 1.0
            Multiplier applied to ending scheduled rent for the renewal term.
        free_rent_periods : int, default 0
            Initial renewal periods with rent set to zero.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def term_periods(self) -> int:
        """
        Renewal term length as a count of model periods.

        Returns
        -------
        int
            Renewal term length in model periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def probability(self) -> float:
        """
        Probability of renewal as a decimal fraction in ``[0, 1]``.

        Renewal is modelled in expected-value terms, so this weights the renewal rent
        rather than selecting a branch.

        Returns
        -------
        float
            Renewal probability in ``[0, 1]``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def downtime_periods(self) -> int:
        """
        Rent-free downtime after the initial term ends, as a count of model periods.

        Returns
        -------
        int
            Downtime length in model periods.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def rent_factor(self) -> float:
        """
        Multiplier applied to the last contractual rent of the initial term (``1.05``
        means renewal starts 5% above the prior rent level).

        Returns
        -------
        float
            Multiplier on the prior rent level at renewal.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def free_rent_periods(self) -> int:
        """
        Number of model periods of free rent at renewal start.

        Returns
        -------
        int
            Count of free-rent model periods at renewal start.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def validate(self) -> None:
        """
        Validate this object's invariants without executing a report.

        Raises
        ------
        ValueError
            If required fields are missing, out of range, or internally inconsistent.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `RenewalSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> RenewalSpec:
        """
        Deserialize renewal assumptions from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing renewal term, probability, downtime, and
            rent assumptions.

        Returns
        -------
        RenewalSpec
            Validated `RenewalSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import RenewalSpec
        >>> renewal = RenewalSpec(4, 0.75)
        >>> RenewalSpec.from_json(renewal.to_json()).probability
        0.75

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the renewal spec as a single-row pandas ``DataFrame``.

        Columns: ``downtime_periods``, ``term_periods``, ``probability``,
        ``rent_factor``, ``free_rent_periods``.

        The three ``*_periods`` columns are counts of model periods;
        ``probability`` is a decimal fraction in ``[0, 1]`` and
        ``rent_factor`` is a multiplier on the prior rent level.

        Returns
        -------
        pd.DataFrame
            One row describing the renewal terms.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class LeaseGrowthConvention:
    """
    Compounding convention for lease rent growth.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import LeaseGrowthConvention
    >>> LeaseGrowthConvention.PerPeriod.value()
    'per_period'

    """

    PerPeriod: LeaseGrowthConvention
    AnnualEscalator: LeaseGrowthConvention

    @staticmethod
    def from_str(value: str) -> LeaseGrowthConvention:
        """
        Parse a lease rent-growth compounding convention.

        Parameters
        ----------
        value : str
            Case-insensitive ``"per_period"`` or ``"annual_escalator"`` value.

        Returns
        -------
        LeaseGrowthConvention
            Enum member corresponding to the exact snake-case growth convention.

        Raises
        ------
        ValueError
            If value is not per_period or annual_escalator.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import LeaseGrowthConvention
        >>> LeaseGrowthConvention.from_str("annual_escalator").value()
        'annual_escalator'

        """
        ...
    def value(self) -> str:
        """
        Return the canonical snake-case identifier for this variant.

        Returns
        -------
        str
            Exact JSON identifier, either ``"per_period"`` or
            ``"annual_escalator"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class LeaseSpec:
    """
    Describe a rich lease schedule for rent-roll generation.

    Parameters
    ----------
    node_id : str
        Statement node ID receiving the lease's rental-revenue series.
    start : str
        First included model period label for the lease term.
    base_rent : float
        Contractual periodic rent before escalators, concessions, and occupancy
        scaling, in the model's currency units.
    end : str or None
        Optional final included model period; ``None`` extends through horizon.
    growth_rate : float
        Decimal rent-growth rate interpreted by ``growth_convention``.
    growth_convention : LeaseGrowthConvention
        Whether rent growth compounds every model period or as an annual
        step. Default ``AnnualEscalator`` (Argus/NCREIF anniversary bump).
        ``PerPeriod`` must be set explicitly.
    rent_steps : list[RentStepSpec]
        Explicit rent resets applied from each step's start period onward.
    free_rent_periods : int
        Number of initial included periods with rent set to zero.
    free_rent_windows : list[FreeRentWindowSpec]
        Additional dated rent-free concession windows within the lease term.
    occupancy : float
        Decimal occupancy multiplier applied to scheduled rent.
    renewal : RenewalSpec or None
        Optional expected-value renewal assumptions applied after the base term.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import LeaseSpec
    >>> lease = LeaseSpec("lease_a", "2025Q1", 100.0, end="2025Q4")
    >>> (lease.node_id, lease.base_rent, lease.growth_convention.value())
    ('lease_a', 100.0, 'annual_escalator')

    """

    def __init__(
        self,
        node_id: str,
        start: str,
        base_rent: float,
        end: str | None = None,
        growth_rate: float = 0.0,
        growth_convention: LeaseGrowthConvention = ...,
        rent_steps: list[RentStepSpec] = ...,
        free_rent_periods: int = 0,
        free_rent_windows: list[FreeRentWindowSpec] = ...,
        occupancy: float = 1.0,
        renewal: RenewalSpec | None = None,
    ) -> None:
        """
        Define a lease schedule with escalators, concessions, and renewal terms.

        Parameters
        ----------
        node_id : str
            Statement node receiving the lease's rental-revenue series.
        start : str
            First included model-period label for the lease term.
        base_rent : float
            Contractual periodic rent before escalators, concessions, and
            occupancy scaling, in the model's currency units.
        end : str or None, default None
            Optional final included model period; ``None`` extends through horizon.
        growth_rate : float, default 0.0
            Decimal rent-growth rate interpreted by ``growth_convention``.
        growth_convention : LeaseGrowthConvention
            Whether growth compounds every model period or as an annual
            step. Default ``AnnualEscalator``; set ``PerPeriod`` explicitly
            for per-period compounding.
        rent_steps : list[RentStepSpec]
            Explicit rent resets applied from each step's start period onward.
        free_rent_periods : int, default 0
            Number of initial included periods with rent set to zero.
        free_rent_windows : list[FreeRentWindowSpec]
            Additional dated rent-free windows within the base lease term.
        occupancy : float, default 1.0
            Decimal occupancy multiplier applied to scheduled rent.
        renewal : RenewalSpec or None, default None
            Optional expected-value renewal assumptions applied after the base term.

        Raises
        ------
        ValueError
            If start or end is not a valid model period.

        """
        ...

    @property
    def node_id(self) -> str:
        """
        Base node id; per-lease detail nodes are derived from it (``{node_id}.pgi``,
        ``.free_rent``, ``.vacancy_loss``, ``.effective_rent``).

        Returns
        -------
        str
            Base node id for the lease's derived detail nodes.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def start(self) -> str:
        """
        First period (inclusive) the lease is active, as a period-id string (e.g.
        ``"2025Q1"``).

        Returns
        -------
        str
            First active period as a period-id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def end(self) -> str | None:
        """
        Last period (inclusive) of the initial term, or ``None`` to run through the
        model end (which also disables renewal modelling).

        Returns
        -------
        str or None
            Last period of the initial term, or ``None``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def base_rent(self) -> float:
        """
        Base rent for one model period at ``start``, in model currency units.

        A quarterly model means rent per quarter, not per year.

        Returns
        -------
        float
            Base rent per model period at ``start``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def growth_rate(self) -> float:
        """
        Growth rate applied within a rent segment as a decimal fraction (``0.03`` =
        +3%), compounded per ``growth_convention``.

        Returns
        -------
        float
            Segment growth rate as a decimal fraction.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def growth_convention(self) -> LeaseGrowthConvention:
        """
        Compounding convention for ``growth_rate``: every model period (``per_period``)
        or once per lease-start anniversary (``annual_escalator``).

        Returns
        -------
        LeaseGrowthConvention
            Compounding convention for ``growth_rate``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def free_rent_periods(self) -> int:
        """
        Number of model periods of free rent counted from ``start``, before any
        additional ``free_rent_windows``.

        Returns
        -------
        int
            Count of free-rent model periods from ``start``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def occupancy(self) -> float:
        """
        Occupancy factor in ``[0, 1]`` applied to non-free contractual rent.

        Returns
        -------
        float
            Occupancy factor in ``[0, 1]``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def renewal(self) -> RenewalSpec | None:
        """
        Renewal modelling applied after ``end``, or ``None`` for no renewal.

        Returns
        -------
        RenewalSpec or None
            Renewal specification, or ``None``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def validate(self) -> None:
        """
        Validate this object's invariants without executing a report.

        Raises
        ------
        ValueError
            If required fields are missing, out of range, or internally inconsistent.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `LeaseSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> LeaseSpec:
        """
        Deserialize a rich lease schedule from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing term, rent, escalation, concession, and
            renewal assumptions.

        Returns
        -------
        LeaseSpec
            Validated `LeaseSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import LeaseSpec
        >>> lease = LeaseSpec("lease_a", "2025Q1", 100.0)
        >>> LeaseSpec.from_json(lease.to_json()).start
        '2025Q1'

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the lease spec as a single-row pandas ``DataFrame``.

        Columns: ``node_id``, ``start``, ``end``, ``base_rent``,
        ``growth_rate``, ``growth_convention``, ``free_rent_periods``,
        ``occupancy``, ``rent_step_count``, ``free_rent_window_count``,
        ``has_renewal``.

        ``start`` and ``end`` are period-id strings, ``base_rent`` is per
        model period, ``growth_rate`` and ``occupancy`` are decimal fractions,
        and ``free_rent_periods`` is a count of model periods. The nested
        collections are summarised as counts here - read ``renewal`` (and its
        own ``to_dataframe``) for the renewal terms.

        Returns
        -------
        pd.DataFrame
            One row describing the lease.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class RentRollOutputNodes:
    """
    Name the aggregate model nodes produced by a rent-roll template.

    Parameters
    ----------
    rent_pgi_node : str
        Node ID for potential gross rent before concessions and vacancy.
    free_rent_node : str
        Node ID for rent waived through free-rent concessions.
    vacancy_loss_node : str
        Node ID for the revenue reduction caused by vacancy or occupancy.
    rent_effective_node : str
        Node ID for effective rent after concessions and vacancy adjustments.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import RentRollOutputNodes
    >>> nodes = RentRollOutputNodes()
    >>> (nodes.rent_pgi_node, nodes.rent_effective_node)
    ('rent_pgi', 'rent_effective')

    """

    def __init__(
        self,
        rent_pgi_node: str = "rent_pgi",
        free_rent_node: str = "free_rent",
        vacancy_loss_node: str = "vacancy_loss",
        rent_effective_node: str = "rent_effective",
    ) -> None:
        """
        Name the four aggregate statement nodes produced by a rent roll.

        Parameters
        ----------
        rent_pgi_node : str, default "rent_pgi"
            Node ID for potential gross rent before concessions and vacancy.
        free_rent_node : str, default "free_rent"
            Node ID for rent waived through free-rent concessions.
        vacancy_loss_node : str, default "vacancy_loss"
            Node ID for the revenue reduction caused by vacancy or occupancy.
        rent_effective_node : str, default "rent_effective"
            Node ID for effective rent after concessions and vacancy adjustments.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def rent_pgi_node(self) -> str:
        """
        Node id holding total contractual rent (potential gross income) across all
        leases.

        Returns
        -------
        str
            Node id for total potential gross income.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def free_rent_node(self) -> str:
        """
        Node id holding total free-rent concessions.

        Returns
        -------
        str
            Node id for total free-rent concessions.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def vacancy_loss_node(self) -> str:
        """
        Node id holding total vacancy loss, including occupancy and renewal-probability
        effects.

        Returns
        -------
        str
            Node id for total vacancy loss.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def rent_effective_node(self) -> str:
        """
        Node id holding total effective rent, the EGI rent component ``rent_pgi -
        free_rent - vacancy_loss``.

        Returns
        -------
        str
            Node id for total effective rent.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `RentRollOutputNodes`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> RentRollOutputNodes:
        """
        Deserialize rent-roll output-node names from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload identifying potential, concession, vacancy, and
            effective-rent output nodes.

        Returns
        -------
        RentRollOutputNodes
            Validated `RentRollOutputNodes` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import RentRollOutputNodes
        >>> nodes = RentRollOutputNodes()
        >>> RentRollOutputNodes.from_json(nodes.to_json()).vacancy_loss_node
        'vacancy_loss'

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the node-id mapping as a single-row pandas ``DataFrame``.

        Columns: ``rent_pgi_node``, ``free_rent_node``, ``vacancy_loss_node``,
        ``rent_effective_node``. Every value is a statement node id, not a
        numeric amount.

        Returns
        -------
        pd.DataFrame
            One row of rent-roll output node ids.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class ManagementFeeBase:
    """
    Basis for management fee calculation.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ManagementFeeBase
    >>> ManagementFeeBase.Egi.value()
    'egi'

    """

    Egi: ManagementFeeBase
    EffectiveRent: ManagementFeeBase

    @staticmethod
    def from_str(value: str) -> ManagementFeeBase:
        """
        Parse a management-fee calculation basis.

        Parameters
        ----------
        value : str
            Case-insensitive ``"egi"`` or ``"effective_rent"`` basis value.

        Returns
        -------
        ManagementFeeBase
            Enum member corresponding to the exact snake-case fee basis.

        Raises
        ------
        ValueError
            If value is not egi or effective_rent.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ManagementFeeBase
        >>> ManagementFeeBase.from_str("effective_rent").value()
        'effective_rent'

        """
        ...
    def value(self) -> str:
        """
        Return the canonical snake-case identifier for this variant.

        Returns
        -------
        str
            Exact JSON identifier, either ``"egi"`` or ``"effective_rent"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class ManagementFeeSpec:
    """
    Set a percentage management fee and the revenue base it applies to.

    Parameters
    ----------
    rate : float
        Decimal fee rate, such as ``0.03`` for a 3% management fee.
    base : ManagementFeeBase
        Effective-rent or EGI base used to calculate the fee; defaults to the
        binding's standard basis.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import ManagementFeeBase, ManagementFeeSpec
    >>> fee = ManagementFeeSpec(0.03, ManagementFeeBase.Egi)
    >>> (fee.rate, fee.base.value())
    (0.03, 'egi')

    """

    def __init__(self, rate: float, base: ManagementFeeBase = ...) -> None:
        """
        Define a percentage management fee and its revenue base.

        Parameters
        ----------
        rate : float
            Decimal fee rate, such as ``0.03`` for a 3% management fee.
        base : ManagementFeeBase
            Effective-rent or EGI base used to calculate the fee.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def rate(self) -> float:
        """
        Management fee rate as a decimal fraction (``0.03`` = 3%).

        Returns
        -------
        float
            Management fee rate as a decimal fraction.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def base(self) -> ManagementFeeBase:
        """
        Basis the fee applies to: ``egi`` (effective gross income) or ``effective_rent``
        (rent only, excluding other income).

        Returns
        -------
        ManagementFeeBase
            Basis the management fee applies to.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `ManagementFeeSpec`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> ManagementFeeSpec:
        """
        Deserialize management-fee assumptions from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload containing the decimal rate and revenue basis.

        Returns
        -------
        ManagementFeeSpec
            Validated `ManagementFeeSpec` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import ManagementFeeSpec
        >>> fee = ManagementFeeSpec(0.03)
        >>> ManagementFeeSpec.from_json(fee.to_json()).rate
        0.03

        """
        ...

class PropertyTemplateNodes:
    """
    Name generated node IDs for a property operating-statement template.

    Parameters
    ----------
    rent_roll : RentRollOutputNodes or None
        Optional rent-roll output names; ``None`` uses the template defaults.
    other_income_total_node : str
        Node ID aggregating other-income components.
    egi_node : str
        Node ID for effective gross income after rent and other income.
    management_fee_node : str
        Node ID for the management-fee expense series.
    opex_total_node : str
        Node ID aggregating operating-expense components.
    noi_node : str
        Node ID for net operating income before capital expenditures.
    capex_total_node : str
        Node ID aggregating capital-expenditure components.
    ncf_node : str
        Node ID for net cash flow after operating items and capital expenditure.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import PropertyTemplateNodes
    >>> nodes = PropertyTemplateNodes()
    >>> (nodes.noi_node, nodes.ncf_node, nodes.rent_roll.rent_effective_node)
    ('noi', 'ncf', 'rent_effective')

    """

    def __init__(
        self,
        rent_roll: RentRollOutputNodes | None = None,
        other_income_total_node: str = "other_income_total",
        egi_node: str = "egi",
        management_fee_node: str = "management_fee",
        opex_total_node: str = "opex_total",
        noi_node: str = "noi",
        capex_total_node: str = "capex_total",
        ncf_node: str = "ncf",
    ) -> None:
        """
        Name the generated statement nodes for a property operating template.

        Parameters
        ----------
        rent_roll : RentRollOutputNodes or None, default None
            Optional rent-roll output names; ``None`` uses the template defaults.
        other_income_total_node : str, default "other_income_total"
            Node ID aggregating other-income components.
        egi_node : str, default "egi"
            Node ID for effective gross income after rent and other income.
        management_fee_node : str, default "management_fee"
            Node ID for the management-fee expense series.
        opex_total_node : str, default "opex_total"
            Node ID aggregating operating-expense components.
        noi_node : str, default "noi"
            Node ID for net operating income before capital expenditures.
        capex_total_node : str, default "capex_total"
            Node ID aggregating capital-expenditure components.
        ncf_node : str, default "ncf"
            Node ID for net cash flow after operating items and capital expenditure.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def rent_roll(self) -> RentRollOutputNodes:
        """
        Rent-roll output node ids (PGI, free rent, vacancy loss, effective rent).

        Returns
        -------
        RentRollOutputNodes
            Rent-roll output node ids.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def other_income_total_node(self) -> str:
        """
        Node id holding total other (non-rent) income.

        Returns
        -------
        str
            Node id for total other income.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def egi_node(self) -> str:
        """
        Node id holding effective gross income (EGI).

        Returns
        -------
        str
            Node id for effective gross income.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def management_fee_node(self) -> str:
        """
        Node id holding the management fee, when one is configured.

        Returns
        -------
        str
            Node id for the management fee.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def opex_total_node(self) -> str:
        """
        Node id holding total operating expenses, inclusive of the management fee when
        one is configured.

        Returns
        -------
        str
            Node id for total operating expenses.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def noi_node(self) -> str:
        """
        Node id holding net operating income (NOI).

        Returns
        -------
        str
            Node id for net operating income.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def capex_total_node(self) -> str:
        """
        Node id holding total capital expenditure.

        Returns
        -------
        str
            Node id for total capital expenditure.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    @property
    def ncf_node(self) -> str:
        """
        Node id holding net cash flow, ``noi - capex_total``.

        Returns
        -------
        str
            Node id for net cash flow.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def to_json(self) -> str:
        """
        Serialize this object to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation of this `PropertyTemplateNodes`, suitable for a matching `from_json` call.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...
    @staticmethod
    def from_json(json: str) -> PropertyTemplateNodes:
        """
        Deserialize property-template node names from canonical JSON.

        Parameters
        ----------
        json : str
            JSON payload identifying rent, income, expense, NOI, capex, and NCF
            nodes.

        Returns
        -------
        PropertyTemplateNodes
            Validated `PropertyTemplateNodes` instance reconstructed from the canonical JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or violates this type's serialized schema.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import PropertyTemplateNodes
        >>> nodes = PropertyTemplateNodes()
        >>> PropertyTemplateNodes.from_json(nodes.to_json()).egi_node
        'egi'

        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the node-id mapping as a single-row pandas ``DataFrame``.

        Columns: ``rent_pgi_node``, ``free_rent_node``, ``vacancy_loss_node``,
        ``rent_effective_node``, ``other_income_total_node``, ``egi_node``,
        ``management_fee_node``, ``opex_total_node``, ``noi_node``,
        ``capex_total_node``, ``ncf_node``.

        The four rent-roll node ids are flattened in rather than nested, so
        every value is a plain statement node id, not a numeric amount.

        Returns
        -------
        pd.DataFrame
            One row of property template node ids.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

def add_noi_buildup(
    model: FinancialModelSpec | str,
    total_revenue_node: str,
    revenue_nodes: list[str],
    total_expenses_node: str,
    expense_nodes: list[str],
    noi_node: str,
) -> FinancialModelSpec:
    """
    Apply the NOI buildup template and return a typed ``FinancialModelSpec``.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with NOI calculations.
    total_revenue_node : str
        Output node ID that sums the selected revenue nodes.
    revenue_nodes : list[str]
        Existing node IDs included as revenue in the NOI calculation.
    total_expenses_node : str
        Output node ID that sums the selected operating-expense nodes.
    expense_nodes : list[str]
        Existing node IDs included as operating expenses in the NOI calculation.
    noi_node : str
        Output node ID for revenue less operating expenses.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing revenue, expense, and NOI aggregation nodes.

    Raises
    ------
    ValueError
        If model JSON or a revenue, expense, or NOI node identifier is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import add_noi_buildup
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> _ = builder.value("rent", [("2025Q1", 100.0)])
    >>> _ = builder.value("opex", [("2025Q1", 30.0)])
    >>> model = builder.build()
    >>> updated = add_noi_buildup(model, "revenue", ["rent"], "expenses", ["opex"], "noi")
    >>> updated.has_node("noi")
    True

    """
    ...

def add_ncf_buildup(
    model: FinancialModelSpec | str,
    noi_node: str,
    capex_nodes: list[str],
    ncf_node: str,
) -> FinancialModelSpec:
    """
    Apply the NCF buildup template and return a typed ``FinancialModelSpec``.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with NCF calculations.
    noi_node : str
        Existing node ID supplying net operating income before capital spending.
    capex_nodes : list[str]
        Existing node IDs whose values are deducted as capital expenditures.
    ncf_node : str
        Output node ID for net operating income less capital expenditures.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing the NCF node after deducting the selected capex nodes.

    Raises
    ------
    ValueError
        If model JSON or an NOI, capital-expenditure, or NCF node identifier is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import add_ncf_buildup
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> _ = builder.value("noi", [("2025Q1", 70.0)])
    >>> _ = builder.value("capex", [("2025Q1", 10.0)])
    >>> model = builder.build()
    >>> add_ncf_buildup(model, "noi", ["capex"], "ncf").has_node("ncf")
    True

    """
    ...

def add_rent_roll(
    model: FinancialModelSpec | str,
    leases: list[LeaseSpec],
    nodes: RentRollOutputNodes | None = None,
) -> FinancialModelSpec:
    """
    Apply the rich rent-roll template and return a typed ``FinancialModelSpec``.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with rental-revenue nodes.
    leases : list[LeaseSpec]
        Rich lease schedules to calculate and aggregate into the rent roll.
    nodes : RentRollOutputNodes or None
        Optional aggregate output-node names; ``None`` uses template defaults.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing per-lease schedules and aggregate rent-roll nodes.

    Raises
    ------
    ValueError
        If model JSON, a lease specification, or a rent-roll output node is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import LeaseSpec, add_rent_roll
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> model = builder.build()
    >>> lease = LeaseSpec("lease_a", "2025Q1", 100.0)
    >>> add_rent_roll(model, [lease]).has_node("rent_effective")
    True

    """
    ...

def add_rent_roll_rental_revenue(
    model: FinancialModelSpec | str,
    leases: list[SimpleLeaseSpec],
    total_rent_node: str,
) -> FinancialModelSpec:
    """
    Apply the simple rent-roll template and return a typed ``FinancialModelSpec``.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with rental-revenue nodes.
    leases : list[SimpleLeaseSpec]
        Simple lease schedules to calculate and aggregate into rental revenue.
    total_rent_node : str
        Output node ID that sums all calculated simple-lease rent series.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing simple lease schedules and total rental revenue.

    Raises
    ------
    ValueError
        If model JSON, a lease specification, or total_rent_node is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import SimpleLeaseSpec, add_rent_roll_rental_revenue
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> model = builder.build()
    >>> lease = SimpleLeaseSpec("lease_a", "2025Q1", 100.0)
    >>> updated = add_rent_roll_rental_revenue(model, [lease], "rental_revenue")
    >>> updated.has_node("rental_revenue")
    True

    """
    ...

def add_property_operating_statement(
    model: FinancialModelSpec | str,
    leases: list[LeaseSpec],
    other_income_nodes: list[str] = ...,
    opex_nodes: list[str] = ...,
    capex_nodes: list[str] = ...,
    management_fee: ManagementFeeSpec | None = None,
    nodes: PropertyTemplateNodes | None = None,
) -> FinancialModelSpec:
    """
    Apply the full property operating-statement template and return a typed model.

    Parameters
    ----------
    model : FinancialModelSpec or str
        Model specification object or JSON to augment with property statements.
    leases : list[LeaseSpec]
        Rich lease schedules used to build rental-revenue and rent-roll outputs.
    other_income_nodes : list[str]
        Existing node IDs aggregated as other income; defaults to an empty list.
    opex_nodes : list[str]
        Existing node IDs aggregated as operating expenses; defaults to empty.
    capex_nodes : list[str]
        Existing node IDs aggregated as capital expenditures; defaults to empty.
    management_fee : ManagementFeeSpec or None
        Optional fee assumptions; ``None`` omits management-fee calculation.
    nodes : PropertyTemplateNodes or None
        Optional generated-node names; ``None`` uses the template defaults.

    Returns
    -------
    FinancialModelSpec
        Typed model specification containing the rent roll, EGI, NOI, capex, and NCF buildup.

    Raises
    ------
    ValueError
        If model JSON, a lease or fee specification, or an operating-statement node is invalid.

    Examples
    --------
    >>> from finstack_quant.statements_analytics import LeaseSpec, add_property_operating_statement
    >>> from finstack_quant.statements import FinancialModelSpec, ModelBuilder
    >>> builder = ModelBuilder("template")
    >>> _ = builder.periods("2025Q1..Q2")
    >>> model = builder.build()
    >>> lease = LeaseSpec("lease_a", "2025Q1", 100.0)
    >>> updated = add_property_operating_statement(model, [lease])
    >>> updated.has_node("ncf")
    True

    """
    ...

class ScenarioDiff:
    """
    Variance between two named scenarios in an evaluated scenario set.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import (
    ...     ScenarioSet,
    ...     evaluate_scenario_set,
    ... )
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> scenarios = ScenarioSet({"base": {}, "down": {"revenue": 90.0}})
    >>> results = evaluate_scenario_set(builder.build(), scenarios)
    >>> from finstack_quant.statements_analytics import scenario_diff
    >>> diff = scenario_diff(scenarios, results, "base", "down", ["profit"], ["2025Q1"])
    >>> diff.comparison
    'down'

    """

    @property
    def baseline(self) -> str:
        """
        Name of the scenario used as the baseline of the diff.

        Returns
        -------
        str
            Baseline scenario name.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison(self) -> str:
        """
        Name of the scenario compared against the baseline.

        Returns
        -------
        str
            Comparison scenario name.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def variance(self) -> VarianceReport:
        """
        Underlying variance report between the two named scenarios.

        Returns
        -------
        VarianceReport
            Variance report for the two named scenarios.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the underlying variance rows as a pandas DataFrame.

        Columns: ``period``, ``metric``, ``baseline``, ``comparison``,
        ``abs_var``, ``pct_var``. One row per (metric, period) pair, in report
        order; an empty diff still carries the full column schema.

        This is the same table as ``variance.to_dataframe()``. The two
        scenario *names* are diff metadata (the ``baseline`` / ``comparison``
        properties) and are not repeated per row; the ``baseline`` and
        ``comparison`` columns hold the metric *values* in each scenario.

        Returns
        -------
        pd.DataFrame
            One row per (metric, period) pair.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class BridgeStep:
    """
    One driver step in a bridge decomposition.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import (
    ...     ScenarioSet,
    ...     evaluate_scenario_set,
    ... )
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> scenarios = ScenarioSet({"base": {}, "down": {"revenue": 90.0}})
    >>> results = evaluate_scenario_set(builder.build(), scenarios)
    >>> from finstack_quant.statements_analytics import variance_bridge
    >>> chart = variance_bridge(
    ...     results.get("base"),
    ...     results.get("down"),
    ...     "profit",
    ...     "2025Q1",
    ...     ["revenue"],
    ...     "base",
    ...     "down",
    ... )
    >>> chart.steps[0].driver
    'revenue'

    """

    @property
    def driver(self) -> str:
        """
        Driver node identifier (e.g. ``"revenue"``).

        Returns
        -------
        str
            Driver node identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def contribution(self) -> float:
        """
        This driver's raw delta between the two scenarios, in the *driver's* own units.

        Contributions are not sensitivities of the target metric, so they generally do
        not sum to the target variance - see ``BridgeChart.unexplained``.

        Returns
        -------
        float
            Raw driver delta in the driver's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class BridgeChart:
    """
    Bridge decomposition of a metric's variance across named drivers.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import (
    ...     ScenarioSet,
    ...     evaluate_scenario_set,
    ... )
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> scenarios = ScenarioSet({"base": {}, "down": {"revenue": 90.0}})
    >>> results = evaluate_scenario_set(builder.build(), scenarios)
    >>> from finstack_quant.statements_analytics import variance_bridge
    >>> chart = variance_bridge(
    ...     results.get("base"),
    ...     results.get("down"),
    ...     "profit",
    ...     "2025Q1",
    ...     ["revenue"],
    ...     "base",
    ...     "down",
    ... )
    >>> chart.comparison_value
    30.0

    """

    @staticmethod
    def from_json(json: str) -> BridgeChart:
        """
        Deserialize a bridge chart from JSON.

        Parameters
        ----------
        json : str
            Canonical JSON produced by :meth:`to_json`.

        Returns
        -------
        BridgeChart
            The deserialized bridge chart.

        Raises
        ------
        ValueError
            If the JSON is malformed or does not describe a bridge chart.

        Examples
        --------
        >>> from finstack_quant.statements_analytics import BridgeChart
        >>> payload = (
        ...     '{"target_metric":"profit","period":"2025Q1",'
        ...     '"baseline_label":"base","comparison_label":"down",'
        ...     '"baseline_value":40.0,"comparison_value":30.0,'
        ...     '"steps":[{"driver":"revenue","contribution":-10.0}],'
        ...     '"unexplained":0.0}'
        ... )
        >>> BridgeChart.from_json(payload).target_metric
        'profit'

        """
        ...

    def to_json(self) -> str:
        """
        Serialize the bridge chart to canonical JSON.

        Returns
        -------
        str
            Canonical JSON representation, suitable for :meth:`from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def target_metric(self) -> str:
        """
        Node identifier of the metric this bridge decomposes (e.g. ``"ebitda"``).

        Returns
        -------
        str
            Node identifier of the decomposed metric.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def period(self) -> str:
        """
        Period the bridge covers, as a period-id string (e.g. ``"2025Q1"``).

        Returns
        -------
        str
            Period-id string covered by the bridge.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def baseline_label(self) -> str:
        """
        Label for the baseline scenario (e.g. ``"management_case"``).

        Returns
        -------
        str
            Baseline scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison_label(self) -> str:
        """
        Label for the comparison scenario (e.g. ``"bank_case"``).

        Returns
        -------
        str
            Comparison scenario label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def baseline_value(self) -> float:
        """
        Target-metric value in the baseline scenario, in the metric's units.

        Returns
        -------
        float
            Baseline target-metric value in the metric's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def comparison_value(self) -> float:
        """
        Target-metric value in the comparison scenario, in the metric's units.

        Returns
        -------
        float
            Comparison target-metric value in the metric's own units.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def steps(self) -> list[BridgeStep]:
        """
        Ordered driver contributions making up the bridge.

        Returns
        -------
        list[BridgeStep]
            Driver contributions in decomposition order.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def unexplained(self) -> float:
        """
        Residual not explained by the driver deltas.

        Driver contributions are raw deltas in driver units rather than
        sensitivities of the target metric, so they generally do not sum to
        the target variance. This term makes that gap explicit.

        Returns
        -------
        float
            ``(comparison_value - baseline_value)`` minus the summed
            contributions.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the driver steps as a pandas ``DataFrame``.

        Columns: ``driver``, ``contribution``. One row per bridge step, in
        decomposition order; an empty bridge still carries both columns.
        Contributions are raw deltas in each driver's own units.

        The scalar header fields (``target_metric``, ``period``,
        ``baseline_label``, ``comparison_label``, ``baseline_value``,
        ``comparison_value``, ``unexplained``) are chart metadata and are not
        repeated on every row.

        Returns
        -------
        pd.DataFrame
            One row per driver step.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

def scenario_diff(
    scenario_set: ScenarioSet | str,
    results: ScenarioResults,
    baseline: str,
    comparison: str,
    metrics: list[str],
    periods: list[str],
) -> ScenarioDiff:
    """
    Compare two evaluated scenarios metric-by-metric.

    Parameters
    ----------
    scenario_set : ScenarioSet or str
        Typed scenario set or JSON string.
    results : ScenarioResults
        Output of :func:`evaluate_scenario_set` for the same scenario set.
    baseline : str
        Name of the scenario to treat as the baseline.
    comparison : str
        Name of the scenario to compare against the baseline.
    metrics : list[str]
        Node identifiers to compare. Must be non-empty.
    periods : list[str]
        Period identifiers, e.g. ``"2025Q1"``. Must be non-empty.

    Returns
    -------
    ScenarioDiff
        Baseline and comparison names alongside the variance report.

    Raises
    ------
    ValueError
        If *metrics* or *periods* is empty, a scenario name is unknown, or a
        period fails to parse.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import (
    ...     ScenarioSet,
    ...     evaluate_scenario_set,
    ...     scenario_diff,
    ... )
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> model = builder.build()
    >>> scenarios = ScenarioSet({"base": {}, "down": {"revenue": 90.0}})
    >>> results = evaluate_scenario_set(model, scenarios)
    >>> diff = scenario_diff(
    ...     scenarios,
    ...     results,
    ...     "base",
    ...     "down",
    ...     ["revenue"],
    ...     ["2025Q1"],
    ... )
    >>> diff.baseline
    'base'

    """
    ...

def variance_bridge(
    base: StatementResult | str,
    comparison: StatementResult | str,
    target_metric: str,
    period: str,
    drivers: list[str],
    baseline_label: str,
    comparison_label: str,
) -> BridgeChart:
    """
    Decompose a metric's scenario variance across named drivers.

    Driver contributions are raw deltas in *driver* units rather than
    sensitivities of the target metric, so they generally do not sum to the
    target variance. The gap is reported in ``BridgeChart.unexplained``.

    Parameters
    ----------
    base : StatementResult or str
        Baseline evaluated statement result, or JSON string.
    comparison : StatementResult or str
        Comparison evaluated statement result, or JSON string.
    target_metric : str
        Node identifier whose variance is being explained.
    period : str
        Period identifier, e.g. ``"2025Q4"``.
    drivers : list[str]
        Node identifiers treated as explanatory drivers.
    baseline_label : str
        Display label for the baseline column.
    comparison_label : str
        Display label for the comparison column.

    Returns
    -------
    BridgeChart
        Ordered driver contributions plus the unexplained residual.

    Raises
    ------
    ValueError
        If the period fails to parse, or the target or any driver is missing
        from either result at *period*.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> from finstack_quant.statements_analytics import (
    ...     ScenarioSet,
    ...     evaluate_scenario_set,
    ...     variance_bridge,
    ... )
    >>> builder = ModelBuilder("demo")
    >>> _ = builder.periods("2025Q1..Q1")
    >>> _ = builder.value("revenue", [("2025Q1", 100.0)])
    >>> _ = builder.value("cost", [("2025Q1", 60.0)])
    >>> _ = builder.compute("profit", "revenue - cost")
    >>> results = evaluate_scenario_set(
    ...     builder.build(),
    ...     ScenarioSet({"base": {}, "down": {"revenue": 90.0}}),
    ... )
    >>> chart = variance_bridge(
    ...     results.get("base"),
    ...     results.get("down"),
    ...     "profit",
    ...     "2025Q1",
    ...     ["revenue", "cost"],
    ...     "base",
    ...     "down",
    ... )
    >>> chart.target_metric
    'profit'

    """
    ...
