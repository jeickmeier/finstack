//! Commonly used types.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_quant_statements::prelude::*;
//! ```
//!
//! This prelude re-exports the core model/evaluation types plus the money and
//! period primitives statements-centric models typically need. Prefer importing
//! from the source module directly when you want a narrower API boundary.

pub use crate::builder::{MixedNodeBuilder, ModelBuilder};
pub use crate::error::{Error, Result};
pub use crate::evaluator::{Evaluator, StatementResult};
pub use crate::registry::Registry;
pub use crate::types::{
    AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeId, NodeSpec, NodeType,
};

pub use finstack_quant_core::currency::Currency;
pub use finstack_quant_core::dates::{Period, PeriodId};
pub use finstack_quant_core::money::Money;
