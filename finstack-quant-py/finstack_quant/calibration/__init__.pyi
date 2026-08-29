"""Quote ingestion, market construction, and explicit model calibration.

Examples
--------
>>> import json
>>> from finstack_quant.calibration import calibrate
>>> envelope = {
...     "schema": "finstack_quant.calibration/1",
...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
... }
>>> calibrate(json.dumps(envelope)).success
True
"""

from __future__ import annotations

import datetime
from typing import Any

import pandas as pd

from finstack_quant.calibration.envelope import CalibrationEnvelope as CalibrationEnvelope
from finstack_quant.calibration import schema as schema
from finstack_quant.core.market_data import MarketContext

__all__ = [
    "CalibrationEnvelope",
    "CalibrationEnvelopeError",
    "CalibrationResult",
    "calibrate",
    "calibrate_bermudan_lmm_base_vol",
    "dry_run",
    "schema",
    "validate_calibration_json",
]

class CalibrationResult:
    """Result of a calibration plan execution.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.calibration import calibrate
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> calibrate(json.dumps(envelope)).success
    True
    """

    @staticmethod
    def from_json(json: str) -> CalibrationResult:
        """Rebuild a calibration result from its JSON envelope.

        Parameters
        ----------
        json : str
            JSON produced by :meth:`to_json`.

        Returns
        -------
        CalibrationResult
            Rehydrated result wrapper.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not encode a result envelope.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.calibration import CalibrationResult, calibrate
        >>> envelope = {
        ...     "schema": "finstack_quant.calibration/1",
        ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
        ... }
        >>> result = calibrate(json.dumps(envelope))
        >>> CalibrationResult.from_json(result.to_json()).success
        True
        """
        ...

    def to_json(self) -> str:
        """Serialize this result to pretty-printed JSON.

        Returns
        -------
        str
            JSON calibration result envelope.

        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @property
    def success(self) -> bool:
        """Whether every calibration step passed its fit tolerance.

        Returns
        -------
        bool
            ``True`` only when the overall plan succeeded.

        Notes
        -----
        This accessor does not raise; it returns the stored plan status.
        """
        ...

    @property
    def market(self) -> MarketContext:
        """Calibrated market context ready for pricing.

        Returns
        -------
        MarketContext
            Curves, surfaces, and snapshot data produced by the plan.

        Raises
        ------
        ValueError
            If the stored market snapshot cannot be rehydrated.
        """
        ...

    @property
    def market_json(self) -> str:
        """Return the calibrated market snapshot as JSON.

        Returns
        -------
        str
            Serialized ``MarketContextState``.

        Raises
        ------
        ValueError
            If the snapshot cannot be serialized.
        """
        ...

    @property
    def report_json(self) -> str:
        """Return the aggregate calibration report as JSON.

        Returns
        -------
        str
            Serialized plan-level report.

        Raises
        ------
        ValueError
            If the report cannot be serialized.
        """
        ...

    @property
    def step_ids(self) -> list[str]:
        """Return calibration step identifiers in deterministic order.

        Returns
        -------
        list[str]
            Lexicographically ordered step identifiers.

        Notes
        -----
        This accessor does not raise; it returns stored report identifiers.
        """
        ...

    @property
    def iterations(self) -> int:
        """Return total solver iterations across all steps.

        Returns
        -------
        int
            Sum of per-step iteration counts.

        Notes
        -----
        This accessor does not raise; it sums stored iteration counts.
        """
        ...

    @property
    def max_residual(self) -> float:
        """Return the largest normalized fit residual.

        Returns
        -------
        float
            Maximum ``abs(residual) / tolerance`` across steps.

        Notes
        -----
        This accessor does not raise; it reads the stored aggregate report.
        """
        ...

    @property
    def rmse(self) -> float:
        """Return root-mean-square normalized fit error.

        Returns
        -------
        float
            RMSE of normalized quote residuals.

        Notes
        -----
        This accessor does not raise; it reads the stored aggregate report.
        """
        ...

    def step_report_json(self, step_id: str) -> str:
        """Return one step's calibration report as JSON.

        Parameters
        ----------
        step_id : str
            Exact plan step identifier.

        Returns
        -------
        str
            Serialized per-step report.

        Raises
        ------
        ValueError
            If ``step_id`` is not present or serialization fails.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """Build a tabular summary with one row per calibration step.

        Returns
        -------
        pandas.DataFrame
            Step ID, status, iterations, residuals, and convergence reason.

        Raises
        ------
        ValueError
            If the report cannot be converted to Python tabular values.
        """
        ...

    def __repr__(self) -> str: ...

class CalibrationEnvelopeError(RuntimeError):
    """Structured calibration ingestion, validation, or execution failure.

    Examples
    --------
    >>> from finstack_quant.calibration import CalibrationEnvelopeError, dry_run
    >>> try:
    ...     dry_run("{ malformed")
    ... except CalibrationEnvelopeError as exc:
    ...     exc.kind
    'strict_load'
    """

    kind: str
    stage: str
    step_id: str | None
    solver_diagnostics: str | None
    details: str

def validate_calibration_json(json: str) -> str:
    """Validate and canonicalize a calibration envelope without solving it.

    Parameters
    ----------
    json : str
        JSON calibration envelope using schema marker
        ``finstack_quant.calibration/1``.

    Returns
    -------
    str
        Canonical pretty-printed envelope JSON.

    Raises
    ------
    CalibrationEnvelopeError
        If ingestion or static validation fails. Static validation is
        fail-fast (first error); use ``dry_run`` to list every static error.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.calibration import validate_calibration_json
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> json.loads(validate_calibration_json(json.dumps(envelope)))["plan"]["id"]
    'smoke'
    """
    ...

def dry_run(json: str) -> str:
    """Validate plan dependencies and return all static errors without solving.

    Parameters
    ----------
    json : str
        JSON calibration envelope.

    Returns
    -------
    str
        JSON validation report containing errors and the dependency graph.

    Raises
    ------
    CalibrationEnvelopeError
        If the input cannot be loaded as an envelope.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.calibration import dry_run
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> json.loads(dry_run(json.dumps(envelope)))["errors"]
    []
    """
    ...

def calibrate(json: str) -> CalibrationResult:
    """Build a market context from quotes and an ordered calibration plan.

    Parameters
    ----------
    json : str
        JSON calibration envelope using schema marker
        ``finstack_quant.calibration/1``.

    Returns
    -------
    CalibrationResult
        Market snapshot and fit reports for the completed plan.

    Raises
    ------
    CalibrationEnvelopeError
        If ingestion, dependency resolution, market construction, or solving
        fails. Static validation is fail-fast (first error); use ``dry_run``
        to list every static error.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.calibration import calibrate
    >>> envelope = {
    ...     "schema": "finstack_quant.calibration/1",
    ...     "plan": {"id": "smoke", "description": None, "quote_sets": {}, "steps": [], "settings": {}},
    ... }
    >>> calibrate(json.dumps(envelope)).success
    True
    """
    ...

def calibrate_bermudan_lmm_base_vol(
    instrument_json: str,
    market: MarketContext | str,
    as_of: datetime.date | str,
) -> float:
    """Fit the explicit Bermudan LMM loading scale from a swaption surface.

    Parameters
    ----------
    instrument_json : str
        Canonical Bermudan-swaption instrument envelope.
    market : MarketContext or str
        Market context, or its JSON form, carrying discount and swaption-vol
        inputs.
    as_of : datetime.date or str
        Valuation date used to resolve expiries and tenor times.

    Returns
    -------
    float
        Positive finite annualized decimal value for
        ``model_config.lmm_base_vol``.

    Raises
    ------
    ValueError
        If the instrument is not a Bermudan swaption or an input is invalid.
    RuntimeError
        If market lookup or the Rebonato calibration fails.

    Examples
    --------
    >>> from finstack_quant.calibration import calibrate_bermudan_lmm_base_vol
    >>> try:
    ...     calibrate_bermudan_lmm_base_vol("{}", "{}", "2025-01-01")
    ... except ValueError as exc:
    ...     "missing field" in str(exc)
    True
    """
    ...
