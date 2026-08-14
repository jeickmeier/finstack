//! Python bindings for `finstack_quant_core::credit::pd` (calibration subset).

use finstack_quant_core::credit::pd::{
    central_tendency as core_central_tendency, pit_to_ttc as core_pit_to_ttc,
    ttc_to_pit as core_ttc_to_pit, MasterScale, MasterScaleGrade, PdCycleParams,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

use crate::bindings::pandas_utils::serde_object_to_single_row_dataframe_with_schema;
use crate::errors::{core_to_py, pd_calibration_to_py};

// PiT / TtC conversion

/// Convert a Point-in-Time PD to a Through-the-Cycle PD.
///
/// Uses the Merton-Vasicek single-factor model (Basel II IRB):
///
///   PD_TtC = Phi( Phi^{-1}(PD_PiT) * sqrt(1 - rho) + sqrt(rho) * z )
///
/// Arguments:
///     pit_pd: Point-in-Time PD in (0, 1).
///     asset_correlation: Asset correlation rho in (0, 1). Basel uses 0.12 - 0.24 for corporates.
///     cycle_index: Systematic risk factor z. 0 = average, < 0 = downturn, > 0 = benign.
#[pyfunction]
#[pyo3(text_signature = "(pit_pd, asset_correlation, cycle_index)")]
fn pit_to_ttc(pit_pd: f64, asset_correlation: f64, cycle_index: f64) -> PyResult<f64> {
    let params = PdCycleParams {
        asset_correlation,
        cycle_index,
    };
    core_pit_to_ttc(pit_pd, &params).map_err(pd_calibration_to_py)
}

/// Convert a Through-the-Cycle PD to a Point-in-Time PD.
///
/// Uses the Merton-Vasicek single-factor model (Basel II IRB):
///
///   PD_PiT = Phi( (Phi^{-1}(PD_TtC) - sqrt(rho) * z) / sqrt(1 - rho) )
///
/// Arguments:
///     ttc_pd: Through-the-Cycle PD in (0, 1).
///     asset_correlation: Asset correlation rho in (0, 1).
///     cycle_index: Systematic risk factor z. 0 = average, < 0 = downturn, > 0 = benign.
#[pyfunction]
#[pyo3(text_signature = "(ttc_pd, asset_correlation, cycle_index)")]
fn ttc_to_pit(ttc_pd: f64, asset_correlation: f64, cycle_index: f64) -> PyResult<f64> {
    let params = PdCycleParams {
        asset_correlation,
        cycle_index,
    };
    core_ttc_to_pit(ttc_pd, &params).map_err(pd_calibration_to_py)
}

/// Calibrate a central tendency (long-run average PD) from annual default rates
/// using the arithmetic mean (the standard regulatory TtC approach per
/// Basel IRB / EBA GL/2017/16).
///
/// Zero-default years are valid observations and are included in the average.
///
/// Returns the arithmetic mean in [0, 1].
#[pyfunction]
#[pyo3(text_signature = "(annual_default_rates)")]
fn central_tendency(annual_default_rates: Vec<f64>) -> PyResult<f64> {
    core_central_tendency(&annual_default_rates).map_err(pd_calibration_to_py)
}

// Master scale

/// One PD band in a rating master scale.
#[pyclass(
    module = "finstack_quant.core.credit.pd",
    name = "MasterScaleGrade",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyMasterScaleGrade {
    inner: MasterScaleGrade,
}

#[pymethods]
impl PyMasterScaleGrade {
    #[new]
    #[pyo3(text_signature = "(label, upper_pd, central_pd)")]
    fn new(label: String, upper_pd: f64, central_pd: f64) -> Self {
        Self {
            inner: MasterScaleGrade {
                label,
                upper_pd,
                central_pd,
            },
        }
    }

    /// Grade label (e.g. ``"BBB"``).
    #[getter]
    fn label(&self) -> &str {
        &self.inner.label
    }

    /// Inclusive upper PD bound of the band.
    #[getter]
    fn upper_pd(&self) -> f64 {
        self.inner.upper_pd
    }

    /// Representative PD assigned to anything falling in the band.
    #[getter]
    fn central_pd(&self) -> f64 {
        self.inner.central_pd
    }

    fn __repr__(&self) -> String {
        format!(
            "MasterScaleGrade(label='{}', upper_pd={}, central_pd={})",
            self.inner.label, self.inner.upper_pd, self.inner.central_pd
        )
    }
}

/// Result of mapping a PD onto a master scale.
#[pyclass(
    module = "finstack_quant.core.credit.pd",
    name = "MasterScaleResult",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMasterScaleResult {
    grade: String,
    central_pd: f64,
    input_pd: f64,
    grade_index: usize,
}

#[pymethods]
impl PyMasterScaleResult {
    /// Label of the grade the PD mapped into.
    #[getter]
    fn grade(&self) -> &str {
        &self.grade
    }

    /// Central PD of the assigned grade — the notched value.
    #[getter]
    fn central_pd(&self) -> f64 {
        self.central_pd
    }

    /// The PD that was mapped, before notching.
    #[getter]
    fn input_pd(&self) -> f64 {
        self.input_pd
    }

    /// Zero-based index of the assigned grade in the scale.
    #[getter]
    fn grade_index(&self) -> usize {
        self.grade_index
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``grade``, ``grade_index``, ``input_pd``, ``central_pd``.
    ///
    /// One mapping is one flat record, so a one-row frame is the right shape:
    /// ``pd.concat([scale.map_pd(p).to_dataframe() for p in pds])`` builds a
    /// whole obligor-level grading table without reshaping.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // This wrapper flattens the core `MasterScaleResult` into owned fields
        // rather than holding an `inner`, so there is no Serialize impl to
        // reuse — the row literal is built from the stored fields directly.
        let row = serde_json::json!({
            "grade": self.grade,
            "grade_index": self.grade_index,
            "input_pd": self.input_pd,
            "central_pd": self.central_pd,
        });
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &["grade", "grade_index", "input_pd", "central_pd"],
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "MasterScaleResult(grade='{}', central_pd={}, input_pd={})",
            self.grade, self.central_pd, self.input_pd
        )
    }

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
    }
}

/// Ordered PD bands mapping a continuous PD onto discrete rating grades.
///
/// Bands must be strictly increasing in `upper_pd`, and each grade's
/// `central_pd` must fall inside its own band.
#[pyclass(
    module = "finstack_quant.core.credit.pd",
    name = "MasterScale",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMasterScale {
    inner: MasterScale,
}

#[pymethods]
impl PyMasterScale {
    #[new]
    #[pyo3(text_signature = "(grades)")]
    fn new(grades: Vec<PyMasterScaleGrade>) -> PyResult<Self> {
        let grades = grades.into_iter().map(|g| g.inner).collect();
        MasterScale::new(grades)
            .map(|inner| Self { inner })
            .map_err(pd_calibration_to_py)
    }

    /// Library PD-band assumptions using S&P-style labels.
    ///
    /// The labels resemble S&P notation as a reporting convention only;
    /// neither the boundaries nor the central PDs are agency-published
    /// statistics.
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn sp_assumptions() -> PyResult<Self> {
        MasterScale::sp_assumptions()
            .map(|inner| Self { inner })
            .map_err(core_to_py)
    }

    /// Library PD-band assumptions using Moody's-style labels.
    ///
    /// As with :meth:`sp_assumptions`, the labels are a reporting convention
    /// rather than an agency calibration.
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn moodys_assumptions() -> PyResult<Self> {
        MasterScale::moodys_assumptions()
            .map(|inner| Self { inner })
            .map_err(core_to_py)
    }

    /// Load a master scale by ID from the embedded credit registry.
    #[staticmethod]
    #[pyo3(text_signature = "(scale_id)")]
    fn from_registry_id(scale_id: &str) -> PyResult<Self> {
        MasterScale::from_registry_id(scale_id)
            .map(|inner| Self { inner })
            .map_err(core_to_py)
    }

    /// Map a PD onto its rating grade.
    ///
    /// Raises `ValueError` when `pd` is non-finite or outside [0, 1].
    #[pyo3(text_signature = "(pd)")]
    fn map_pd(&self, pd: f64) -> PyResult<PyMasterScaleResult> {
        let result = self.inner.map_pd(pd).map_err(pd_calibration_to_py)?;
        Ok(PyMasterScaleResult {
            grade: result.grade,
            central_pd: result.central_pd,
            input_pd: result.input_pd,
            grade_index: result.grade_index,
        })
    }

    /// Number of grades in the scale.
    #[getter]
    fn n_grades(&self) -> usize {
        self.inner.n_grades()
    }

    /// The scale's grades, in ascending PD order.
    #[getter]
    fn grades(&self) -> Vec<PyMasterScaleGrade> {
        self.inner
            .grades()
            .iter()
            .cloned()
            .map(|inner| PyMasterScaleGrade { inner })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.n_grades()
    }

    fn __repr__(&self) -> String {
        format!("MasterScale(n_grades={})", self.inner.n_grades())
    }
}

/// Build the `finstack_quant.core.credit.pd` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "pd")?;
    m.setattr(
        "__doc__",
        "Probability of default: PiT/TtC conversion (Merton-Vasicek), central-tendency calibration, and rating master scales.",
    )?;

    m.add_function(wrap_pyfunction!(pit_to_ttc, &m)?)?;
    m.add_function(wrap_pyfunction!(ttc_to_pit, &m)?)?;
    m.add_function(wrap_pyfunction!(central_tendency, &m)?)?;
    m.add_class::<PyMasterScale>()?;
    m.add_class::<PyMasterScaleGrade>()?;
    m.add_class::<PyMasterScaleResult>()?;

    let all = PyList::new(
        py,
        [
            "MasterScale",
            "MasterScaleGrade",
            "MasterScaleResult",
            "central_tendency",
            "pit_to_ttc",
            "ttc_to_pit",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "pd",
        "finstack_quant.core.credit",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;

    Ok(())
}
