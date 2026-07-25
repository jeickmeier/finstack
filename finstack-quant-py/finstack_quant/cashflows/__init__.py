"""Cashflow schedules: typed builder, primitives, accrual, aggregation, JSON bridge.

Examples:
--------
>>> import finstack_quant.cashflows as cashflows
>>> cashflows.__name__
'finstack_quant.cashflows'
"""

from __future__ import annotations

import sys

from finstack_quant.finstack_quant import cashflows as _cashflows

accrual = _cashflows.accrual
aggregation = _cashflows.aggregation
builder = _cashflows.builder
primitives = _cashflows.primitives

_submodules = {
    "accrual": accrual,
    "aggregation": aggregation,
    "builder": builder,
    "primitives": primitives,
}

for _name, _mod in _submodules.items():
    _key = f"finstack_quant.cashflows.{_name}"
    if _key not in sys.modules:
        sys.modules[_key] = _mod

build_cashflow_schedule_json = _cashflows.build_cashflow_schedule_json
validate_cashflow_schedule_json = _cashflows.validate_cashflow_schedule_json
dated_flows_json = _cashflows.dated_flows_json
accrued_interest_json = _cashflows.accrued_interest_json

for _fn in (
    "accrued_interest_json",
    "build_cashflow_schedule_json",
    "dated_flows_json",
    "validate_cashflow_schedule_json",
):
    globals()[_fn].__module__ = __name__

__all__: list[str] = [
    "accrual",
    "accrued_interest_json",
    "aggregation",
    "build_cashflow_schedule_json",
    "builder",
    "dated_flows_json",
    "primitives",
    "validate_cashflow_schedule_json",
]
