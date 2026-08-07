//! Python bindings for `finstack_quant_core::credit::recovery_waterfall`.

use crate::bindings::pandas_utils::serde_rows_to_dataframe_with_schema;
use crate::bindings::pandas_utils::ColumnSchema;
use crate::errors::core_to_py;
use finstack_quant_core::credit::recovery_waterfall::{
    self as waterfall, RecoveryAllocation, RecoveryClaim, RecoveryWaterfallResult,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Column schema of `PyRecoveryWaterfallResult::to_dataframe`, kept so a
/// claim-free waterfall still exports a frame with the documented columns.
const ALLOCATION_COLUMNS: &[ColumnSchema<'static>] = &[
    ("id", "str"),
    ("seniority", "str"),
    ("priority", "int64"),
    ("total_claim", "float64"),
    ("collateral_recovery", "float64"),
    ("general_recovery", "float64"),
    ("total_recovery", "float64"),
    ("recovery_rate", "float64"),
    ("deficiency", "float64"),
];

/// A claim participating in an absolute-priority recovery waterfall.
#[pyclass(
    name = "RecoveryClaim",
    module = "finstack_quant.core.credit.recovery_waterfall",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecoveryClaim {
    pub(crate) inner: RecoveryClaim,
}

#[pymethods]
impl PyRecoveryClaim {
    /// Create a recovery claim.
    #[new]
    #[pyo3(signature = (id, seniority, priority, principal, accrued=0.0, penalties=0.0, collateral=None))]
    fn new(
        id: String,
        seniority: String,
        priority: u32,
        principal: f64,
        accrued: f64,
        penalties: f64,
        collateral: Option<(f64, f64)>,
    ) -> Self {
        let (collateral_value, collateral_haircut) = match collateral {
            Some((value, haircut)) => (Some(value), haircut),
            None => (None, 0.0),
        };
        Self {
            inner: RecoveryClaim {
                id,
                seniority,
                priority,
                principal,
                accrued,
                penalties,
                collateral_value,
                collateral_haircut,
            },
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn seniority(&self) -> String {
        self.inner.seniority.clone()
    }

    #[getter]
    fn priority(&self) -> u32 {
        self.inner.priority
    }

    #[getter]
    fn principal(&self) -> f64 {
        self.inner.principal
    }

    #[getter]
    fn accrued(&self) -> f64 {
        self.inner.accrued
    }

    #[getter]
    fn penalties(&self) -> f64 {
        self.inner.penalties
    }

    #[getter]
    fn collateral_value(&self) -> Option<f64> {
        self.inner.collateral_value
    }

    #[getter]
    fn collateral_haircut(&self) -> f64 {
        self.inner.collateral_haircut
    }

    #[getter]
    fn total_claim(&self) -> f64 {
        self.inner.total_claim()
    }

    fn __repr__(&self) -> String {
        format!(
            "RecoveryClaim(id='{}', seniority='{}', priority={}, total_claim={})",
            self.inner.id,
            self.inner.seniority,
            self.inner.priority,
            self.inner.total_claim()
        )
    }
}

/// Recovery allocated to one claim.
#[pyclass(
    name = "RecoveryAllocation",
    module = "finstack_quant.core.credit.recovery_waterfall",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecoveryAllocation {
    inner: RecoveryAllocation,
}

#[pymethods]
impl PyRecoveryAllocation {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn seniority(&self) -> String {
        self.inner.seniority.clone()
    }

    #[getter]
    fn priority(&self) -> u32 {
        self.inner.priority
    }

    #[getter]
    fn total_claim(&self) -> f64 {
        self.inner.total_claim
    }

    #[getter]
    fn collateral_recovery(&self) -> f64 {
        self.inner.collateral_recovery
    }

    #[getter]
    fn general_recovery(&self) -> f64 {
        self.inner.general_recovery
    }

    #[getter]
    fn total_recovery(&self) -> f64 {
        self.inner.total_recovery
    }

    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    #[getter]
    fn deficiency(&self) -> f64 {
        self.inner.deficiency
    }
}

/// Result of allocating a distributable estate across claims.
#[pyclass(
    name = "RecoveryWaterfallResult",
    module = "finstack_quant.core.credit.recovery_waterfall",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecoveryWaterfallResult {
    inner: RecoveryWaterfallResult,
}

#[pymethods]
impl PyRecoveryWaterfallResult {
    #[getter]
    fn total_distributed(&self) -> f64 {
        self.inner.total_distributed
    }

    #[getter]
    fn undistributed_estate(&self) -> f64 {
        self.inner.undistributed_estate
    }

    #[getter]
    fn apr_satisfied(&self) -> bool {
        self.inner.apr_satisfied
    }

    #[getter]
    fn allocations(&self) -> Vec<PyRecoveryAllocation> {
        self.inner
            .allocations
            .iter()
            .cloned()
            .map(|inner| PyRecoveryAllocation { inner })
            .collect()
    }

    /// Export the per-claim allocations as a pandas ``DataFrame``.
    ///
    /// Columns: ``id``, ``seniority``, ``priority``, ``total_claim``,
    /// ``collateral_recovery``, ``general_recovery``, ``total_recovery``,
    /// ``recovery_rate``, ``deficiency``.
    ///
    /// One row per claim — the natural grain of a waterfall. Rows keep the
    /// Rust ordering (ascending ``priority``, then original claim order), so
    /// repeated exports of the same result are byte-identical. The
    /// estate-level fields (:attr:`total_distributed`,
    /// :attr:`undistributed_estate`, :attr:`apr_satisfied`) are deliberately
    /// not repeated on every row; read them from the result object.
    ///
    /// A waterfall with no claims yields a zero-row frame that still carries
    /// the columns above.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // `RecoveryAllocation` derives Serialize and is already a flat record
        // of `f64`/`String`/`u32` fields whose serde names match the columns
        // above, so the rows go straight through the serde helper.
        serde_rows_to_dataframe_with_schema(py, &self.inner.allocations, ALLOCATION_COLUMNS)
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

/// Allocate an estate, inclusive of collateral, across recovery claims.
#[pyfunction]
#[pyo3(text_signature = "(estate_value, claims)")]
fn allocate_recovery(
    py: Python<'_>,
    estate_value: f64,
    claims: Vec<PyRecoveryClaim>,
) -> PyResult<PyRecoveryWaterfallResult> {
    let claims = claims
        .into_iter()
        .map(|claim| claim.inner)
        .collect::<Vec<_>>();
    py.detach(|| waterfall::allocate_recovery(estate_value, &claims))
        .map(|inner| PyRecoveryWaterfallResult { inner })
        .map_err(core_to_py)
}

/// Build the `finstack_quant.core.credit.recovery_waterfall` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "recovery_waterfall")?;
    m.setattr(
        "__doc__",
        "Absolute-priority recovery allocation with estate-inclusive collateral.",
    )?;

    m.add_class::<PyRecoveryClaim>()?;
    m.add_class::<PyRecoveryAllocation>()?;
    m.add_class::<PyRecoveryWaterfallResult>()?;
    m.add_function(wrap_pyfunction!(allocate_recovery, &m)?)?;

    let all = PyList::new(
        py,
        [
            "RecoveryClaim",
            "RecoveryAllocation",
            "RecoveryWaterfallResult",
            "allocate_recovery",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "recovery_waterfall",
        "finstack_quant.core.credit",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
