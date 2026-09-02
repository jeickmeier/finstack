"""Financial statement modeling: builders, evaluators, forecasts, DSL, adjustments.

Bindings for the ``finstack-quant-statements`` Rust crate.

Examples:
--------
>>> from finstack_quant.statements import NodeId
>>> NodeId("revenue").as_str()
'revenue'

"""

import sys as _sys

from finstack_quant.finstack_quant import statements as _statements

ForecastMethod = _statements.ForecastMethod
ForecastSpec = _statements.ForecastSpec
NodeType = _statements.NodeType
NodeId = _statements.NodeId
NumericMode = _statements.NumericMode
FinancialModelSpec = _statements.FinancialModelSpec
ModelBuilder = _statements.ModelBuilder
MixedNodeBuilder = _statements.MixedNodeBuilder
Registry = _statements.Registry
StatementResult = _statements.StatementResult
Evaluator = _statements.Evaluator
MonteCarloConfig = _statements.MonteCarloConfig
MonteCarloResults = _statements.MonteCarloResults
run_monte_carlo = _statements.run_monte_carlo
parse_formula_text = _statements.parse_formula_text
validate_formula = _statements.validate_formula
AppliedAdjustment = _statements.AppliedAdjustment
NormalizationConfig = _statements.NormalizationConfig
NormalizationResult = _statements.NormalizationResult
normalize = _statements.normalize
normalize_json = _statements.normalize_json
CheckSuiteSpec = _statements.CheckSuiteSpec
CheckReport = _statements.CheckReport
EcfSweepSpec = _statements.EcfSweepSpec
PaymentClassSpec = _statements.PaymentClassSpec
PikToggleSpec = _statements.PikToggleSpec
WaterfallSpec = _statements.WaterfallSpec

# `schema` is a compiled submodule with no pure-Python shim package, so alias it
# onto the public dotted path that `import finstack_quant.statements.schema` uses.
schema = _statements.schema
_sys.modules.setdefault("finstack_quant.statements.schema", schema)

__all__: list[str] = [
    "AppliedAdjustment",
    "CheckReport",
    "CheckSuiteSpec",
    "EcfSweepSpec",
    "Evaluator",
    "FinancialModelSpec",
    "ForecastMethod",
    "ForecastSpec",
    "MixedNodeBuilder",
    "ModelBuilder",
    "MonteCarloConfig",
    "MonteCarloResults",
    "NodeId",
    "NodeType",
    "NormalizationConfig",
    "NormalizationResult",
    "NumericMode",
    "PaymentClassSpec",
    "PikToggleSpec",
    "Registry",
    "StatementResult",
    "WaterfallSpec",
    "normalize",
    "normalize_json",
    "parse_formula_text",
    "run_monte_carlo",
    "schema",
    "validate_formula",
]
