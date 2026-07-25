//! Typed rates instruments: `InterestRateSwap` (this task), `Swaption` and
//! `CapFloor` (next task). Mirrors the `PyBond` pattern in `instruments.rs`.

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, serde_json_to_py, value_error};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::enum_from_str;
use super::typed_legs::{PyFixedLegSpec, PyFloatLegSpec};

type IrsBuilder = finstack_quant_valuations::instruments::rates::irs::InterestRateSwapBuilder;

/// Typed wrapper for the Rust `InterestRateSwap` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "InterestRateSwap",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyInterestRateSwap {
    /// Inner canonical Rust swap.
    pub(crate) inner: finstack_quant_valuations::instruments::InterestRateSwap,
}

impl PyInterestRateSwap {
    /// Serialize as the tagged instrument JSON accepted by the JSON loader.
    pub(crate) fn tagged_json(&self) -> PyResult<String> {
        serde_json::to_string(&InstrumentJson::InterestRateSwap(self.inner.clone()))
            .map_err(|err| serde_json_to_py(err, "failed to serialize InterestRateSwap"))
    }
}

#[pymethods]
impl PyInterestRateSwap {
    /// Create a fluent builder (mirrors Rust ``InterestRateSwap::builder()``).
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import InterestRateSwap
    /// >>> callable(InterestRateSwap.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyInterestRateSwapBuilder {
        PyInterestRateSwapBuilder {
            inner: Some(finstack_quant_valuations::instruments::InterestRateSwap::builder()),
        }
    }

    /// Deserialize from tagged instrument JSON (``{"type": "interest_rate_swap", ...}``).
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     Tagged instrument JSON with type ``"interest_rate_swap"``.
    ///
    /// Returns
    /// -------
    /// InterestRateSwap
    ///     The validated swap.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON is malformed, has a different instrument type, or
    ///     fails validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import InterestRateSwap
    /// >>> callable(InterestRateSwap.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match serde_json::from_str::<InstrumentJson>(json)
            .map_err(|err| serde_json_to_py(err, "invalid instrument JSON"))?
        {
            InstrumentJson::InterestRateSwap(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"interest_rate_swap\", got a different instrument type",
            )),
        }
    }

    /// Serialize to tagged instrument JSON accepted by ``price_instrument``.
    ///
    /// Returns
    /// -------
    /// str
    ///     Tagged instrument JSON accepted by ``price_instrument`` and
    ///     ``InterestRateSwap.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.tagged_json()
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!("InterestRateSwap(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyInterestRateSwap`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "InterestRateSwapBuilder",
    skip_from_py_object
)]
pub struct PyInterestRateSwapBuilder {
    inner: Option<IrsBuilder>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_irs(b: &mut PyInterestRateSwapBuilder) -> PyResult<IrsBuilder> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyInterestRateSwapBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the swap.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.id(finstack_quant_core::types::InstrumentId::new(
            value.to_string(),
        )));
        Ok(slf)
    }

    /// Set the notional (both legs).
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount shared by both legs.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the swap direction: ``"pay"`` or ``"receive"`` (fixed leg).
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     ``"pay"`` to pay fixed/receive floating, ``"receive"`` for the
    ///     opposite.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized side.
    #[pyo3(text_signature = "($self, value)")]
    fn side<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let side = enum_from_str(value, "side")?;
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.side(side));
        Ok(slf)
    }

    /// Set the fixed leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : FixedLegSpec
    ///     Fixed leg specification.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn fixed<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFixedLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.fixed(value.inner.clone()));
        Ok(slf)
    }

    /// Set the floating leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : FloatLegSpec
    ///     Floating leg specification.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn float<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFloatLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.float(value.inner.clone()));
        Ok(slf)
    }

    /// Build the validated swap.
    ///
    /// Returns
    /// -------
    /// InterestRateSwap
    ///     The validated swap.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If a required field is missing or Rust validation fails.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyInterestRateSwap> {
        let b = take_irs(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyInterestRateSwap { inner })
    }
}

/// Register the typed rates instruments on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyInterestRateSwap>()?;
    m.add_class::<PyInterestRateSwapBuilder>()?;
    Ok(())
}
