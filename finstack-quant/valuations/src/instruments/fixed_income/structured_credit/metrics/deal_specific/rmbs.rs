//! RMBS-specific metrics (LTV, FICO, WAL with PSA adjustments).

use crate::cashflow::builder::schedule::weighted_average_life_from_principal;
use crate::instruments::fixed_income::structured_credit::pricing::run_simulation;
use crate::instruments::fixed_income::structured_credit::{DealType, StructuredCredit};
use crate::metrics::MetricContext;

/// RMBS WAL calculator with PSA prepayment adjustments
pub struct RmbsWalCalculator;

impl crate::metrics::MetricCalculator for RmbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_quant_core::InputError::Invalid)?;

        if rmbs.deal_type != DealType::Rmbs {
            return Err(finstack_quant_core::InputError::Invalid.into());
        }

        let tranche_flows = run_simulation(rmbs, context.curves.as_ref(), context.as_of)?;
        weighted_average_life_from_principal(
            tranche_flows
                .values()
                .flat_map(|flows| flows.principal_flows.iter().copied()),
            context.as_of,
        )
    }
}
