"""Factor-model primitives, calibration, and decomposition.

Bindings for the ``finstack-quant-factor-model`` Rust crate. Credit hierarchy
calibration lives under :mod:`finstack_quant.factor_model.credit`.

Examples:
--------
>>> from finstack_quant.factor_model import credit
>>> try:
...     credit.CreditFactorModel.from_json("{}")
... except ValueError as exc:
...     "missing field" in str(exc)
True
"""

import sys as _sys

from finstack_quant.factor_model import credit as credit
from finstack_quant.finstack_quant import factor_model as _factor_model

# `schema` is a compiled submodule with no pure-Python shim package, so alias it
# onto the public dotted path that `import finstack_quant.factor_model.schema` uses.
schema = _factor_model.schema
_sys.modules.setdefault("finstack_quant.factor_model.schema", schema)

__all__: list[str] = [
    "credit",
    "schema",
]
