//! ABS-specific metrics (speed, delinquency, excess spread, credit enhancement).

use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::fixed_income::structured_credit::StructuredCredit;
use crate::metrics::MetricContext;

/// ABS Charge-Off Rate calculator
pub struct AbsChargeOffCalculator;

impl crate::metrics::MetricCalculator for AbsChargeOffCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let abs = context.instrument_as::<StructuredCredit>()?;

        let total_balance = abs.pool.total_balance()?;
        if total_balance.amount() > 0.0 {
            Ok(abs.pool.cumulative_defaults.amount() / total_balance.amount() * DECIMAL_TO_PERCENT)
        } else {
            Ok(0.0)
        }
    }
}

/// ABS Credit Enhancement Level calculator
pub struct AbsCreditEnhancementCalculator;

impl crate::metrics::MetricCalculator for AbsCreditEnhancementCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let abs = context.instrument_as::<StructuredCredit>()?;

        // Credit Enhancement = Subordination + OC + Excess Spread
        // Simplified: subordination for most senior tranche
        if let Some(senior_tranche) = abs.tranches.tranches.first() {
            let subordination = abs
                .tranches
                .subordination_amount(senior_tranche.id.as_str());
            let pool_balance = abs.pool.total_balance()?;

            if pool_balance.amount() > 0.0 {
                Ok(subordination.amount() / pool_balance.amount() * DECIMAL_TO_PERCENT)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
}
