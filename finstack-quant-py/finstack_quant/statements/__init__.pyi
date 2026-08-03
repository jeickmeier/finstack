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

import pandas as pd

from finstack_quant.core.currency import Currency
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.table import ArrowTable

__all__ = [
    "ForecastMethod",
    "ForecastSpec",
    "NodeType",
    "NodeId",
    "NumericMode",
    "FinancialModelSpec",
    "ModelBuilder",
    "MixedNodeBuilder",
    "MetricRegistry",
    "StatementResult",
    "Evaluator",
    "parse_formula",
    "validate_formula",
    "NormalizationConfig",
    "normalize",
    "CheckSuiteSpec",
    "CheckReport",
    "EcfSweepSpec",
    "PikToggleSpec",
    "WaterfallSpec",
]

class ForecastMethod:
    """
    Available forecast methods for projecting node values.

    Construct variants via static factory methods (e.g. ``growth_pct()``).

    Examples
    --------
    >>> from finstack_quant.statements import ForecastMethod
    >>> ForecastMethod.forward_fill() == ForecastMethod.forward_fill()
    True

    """

    @staticmethod
    def forward_fill() -> ForecastMethod:
        """
        Carry the last observed value forward into future periods.

        Returns
        -------
        ForecastMethod
            Forward-fill forecast method.

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
        Apply compound percentage growth between periods.

        Returns
        -------
        ForecastMethod
            Growth-percentage forecast method.

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
        Apply period-specific percentage growth from a curve.

        Returns
        -------
        ForecastMethod
            Curve-percentage forecast method.

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
        Normal-distribution sampling (deterministic under a fixed seed).

        Returns
        -------
        ForecastMethod
            Normal distribution forecast method.

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
        Log-normal distribution sampling (deterministic under a fixed seed).

        Returns
        -------
        ForecastMethod
            Log-normal forecast method.

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
        Use explicit period overrides instead of a statistical rule.

        Returns
        -------
        ForecastMethod
            Override forecast method.

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
        Reference an external time series as the forecast source.

        Returns
        -------
        ForecastMethod
            External time-series forecast method.

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
        Apply a seasonal pattern (additive or multiplicative).

        Returns
        -------
        ForecastMethod
            Seasonal forecast method.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastMethod
        >>> ForecastMethod.seasonal() == ForecastMethod.seasonal()
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
        Create a forecast spec from a method and optional JSON params.

        Parameters
        ----------
        method:
            A :class:`ForecastMethod` describing the projection approach.
        params_json:
            Optional JSON string with method-specific parameters.

        Raises
        ------
        ValueError
            If params_json is not valid JSON for the method parameter mapping.

        """
        ...

    @staticmethod
    def forward_fill() -> ForecastSpec:
        """
        Carry the last observed value forward.

        Returns
        -------
        ForecastSpec
            A forward-fill forecast specification.

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
        Compound each future period by ``rate``.

        Parameters
        ----------
        rate:
            Period-over-period growth rate as a decimal (e.g. ``0.05`` for 5%).

        Returns
        -------
        ForecastSpec
            A constant-growth forecast specification.

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
        Use period-specific growth rates.

        Parameters
        ----------
        curve:
            Per-period growth rates as decimals, aligned to future periods.

        Returns
        -------
        ForecastSpec
            A curve-based forecast specification.

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
        Use deterministic additive normal draws.

        Parameters
        ----------
        mean:
            Mean of the normal distribution.
        std_dev:
            Standard deviation of the normal distribution.
        seed:
            Random seed for deterministic reproducibility.

        Returns
        -------
        ForecastSpec
            A normal-draw forecast specification.

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
        Use deterministic multiplicative log-normal draws.

        Parameters
        ----------
        mean:
            Mean of the underlying normal distribution.
        std_dev:
            Standard deviation of the underlying normal distribution.
        seed:
            Random seed for deterministic reproducibility.

        Returns
        -------
        ForecastSpec
            A log-normal-draw forecast specification.

        Examples
        --------
        >>> from finstack_quant.statements import ForecastSpec
        >>> spec = ForecastSpec.lognormal(0.0, 0.1, 7)
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
        Node holds only explicit values (actuals or assumptions).

        Returns
        -------
        NodeType
            Value-only node type.

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
        Node is derived entirely from a formula.

        Returns
        -------
        NodeType
            Calculated node type.

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
        Node may combine values, forecasts, and formulas with precedence rules.

        Returns
        -------
        NodeType
            Mixed node type.

        Examples
        --------
        >>> from finstack_quant.statements import NodeType
        >>> NodeType.mixed() == NodeType.mixed()
        True

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
        Create a node identifier from a string.

        Parameters
        ----------
        id:
            Raw node identifier (for example ``"revenue"``).

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
            Node id string.

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
    >>> NumericMode.decimal() == NumericMode.decimal()
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

        Examples
        --------
        >>> from finstack_quant.statements import NumericMode
        >>> NumericMode.float64() == NumericMode.float64()
        True

        """
        ...

    @staticmethod
    def decimal() -> NumericMode:
        """
        Reserved decimal-arithmetic mode.

        This variant exists so saved result metadata can evolve, but statement
        evaluation always runs in ``float64``; selecting it does not change the
        arithmetic today.

        Returns
        -------
        NumericMode
            Decimal arithmetic mode (reserved).

        Examples
        --------
        >>> from finstack_quant.statements import NumericMode
        >>> NumericMode.decimal() == NumericMode.decimal()
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
    >>> builder.periods("2025Q1..Q1")
    >>> builder.build().id
    'demo'

    """

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
        >>> builder.periods("2025Q1..Q1")
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
            The id exposed by this `FinancialModelSpec`.
        """
        ...

    @property
    def period_count(self) -> int:
        """
        Number of periods defined on the model.
        Returns
        -------
        int
            The period count exposed by this `FinancialModelSpec`.
        """
        ...

    @property
    def node_count(self) -> int:
        """
        Number of nodes defined on the model.
        Returns
        -------
        int
            The node count exposed by this `FinancialModelSpec`.
        """
        ...

    def node_ids(self) -> list[str]:
        """
        List node identifiers in declaration order.

        Returns
        -------
        list[str]
            Ordered node id strings.

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> builder.periods("2025Q1..Q1")
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

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> builder.periods("2025Q1..Q1")
        >>> builder.build().has_node("revenue")
        False

        """
        ...

    @property
    def schema_version(self) -> int:
        """
        Wire-format schema version of this specification.
        Returns
        -------
        int
            The schema version exposed by this `FinancialModelSpec`.
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
    Methods on this class mutate the builder in place and return ``None``.
    Call them sequentially rather than chaining.

    Examples
    --------
    >>> from finstack_quant.statements import ModelBuilder
    >>> builder = ModelBuilder("demo")
    >>> builder.periods("2025Q1..Q1")
    >>> builder.value("revenue", [("2025Q1", 100.0)])
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

        Examples
        --------
        >>> from finstack_quant.statements import ModelBuilder
        >>> builder = ModelBuilder("demo")
        >>> builder.periods("2025Q1..Q1")
        >>> builder.build().id
        'demo'

        """
        ...

    def periods(self, range: str, actuals_until: str | None = None) -> None:
        """
        Define the model's period lattice from a range expression.

        Parameters
        ----------
        range:
            Period range expression such as ``"2025Q1..Q4"``.
        actuals_until:
            Optional last actual period label; ``None`` if not used.

        Raises
        ------
        ValueError
            If periods are already set, the range is invalid, or the builder was consumed.

        """
        ...

    def value(self, node_id: str, values: list[tuple[str, float]]) -> None:
        """
        Add a value node with explicit per-period scalars.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, value)`` pairs, for example ``[("2025Q1", 1.0)]``.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def value_scalar(self, node_id: str, values: list[tuple[str, float]]) -> None:
        """
        Add a scalar value node with explicit per-period values.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, value)`` pairs, for example ``[("2025Q1", 1.0)]``.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def value_money(self, node_id: str, values: list[tuple[str, Money]]) -> None:
        """
        Add a monetary value node with explicit per-period values.

        Parameters
        ----------
        node_id:
            Identifier for the new node.
        values:
            ``(period_id, Money)`` pairs, for example ``[("2025Q1", Money(100.0, "USD"))]``.

        Raises
        ------
        ValueError
            If periods were not configured, a period id is invalid, or the builder was consumed.

        """
        ...

    def compute(self, node_id: str, formula: str) -> None:
        """
        Add a calculated node from a DSL formula.

        Parameters
        ----------
        node_id:
            Identifier for the computed node.
        formula:
            Expression in the statements DSL (for example ``"revenue - cogs"``).

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

    def forecast(self, node_id: str, forecast_spec: ForecastSpec) -> None:
        """
        Attach a forecast to an existing node or create a forecast-only mixed node.

        Parameters
        ----------
        node_id:
            Identifier for the node to forecast.
        forecast_spec:
            A :class:`ForecastSpec` describing the projection method and parameters.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.

        """
        ...

    def where_clause(self, where_clause: str) -> None:
        """
        Attach a conditional expression to the last added node.

        Parameters
        ----------
        where_clause:
            DSL expression evaluated per period to gate the node's value.

        Raises
        ------
        ValueError
            If periods have not been configured or the builder has already been consumed.

        """
        ...

    def with_meta(self, key: str, value_json: str) -> None:
        """
        Add model-level metadata from a JSON payload.

        Parameters
        ----------
        key:
            Namespaced model-metadata key used to identify the supplied JSON
            value in serialized model output.
        value_json:
            JSON-serialized metadata value.

        Raises
        ------
        ValueError
            If value_json is malformed, periods are not configured, or the builder
            has already been consumed.

        """
        ...

    def with_name_normalization(self) -> None:
        """
        Enable standard accounting term alias normalization.

        """
        ...

    def with_builtin_metrics(self) -> None:
        """
        Add all built-in statement metrics to the model.

        """
        ...

    def add_metric_from_registry(self, qualified_id: str, registry: MetricRegistry) -> None:
        """
        Add one metric and its dependencies from a metric registry.

        Parameters
        ----------
        qualified_id:
            Fully qualified metric identifier.
        registry:
            A :class:`MetricRegistry` containing the metric definition.

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
    ) -> None:
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
    ) -> None:
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

        Raises
        ------
        ValueError
            If a date is invalid or the builder has already been consumed.
        RuntimeError
            If the swap cannot be added to the model capital structure.

        """
        ...

    def add_debt(self, id: str, spec_json: str) -> None:
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

                Raises
                ------
                ValueError
                    If the envelope is invalid or its instrument type is not supported by
                    financial statement capital structures.

        """
        ...

    def reporting_currency(self, currency: Currency) -> None:
        """
        Set the reporting currency used for capital-structure totals.

        Parameters
        ----------
        currency:
            A :class:`Currency` instance. A bare ISO-4217 string is not
            accepted; construct ``Currency("USD")`` first.

        Raises
        ------
        ValueError
            If the builder has already been consumed.

        """
        ...

    def fx_policy(self, policy: str) -> None:
        """
        Set the FX policy (``cashflow_date``/``period_end``/``period_average``/``custom``).

        Parameters
        ----------
        policy:
            FX conversion policy label.

        Raises
        ------
        ValueError
            If policy is unknown or the builder has already been consumed.

        """
        ...

    def waterfall(self, waterfall_spec: WaterfallSpec) -> None:
        """
        Attach a waterfall specification (PIK toggle + ECF sweep + priorities).

        Parameters
        ----------
        waterfall_spec:
            A :class:`WaterfallSpec` defining cash distribution priorities.

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
    >>> builder.periods("2025Q1..Q1")
    >>> mixed = builder.mixed("profit")
    >>> mixed.values([("2025Q1", 40.0)])
    >>> mixed.formula("revenue - cost")
    >>> mixed.build().build().has_node("profit")
    True

    """

    def values(self, values: list[tuple[str, float]]) -> None:
        """
        Set scalar explicit values.

        Parameters
        ----------
        values:
            ``(period_id, value)`` pairs for periods where an explicit scalar
            overrides the formula or forecast.

        Raises
        ------
        ValueError
            If a period label is invalid or the mixed-node builder has been consumed.

        """
        ...

    def values_money(self, values: list[tuple[str, Money]]) -> None:
        """
        Set monetary explicit values.

        Parameters
        ----------
        values:
            ``(period_id, Money)`` pairs for periods where an explicit monetary
            value overrides the formula or forecast.

        Raises
        ------
        ValueError
            If a period label is invalid or the mixed-node builder has been consumed.

        """
        ...

    def forecast(self, forecast_spec: ForecastSpec) -> None:
        """
        Set the forecast spec.

        Parameters
        ----------
        forecast_spec:
            A :class:`ForecastSpec` describing the projection method.

        Raises
        ------
        ValueError
            If the mixed-node builder has already been consumed.

        """
        ...

    def formula(self, formula: str) -> None:
        """
        Set the fallback formula.

        Parameters
        ----------
        formula:
            DSL expression used when no explicit value or forecast is available.

        Raises
        ------
        ValueError
            If formula is empty or invalid, or the mixed-node builder has been consumed.

        """
        ...

    def name(self, name: str) -> None:
        """
        Set the display name.

        Parameters
        ----------
        name:
            Human-readable node name.

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
        Create an empty registry.

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

        Examples
        --------
        >>> from finstack_quant.statements import MetricRegistry
        >>> len(MetricRegistry.with_builtins()) > 0
        True

        """
        ...

    def load_builtins(self) -> None:
        """
        Load built-in metrics into this registry.

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
    >>> builder.periods("2025Q1..Q1")
    >>> builder.value("revenue", [("2025Q1", 100.0)])
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
        >>> builder.periods("2025Q1..Q1")
        >>> builder.value("revenue", [("2025Q1", 100.0)])
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
        Return the scalar for ``node_id`` at ``period``, if present.

        Parameters
        ----------
        node_id:
            Node identifier.
        period:
            Period label such as ``"2025Q1"``.

        Returns
        -------
        float | None
            Value when found, otherwise ``None``.

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

    def get_node(self, node_id: str) -> dict[str, float] | None:
        """
        Return all period values for a node as a mapping.

        Parameters
        ----------
        node_id:
            Node identifier.

        Returns
        -------
        dict[str, float] | None
            Mapping from period string to float, or ``None`` if the node is missing.

        """
        ...

    def node_ids(self) -> list[str]:
        """
        Return every node id present in this result set.

        Returns
        -------
        list[str]
            Node identifiers.

        """
        ...

    @property
    def node_count(self) -> int:
        """
        Number of nodes in the result.
        Returns
        -------
        int
            The node count exposed by this `StatementResult`.
        """
        ...

    @property
    def num_periods(self) -> int:
        """
        Number of periods covered by the evaluation metadata.
        Returns
        -------
        int
            The num periods exposed by this `StatementResult`.
        """
        ...

    @property
    def eval_time_ms(self) -> int | None:
        """
        Wall-clock evaluation time in milliseconds, if recorded.
        Returns
        -------
        int or None
            The eval time ms exposed by this `StatementResult`.
        """
        ...

    @property
    def warning_count(self) -> int:
        """
        Count of evaluation warnings attached to metadata.
        Returns
        -------
        int
            The warning count exposed by this `StatementResult`.
        """
        ...

    @property
    def warnings(self) -> list[str]:
        """
        Evaluation warnings as human-readable strings.

        Returns
        -------
        list[str]
            The warnings exposed by this `StatementResult`.
        """
        ...

    @property
    def numeric_mode(self) -> NumericMode:
        """
        Numeric mode stamped into the result envelope (policy visibility).

        Returns
        -------
        NumericMode
            The numeric mode exposed by this `StatementResult`.
        """
        ...

    @property
    def parallel(self) -> bool:
        """
        Whether the evaluation ran in parallel (policy visibility).

        Returns
        -------
        bool
            The parallel exposed by this `StatementResult`.
        """
        ...

    def to_pandas_long(self) -> pd.DataFrame:
        """
        Export results as a pandas DataFrame in long (tidy) form.

        Columns: ``node_id``, ``period``, ``value``, ``value_money``,
        ``currency``, ``value_type``. The monetary columns are populated for
        nodes carrying currency information and are otherwise null.
        ``value_money`` is a float64 mirror of the monetary amount (f64, not
        fixed-point Decimal, precision); use ``to_json()`` or ``get_money()``
        when full fixed-point precision is required.

        Returns
        -------
        pd.DataFrame
            Long-format frame with one row per (node, period) pair.
        """
        ...

    def to_pandas_wide(self) -> pd.DataFrame:
        """
        Export results as a pandas DataFrame in wide form.

        Rows are node identifiers, columns are period identifiers.

        Returns
        -------
        pd.DataFrame
            Wide-format frame with node ids as index.
        """
        ...

    def to_arrow_long(self) -> ArrowTable:
        """
        Export the long-format table via Arrow (zero-copy for consumers).

        Returns an `ArrowTable` implementing ``__arrow_c_stream__``; pass it
        to ``pyarrow.table(...)``, ``polars.DataFrame(...)``, or DuckDB.
        Column values and monetary-mirror semantics match `to_pandas_long`,
        plus column roles and table metadata are preserved as Arrow
        field/schema metadata. One column name differs: the period column
        here is ``period_id`` (the table envelope's native name), whereas
        `to_pandas_long` renames it to ``period``.

        Returns
        -------
        ArrowTable
            Long-format Arrow table with one row per (node, period) pair.
        """
        ...

    def to_arrow_wide(self) -> ArrowTable:
        """
        Export the wide-format table via Arrow (zero-copy for consumers).

        Rows are periods (column ``period_id``), one ``float64`` column per
        node, matching `to_pandas_wide` before its transpose.

        Returns
        -------
        ArrowTable
            Wide-format Arrow table with one row per period.
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
    >>> builder.periods("2025Q1..Q1")
    >>> builder.value("revenue", [("2025Q1", 100.0)])
    >>> Evaluator().evaluate(builder.build()).node_count
    1

    """

    def __init__(self) -> None:
        """
        Create a fresh evaluator with default configuration.

        Returns
        -------
        None

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

def validate_formula(formula: str) -> bool:
    """
    Return ``True`` if ``formula`` parses and compiles successfully.

    Parameters
    ----------
    formula:
        DSL expression to validate.

    Returns
    -------
    bool
        Always ``True`` when no error is raised.

    Raises
    ------
    ValueError
        If parsing or compilation fails.

    Examples
    --------
    >>> from finstack_quant.statements import validate_formula
    >>> validate_formula("revenue - cogs")
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
            The target node exposed by this `NormalizationConfig`.
        """
        ...

    @property
    def adjustment_count(self) -> int:
        """
        Number of adjustment line items configured.
        Returns
        -------
        int
            The adjustment count exposed by this `NormalizationConfig`.
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
    >>> builder.periods("2025Q1..Q1")
    >>> builder.value("ebitda", [("2025Q1", 25.0)])
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
        Return the name for `CheckSuiteSpec`.
        Suite name.
        Returns
        -------
        str
            The name exposed by this `CheckSuiteSpec`.
        """
        ...

    @property
    def builtin_check_count(self) -> int:
        """
        Number of built-in checks in the suite spec.
        Returns
        -------
        int
            The builtin check count exposed by this `CheckSuiteSpec`.
        """
        ...

    @property
    def formula_check_count(self) -> int:
        """
        Number of formula checks in the suite spec.
        Returns
        -------
        int
            The formula check count exposed by this `CheckSuiteSpec`.
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
        Whether all checks passed (no error-severity findings).
        Returns
        -------
        bool
            The passed exposed by this `CheckReport`.
        """
        ...

    @property
    def total_checks(self) -> int:
        """
        Number of individual check results in the report.
        Returns
        -------
        int
            The total checks exposed by this `CheckReport`.
        """
        ...

    @property
    def total_findings(self) -> int:
        """
        Total number of findings across all checks.
        Returns
        -------
        int
            The total findings exposed by this `CheckReport`.
        """
        ...

    @property
    def total_errors(self) -> int:
        """
        Number of error-severity findings.
        Returns
        -------
        int
            The total errors exposed by this `CheckReport`.
        """
        ...

    @property
    def total_warnings(self) -> int:
        """
        Number of warning-severity findings.
        Returns
        -------
        int
            The total warnings exposed by this `CheckReport`.
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
            If the JSON payload cannot be parsed or does not satisfy the `ValueError` schema and invariants.

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
        """
        ...

    @property
    def ebitda_node(self) -> str:
        """
        Return the ebitda node for `EcfSweepSpec`.

        Returns
        -------
        str
            The ebitda node exposed by this `EcfSweepSpec`.
        """
        ...

    @property
    def sweep_percentage(self) -> float:
        """
        Return the sweep percentage for `EcfSweepSpec`.

        Returns
        -------
        float
            The sweep percentage exposed by this `EcfSweepSpec`.
        """
        ...

    @property
    def target_instrument_id(self) -> str | None:
        """
        Return the target instrument id for `EcfSweepSpec`.

        Returns
        -------
        str | None
            The target instrument id exposed by this `EcfSweepSpec`.
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
            Minimum number of forecast periods to remain in PIK after activation.

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
            If the JSON payload cannot be parsed or does not satisfy the `ValueError` schema and invariants.

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
        """
        ...

    @property
    def liquidity_metric(self) -> str:
        """
        Return the liquidity metric for `PikToggleSpec`.

        Returns
        -------
        str
            The liquidity metric exposed by this `PikToggleSpec`.
        """
        ...

    @property
    def threshold(self) -> float:
        """
        Return the threshold for `PikToggleSpec`.

        Returns
        -------
        float
            The threshold exposed by this `PikToggleSpec`.
        """
        ...

    @property
    def min_periods_in_pik(self) -> int:
        """
        Return the min periods in pik for `PikToggleSpec`.

        Returns
        -------
        int
            The min periods in pik exposed by this `PikToggleSpec`.
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
            If the JSON payload cannot be parsed or does not satisfy the `ValueError` schema and invariants.

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
        """
        ...

    def validate(self) -> None:
        """
        Compute validate for `WaterfallSpec`.
        """
        ...

    @property
    def priority_of_payments(self) -> list[str]:
        """
        Return the priority of payments for `WaterfallSpec`.

        Returns
        -------
        list[str]
            The priority of payments exposed by this `WaterfallSpec`.
        """
        ...

    @property
    def available_cash_node(self) -> str | None:
        """
        Return the available cash node for `WaterfallSpec`.

        Returns
        -------
        str | None
            The available cash node exposed by this `WaterfallSpec`.
        """
        ...

    @property
    def has_ecf_sweep(self) -> bool:
        """
        Return the has ecf sweep for `WaterfallSpec`.

        Returns
        -------
        bool
            Whether this `WaterfallSpec` has ecf sweep.
        """
        ...

    @property
    def has_pik_toggle(self) -> bool:
        """
        Return the has pik toggle for `WaterfallSpec`.

        Returns
        -------
        bool
            Whether this `WaterfallSpec` has pik toggle.
        """
        ...

    def __repr__(self) -> str: ...
