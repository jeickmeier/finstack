"""Portfolio construction, valuation, optimization, cashflows, scenarios, and metrics.

Bindings for the ``finstack-quant-portfolio`` Rust crate.

Stability tiers
---------------

The exports below fall into three stability tiers. Treat the tier as a
contract about how disruptive future changes are likely to be.

**Stable** — covered by golden tests and meant to round-trip across releases:

* ``Portfolio``, ``PortfolioValuation``, ``PortfolioResult``,
  ``PortfolioCashflows`` (the typed handles)
* ``parse_portfolio_spec``, ``build_portfolio_from_spec``
* ``value_portfolio``, ``aggregate_full_cashflows``,
  ``apply_scenario_and_revalue``
* ``aggregate_metrics``, ``portfolio_result_total_value``,
  ``portfolio_result_get_metric``
* ``replay_portfolio``

**Stable, typed contracts may evolve** — function signatures are stable, but
typed result classes may gain fields between releases:

* ``optimize_portfolio`` (``PortfolioOptimizationSpec`` /
  ``PortfolioOptimizationResult``)
* ``parametric_var_decomposition``, ``parametric_es_decomposition``,
  ``historical_var_decomposition``, ``evaluate_risk_budget``

**Experimental** — calibration constants and convenience defaults still
under review; signatures or default coefficients may change:

* ``lvar_bangia`` — exogenous spread-cost model; calibration defaults live
  in the Rust crate's ``liquidity`` module.
* ``almgren_chriss_impact`` — fixes ``delta`` at 0.5; the underlying
  ``optimal_trajectory`` accepts only ``delta = 1`` (linear impact).
* ``kyle_lambda``, ``roll_effective_spread``, ``amihud_illiquidity``,
  ``days_to_liquidate``, ``liquidity_tier`` — small free functions, may be
  re-grouped or renamed.

Bindings should be considered cross-version-compatible only within a single
``finstack-quant-portfolio`` minor release; pin the upstream version when
exporting to downstream services.

Exceptions
----------

``FinstackError`` is the common base for the library's named exceptions, so
``except FinstackError`` catches ``PortfolioError``, ``ContractValidationError``
and their subclasses in one clause. It derives from ``ValueError``, so every
existing ``except ValueError`` keeps working unchanged. Its canonical home is
``finstack_quant.core``; it is re-exported here for convenience.

Examples:
--------
>>> from finstack_quant.portfolio import Portfolio
>>> spec = '{"id":"empty","base_currency":"USD","as_of":"2025-01-01","entities":{},"positions":[]}'
>>> (Portfolio.from_spec(spec).id, len(Portfolio.from_spec(spec)))
('empty', 0)
"""

import sys as _sys

from finstack_quant.finstack_quant import portfolio as _portfolio

FinstackError = _portfolio.FinstackError
PortfolioError = _portfolio.PortfolioError
ValuationError = _portfolio.ValuationError
FxError = _portfolio.FxError
OptimizationError = _portfolio.OptimizationError
ContractValidationError = _portfolio.ContractValidationError
UnsupportedContractVersionError = _portfolio.UnsupportedContractVersionError
MissingContractVersionError = _portfolio.MissingContractVersionError
MalformedContractSchemaError = _portfolio.MalformedContractSchemaError
ContractLimitExceededError = _portfolio.ContractLimitExceededError

Portfolio = _portfolio.Portfolio
InstrumentArtifactCache = _portfolio.InstrumentArtifactCache
MaterializationReport = _portfolio.MaterializationReport
PortfolioValuation = _portfolio.PortfolioValuation
ScenarioPnl = _portfolio.ScenarioPnl
ScenarioPnlBatchItem = _portfolio.ScenarioPnlBatchItem
PortfolioResult = _portfolio.PortfolioResult
PortfolioMetrics = _portfolio.PortfolioMetrics
PortfolioCashflows = _portfolio.PortfolioCashflows
PortfolioAttribution = _portfolio.PortfolioAttribution

# Typed attribution/performance result wrappers
BrinsonPeriodResult = _portfolio.BrinsonPeriodResult
CarinoLinkedAttribution = _portfolio.CarinoLinkedAttribution
DurationCellTable = _portfolio.DurationCellTable
ExcessReturnResult = _portfolio.ExcessReturnResult
FactorBrinsonResult = _portfolio.FactorBrinsonResult
FiAttributionResult = _portfolio.FiAttributionResult
FiCarinoLinkedResult = _portfolio.FiCarinoLinkedResult
FiReconciliationReport = _portfolio.FiReconciliationReport
GridAttributionResult = _portfolio.GridAttributionResult
GridCarinoLinkedResult = _portfolio.GridCarinoLinkedResult
LinkedReturn = _portfolio.LinkedReturn
ReplayResult = _portfolio.ReplayResult
WeightAllocationResult = _portfolio.WeightAllocationResult

parse_portfolio_spec = _portfolio.parse_portfolio_spec
build_portfolio_from_spec = _portfolio.build_portfolio_from_spec
portfolio_result_total_value = _portfolio.portfolio_result_total_value
portfolio_result_get_metric = _portfolio.portfolio_result_get_metric
aggregate_metrics = _portfolio.aggregate_metrics
aggregate_metrics_json = _portfolio.aggregate_metrics_json
value_portfolio = _portfolio.value_portfolio
aggregate_full_cashflows = _portfolio.aggregate_full_cashflows
net_in_currency_by_date = _portfolio.net_in_currency_by_date
apply_scenario_and_revalue = _portfolio.apply_scenario_and_revalue
scenario_pnl = _portfolio.scenario_pnl
scenario_pnl_batch = _portfolio.scenario_pnl_batch
scenario_pnl_batch_json = _portfolio.scenario_pnl_batch_json
attribute_portfolio_pnl = _portfolio.attribute_portfolio_pnl
allocate_weights = _portfolio.allocate_weights
allocate_weights_json = _portfolio.allocate_weights_json
validate_allocation_json = _portfolio.validate_allocation_json
optimize_portfolio = _portfolio.optimize_portfolio
replay_portfolio = _portfolio.replay_portfolio
replay_portfolio_json = _portfolio.replay_portfolio_json
parametric_var_decomposition = _portfolio.parametric_var_decomposition
parametric_es_decomposition = _portfolio.parametric_es_decomposition
historical_var_decomposition = _portfolio.historical_var_decomposition
evaluate_risk_budget = _portfolio.evaluate_risk_budget
roll_effective_spread = _portfolio.roll_effective_spread
amihud_illiquidity = _portfolio.amihud_illiquidity
days_to_liquidate = _portfolio.days_to_liquidate
liquidity_tier = _portfolio.liquidity_tier
lvar_bangia = _portfolio.lvar_bangia
almgren_chriss_impact = _portfolio.almgren_chriss_impact
kyle_lambda = _portfolio.kyle_lambda
brinson_fachler = _portfolio.brinson_fachler
brinson_fachler_json = _portfolio.brinson_fachler_json
carino_link = _portfolio.carino_link
carino_link_json = _portfolio.carino_link_json
campisi_attribution = _portfolio.campisi_attribution
campisi_attribution_json = _portfolio.campisi_attribution_json
campisi_carino_link = _portfolio.campisi_carino_link
campisi_carino_link_json = _portfolio.campisi_carino_link_json
campisi_carino_link_from_snapshots = _portfolio.campisi_carino_link_from_snapshots
campisi_carino_link_from_snapshots_json = _portfolio.campisi_carino_link_from_snapshots_json
campisi_reconciliation_check = _portfolio.campisi_reconciliation_check
campisi_reconciliation_check_json = _portfolio.campisi_reconciliation_check_json
cell_returns_from_curves = _portfolio.cell_returns_from_curves
cell_returns_from_curves_json = _portfolio.cell_returns_from_curves_json
cell_returns_from_reference = _portfolio.cell_returns_from_reference
cell_returns_from_reference_json = _portfolio.cell_returns_from_reference_json
excess_returns = _portfolio.excess_returns
excess_returns_json = _portfolio.excess_returns_json
factor_brinson_attribution = _portfolio.factor_brinson_attribution
factor_brinson_attribution_json = _portfolio.factor_brinson_attribution_json
grid_attribution = _portfolio.grid_attribution
grid_attribution_json = _portfolio.grid_attribution_json
grid_carino_link = _portfolio.grid_carino_link
grid_carino_link_json = _portfolio.grid_carino_link_json
twrr_modified_dietz = _portfolio.twrr_modified_dietz
twrr_linked = _portfolio.twrr_linked
twrr_linked_json = _portfolio.twrr_linked_json
mwr_xirr = _portfolio.mwr_xirr

# Factor-model decomposition results
FactorContribution = _portfolio.FactorContribution
PositionFactorContribution = _portfolio.PositionFactorContribution
PositionResidualContribution = _portfolio.PositionResidualContribution
RiskDecomposition = _portfolio.RiskDecomposition
FactorRiskDecomposition = _portfolio.FactorRiskDecomposition
SensitivityMatrix = _portfolio.SensitivityMatrix
FactorPnlProfile = _portfolio.FactorPnlProfile
compute_factor_sensitivities = _portfolio.compute_factor_sensitivities
compute_pnl_profiles = _portfolio.compute_pnl_profiles
decompose_factor_risk = _portfolio.decompose_factor_risk
PositionVarContribution = _portfolio.PositionVarContribution
PositionEsContribution = _portfolio.PositionEsContribution
PositionRiskDecomposition = _portfolio.PositionRiskDecomposition
PositionBudgetEntry = _portfolio.PositionBudgetEntry
RiskBudgetResult = _portfolio.RiskBudgetResult
FactorContributionDelta = _portfolio.FactorContributionDelta
WhatIfResult = _portfolio.WhatIfResult
StressResult = _portfolio.StressResult
StressPositionEntry = _portfolio.StressPositionEntry
TailScenarioBreakdown = _portfolio.TailScenarioBreakdown
StressAttribution = _portfolio.StressAttribution
PositionAssignment = _portfolio.PositionAssignment
UnmatchedEntry = _portfolio.UnmatchedEntry
FactorAssignmentReport = _portfolio.FactorAssignmentReport
LevelVolContribution = _portfolio.LevelVolContribution
PositionVolContribution = _portfolio.PositionVolContribution
CreditVolReport = _portfolio.CreditVolReport
VolHorizon = _portfolio.VolHorizon
DecompositionConfig = _portfolio.DecompositionConfig
factor_stress = _portfolio.factor_stress
position_what_if = _portfolio.position_what_if
build_stress_attribution = _portfolio.build_stress_attribution
build_credit_vol_report = _portfolio.build_credit_vol_report
position_component_var = _portfolio.position_component_var

# Portfolio optimization specifications and results
WeightingScheme = _portfolio.WeightingScheme
MissingMetricPolicy = _portfolio.MissingMetricPolicy
Inequality = _portfolio.Inequality
OptimizationStatus = _portfolio.OptimizationStatus
TradeDirection = _portfolio.TradeDirection
TradeType = _portfolio.TradeType
PerPositionMetric = _portfolio.PerPositionMetric
PositionFilter = _portfolio.PositionFilter
MetricExpr = _portfolio.MetricExpr
Objective = _portfolio.Objective
Constraint = _portfolio.Constraint
TradeSpec = _portfolio.TradeSpec
PortfolioOptimizationSpec = _portfolio.PortfolioOptimizationSpec
PortfolioOptimizationResult = _portfolio.PortfolioOptimizationResult
CandidatePosition = _portfolio.CandidatePosition
TradeUniverse = _portfolio.TradeUniverse
schema = _portfolio.schema

# `schema` is a real submodule, so `import finstack_quant.portfolio.schema`
# must work as well as attribute access.
if "finstack_quant.portfolio.schema" not in _sys.modules:
    _sys.modules["finstack_quant.portfolio.schema"] = schema

__all__: list[str] = [
    "BrinsonPeriodResult",
    "CandidatePosition",
    "CarinoLinkedAttribution",
    "Constraint",
    "ContractLimitExceededError",
    "ContractValidationError",
    "CreditVolReport",
    "DecompositionConfig",
    "DurationCellTable",
    "ExcessReturnResult",
    "FactorAssignmentReport",
    "FactorBrinsonResult",
    "FactorContribution",
    "FactorContributionDelta",
    "FactorPnlProfile",
    "FactorRiskDecomposition",
    "FiAttributionResult",
    "FiCarinoLinkedResult",
    "FiReconciliationReport",
    "FinstackError",
    "FxError",
    "GridAttributionResult",
    "GridCarinoLinkedResult",
    "Inequality",
    "InstrumentArtifactCache",
    "LevelVolContribution",
    "LinkedReturn",
    "MalformedContractSchemaError",
    "MaterializationReport",
    "MetricExpr",
    "MissingContractVersionError",
    "MissingMetricPolicy",
    "Objective",
    "OptimizationError",
    "OptimizationStatus",
    "PerPositionMetric",
    "Portfolio",
    "PortfolioAttribution",
    "PortfolioCashflows",
    "PortfolioError",
    "PortfolioMetrics",
    "PortfolioOptimizationResult",
    "PortfolioOptimizationSpec",
    "PortfolioResult",
    "PortfolioValuation",
    "PositionAssignment",
    "PositionBudgetEntry",
    "PositionEsContribution",
    "PositionFactorContribution",
    "PositionFilter",
    "PositionResidualContribution",
    "PositionRiskDecomposition",
    "PositionVarContribution",
    "PositionVolContribution",
    "ReplayResult",
    "RiskBudgetResult",
    "RiskDecomposition",
    "ScenarioPnl",
    "ScenarioPnlBatchItem",
    "SensitivityMatrix",
    "StressAttribution",
    "StressPositionEntry",
    "StressResult",
    "TailScenarioBreakdown",
    "TradeDirection",
    "TradeSpec",
    "TradeType",
    "TradeUniverse",
    "UnmatchedEntry",
    "UnsupportedContractVersionError",
    "ValuationError",
    "VolHorizon",
    "WeightAllocationResult",
    "WeightingScheme",
    "WhatIfResult",
    "aggregate_full_cashflows",
    "aggregate_metrics",
    "aggregate_metrics_json",
    "allocate_weights",
    "allocate_weights_json",
    "almgren_chriss_impact",
    "amihud_illiquidity",
    "apply_scenario_and_revalue",
    "attribute_portfolio_pnl",
    "brinson_fachler",
    "brinson_fachler_json",
    "build_credit_vol_report",
    "build_portfolio_from_spec",
    "build_stress_attribution",
    "campisi_attribution",
    "campisi_attribution_json",
    "campisi_carino_link",
    "campisi_carino_link_from_snapshots",
    "campisi_carino_link_from_snapshots_json",
    "campisi_carino_link_json",
    "campisi_reconciliation_check",
    "campisi_reconciliation_check_json",
    "carino_link",
    "carino_link_json",
    "cell_returns_from_curves",
    "cell_returns_from_curves_json",
    "cell_returns_from_reference",
    "cell_returns_from_reference_json",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "days_to_liquidate",
    "decompose_factor_risk",
    "evaluate_risk_budget",
    "excess_returns",
    "excess_returns_json",
    "factor_brinson_attribution",
    "factor_brinson_attribution_json",
    "factor_stress",
    "grid_attribution",
    "grid_attribution_json",
    "grid_carino_link",
    "grid_carino_link_json",
    "historical_var_decomposition",
    "kyle_lambda",
    "liquidity_tier",
    "lvar_bangia",
    "mwr_xirr",
    "net_in_currency_by_date",
    "optimize_portfolio",
    "parametric_es_decomposition",
    "parametric_var_decomposition",
    "parse_portfolio_spec",
    "portfolio_result_get_metric",
    "portfolio_result_total_value",
    "position_component_var",
    "position_what_if",
    "replay_portfolio",
    "replay_portfolio_json",
    "roll_effective_spread",
    "scenario_pnl",
    "scenario_pnl_batch",
    "scenario_pnl_batch_json",
    "schema",
    "twrr_linked",
    "twrr_linked_json",
    "twrr_modified_dietz",
    "validate_allocation_json",
    "value_portfolio",
]
