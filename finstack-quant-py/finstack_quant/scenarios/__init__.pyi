"""
Scenario specification, validation, composition, application, and built-in templates.

Examples
--------
>>> from finstack_quant.scenarios import list_builtin_templates
>>> list_builtin_templates()[:2]
['gfc_2008', 'covid_2020']
"""

from __future__ import annotations

from typing import Any, Literal

from finstack_quant.attribution import PnlAttribution
from finstack_quant.core.dates import DayCount

__all__ = [
    "parse_scenario_spec",
    "build_scenario_spec",
    "compose_scenarios",
    "validate_scenario_spec",
    "list_builtin_templates",
    "list_builtin_template_metadata",
    "build_from_template",
    "list_template_components",
    "build_template_component",
    "apply_scenario",
    "apply_scenario_to_market",
    "compute_horizon_return",
    "HorizonResult",
    "OperationSpec",
    "RateBindingSpec",
    "CurveKind",
    "VolSurfaceKind",
    "TenorMatchMode",
    "TimeRollMode",
    "Compounding",
]

def parse_scenario_spec(json_str: str) -> str:
    """
    Parse, validate, and re-serialize a ``ScenarioSpec`` from JSON.

    Parameters
    ----------
    json_str : str
        JSON-serialized ``ScenarioSpec``.

    Returns
    -------
    str
        Validated canonical JSON string.

    Raises
    ------
    ValueError
        If the JSON is malformed or fails scenario-spec validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.scenarios import parse_scenario_spec
    >>> parsed = parse_scenario_spec('{"id":"s","name":"S","operations":[]}')
    >>> json.loads(parsed)["resolution_mode"]
    'most_specific_wins'
    """
    ...

def build_scenario_spec(
    id: str,
    operations_json: str,
    name: str | None = None,
    description: str | None = None,
    priority: int = 0,
    resolution_mode: Literal["most_specific_wins", "cumulative"] = "most_specific_wins",
) -> str:
    """
    Construct a ``ScenarioSpec`` from fields plus a JSON operations list.

    Parameters
    ----------
    id : str
        Stable scenario identifier.
    operations_json : str
        JSON list of ``OperationSpec``.
    name : str, optional
        Display name.
    description : str, optional
        Long description.
    priority : int, default 0
        Composition priority (lower runs first).
    resolution_mode : str, default "most_specific_wins"
        Hierarchy conflict policy. Accepted values are
        ``"most_specific_wins"`` and ``"cumulative"``.

    Returns
    -------
    str
        Validated JSON ``ScenarioSpec``.

    Raises
    ------
    ValueError
        If ``operations_json`` is not valid JSON, ``resolution_mode`` is not
        recognized, or the resulting scenario fails validation.

    Examples
    --------
    >>> from finstack_quant.scenarios import build_scenario_spec
    >>> import json
    >>> built = build_scenario_spec("s1", "[]", resolution_mode="cumulative")
    >>> json.loads(built)["resolution_mode"]
    'cumulative'
    """
    ...

def compose_scenarios(specs_json: str) -> str:
    """
    Merge multiple scenario specs using the scenario engine composer.

    Parameters
    ----------
    specs_json : str
        JSON list of ``ScenarioSpec``.

    Returns
    -------
    str
        JSON-serialized composed ``ScenarioSpec``.

    Raises
    ------
    ValueError
        If ``specs_json`` is not valid JSON or composition fails.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.scenarios import compose_scenarios
    >>> json.loads(compose_scenarios("[]"))["operations"]
    []
    """
    ...

def validate_scenario_spec(json_str: str) -> bool:
    """
    Return ``True`` after successfully parsing and validating JSON.

    Parameters
    ----------
    json_str : str
        JSON-serialized ``ScenarioSpec``.

    Returns
    -------
    bool
        Always ``True`` on success.

    Raises
    ------
    ValueError
        If ``json_str`` is not valid JSON or fails validation.

    Examples
    --------
    >>> from finstack_quant.scenarios import validate_scenario_spec
    >>> spec = '{"id":"s","name":"S","operations":[]}'
    >>> validate_scenario_spec(spec)
    True
    """
    ...

def list_builtin_templates() -> list[str]:
    """
    List template IDs from the embedded built-in registry.

    Returns
    -------
    list[str]
        Template identifier strings.

    Examples
    --------
    >>> from finstack_quant.scenarios import list_builtin_templates
    >>> list_builtin_templates()[:2]
    ['gfc_2008', 'covid_2020']
    """
    ...

def list_builtin_template_metadata() -> str:
    """
    Serialize metadata for all built-in templates to JSON.

    Returns
    -------
    str
        JSON list of ``TemplateMetadata`` objects.

    Examples
    --------
    >>> from finstack_quant.scenarios import list_builtin_template_metadata
    >>> meta_json = list_builtin_template_metadata()
    """
    ...

def build_from_template(template_id: str) -> str:
    """
    Instantiate a ``ScenarioSpec`` from a built-in template.

    Parameters
    ----------
    template_id : str
        Registry key for the template.

    Returns
    -------
    str
        JSON-serialized ``ScenarioSpec``.

    Raises
    ------
    ValueError
        If ``template_id`` is not found in the registry.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.scenarios import build_from_template
    >>> json.loads(build_from_template("gfc_2008"))["id"]
    'gfc_2008'
    """
    ...

def list_template_components(template_id: str) -> list[str]:
    """
    List sub-component IDs for composite templates.

    Parameters
    ----------
    template_id : str
        Parent template identifier.

    Returns
    -------
    list[str]
        Component identifiers.

    Raises
    ------
    ValueError
        If ``template_id`` is not found in the registry.

    Examples
    --------
    >>> from finstack_quant.scenarios import list_template_components
    >>> list_template_components("gfc_2008")[:2]
    ['gfc_2008_rates', 'gfc_2008_credit']
    """
    ...

def build_template_component(template_id: str, component_id: str) -> str:
    """
    Build a single component spec from a composite template.

    Parameters
    ----------
    template_id : str
        Parent template identifier.
    component_id : str
        Component key inside the template.

    Returns
    -------
    str
        JSON-serialized component ``ScenarioSpec``.

    Raises
    ------
    ValueError
        If ``template_id`` or ``component_id`` is not found.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.scenarios import build_template_component
    >>> component = build_template_component("gfc_2008", "gfc_2008_rates")
    >>> json.loads(component)["id"]
    'gfc_2008_rates'
    """
    ...

def apply_scenario(
    scenario_json: str,
    market: Any,
    model: Any,
    as_of: str,
) -> dict[str, Any]:
    """
    Apply a scenario to both market data and a financial model.

    Parameters
    ----------
    scenario_json : str
        JSON ``ScenarioSpec``.
    market : Any
        ``MarketContext`` object or JSON ``MarketContext`` string.
    model : Any
        ``FinancialModelSpec`` object or JSON ``FinancialModelSpec`` string.
    as_of : str
        ISO 8601 valuation date.

    Returns
    -------
    dict[str, Any]
        Dict with ``market_json``, ``model_json``, ``operations_applied`` (``int``),
        ``user_operations`` (``int``), ``expanded_operations`` (``int``),
        ``warnings`` (``list[str]``, rendered Display form), and
        ``warnings_json`` (``str``, JSON-encoded list of structured ``Warning``
        records — parse with ``json.loads(...)`` for programmatic
        ``kind``-based dispatch).

    Raises
    ------
    ValueError
        If the scenario JSON is malformed or application fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.scenarios import apply_scenario, compose_scenarios
    >>> model = (
    ...     '{"schema_version":1,"id":"m","periods":['
    ...     '{"id":"2025Q1","start":"2025-01-01","end":"2025-04-01",'
    ...     '"is_actual":false}],"nodes":{}}'
    ... )
    >>> applied = apply_scenario(compose_scenarios("[]"), MarketContext(), model, "2025-01-15")
    >>> applied["operations_applied"]
    0
    """
    ...

def apply_scenario_to_market(
    scenario_json: str,
    market: Any,
    as_of: str,
) -> dict[str, Any]:
    """
    Apply a scenario to market data only (no model mutations returned).

    Parameters
    ----------
    scenario_json : str
        JSON ``ScenarioSpec``.
    market : Any
        ``MarketContext`` object or JSON ``MarketContext`` string.
    as_of : str
        ISO 8601 valuation date.

    Returns
    -------
    dict[str, Any]
        Dict with ``market_json``, ``operations_applied``, ``user_operations``,
        ``expanded_operations``, ``warnings`` (``list[str]``), and
        ``warnings_json`` (``str``, JSON-encoded list of structured warnings).

    Raises
    ------
    ValueError
        If the scenario JSON is malformed or application fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.scenarios import apply_scenario_to_market, compose_scenarios
    >>> applied = apply_scenario_to_market(compose_scenarios("[]"), MarketContext(), "2025-01-15")
    >>> applied["operations_applied"]
    0
    """
    ...

class HorizonResult:
    """
    Horizon total return result with full P&L attribution.

    Produced by :func:`compute_horizon_return`. Access factor-level
    contributions via :meth:`factor_contribution` and the full breakdown
    via :attr:`attribution`.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.scenarios import compose_scenarios, compute_horizon_return
    >>> try:
    ...     compute_horizon_return("{}", MarketContext(), "2025-01-15", compose_scenarios("[]"))
    ... except ValueError as exc:
    ...     print(str(exc).split(":")[0])
    Validation error
    """

    @property
    def attribution(self) -> PnlAttribution:
        """
        Full P&L attribution breakdown.

        Returns
        -------
        PnlAttribution
            Carry, rate, credit, inflation, FX, volatility, and model-parameter
            contributions.

        """
        ...

    @property
    def initial_value(self) -> float:
        """
        Initial instrument value.

        Returns
        -------
        float
            Present value at the original valuation date.
        """
        ...

    @property
    def terminal_value(self) -> float:
        """
        Final instrument value after the scenario is applied.

        Returns
        -------
        float
            Present value after scenario shocks and time roll.
        """
        ...

    @property
    def horizon_days(self) -> int | None:
        """
        Horizon in calendar days (``None`` if no time-roll).

        Returns
        -------
        int or None
            Number of days rolled forward, or ``None`` when the scenario
            contains no ``time_roll_forward`` operation.
        """
        ...

    @property
    def total_return_pct(self) -> float:
        """
        Total return as decimal fraction (0.05 = 5%).

        Returns
        -------
        float
            ``(terminal_value - initial_value) / initial_value``.
        """
        ...

    @property
    def annualized_return(self) -> float | None:
        """
        Annualized return (``None`` if no time-roll).

        Returns
        -------
        float or None
            Annualized total return, or ``None`` when ``horizon_days`` is
            ``None`` or zero.
        """
        ...

    @property
    def operations_applied(self) -> int:
        """
        Number of scenario operations applied.

        Returns
        -------
        int
            Count of operations executed after hierarchy expansion.
        """
        ...

    @property
    def user_operations(self) -> int:
        """
        Number of user-provided scenario operations before hierarchy expansion.

        Returns
        -------
        int
            Count of operations in the original ``ScenarioSpec``.
        """
        ...

    @property
    def expanded_operations(self) -> int:
        """
        Number of direct operations after hierarchy expansion and deduplication.

        Returns
        -------
        int
            Count of unique operations after template hierarchy expansion.
        """
        ...

    @property
    def warnings(self) -> list[str]:
        """
        Warnings emitted during scenario application (rendered Display form).

        Returns
        -------
        list[str]
            Human-readable warning strings.
        """
        ...

    @property
    def warnings_json(self) -> str:
        """
        JSON-encoded structured warnings.

        Each entry is a `Warning` record with a ``kind`` discriminator plus
        variant-specific fields, mirroring the WASM binding. Parse with
        ``json.loads(...)`` to dispatch on ``kind`` programmatically.

        Returns
        -------
        str
            JSON array of structured warning objects.
        """
        ...

    def factor_contribution(self, factor: str) -> float:
        """
        Factor contribution as decimal fraction of initial value.

        Parameters
        ----------
        factor : str
            One of ``"carry"``, ``"rates"``/``"rates_curves"``,
            ``"credit"``/``"credit_curves"``, ``"inflation"``/``"inflation_curves"``,
            ``"correlations"``, ``"fx"``, ``"volatility"``/``"vol"``,
            ``"model_parameters"``/``"model_params"``, or
            ``"market_scalars"``/``"scalars"``.

        Returns
        -------
        float
            Contribution of the given factor as a decimal fraction.

        Raises
        ------
        ValueError
            If ``factor`` is not a recognized factor key.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize the result to JSON.

        Returns
        -------
        str
            JSON-serialized ``HorizonResult`` envelope.
        """
        ...

    def explain(self) -> str:
        """
        Human-readable summary of horizon return and attribution.

        Returns
        -------
        str
            Multi-line text suitable for notebook display.
        """
        ...

def compute_horizon_return(
    instrument_json: str,
    market: Any,
    as_of: str,
    scenario_json: str,
    method: str = "parallel",
    config: str | None = None,
    calendar_id: str | None = None,
) -> HorizonResult:
    """
    Compute horizon total return under a scenario.

    Parameters
    ----------
    instrument_json : str
        Canonical v1 instrument envelope.
    market : Any
        ``MarketContext`` object or JSON string.
    as_of : str
        Valuation date in ISO 8601 format.
    scenario_json : str
        JSON-serialized ``ScenarioSpec``.
    method : str, default "parallel"
        Attribution method — ``"parallel"``, ``"waterfall"``,
        ``"metrics_based"``, or ``"taylor"``.
    config : str, optional
        JSON-serialized ``FinstackConfig``.
    calendar_id : str, optional
        Holiday calendar used to business-day adjust ``time_roll_forward``
        targets under ``TimeRollMode.business_days`` (e.g. ``"nyse"``,
        ``"target"``). Defaults to a weekends-only calendar, so business-day
        rolls always avoid weekends but not market holidays. Raises
        ``ValueError`` if the identifier is not a built-in calendar.

    Returns
    -------
    HorizonResult
        Decomposed total return and factor attribution.

    Raises
    ------
    ValueError
        If any input JSON is malformed or the scenario application fails.

    Examples
    --------
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.scenarios import compose_scenarios, compute_horizon_return
    >>> try:
    ...     compute_horizon_return("{}", MarketContext(), "2025-01-15", compose_scenarios("[]"))
    ... except ValueError as exc:
    ...     print(str(exc).split(":")[0])
    Validation error
    """
    ...

# ---------------------------------------------------------------------------
# Typed operation builders
#
# These mirror the Rust ``OperationSpec`` enum and its supporting enums. They
# replace the raw-JSON authoring path so quants can write
# ``OperationSpec.curve_parallel_bp(...)`` and feed the result straight into
# ``build_scenario_spec`` via ``op.to_json()``.
# ---------------------------------------------------------------------------

class CurveKind:
    """
    Type of market curve targeted by a scenario operation.

    Examples
    --------
    >>> from finstack_quant.scenarios import CurveKind
    >>> CurveKind.discount().value
    'discount'
    """

    @classmethod
    def discount(cls) -> CurveKind:
        """
        Discount factor curve.

        Returns
        -------
        CurveKind
            The ``discount`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind
        >>> str(CurveKind.discount())
        'CurveKind.Discount'
        """
        ...

    @classmethod
    def forward(cls) -> CurveKind:
        """
        Forward rate curve.

        Returns
        -------
        CurveKind
            The ``forward`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind
        >>> str(CurveKind.forward())
        'CurveKind.Forward'
        """
        ...

    @classmethod
    def par_cds(cls) -> CurveKind:
        """
        Par CDS spread curve.

        Returns
        -------
        CurveKind
            The ``par_cds`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind
        >>> str(CurveKind.par_cds())
        'CurveKind.ParCDS'
        """
        ...

    @classmethod
    def inflation(cls) -> CurveKind:
        """
        Inflation index curve.

        Returns
        -------
        CurveKind
            The ``inflation`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind
        >>> str(CurveKind.inflation())
        'CurveKind.Inflation'
        """
        ...

    @classmethod
    def commodity(cls) -> CurveKind:
        """
        Commodity forward curve.

        Returns
        -------
        CurveKind
            The ``commodity`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind
        >>> str(CurveKind.commodity())
        'CurveKind.Commodity'
        """
        ...

    @property
    def name(self) -> str:
        """
        Variant name, e.g. ``"Discount"``.

        Returns
        -------
        str
            Pascal-case variant name.
        """
        ...

    @property
    def value(self) -> str:
        """
        Serialized wire value, e.g. ``"discount"`` or ``"par_cds"``.

        Returns
        -------
        str
            Snake-case wire value used in JSON serialization.
        """
        ...

class VolSurfaceKind:
    """
    Category of volatility surface targeted by a scenario operation.

    Examples
    --------
    >>> from finstack_quant.scenarios import VolSurfaceKind
    >>> VolSurfaceKind.equity().value
    'equity'
    """

    @classmethod
    def equity(cls) -> VolSurfaceKind:
        """
        Equity volatility surface.

        Returns
        -------
        VolSurfaceKind
            The ``equity`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import VolSurfaceKind
        >>> str(VolSurfaceKind.equity())
        'VolSurfaceKind.Equity'
        """
        ...

    @classmethod
    def credit(cls) -> VolSurfaceKind:
        """
        Credit volatility surface.

        Returns
        -------
        VolSurfaceKind
            The ``credit`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import VolSurfaceKind
        >>> str(VolSurfaceKind.credit())
        'VolSurfaceKind.Credit'
        """
        ...

    @classmethod
    def swaption(cls) -> VolSurfaceKind:
        """
        Swaption volatility surface.

        Returns
        -------
        VolSurfaceKind
            The ``swaption`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import VolSurfaceKind
        >>> str(VolSurfaceKind.swaption())
        'VolSurfaceKind.Swaption'
        """
        ...

    @property
    def name(self) -> str:
        """
        Variant name, e.g. ``"Equity"``.

        Returns
        -------
        str
            Pascal-case variant name.
        """
        ...

    @property
    def value(self) -> str:
        """
        Serialized wire value, e.g. ``"equity"``.

        Returns
        -------
        str
            Snake-case wire value used in JSON serialization.
        """
        ...

class TenorMatchMode:
    """
    Tenor-pillar alignment strategy for curve-node operations.

    Examples
    --------
    >>> from finstack_quant.scenarios import TenorMatchMode
    >>> TenorMatchMode.exact().value
    'exact'
    """

    @classmethod
    def exact(cls) -> TenorMatchMode:
        """
        Match curve nodes by exact tenor string.

        Returns
        -------
        TenorMatchMode
            The ``exact`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import TenorMatchMode
        >>> str(TenorMatchMode.exact())
        'TenorMatchMode.Exact'
        """
        ...

    @classmethod
    def interpolate(cls) -> TenorMatchMode:
        """
        Interpolate between adjacent curve nodes when tenor is not exact.

        Returns
        -------
        TenorMatchMode
            The ``interpolate`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import TenorMatchMode
        >>> str(TenorMatchMode.interpolate())
        'TenorMatchMode.Interpolate'
        """
        ...

    @property
    def name(self) -> str:
        """
        Variant name, e.g. ``"Exact"``.

        Returns
        -------
        str
            Pascal-case variant name.
        """
        ...

    @property
    def value(self) -> str:
        """
        Serialized wire value, e.g. ``"exact"``.

        Returns
        -------
        str
            Snake-case wire value used in JSON serialization.
        """
        ...

class TimeRollMode:
    """
    Calendar-vs-business-day semantics for time-roll operations.

    Examples
    --------
    >>> from finstack_quant.scenarios import TimeRollMode
    >>> TimeRollMode.calendar_days().value
    'calendar_days'
    """

    @classmethod
    def business_days(cls) -> TimeRollMode:
        """
        Roll by business days using the market calendar.

        Returns
        -------
        TimeRollMode
            The ``business_days`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import TimeRollMode
        >>> str(TimeRollMode.business_days())
        'TimeRollMode.BusinessDays'
        """
        ...

    @classmethod
    def calendar_days(cls) -> TimeRollMode:
        """
        Roll by calendar days (no holiday adjustment).

        Returns
        -------
        TimeRollMode
            The ``calendar_days`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import TimeRollMode
        >>> str(TimeRollMode.calendar_days())
        'TimeRollMode.CalendarDays'
        """
        ...

    @classmethod
    def approximate(cls) -> TimeRollMode:
        """
        Approximate roll (e.g. 30/360 day count).

        Returns
        -------
        TimeRollMode
            The ``approximate`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import TimeRollMode
        >>> str(TimeRollMode.approximate())
        'TimeRollMode.Approximate'
        """
        ...

    @property
    def name(self) -> str:
        """
        Variant name, e.g. ``"CalendarDays"``.

        Returns
        -------
        str
            Pascal-case variant name.
        """
        ...

    @property
    def value(self) -> str:
        """
        Serialized wire value, e.g. ``"calendar_days"``.

        Returns
        -------
        str
            Snake-case wire value used in JSON serialization.
        """
        ...

class Compounding:
    """
    Compounding convention for rate-extraction operations.

    Examples
    --------
    >>> from finstack_quant.scenarios import Compounding
    >>> Compounding.continuous().value
    'continuous'
    """

    @classmethod
    def simple(cls) -> Compounding:
        """
        Simple (zero-rate) compounding.

        Returns
        -------
        Compounding
            The ``simple`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.simple())
        'Compounding.Simple'
        """
        ...

    @classmethod
    def continuous(cls) -> Compounding:
        """
        Continuously compounded rate.

        Returns
        -------
        Compounding
            The ``continuous`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.continuous())
        'Compounding.Continuous'
        """
        ...

    @classmethod
    def annual(cls) -> Compounding:
        """
        Annual compounding.

        Returns
        -------
        Compounding
            The ``annual`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.annual())
        'Compounding.Annual'
        """
        ...

    @classmethod
    def semi_annual(cls) -> Compounding:
        """
        Semi-annual compounding.

        Returns
        -------
        Compounding
            The ``semi_annual`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.semi_annual())
        'Compounding.SemiAnnual'
        """
        ...

    @classmethod
    def quarterly(cls) -> Compounding:
        """
        Quarterly compounding.

        Returns
        -------
        Compounding
            The ``quarterly`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.quarterly())
        'Compounding.Quarterly'
        """
        ...

    @classmethod
    def monthly(cls) -> Compounding:
        """
        Monthly compounding.

        Returns
        -------
        Compounding
            The ``monthly`` variant.

        Examples
        --------
        >>> from finstack_quant.scenarios import Compounding
        >>> str(Compounding.monthly())
        'Compounding.Monthly'
        """
        ...

    @property
    def name(self) -> str:
        """
        Variant name, e.g. ``"Continuous"``.

        Returns
        -------
        str
            Pascal-case variant name.
        """
        ...

    @property
    def value(self) -> str:
        """
        Serialized wire value, e.g. ``"continuous"``.

        Returns
        -------
        str
            Snake-case wire value used in JSON serialization.
        """
        ...

class RateBindingSpec:
    """
    Configuration linking a statement rate node to a market curve.

    Examples
    --------
    >>> from finstack_quant.scenarios import RateBindingSpec, Compounding
    >>> spec = RateBindingSpec("node_1", "USD-OIS", "5Y", Compounding.continuous())
    >>> (spec.curve_id, spec.tenor, spec.compounding.value)
    ('USD-OIS', '5Y', 'continuous')
    """

    def __init__(
        self,
        node_id: str,
        curve_id: str,
        tenor: str,
        compounding: Compounding | None = None,
        day_count: DayCount | None = None,
    ) -> None:
        """
        Create a rate binding specification.

        Parameters
        ----------
        node_id : str
            Statement rate node identifier.
        curve_id : str
            Market curve identifier (e.g. ``"USD-OIS"``).
        tenor : str
            Tenor string (e.g. ``"5Y"``).
        compounding : Compounding, optional
            Compounding convention. Defaults to ``None`` (use curve default).
        day_count : DayCount, optional
            Typed day-count convention. Defaults to ``None`` (use curve default).

        Raises
        ------
        ValueError
            If required fields are empty or invalid.
        """
        ...

    @property
    def node_id(self) -> str:
        """
        Statement rate node identifier.

        Returns
        -------
        str
            Node ID string.
        """
        ...

    @property
    def curve_id(self) -> str:
        """
        Market curve identifier.

        Returns
        -------
        str
            Curve ID string (e.g. ``"USD-OIS"``).
        """
        ...

    @property
    def tenor(self) -> str:
        """
        Return the tenor for `RateBindingSpec`.
        Tenor string.

        Returns
        -------
        str
            Tenor label (e.g. ``"5Y"``).
        """
        ...

    @property
    def compounding(self) -> Compounding:
        """
        Compounding convention.

        Returns
        -------
        Compounding
            Compounding enum value, or the curve default when not specified.
        """
        ...

    @property
    def day_count(self) -> DayCount | None:
        """
        Day-count convention.

        Returns
        -------
        DayCount or None
            Typed day-count convention, or ``None`` when not specified.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to JSON.

        Returns
        -------
        str
            JSON-serialized ``RateBindingSpec``.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> RateBindingSpec:
        """
        Deserialize a ``RateBindingSpec`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        RateBindingSpec
            Parsed rate binding specification.

        Raises
        ------
        ValueError
            If the JSON is malformed or fields are invalid.

        Examples
        --------
        >>> from finstack_quant.scenarios import RateBindingSpec
        >>> spec = RateBindingSpec.from_json(
        ...     '{"node_id":"node_1","curve_id":"USD-OIS","tenor":"5Y","compounding":"continuous","day_count":null}'
        ... )
        >>> spec.tenor
        '5Y'
        """
        ...

class OperationSpec:
    """
    Typed builder for ``finstack_quant_scenarios::OperationSpec``.

    Each classmethod corresponds to one Rust enum variant; ``to_json()``
    produces the canonical wire form expected by ``build_scenario_spec`` and
    the scenario engine.

    Examples
    --------
    >>> from finstack_quant.scenarios import OperationSpec, CurveKind
    >>> op = OperationSpec.curve_parallel_bp(CurveKind.discount(), "USD-OIS", 10.0)
    >>> op.kind
    'curve_parallel_bp'
    """

    @classmethod
    def market_fx_pct(cls, base: str, quote: str, pct: float) -> OperationSpec:
        """
        FX rate percent shift (``pct = 5.0`` strengthens ``base`` by 5%).

        Parameters
        ----------
        base : str
            Base currency code.
        quote : str
            Quote currency code.
        pct : float
            Percent shift applied to the FX rate.

        Returns
        -------
        OperationSpec
            The ``market_fx_pct`` operation.

        Raises
        ------
        ValueError
            If ``base`` or ``quote`` is not a recognized ISO currency code.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.market_fx_pct("USD", "EUR", -5.0).kind
        'market_fx_pct'
        """
        ...

    @classmethod
    def equity_price_pct(cls, ids: list[str], pct: float) -> OperationSpec:
        """
        Equity price percent shock applied to all supplied identifiers.

        Parameters
        ----------
        ids : list[str]
            Equity identifier strings.
        pct : float
            Percent shock applied to each price.

        Returns
        -------
        OperationSpec
            The ``equity_price_pct`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.equity_price_pct(["SPY"], -10.0).kind
        'equity_price_pct'
        """
        ...

    @classmethod
    def instrument_price_pct_by_attr(cls, attrs: list[tuple[str, str]], pct: float) -> OperationSpec:
        """
        Instrument price shock by exact attribute match.

        ``attrs`` is a list of ``(key, value)`` pairs preserving order.

        Parameters
        ----------
        attrs : list[tuple[str, str]]
            Attribute key-value pairs to match.
        pct : float
            Percent shock applied to matched instruments.

        Returns
        -------
        OperationSpec
            The ``instrument_price_pct_by_attr`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.instrument_price_pct_by_attr([("sector", "tech")], -5.0).kind
        'instrument_price_pct_by_attr'
        """
        ...

    @classmethod
    def curve_parallel_bp(
        cls,
        curve_kind: CurveKind,
        curve_id: str,
        bp: float,
        discount_curve_id: str | None = None,
    ) -> OperationSpec:
        """
        Parallel basis-point shift on a rate-style curve.

        Parameters
        ----------
        curve_kind : CurveKind
            Type of curve to shock.
        curve_id : str
            Curve identifier in ``MarketContext``.
        bp : float
            Basis-point shift applied to every node.
        discount_curve_id : str, optional
            Discount curve ID for forward/inflation curves that require one.

        Returns
        -------
        OperationSpec
            The ``curve_parallel_bp`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind, OperationSpec
        >>> OperationSpec.curve_parallel_bp(CurveKind.discount(), "USD-OIS", 10.0).kind
        'curve_parallel_bp'
        """
        ...

    @classmethod
    def curve_node_bp(
        cls,
        curve_kind: CurveKind,
        curve_id: str,
        nodes: list[tuple[str, float]],
        match_mode: TenorMatchMode | None = None,
        discount_curve_id: str | None = None,
    ) -> OperationSpec:
        """
        Node-level basis-point shifts on a rate-style curve.

        Parameters
        ----------
        curve_kind : CurveKind
            Type of curve to shock.
        curve_id : str
            Curve identifier in ``MarketContext``.
        nodes : list[tuple[str, float]]
            List of ``(tenor, bp)`` pairs.
        match_mode : TenorMatchMode, optional
            Tenor alignment strategy. Defaults to exact matching.
        discount_curve_id : str, optional
            Discount curve ID for forward/inflation curves.

        Returns
        -------
        OperationSpec
            The ``curve_node_bp`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind, OperationSpec
        >>> OperationSpec.curve_node_bp(CurveKind.discount(), "USD-OIS", [("5Y", 10.0)]).kind
        'curve_node_bp'
        """
        ...

    @classmethod
    def vol_index_parallel_pts(cls, curve_id: str, points: float) -> OperationSpec:
        """
        Parallel shock to a volatility-index curve in absolute index points.

        Parameters
        ----------
        curve_id : str
            Volatility-index curve identifier.
        points : float
            Absolute index-point shift.

        Returns
        -------
        OperationSpec
            The ``vol_index_parallel_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.vol_index_parallel_pts("VIX", 2.0).kind
        'vol_index_parallel_pts'
        """
        ...

    @classmethod
    def vol_index_node_pts(
        cls,
        curve_id: str,
        nodes: list[tuple[str, float]],
        match_mode: TenorMatchMode | None = None,
    ) -> OperationSpec:
        """
        Node-level shocks to a volatility-index curve in absolute index points.

        Parameters
        ----------
        curve_id : str
            Volatility-index curve identifier.
        nodes : list[tuple[str, float]]
            List of ``(tenor, points)`` pairs.
        match_mode : TenorMatchMode, optional
            Tenor alignment strategy.

        Returns
        -------
        OperationSpec
            The ``vol_index_node_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.vol_index_node_pts("VIX", [("1M", 2.0)]).kind
        'vol_index_node_pts'
        """
        ...

    @classmethod
    def base_corr_parallel_pts(cls, surface_id: str, points: float) -> OperationSpec:
        """
        Parallel base-correlation shift (absolute correlation points).

        Parameters
        ----------
        surface_id : str
            Base-correlation surface identifier.
        points : float
            Absolute correlation-point shift.

        Returns
        -------
        OperationSpec
            The ``base_corr_parallel_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.base_corr_parallel_pts("CDX", 0.01).kind
        'base_corr_parallel_pts'
        """
        ...

    @classmethod
    def base_corr_bucket_pts(
        cls,
        surface_id: str,
        points: float,
        detachment_bp: list[int] | None = None,
    ) -> OperationSpec:
        """
        Bucketed base-correlation shock by detachment.

        Parameters
        ----------
        surface_id : str
            Base-correlation surface identifier.
        points : float
            Absolute correlation-point shift.
        detachment_bp : list[int], optional
            Detachment points (in bp) to target. ``None`` targets all.

        Returns
        -------
        OperationSpec
            The ``base_corr_bucket_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.base_corr_bucket_pts("CDX", 0.01).kind
        'base_corr_bucket_pts'
        """
        ...

    @classmethod
    def vol_surface_parallel_pct(cls, surface_kind: VolSurfaceKind, vol_surface_id: str, pct: float) -> OperationSpec:
        """
        Parallel percent shift to a volatility surface.

        Parameters
        ----------
        surface_kind : VolSurfaceKind
            Category of volatility surface.
        vol_surface_id : str
            Volatility-surface identifier.
        pct : float
            Percent shift applied to every vol quote.

        Returns
        -------
        OperationSpec
            The ``vol_surface_parallel_pct`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec, VolSurfaceKind
        >>> OperationSpec.vol_surface_parallel_pct(VolSurfaceKind.equity(), "SPX", 10.0).kind
        'vol_surface_parallel_pct'
        """
        ...

    @classmethod
    def vol_surface_bucket_pct(
        cls,
        surface_kind: VolSurfaceKind,
        vol_surface_id: str,
        pct: float,
        tenors: list[str] | None = None,
        strikes: list[float] | None = None,
    ) -> OperationSpec:
        """
        Bucketed volatility surface percent shock.

        Parameters
        ----------
        surface_kind : VolSurfaceKind
            Category of volatility surface.
        vol_surface_id : str
            Volatility-surface identifier.
        pct : float
            Percent shift applied to matched vol quotes.
        tenors : list[str], optional
            Tenor labels to target. ``None`` targets all.
        strikes : list[float], optional
            Strike levels to target. ``None`` targets all.

        Returns
        -------
        OperationSpec
            The ``vol_surface_bucket_pct`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec, VolSurfaceKind
        >>> OperationSpec.vol_surface_bucket_pct(VolSurfaceKind.equity(), "SPX", 10.0).kind
        'vol_surface_bucket_pct'
        """
        ...

    @classmethod
    def stmt_forecast_percent(cls, node_id: str, pct: float) -> OperationSpec:
        """
        Statement forecast percent change.

        Parameters
        ----------
        node_id : str
            Statement forecast node identifier.
        pct : float
            Percent change applied to the forecast value.

        Returns
        -------
        OperationSpec
            The ``stmt_forecast_percent`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.stmt_forecast_percent("revenue", 5.0).kind
        'stmt_forecast_percent'
        """
        ...

    @classmethod
    def stmt_forecast_assign(cls, node_id: str, value: float) -> OperationSpec:
        """
        Statement forecast value assignment.

        Parameters
        ----------
        node_id : str
            Statement forecast node identifier.
        value : float
            Absolute value to assign.

        Returns
        -------
        OperationSpec
            The ``stmt_forecast_assign`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.stmt_forecast_assign("revenue", 100.0).kind
        'stmt_forecast_assign'
        """
        ...

    @classmethod
    def rate_binding(cls, binding: RateBindingSpec) -> OperationSpec:
        """
        Bind a statement rate node to a curve for the lifetime of the scenario.

        Parameters
        ----------
        binding : RateBindingSpec
            Rate binding configuration.

        Returns
        -------
        OperationSpec
            The ``rate_binding`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec, RateBindingSpec
        >>> binding = RateBindingSpec("revenue", "USD-OIS", "5Y")
        >>> OperationSpec.rate_binding(binding).kind
        'rate_binding'
        """
        ...

    @classmethod
    def instrument_spread_bp_by_attr(cls, attrs: list[tuple[str, str]], bp: float) -> OperationSpec:
        """
        Instrument spread shock (basis points) by exact attribute match.

        Parameters
        ----------
        attrs : list[tuple[str, str]]
            Attribute key-value pairs to match.
        bp : float
            Basis-point shift applied to matched instruments.

        Returns
        -------
        OperationSpec
            The ``instrument_spread_bp_by_attr`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.instrument_spread_bp_by_attr([("sector", "tech")], 20.0).kind
        'instrument_spread_bp_by_attr'
        """
        ...

    @classmethod
    def instrument_price_pct_by_type(cls, instrument_types: list[str], pct: float) -> OperationSpec:
        """
        Instrument price shock by ``InstrumentType`` (snake_case strings).

        Parameters
        ----------
        instrument_types : list[str]
            Instrument type identifiers in snake_case.
        pct : float
            Percent shock applied to matched instruments.

        Returns
        -------
        OperationSpec
            The ``instrument_price_pct_by_type`` operation.

        Raises
        ------
        ValueError
            If any entry in ``instrument_types`` is not a recognized instrument type.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.instrument_price_pct_by_type(["bond"], -5.0).kind
        'instrument_price_pct_by_type'
        """
        ...

    @classmethod
    def instrument_spread_bp_by_type(cls, instrument_types: list[str], bp: float) -> OperationSpec:
        """
        Instrument spread shock by ``InstrumentType`` (snake_case strings).

        Parameters
        ----------
        instrument_types : list[str]
            Instrument type identifiers in snake_case.
        bp : float
            Basis-point shift applied to matched instruments.

        Returns
        -------
        OperationSpec
            The ``instrument_spread_bp_by_type`` operation.

        Raises
        ------
        ValueError
            If any entry in ``instrument_types`` is not a recognized instrument type.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.instrument_spread_bp_by_type(["bond"], 20.0).kind
        'instrument_spread_bp_by_type'
        """
        ...

    @classmethod
    def asset_correlation_pts(cls, delta_pts: float) -> OperationSpec:
        """
        Asset-correlation shock for structured credit.

        Parameters
        ----------
        delta_pts : float
            Absolute correlation-point shift.

        Returns
        -------
        OperationSpec
            The ``asset_correlation_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.asset_correlation_pts(0.05).kind
        'asset_correlation_pts'
        """
        ...

    @classmethod
    def prepay_default_correlation_pts(cls, delta_pts: float) -> OperationSpec:
        """
        Prepay-default correlation shock for structured credit.

        Parameters
        ----------
        delta_pts : float
            Absolute correlation-point shift.

        Returns
        -------
        OperationSpec
            The ``prepay_default_correlation_pts`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.prepay_default_correlation_pts(0.05).kind
        'prepay_default_correlation_pts'
        """
        ...

    @classmethod
    def hierarchy_curve_parallel_bp(
        cls,
        curve_kind: CurveKind,
        target_json: str,
        bp: float,
        discount_curve_id: str | None = None,
    ) -> OperationSpec:
        """
        Hierarchy-targeted parallel curve shift.

        ``target_json`` is a JSON-serialized ``HierarchyTarget``
        (``{"path": [...], "tag_filter": {...}}``).

        Parameters
        ----------
        curve_kind : CurveKind
            Type of curve to shock.
        target_json : str
            JSON-serialized ``HierarchyTarget``.
        bp : float
            Basis-point shift applied to every node.
        discount_curve_id : str, optional
            Discount curve ID for forward/inflation curves.

        Returns
        -------
        OperationSpec
            The ``hierarchy_curve_parallel_bp`` operation.

        Raises
        ------
        ValueError
            If ``target_json`` is not valid JSON for a ``HierarchyTarget``.

        Examples
        --------
        >>> from finstack_quant.scenarios import CurveKind, OperationSpec
        >>> OperationSpec.hierarchy_curve_parallel_bp(CurveKind.discount(), '{"path":["Credit"]}', 10.0).kind
        'hierarchy_curve_parallel_bp'
        """
        ...

    @classmethod
    def hierarchy_vol_surface_parallel_pct(
        cls, surface_kind: VolSurfaceKind, target_json: str, pct: float
    ) -> OperationSpec:
        """
        Hierarchy-targeted vol-surface percent shift.

        Parameters
        ----------
        surface_kind : VolSurfaceKind
            Category of volatility surface.
        target_json : str
            JSON-serialized ``HierarchyTarget``.
        pct : float
            Percent shift applied to matched vol quotes.

        Returns
        -------
        OperationSpec
            The ``hierarchy_vol_surface_parallel_pct`` operation.

        Raises
        ------
        ValueError
            If ``target_json`` is not valid JSON for a ``HierarchyTarget``.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec, VolSurfaceKind
        >>> OperationSpec.hierarchy_vol_surface_parallel_pct(VolSurfaceKind.equity(), '{"path":["Equity"]}', 10.0).kind
        'hierarchy_vol_surface_parallel_pct'
        """
        ...

    @classmethod
    def hierarchy_equity_price_pct(cls, target_json: str, pct: float) -> OperationSpec:
        """
        Hierarchy-targeted equity price shift.

        Parameters
        ----------
        target_json : str
            JSON-serialized ``HierarchyTarget``.
        pct : float
            Percent shift applied to matched equity prices.

        Returns
        -------
        OperationSpec
            The ``hierarchy_equity_price_pct`` operation.

        Raises
        ------
        ValueError
            If ``target_json`` is not valid JSON for a ``HierarchyTarget``.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.hierarchy_equity_price_pct('{"path":["Equity"]}', -5.0).kind
        'hierarchy_equity_price_pct'
        """
        ...

    @classmethod
    def hierarchy_base_corr_parallel_pts(cls, target_json: str, points: float) -> OperationSpec:
        """
        Hierarchy-targeted base-correlation parallel shift.

        Parameters
        ----------
        target_json : str
            JSON-serialized ``HierarchyTarget``.
        points : float
            Absolute correlation-point shift.

        Returns
        -------
        OperationSpec
            The ``hierarchy_base_corr_parallel_pts`` operation.

        Raises
        ------
        ValueError
            If ``target_json`` is not valid JSON for a ``HierarchyTarget``.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.hierarchy_base_corr_parallel_pts('{"path":["Credit"]}', 0.01).kind
        'hierarchy_base_corr_parallel_pts'
        """
        ...

    @classmethod
    def time_roll_forward(
        cls,
        period: str,
        apply_shocks: bool = True,
        roll_mode: TimeRollMode | None = None,
    ) -> OperationSpec:
        """
        Roll the valuation horizon forward (e.g. ``"1M"``).

        ``apply_shocks`` defaults to ``True`` to mirror the Rust
        ``#[serde(default = "default_true")]`` attribute.

        Parameters
        ----------
        period : str
            Tenor string for the roll period (e.g. ``"1M"``, ``"3M"``, ``"1Y"``).
        apply_shocks : bool, default True
            Whether to apply scenario shocks after the time roll.
        roll_mode : TimeRollMode, optional
            Calendar-vs-business-day roll mode.

        Returns
        -------
        OperationSpec
            The ``time_roll_forward`` operation.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.time_roll_forward("1M").kind
        'time_roll_forward'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to the canonical JSON wire format.

        Returns
        -------
        str
            JSON-serialized ``OperationSpec``.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> OperationSpec:
        """
        Deserialize an ``OperationSpec`` from JSON.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        OperationSpec
            Parsed operation specification.

        Raises
        ------
        ValueError
            If the JSON is malformed or the operation kind is unknown.

        Examples
        --------
        >>> from finstack_quant.scenarios import OperationSpec
        >>> OperationSpec.from_json('{"kind":"market_fx_pct","base":"USD","quote":"EUR","pct":-5.0}').kind
        'market_fx_pct'
        """
        ...

    @property
    def kind(self) -> str:
        """
        Variant discriminator (the serde ``kind`` tag value).

        Returns
        -------
        str
            Snake-case operation kind string.
        """
        ...
