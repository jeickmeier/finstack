"""
Covenant package JSON validation, templates, and map-backed evaluation.

Bindings for ``finstack-quant-covenants``. Validate covenant specs, reports, and
engines; evaluate an engine against a metric map into typed
:class:`CovenantReport` results; or instantiate standard covenant packages
(LBO, covenant-lite, real estate, project finance) as JSON.

Examples
--------
>>> import json
>>> from finstack_quant.covenants import cov_lite_json
>>> len(json.loads(cov_lite_json(7.0, 4.5)))
3
"""

from __future__ import annotations

import datetime

from typing import Any

import pandas as pd

__all__ = [
    "CovenantReport",
    "cov_lite_json",
    "evaluate_engine",
    "lbo_standard_json",
    "project_finance_json",
    "real_estate_json",
    "validate_covenant_engine_json",
    "validate_covenant_report_json",
    "validate_covenant_spec_json",
]

class CovenantReport:
    """
    Result of a single covenant evaluation.

    Carries pass/fail status, the tested value against its threshold, the
    headroom (positive is cushion, negative is deficit), an optional
    human-readable explanation, and the audit stamp in force when the covenant
    was evaluated.

    Construct via :func:`evaluate_engine` or :meth:`from_json`.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import CovenantReport
    >>> report = CovenantReport.from_json(
    ...     json.dumps({
    ...         "covenant_type": "Debt/EBITDA <= 5.00x",
    ...         "passed": False,
    ...         "actual_value": 5.5,
    ...         "threshold": 5.0,
    ...         "details": "Exceeded",
    ...         "headroom": -0.5,
    ...     })
    ... )
    >>> report.passed
    False
    """

    @staticmethod
    def from_json(json: str) -> CovenantReport:
        """
        Deserialize a ``CovenantReport`` from JSON.

        Parameters
        ----------
        json : str
            JSON string matching the ``CovenantReport`` wire schema.

        Returns
        -------
        CovenantReport
            Parsed report.

        Raises
        ------
        ValueError
            If ``json`` is malformed or omits required fields.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.covenants import CovenantReport
        >>> report = CovenantReport.from_json(
        ...     json.dumps({
        ...         "covenant_type": "Debt/EBITDA <= 5.00x",
        ...         "passed": False,
        ...         "actual_value": 5.5,
        ...         "threshold": 5.0,
        ...         "details": "Exceeded",
        ...         "headroom": -0.5,
        ...     })
        ... )
        >>> CovenantReport.from_json(report.to_json()).headroom
        -0.5
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

    @property
    def covenant_type(self) -> str:
        """
        Human-readable description of the covenant being tested.

        Returns
        -------
        str
            For example ``"Debt/EBITDA <= 5.00x"``.
        """
        ...

    @property
    def covenant_id(self) -> str | None:
        """
        Stable machine-readable covenant instance identifier.

        Returns
        -------
        str | None
            ``None`` when the report was produced without an identifier.
        """
        ...

    @property
    def passed(self) -> bool:
        """
        Whether the covenant passed.

        Inactive covenants, unmet springing conditions, and full waivers also
        report ``True``; :attr:`details` carries the reason.

        Returns
        -------
        bool
            ``True`` when the tested metric satisfied its threshold. Because an
            untested covenant also reports ``True``, read it together with
            :attr:`actual_value`, which is ``None`` exactly when no numeric
            test ran.
        """
        ...

    @property
    def actual_value(self) -> float | None:
        """
        Tested metric value.

        Returns
        -------
        float | None
            ``None`` when the covenant was not evaluated numerically (inactive,
            waived, or springing condition unmet).
        """
        ...

    @property
    def threshold(self) -> float | None:
        """
        Threshold the metric was tested against.

        Returns
        -------
        float | None
            ``None`` when no numeric test was applied.
        """
        ...

    @property
    def details(self) -> str | None:
        """
        Explanation of the outcome.

        Returns
        -------
        str | None
            Free text such as ``"Covenant inactive"``, ``"Waived by lender
            agreement"``, or ``"In cure period"``. ``None`` for a plain numeric
            result whose evaluator supplied no commentary, so absence carries
            no meaning of its own.
        """
        ...

    @property
    def headroom(self) -> float | None:
        """
        Cushion relative to the threshold.

        Returns
        -------
        float | None
            Positive is a passing buffer, negative a deficit. ``None`` when no
            numeric test was applied.
        """
        ...

    @property
    def meta(self) -> dict[str, Any]:
        """
        Audit stamp: numeric mode, rounding context, and FX policy in force.

        Returns
        -------
        dict[str, Any]
            Keys ``numeric_mode``, ``rounding``, ``fx_policy_applied`` and
            ``version``; ``fx_policy_applied`` is ``None`` when the evaluation
            stayed in one currency. Reproducing a report requires re-running
            under the same ``rounding`` context.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the report as a single-row pandas DataFrame.

        Columns: ``covenant_type``, ``covenant_id``, ``passed``,
        ``actual_value``, ``threshold``, ``headroom``, ``details``.

        Returns
        -------
        pd.DataFrame
            One row describing this covenant evaluation.
        """
        ...

def validate_covenant_spec_json(spec_json: str) -> str:
    """
    Validate and canonicalize a covenant specification JSON string.

    Parameters
    ----------
    spec_json : str
        JSON-encoded ``CovenantSpec`` describing thresholds, tests, and covenants.

    Returns
    -------
    str
        Canonical JSON after validation.

    Raises
    ------
    ValueError
        If the spec fails schema or semantic validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import lbo_standard_json, validate_covenant_spec_json
    >>> spec = json.loads(lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0))[0]
    >>> json.loads(validate_covenant_spec_json(json.dumps(spec)))["metric_id"]
    'debt_to_ebitda'
    """

def validate_covenant_report_json(report_json: str) -> str:
    """
    Validate and canonicalize a covenant evaluation report JSON string.

    Parameters
    ----------
    report_json : str
        JSON-encoded ``CovenantReport`` with pass/fail and headroom per covenant.

    Returns
    -------
    str
        Canonical JSON after validation.

    Raises
    ------
    ValueError
        If the report JSON is malformed or fails validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import validate_covenant_report_json
    >>> report = {
    ...     "covenant_type": "Debt/EBITDA <= 5.00x",
    ...     "covenant_id": "max_debt_ebitda",
    ...     "passed": False,
    ...     "actual_value": 5.5,
    ...     "threshold": 5.0,
    ...     "details": "Exceeded",
    ...     "headroom": -0.1,
    ... }
    >>> json.loads(validate_covenant_report_json(json.dumps(report)))["passed"]
    False
    """

def validate_covenant_engine_json(engine_json: str) -> str:
    """
    Validate and canonicalize a covenant engine JSON string.

    Parameters
    ----------
    engine_json : str
        JSON-encoded covenant engine configuration bundling specs and evaluation
        policy.

    Returns
    -------
    str
        Canonical JSON after validation.

    Raises
    ------
    ValueError
        If the engine JSON is malformed or fails validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import validate_covenant_engine_json
    >>> engine = {"specs": [], "breach_history": [], "windows": [], "waivers": []}
    >>> len(json.loads(validate_covenant_engine_json(json.dumps(engine)))["specs"])
    0
    """

def evaluate_engine(engine_json: str, metrics_json: str, as_of: datetime.date | str) -> dict[str, CovenantReport]:
    """
    Evaluate a covenant engine against a JSON metric map.

    Parameters
    ----------
    engine_json : str
        Serialized covenant engine configuration.
    metrics_json : str
        JSON map of metric name to numeric value (e.g. leverage, DSCR, coverage).
    as_of : datetime.date | str
        Evaluation date, either a date-like object or an ISO 8601 string.

    Returns
    -------
    dict[str, CovenantReport]
        Typed report per covenant, keyed by stable covenant instance key.

    Raises
    ------
    ValueError
        If engine or metrics JSON is invalid, or required metrics are missing.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import evaluate_engine, lbo_standard_json
    >>> spec = json.loads(lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0))[0]
    >>> engine = json.dumps({"specs": [spec], "breach_history": [], "windows": [], "waivers": []})
    >>> reports = evaluate_engine(engine, '{"debt_to_ebitda": 4.0}', "2026-03-31")
    >>> reports["max_debt_ebitda"].passed
    True
    """

def lbo_standard_json(
    initial_leverage: float,
    interest_coverage: float,
    fixed_charge_coverage: float,
    max_capex: float,
) -> str:
    """
    Return a standard leveraged-buyout covenant package as JSON.

    Parameters
    ----------
    initial_leverage : float
        Maximum net leverage ratio (e.g. ``6.0`` for 6.0x).
    interest_coverage : float
        Minimum interest coverage ratio.
    fixed_charge_coverage : float
        Minimum fixed-charge coverage ratio.
    max_capex : float
        Maximum capital expenditure as a fraction of EBITDA or similar base.

    Returns
    -------
    str
        JSON-encoded ``CovenantSpec`` for a typical LBO covenant suite.

    Raises
    ------
    ValueError
        If any threshold is non-finite or out of range.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import lbo_standard_json
    >>> len(json.loads(lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0)))
    4
    """

def cov_lite_json(max_leverage: float, max_senior_leverage: float) -> str:
    """
    Return a covenant-lite package as JSON.

    Parameters
    ----------
    max_leverage : float
        Maximum total leverage ratio.
    max_senior_leverage : float
        Maximum senior secured leverage ratio.

    Returns
    -------
    str
        JSON-encoded ``CovenantSpec`` with minimal maintenance covenants.

    Raises
    ------
    ValueError
        If thresholds are non-finite or out of range.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import cov_lite_json
    >>> len(json.loads(cov_lite_json(7.0, 4.5)))
    3
    """

def real_estate_json(min_dscr: float, min_debt_yield: float, max_ltv: float) -> str:
    """
    Return a real-estate covenant package as JSON.

    Parameters
    ----------
    min_dscr : float
        Minimum debt-service coverage ratio.
    min_debt_yield : float
        Minimum debt yield (decimal, e.g. ``0.08`` for 8%).
    max_ltv : float
        Maximum loan-to-value ratio (decimal, e.g. ``0.75`` for 75%).

    Returns
    -------
    str
        JSON-encoded ``CovenantSpec`` for commercial real-estate lending.

    Raises
    ------
    ValueError
        If thresholds are non-finite or out of range.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import real_estate_json
    >>> len(json.loads(real_estate_json(1.25, 0.08, 0.75)))
    3
    """

def project_finance_json(
    min_dscr: float,
    distribution_lockup_dscr: float,
    min_liquidity: float,
    max_net_leverage: float,
) -> str:
    """
    Return a project-finance covenant package as JSON.

    Parameters
    ----------
    min_dscr : float
        Minimum debt-service coverage ratio.
    distribution_lockup_dscr : float
        DSCR threshold below which distributions are locked up.
    min_liquidity : float
        Minimum liquidity reserve (currency units or ratio per spec convention).
    max_net_leverage : float
        Maximum net leverage ratio.

    Returns
    -------
    str
        JSON-encoded ``CovenantSpec`` for project-finance structures.

    Raises
    ------
    ValueError
        If thresholds are non-finite or out of range.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.covenants import project_finance_json
    >>> len(json.loads(project_finance_json(1.30, 1.10, 5_000_000.0, 5.0)))
    4
    """
