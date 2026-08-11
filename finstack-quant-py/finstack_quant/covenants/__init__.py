"""Covenant package JSON validation, templates, and map-backed evaluation.

Examples:
--------
>>> import json
>>> from finstack_quant.covenants import cov_lite_json
>>> len(json.loads(cov_lite_json(7.0, 4.5)))
3
"""

from __future__ import annotations

from finstack_quant.finstack_quant import covenants as _covenants

CovenantReport = _covenants.CovenantReport
cov_lite_json = _covenants.cov_lite_json
evaluate_engine = _covenants.evaluate_engine
lbo_standard_json = _covenants.lbo_standard_json
project_finance_json = _covenants.project_finance_json
real_estate_json = _covenants.real_estate_json
validate_covenant_engine_json = _covenants.validate_covenant_engine_json
validate_covenant_report_json = _covenants.validate_covenant_report_json
validate_covenant_spec_json = _covenants.validate_covenant_spec_json

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
