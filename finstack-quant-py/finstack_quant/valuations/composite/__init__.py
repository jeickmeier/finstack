"""Generic cross-asset composite instruments with frozen resolved quantities.

Examples:
--------
>>> from finstack_quant.valuations.composite import RebalanceRule, WeightingMethod
>>> WeightingMethod.fixed_quantity().to_json()
'{"kind":"fixed_quantity"}'
>>> RebalanceRule.manual().to_json()
'{"kind":"manual"}'

"""

from finstack_quant.finstack_quant import valuations as _valuations

_composite = _valuations.composite

CompositeExposureReport = _composite.CompositeExposureReport
CompositeHistoryEngine = _composite.CompositeHistoryEngine
CompositeHistoryResult = _composite.CompositeHistoryResult
CompositeInstrument = _composite.CompositeInstrument
CompositeLegSpec = _composite.CompositeLegSpec
CompositeRebalanceResult = _composite.CompositeRebalanceResult
CompositeSpec = _composite.CompositeSpec
CompositeState = _composite.CompositeState
RebalanceRule = _composite.RebalanceRule
WeightingMethod = _composite.WeightingMethod

__all__: list[str] = [
    "CompositeExposureReport",
    "CompositeHistoryEngine",
    "CompositeHistoryResult",
    "CompositeInstrument",
    "CompositeLegSpec",
    "CompositeRebalanceResult",
    "CompositeSpec",
    "CompositeState",
    "RebalanceRule",
    "WeightingMethod",
]
