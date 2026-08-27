"""Structural credit model bindings.

Mirrors ``finstack_quant_models::credit``.

Examples:
--------
>>> from finstack_quant.models.credit import MertonModel
>>> round(MertonModel(100.0, 0.25, 80.0, 0.05).default_probability(1.0), 6)
0.166629

"""

from finstack_quant.finstack_quant import models as _models

MertonModel = _models.credit.MertonModel
AssetDynamics = _models.credit.AssetDynamics
BarrierType = _models.credit.BarrierType
SimulatedPaths = _models.credit.SimulatedPaths
DynamicRecoverySpec = _models.credit.DynamicRecoverySpec
EndogenousHazardSpec = _models.credit.EndogenousHazardSpec
CreditState = _models.credit.CreditState
ToggleExerciseModel = _models.credit.ToggleExerciseModel

__all__ = [
    "AssetDynamics",
    "BarrierType",
    "CreditState",
    "DynamicRecoverySpec",
    "EndogenousHazardSpec",
    "MertonModel",
    "SimulatedPaths",
    "ToggleExerciseModel",
]
