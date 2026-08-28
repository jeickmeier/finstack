"""Quote ingestion, market construction, and explicit model calibration.

Examples:
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

import sys as _sys

from finstack_quant.calibration.envelope import CalibrationEnvelope as CalibrationEnvelope
from finstack_quant.finstack_quant import calibration as _calibration

CalibrationEnvelopeError = _calibration.CalibrationEnvelopeError
CalibrationResult = _calibration.CalibrationResult
calibrate = _calibration.calibrate
calibrate_bermudan_lmm_base_vol = _calibration.calibrate_bermudan_lmm_base_vol
dependency_graph_json = _calibration.dependency_graph_json
dry_run = _calibration.dry_run
validate_calibration_json = _calibration.validate_calibration_json
schema = _calibration.schema

if "finstack_quant.calibration.schema" not in _sys.modules:
    _sys.modules["finstack_quant.calibration.schema"] = schema

__all__ = [
    "CalibrationEnvelope",
    "CalibrationEnvelopeError",
    "CalibrationResult",
    "calibrate",
    "calibrate_bermudan_lmm_base_vol",
    "dependency_graph_json",
    "dry_run",
    "schema",
    "validate_calibration_json",
]
