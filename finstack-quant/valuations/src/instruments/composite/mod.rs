//! Generic cross-asset composite and synthetic instruments.
//!
//! A composite stores self-contained instrument specifications and an immutable
//! resolved quantity state. Valuation and risk use those quantities exactly as
//! stored; only [`CompositeSpec::initialize`] or [`CompositeInstrument::rebalance`]
//! can calculate a new state.

mod history;
mod types;

pub use history::{CompositeHistoryEngine, CompositeHistoryRow};
pub use types::{
    CompositeExposureReport, CompositeInstrument, CompositeLegSpec, CompositeLegValuation,
    CompositeMarketObservation, CompositeRebalanceResult, CompositeSpec, CompositeState,
    CompositeTrade, CompositeValuationDetails, PrimitiveAggregate, PrimitiveExposure,
    RebalanceFrequency, RebalanceRule, ResolvedCompositeLeg, WeightingMethod, MAX_COMPOSITE_DEPTH,
    MAX_COMPOSITE_LEGS,
};
