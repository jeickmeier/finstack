"""Factor and position risk decomposition kernels.

Examples:
--------
>>> from finstack_quant.models.factor.risk import DecompositionConfig
>>> DecompositionConfig.parametric_95().confidence
0.95
"""

from finstack_quant.finstack_quant import models as _models

_risk = _models.factor.risk

FactorContribution = _risk.FactorContribution
PositionFactorContribution = _risk.PositionFactorContribution
PositionResidualContribution = _risk.PositionResidualContribution
RiskDecomposition = _risk.RiskDecomposition
PositionVarContribution = _risk.PositionVarContribution
PositionEsContribution = _risk.PositionEsContribution
PositionRiskDecomposition = _risk.PositionRiskDecomposition
PositionBudgetEntry = _risk.PositionBudgetEntry
RiskBudgetResult = _risk.RiskBudgetResult
StressPositionEntry = _risk.StressPositionEntry
TailScenarioBreakdown = _risk.TailScenarioBreakdown
StressAttribution = _risk.StressAttribution
DecompositionConfig = _risk.DecompositionConfig
parametric_var_decomposition = _risk.parametric_var_decomposition
parametric_es_decomposition = _risk.parametric_es_decomposition
historical_var_decomposition = _risk.historical_var_decomposition
evaluate_risk_budget = _risk.evaluate_risk_budget
build_stress_attribution = _risk.build_stress_attribution
position_component_var = _risk.position_component_var

__all__: list[str] = [
    "DecompositionConfig",
    "FactorContribution",
    "PositionBudgetEntry",
    "PositionEsContribution",
    "PositionFactorContribution",
    "PositionResidualContribution",
    "PositionRiskDecomposition",
    "PositionVarContribution",
    "RiskBudgetResult",
    "RiskDecomposition",
    "StressAttribution",
    "StressPositionEntry",
    "TailScenarioBreakdown",
    "build_stress_attribution",
    "evaluate_risk_budget",
    "historical_var_decomposition",
    "parametric_es_decomposition",
    "parametric_var_decomposition",
    "position_component_var",
]
