//! Dividend risk calculator for equity options.
//!
//! Dividend01 is the change in PV for a 1bp (0.0001) move in dividend yield;
//! see [`crate::instruments::equity::dividend01`] for the finite-difference
//! contract. For options, dividend yield affects the forward price
//! `F = S * exp((r - q) * T)`: a higher yield reduces the forward, making
//! calls less valuable and puts more valuable.

use crate::instruments::equity::dividend01::dividend01_central_diff;
use crate::instruments::equity::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_quant_core::Result;

/// Dividend risk calculator for equity options.
pub(crate) struct DividendRiskCalculator;

impl MetricCalculator for DividendRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;

        let t = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_quant_core::dates::DayCountContext::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        dividend01_central_diff(option, option.div_yield_id.as_ref(), context)
    }
}
