"""Product-independent volatility models, evaluators, and fitting tools.

Examples:
--------
>>> from finstack_quant.models.volatility import SabrParameters
>>> SabrParameters.rates_default().beta
0.5
"""

from finstack_quant.finstack_quant.models import volatility as _volatility

SabrCalibrator = _volatility.SabrCalibrator
SabrModel = _volatility.SabrModel
SabrParameters = _volatility.SabrParameters
SabrSmile = _volatility.SabrSmile
check_butterfly_grid = _volatility.check_butterfly_grid
check_calendar_spread_grid = _volatility.check_calendar_spread_grid
check_local_vol_density_grid = _volatility.check_local_vol_density_grid
check_surface_grid = _volatility.check_surface_grid
delta_to_strike = _volatility.delta_to_strike
get_cube_normal_vol = _volatility.get_cube_normal_vol
get_cube_normal_vol_clamped = _volatility.get_cube_normal_vol_clamped
get_cube_vol = _volatility.get_cube_vol
get_cube_vol_clamped = _volatility.get_cube_vol_clamped
get_fx_delta_pillar_vols = _volatility.get_fx_delta_pillar_vols
get_fx_delta_vol = _volatility.get_fx_delta_vol
get_surface_vol = _volatility.get_surface_vol
get_surface_vol_clamped = _volatility.get_surface_vol_clamped
materialize_cube_expiry_slice = _volatility.materialize_cube_expiry_slice
materialize_cube_expiry_slice_normal = _volatility.materialize_cube_expiry_slice_normal
materialize_cube_tenor_slice = _volatility.materialize_cube_tenor_slice
materialize_cube_tenor_slice_normal = _volatility.materialize_cube_tenor_slice_normal
materialize_fx_delta_surface = _volatility.materialize_fx_delta_surface
strike_to_delta = _volatility.strike_to_delta

__all__ = [
    "SabrCalibrator",
    "SabrModel",
    "SabrParameters",
    "SabrSmile",
    "check_butterfly_grid",
    "check_calendar_spread_grid",
    "check_local_vol_density_grid",
    "check_surface_grid",
    "delta_to_strike",
    "get_cube_normal_vol",
    "get_cube_normal_vol_clamped",
    "get_cube_vol",
    "get_cube_vol_clamped",
    "get_fx_delta_pillar_vols",
    "get_fx_delta_vol",
    "get_surface_vol",
    "get_surface_vol_clamped",
    "materialize_cube_expiry_slice",
    "materialize_cube_expiry_slice_normal",
    "materialize_cube_tenor_slice",
    "materialize_cube_tenor_slice_normal",
    "materialize_fx_delta_surface",
    "strike_to_delta",
]
