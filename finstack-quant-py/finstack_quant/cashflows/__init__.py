"""Cashflow schedules: typed builder, primitives, accrual, aggregation, JSON bridge.

Examples:
>>> import datetime
>>> from finstack_quant.cashflows.primitives import CFKind, CashFlow
>>> from finstack_quant.core.money import Money
>>> CashFlow(datetime.date(2025, 6, 15), Money(100.0, "USD"), CFKind.FIXED).amount.amount
100.0

"""

import sys as _sys

from finstack_quant.finstack_quant import cashflows as _cashflows

accrual = _cashflows.accrual
aggregation = _cashflows.aggregation
builder = _cashflows.builder
primitives = _cashflows.primitives
schema = _cashflows.schema

_submodules = {
    "accrual": accrual,
    "aggregation": aggregation,
    "builder": builder,
    "primitives": primitives,
    "schema": schema,
}

for _name, _mod in _submodules.items():
    _key = f"finstack_quant.cashflows.{_name}"
    if _key not in _sys.modules:
        _sys.modules[_key] = _mod

build_cashflow_schedule_json = _cashflows.build_cashflow_schedule_json
validate_cashflow_schedule_json = _cashflows.validate_cashflow_schedule_json
dated_flows_json = _cashflows.dated_flows_json
accrued_interest = _cashflows.accrued_interest

cpr_to_smm = _cashflows.cpr_to_smm
smm_to_cpr = _cashflows.smm_to_cpr
cdr_to_mdr = _cashflows.cdr_to_mdr
mdr_to_cdr = _cashflows.mdr_to_cdr

for _fn in (
    "accrued_interest",
    "build_cashflow_schedule_json",
    "cdr_to_mdr",
    "cpr_to_smm",
    "dated_flows_json",
    "mdr_to_cdr",
    "smm_to_cpr",
    "validate_cashflow_schedule_json",
):
    globals()[_fn].__module__ = __name__

__all__: list[str] = [
    "accrual",
    "accrued_interest",
    "aggregation",
    "build_cashflow_schedule_json",
    "builder",
    "cdr_to_mdr",
    "cpr_to_smm",
    "dated_flows_json",
    "mdr_to_cdr",
    "primitives",
    "schema",
    "smm_to_cpr",
    "validate_cashflow_schedule_json",
]
