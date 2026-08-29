//! Path-level revolving-credit pricing results.

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::fixed_income::revolving_credit::cashflow_engine::ThreeFactorPathData;
use finstack_quant_core::money::Money;
use finstack_quant_models::monte_carlo::results::MonteCarloResult;

/// Result for a single path valuation.
///
/// Contains the present value, optional 3-factor path data, and the detailed cashflow schedule.
#[derive(Debug, Clone)]
pub struct PathResult {
    /// Present value for this path
    pub pv: Money,
    /// 3-factor path data (if from MC)
    pub path_data: Option<ThreeFactorPathData>,
    /// Cashflow schedule for this path
    pub cashflows: CashFlowSchedule,
}

/// Enhanced Monte Carlo results with full path details.
///
/// Extends the standard `MonteCarloResult` with individual path results
/// for distribution analysis and visualization.
#[derive(Debug)]
pub struct EnhancedMonteCarloResult {
    /// Standard MC statistics (mean, std error, CI)
    pub mc_result: MonteCarloResult,
    /// Individual path results for distribution analysis
    pub path_results: Vec<PathResult>,
}
