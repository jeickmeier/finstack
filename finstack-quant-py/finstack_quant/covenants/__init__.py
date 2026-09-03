"""Typed covenant definitions, engine evaluation, templates and forecasting.

Examples:
--------
>>> from finstack_quant.covenants import CovenantEngine, cov_lite
>>> engine = CovenantEngine.from_specs(cov_lite(7.0, 4.5))
>>> reports = engine.evaluate({"total_leverage": 5.0, "senior_leverage": 3.0}, "2026-03-31")
>>> reports["max_total_leverage"].passed
True
"""

from finstack_quant.finstack_quant import covenants as _covenants

Covenant = _covenants.Covenant
CovenantBreach = _covenants.CovenantBreach
CovenantConsequence = _covenants.CovenantConsequence
CovenantEngine = _covenants.CovenantEngine
CovenantForecast = _covenants.CovenantForecast
CovenantForecastConfig = _covenants.CovenantForecastConfig
CovenantReport = _covenants.CovenantReport
CovenantSpec = _covenants.CovenantSpec
CovenantType = _covenants.CovenantType
CovenantWaiver = _covenants.CovenantWaiver
FutureBreach = _covenants.FutureBreach
SpringingCondition = _covenants.SpringingCondition
ThresholdSchedule = _covenants.ThresholdSchedule
breaches_to_dataframe = _covenants.breaches_to_dataframe
cov_lite = _covenants.cov_lite
cov_lite_json = _covenants.cov_lite_json
evaluate_engine = _covenants.evaluate_engine
forecast_breaches = _covenants.forecast_breaches
forecast_covenant = _covenants.forecast_covenant
lbo_standard = _covenants.lbo_standard
lbo_standard_json = _covenants.lbo_standard_json
project_finance = _covenants.project_finance
project_finance_json = _covenants.project_finance_json
real_estate = _covenants.real_estate
real_estate_json = _covenants.real_estate_json
reports_to_dataframe = _covenants.reports_to_dataframe
validate_covenant_engine_json = _covenants.validate_covenant_engine_json
validate_covenant_report_json = _covenants.validate_covenant_report_json
validate_covenant_spec_json = _covenants.validate_covenant_spec_json

__all__ = [
    "Covenant",
    "CovenantBreach",
    "CovenantConsequence",
    "CovenantEngine",
    "CovenantForecast",
    "CovenantForecastConfig",
    "CovenantReport",
    "CovenantSpec",
    "CovenantType",
    "CovenantWaiver",
    "FutureBreach",
    "SpringingCondition",
    "ThresholdSchedule",
    "breaches_to_dataframe",
    "cov_lite",
    "cov_lite_json",
    "evaluate_engine",
    "forecast_breaches",
    "forecast_covenant",
    "lbo_standard",
    "lbo_standard_json",
    "project_finance",
    "project_finance_json",
    "real_estate",
    "real_estate_json",
    "reports_to_dataframe",
    "validate_covenant_engine_json",
    "validate_covenant_report_json",
    "validate_covenant_spec_json",
]
