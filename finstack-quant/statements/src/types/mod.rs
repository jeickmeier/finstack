//! Core types for financial statement modeling.

mod model;
mod node;
mod value;

pub use model::{
    upgrade_v1_to_v2, CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec,
    FINANCIAL_MODEL_CONTRACT,
};
pub use node::{
    ForecastMethod, ForecastSpec, NodeId, NodeSpec, NodeType, NodeValueType, SeasonalMode,
};
pub use value::{infer_series_value_type, AmountOrScalar};
