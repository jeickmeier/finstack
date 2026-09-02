//! Volatility surface bindings.

use finstack_quant_core::market_data::surfaces::{
    FxDeltaVolSurface, SabrParameterData, VolCube, VolGridOpts, VolInterpolationMode, VolSurface,
};
use finstack_quant_core::market_data::term_structures::{PriceCurve, PriceCurveKind};
use pyo3::types::PyDict;

use std::sync::Arc;

use pyo3::prelude::*;

use super::helpers::{
    parse_day_count, parse_extrapolation, parse_interp_style, parse_vol_interpolation_mode,
    parse_vol_quote_type, parse_vol_surface_axis,
};
use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::core_to_py;

/// Two-dimensional implied volatility surface on an expiry x strike grid.
///
/// Wraps [`VolSurface`] from `finstack-quant-core`.
#[pyclass(
    name = "VolSurface",
    module = "finstack_quant.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVolSurface {
    /// Shared Rust surface.
    pub(crate) inner: Arc<VolSurface>,
}

impl PyVolSurface {
    /// Build from an existing `Arc<VolSurface>`.
    pub(crate) fn from_inner(inner: Arc<VolSurface>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolSurface {
    /// Construct a vol surface from row-major grid data.
    #[new]
    #[pyo3(signature = (id, expiries, strikes, vols_row_major, secondary_axis="strike", interpolation_mode="vol", quote_type="black_lognormal"))]
    fn new(
        id: &str,
        expiries: Vec<f64>,
        strikes: Vec<f64>,
        vols_row_major: Vec<f64>,
        secondary_axis: &str,
        interpolation_mode: &str,
        quote_type: &str,
    ) -> PyResult<Self> {
        let axis = parse_vol_surface_axis(secondary_axis)?;
        let mode = parse_vol_interpolation_mode(interpolation_mode)?;
        let quote = parse_vol_quote_type(quote_type)?;
        let surface = VolSurface::from_grid_opts(
            id,
            &expiries,
            &strikes,
            &vols_row_major,
            VolGridOpts {
                secondary_axis: axis,
                quote_type: quote,
                interpolation_mode: mode,
            },
        )
        .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(surface),
        })
    }

    /// Surface identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Expiry axis in years.
    #[getter]
    fn expiries(&self) -> Vec<f64> {
        self.inner.expiries().to_vec()
    }

    /// Strike axis of the stored volatility grid.
    #[getter]
    fn strikes(&self) -> Vec<f64> {
        self.inner.strikes().to_vec()
    }

    /// Secondary-axis semantic meaning.
    #[getter]
    fn secondary_axis(&self) -> String {
        self.inner.secondary_axis().to_string()
    }

    /// Quoting convention of the stored volatilities.
    #[getter]
    fn quote_type(&self) -> String {
        self.inner.quote_type().to_string()
    }

    /// Interpolation contract used between grid points.
    #[getter]
    fn interpolation_mode(&self) -> String {
        match self.inner.interpolation_mode() {
            VolInterpolationMode::Vol => "vol".to_string(),
            VolInterpolationMode::TotalVariance => "total_variance".to_string(),
        }
    }

    /// Surface grid shape as `(n_expiries, n_strikes)`.
    #[getter]
    fn grid_shape(&self) -> (usize, usize) {
        self.inner.grid_shape()
    }

    fn __repr__(&self) -> String {
        format!("VolSurface(id={:?})", self.inner.id().as_str())
    }
}

/// Delta-quoted FX volatility surface (ATM, 25-delta RR/BF, optional 10-delta wings).
///
/// Uses forward delta (premium-unadjusted). Converts to strikes via Garman-Kohlhagen
/// for strike-axis pricing. See Wystup (2006) and Clark (2011).
#[pyclass(
    name = "FxDeltaVolSurface",
    module = "finstack_quant.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFxDeltaVolSurface {
    /// Shared Rust surface.
    pub(crate) inner: Arc<FxDeltaVolSurface>,
}

impl PyFxDeltaVolSurface {
    /// Build from an existing `Arc<FxDeltaVolSurface>`.
    pub(crate) fn from_inner(inner: Arc<FxDeltaVolSurface>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxDeltaVolSurface {
    /// Construct an FX delta-quoted vol surface with 25-delta wings.
    ///
    /// Optional `rr_10d` and `bf_10d` add 10-delta wings for richer smile
    /// interpolation in the wings.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique surface identifier.
    /// expiries : list[float]
    ///     Strictly increasing positive expiry times in years.
    /// atm_vols : list[float]
    ///     ATM delta-neutral straddle vols per expiry (must be positive).
    /// rr_25d : list[float]
    ///     25-delta risk reversal per expiry (call vol - put vol).
    /// bf_25d : list[float]
    ///     25-delta butterfly per expiry (wing average - ATM).
    /// rr_10d : list[float], optional
    ///     10-delta risk reversal per expiry. If provided, `bf_10d` is required.
    /// bf_10d : list[float], optional
    ///     10-delta butterfly per expiry. If provided, `rr_10d` is required.
    #[new]
    #[pyo3(signature = (id, expiries, atm_vols, rr_25d, bf_25d, rr_10d=None, bf_10d=None))]
    fn new(
        id: &str,
        expiries: Vec<f64>,
        atm_vols: Vec<f64>,
        rr_25d: Vec<f64>,
        bf_25d: Vec<f64>,
        rr_10d: Option<Vec<f64>>,
        bf_10d: Option<Vec<f64>>,
    ) -> PyResult<Self> {
        let surface = match (rr_10d, bf_10d) {
            (Some(rr), Some(bf)) => {
                FxDeltaVolSurface::with_10d(id, expiries, atm_vols, rr_25d, bf_25d, rr, bf)
                    .map_err(core_to_py)?
            }
            (None, None) => FxDeltaVolSurface::new(id, expiries, atm_vols, rr_25d, bf_25d)
                .map_err(core_to_py)?,
            _ => {
                return Err(crate::errors::value_error(
                    "rr_10d and bf_10d must both be provided or both omitted",
                ));
            }
        };
        Ok(Self {
            inner: Arc::new(surface),
        })
    }

    /// Surface identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Expiry axis in years.
    #[getter]
    fn expiries(&self) -> Vec<f64> {
        self.inner.expiries().to_vec()
    }

    /// Number of expiry pillars.
    #[getter]
    fn num_expiries(&self) -> usize {
        self.inner.num_expiries()
    }

    fn __repr__(&self) -> String {
        format!(
            "FxDeltaVolSurface(id={:?}, num_expiries={})",
            self.inner.id().as_str(),
            self.inner.num_expiries()
        )
    }
}

/// SABR volatility cube on an expiry x tenor grid.
///
/// Wraps [`VolCube`] from `finstack-quant-core`.
#[pyclass(
    name = "VolCube",
    module = "finstack_quant.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVolCube {
    /// Shared Rust cube.
    pub(crate) inner: Arc<VolCube>,
}

impl PyVolCube {
    /// Build from an existing `Arc<VolCube>`.
    pub(crate) fn from_inner(inner: Arc<VolCube>) -> Self {
        Self { inner }
    }
}

/// Parse a Python dict to [`SabrParameterData`].
///
/// Required keys: `"alpha"`, `"beta"`, `"rho"`, `"nu"`.
/// Optional key: `"shift"`.
fn parse_sabr_dict(dict: &Bound<'_, PyDict>, idx: usize) -> PyResult<SabrParameterData> {
    let get = |key: &str| -> PyResult<f64> {
        dict.get_item(key)?
            .ok_or_else(|| {
                crate::errors::value_error(format!(
                    "params_row_major[{idx}]: missing required key {key:?}"
                ))
            })?
            .extract::<f64>()
    };

    let alpha = get("alpha")?;
    let beta = get("beta")?;
    let rho = get("rho")?;
    let nu = get("nu")?;

    let shift = dict
        .get_item("shift")?
        .map(|value| value.extract::<f64>())
        .transpose()?;
    SabrParameterData::new_with_shift(alpha, beta, rho, nu, shift).map_err(core_to_py)
}

#[pymethods]
impl PyVolCube {
    /// Construct a vol cube from row-major grid data.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique cube identifier.
    /// expiries : list[float]
    ///     Option expiry axis in years.
    /// tenors : list[float]
    ///     Underlying swap tenor axis in years.
    /// params_row_major : list[dict]
    ///     SABR parameter dicts with keys ``"alpha"``, ``"beta"``, ``"rho"``,
    ///     ``"nu"``, and optionally ``"shift"``.
    /// forwards_row_major : list[float]
    ///     Forward rates in row-major order.
    /// interpolation_mode : str, optional
    ///     Interpolation contract: ``"vol"`` or ``"total_variance"``
    ///     (default ``"vol"``).
    #[new]
    #[pyo3(signature = (id, expiries, tenors, params_row_major, forwards_row_major, interpolation_mode="vol"))]
    fn new(
        id: &str,
        expiries: Vec<f64>,
        tenors: Vec<f64>,
        params_row_major: Vec<Bound<'_, PyDict>>,
        forwards_row_major: Vec<f64>,
        interpolation_mode: &str,
    ) -> PyResult<Self> {
        let mode = parse_vol_interpolation_mode(interpolation_mode)?;

        let sabr_params: Vec<SabrParameterData> = params_row_major
            .iter()
            .enumerate()
            .map(|(i, d)| parse_sabr_dict(d, i))
            .collect::<PyResult<Vec<_>>>()?;

        let cube = VolCube::from_grid(id, &expiries, &tenors, &sabr_params, &forwards_row_major)
            .map_err(core_to_py)?
            .with_interpolation_mode(mode);

        Ok(Self {
            inner: Arc::new(cube),
        })
    }

    /// Cube identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Option expiry axis in years.
    #[getter]
    fn expiries(&self) -> Vec<f64> {
        self.inner.expiries().to_vec()
    }

    /// Underlying swap tenor axis in years.
    #[getter]
    fn tenors(&self) -> Vec<f64> {
        self.inner.tenors().to_vec()
    }

    /// Grid shape as `(n_expiries, n_tenors)`.
    #[getter]
    fn grid_shape(&self) -> (usize, usize) {
        self.inner.grid_shape()
    }

    /// Interpolation contract (``"vol"`` or ``"total_variance"``).
    #[getter]
    fn interpolation_mode(&self) -> &'static str {
        match self.inner.interpolation_mode() {
            VolInterpolationMode::Vol => "vol",
            VolInterpolationMode::TotalVariance => "total_variance",
        }
    }

    fn __repr__(&self) -> String {
        format!("VolCube(id={:?})", self.inner.id().as_str())
    }
}

/// Volatility index forward curve (e.g. VIX term structure).
///
/// Wraps a [`PriceCurve`] with [`PriceCurveKind::VolIndex`] from
/// `finstack-quant-core`.
#[pyclass(
    name = "VolatilityIndexCurve",
    module = "finstack_quant.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVolatilityIndexCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<PriceCurve>,
}

impl PyVolatilityIndexCurve {
    /// Build from an existing `Arc<PriceCurve>` stored as a vol-index curve.
    pub(crate) fn from_inner(inner: Arc<PriceCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityIndexCurve {
    /// Construct a volatility index curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"VIX"``).
    /// base_date : datetime.date
    ///     Valuation date.
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, forward_level)`` pairs.
    /// extrapolation : str, optional
    ///     Extrapolation policy (default ``"flat_zero"``).
    /// interp : str, optional
    ///     Interpolation style (default ``"linear"``).
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_365f"``).
    #[new]
    #[pyo3(signature = (id, base_date, knots, extrapolation="flat_zero", interp="linear", day_count="act_365f"))]
    fn new(
        id: &str,
        base_date: &Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        extrapolation: &str,
        interp: &str,
        day_count: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let extrap = parse_extrapolation(extrapolation)?;
        let style = parse_interp_style(interp)?;
        let day_count = parse_day_count(day_count)?;

        let curve = PriceCurve::builder(id)
            .kind(PriceCurveKind::VolIndex)
            .base_date(base)
            .day_count(day_count)
            .knots(knots)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Forward volatility index level at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn forward_level(&self, t: f64) -> f64 {
        self.inner.price(t)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("VolatilityIndexCurve(id={:?})", self.inner.id().as_str())
    }
}
