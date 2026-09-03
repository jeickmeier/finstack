//! Supporting enum wrappers for scenario operations.
//!
//! Each wrapper exposes one classmethod per Rust variant, a constructor from
//! the canonical snake-case wire label, ``name`` / ``value`` accessors, and
//! value semantics (``==`` / ``hash``).

use finstack_quant_scenarios::spec::{Compounding, CurveKind, TenorMatchMode, TimeRollMode};
use pyo3::prelude::*;
use pyo3::types::PyType;

use super::helpers::{enum_to_label, label_to_enum};

macro_rules! scenario_enum {
    (
        $(#[$meta:meta])*
        $py_name:literal, $wrapper:ident, $inner:ident, $accepted:literal,
        { $( $(#[$vmeta:meta])* $method:ident => $variant:ident ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[pyclass(
            name = $py_name,
            module = "finstack_quant.scenarios",
            eq,
            hash,
            frozen,
            from_py_object
        )]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $wrapper {
            pub(crate) inner: $inner,
        }

        #[pymethods]
        impl $wrapper {
            #[new]
            fn new(label: &str) -> PyResult<Self> {
                Ok(Self {
                    inner: label_to_enum::<$inner>($py_name, label, $accepted)?,
                })
            }

            $(
                $(#[$vmeta])*
                #[classmethod]
                fn $method(_cls: &Bound<'_, PyType>) -> Self {
                    Self { inner: $inner::$variant }
                }
            )+

            /// Rust variant name, e.g. ``"Discount"``.
            #[getter]
            fn name(&self) -> String {
                format!("{:?}", self.inner)
            }

            /// Serialized wire label, e.g. ``"discount"`` or ``"par_cds"``.
            #[getter]
            fn value(&self) -> PyResult<String> {
                enum_to_label(&self.inner)
            }

            fn __repr__(&self) -> String {
                format!("{}.{:?}", $py_name, self.inner)
            }
        }
    };
}

scenario_enum!(
    /// Type of market curve targeted by a scenario operation.
    ///
    /// Construct from a wire label (``CurveKind("par_cds")``) or a classmethod
    /// (``CurveKind.par_cds()``); every ``OperationSpec`` constructor accepts
    /// either form.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.scenarios import CurveKind
    /// >>> CurveKind("par_cds") == CurveKind.par_cds()
    /// True
    /// >>> CurveKind.discount().value
    /// 'discount'
    "CurveKind", PyCurveKind, CurveKind, "discount, forward, par_cds, inflation, commodity",
    {
        /// Discount factor curve.
        discount => Discount,
        /// Forward rate curve.
        forward => Forward,
        /// Par CDS spread curve.
        par_cds => ParCDS,
        /// Inflation index curve.
        inflation => Inflation,
        /// Commodity forward (price) curve. Basis-point shocks on this kind are
        /// interpreted as percent of the forward, not additive bp.
        commodity => Commodity,
    }
);

scenario_enum!(
    /// Tenor-pillar alignment strategy for curve-node operations.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.scenarios import TenorMatchMode
    /// >>> TenorMatchMode("interpolate") == TenorMatchMode.interpolate()
    /// True
    "TenorMatchMode", PyTenorMatchMode, TenorMatchMode, "exact, interpolate",
    {
        /// Match the exact pillar only (errors if missing).
        exact => Exact,
        /// Interpolate the bump across adjacent knots.
        interpolate => Interpolate,
    }
);

scenario_enum!(
    /// Calendar-vs-business-day semantics for time-roll operations.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.scenarios import TimeRollMode
    /// >>> TimeRollMode("calendar_days").value
    /// 'calendar_days'
    "TimeRollMode", PyTimeRollMode, TimeRollMode, "business_days, calendar_days, approximate",
    {
        /// Business-day-aware roll (respects calendars when provided).
        business_days => BusinessDays,
        /// Pure calendar-day arithmetic.
        calendar_days => CalendarDays,
        /// Approximate day-count mode (non-additive across successive rolls).
        approximate => Approximate,
    }
);

scenario_enum!(
    /// Compounding convention for rate-extraction operations.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.scenarios import Compounding
    /// >>> Compounding("annual") == Compounding.annual()
    /// True
    "Compounding", PyCompounding, Compounding,
    "simple, continuous, annual, semi_annual, quarterly, monthly",
    {
        /// Simple interest (no compounding).
        simple => Simple,
        /// Continuous compounding (default).
        continuous => Continuous,
        /// Annual compounding.
        annual => Annual,
        /// Semi-annual compounding.
        semi_annual => SemiAnnual,
        /// Quarterly compounding.
        quarterly => Quarterly,
        /// Monthly compounding.
        monthly => Monthly,
    }
);
