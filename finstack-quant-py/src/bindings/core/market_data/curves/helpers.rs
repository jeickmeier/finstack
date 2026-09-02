//! Shared curve binding helpers.

use finstack_quant_core::dates::DayCount;
use finstack_quant_core::market_data::surfaces::{
    VolInterpolationMode, VolQuoteType, VolSurfaceAxis,
};
use finstack_quant_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::prelude::*;

/// Parse a DayCount from a Python string like `"act_365f"`, `"act_360"`, etc.
pub(crate) fn parse_day_count(s: &str) -> PyResult<DayCount> {
    s.parse::<DayCount>()
        .map_err(|e| crate::errors::value_error(format!("Invalid day_count {s:?}: {e}")))
}

/// Parse an [`InterpStyle`] from a Python string.
pub(super) fn parse_interp_style(s: &str) -> PyResult<InterpStyle> {
    s.parse::<InterpStyle>()
        .map_err(|e| crate::errors::value_error(format!("Invalid interp style {s:?}: {e}")))
}

/// Parse an [`ExtrapolationPolicy`] from a Python string.
pub(super) fn parse_extrapolation(s: &str) -> PyResult<ExtrapolationPolicy> {
    s.parse::<ExtrapolationPolicy>()
        .map_err(|e| crate::errors::value_error(format!("Invalid extrapolation {s:?}: {e}")))
}

/// Parse a [`VolSurfaceAxis`] from its serde name (`"strike"` or `"tenor"`).
pub(super) fn parse_vol_surface_axis(s: &str) -> PyResult<VolSurfaceAxis> {
    finstack_quant_core::wire::serde_parse(s).map_err(crate::errors::core_to_py)
}

/// Parse a [`VolQuoteType`] from a Python string.
pub(super) fn parse_vol_quote_type(s: &str) -> PyResult<VolQuoteType> {
    s.parse::<VolQuoteType>()
        .map_err(crate::errors::value_error)
}

/// Parse a [`VolInterpolationMode`] from its serde name (`"vol"` or `"total_variance"`).
pub(super) fn parse_vol_interpolation_mode(s: &str) -> PyResult<VolInterpolationMode> {
    finstack_quant_core::wire::serde_parse(s).map_err(crate::errors::core_to_py)
}

/// Serde name of a [`VolInterpolationMode`] (`"vol"` or `"total_variance"`).
pub(super) fn vol_interpolation_mode_name(mode: VolInterpolationMode) -> PyResult<String> {
    finstack_quant_core::wire::serde_label(&mode).map_err(crate::errors::core_to_py)
}
