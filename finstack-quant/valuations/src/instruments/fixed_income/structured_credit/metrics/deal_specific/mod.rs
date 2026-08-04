//! Deal-type specific metrics for structured credit.

pub(crate) mod abs;
pub(crate) mod cmbs;
pub(crate) mod rmbs;

// Re-export ABS metrics
pub use abs::{AbsChargeOffCalculator, AbsCreditEnhancementCalculator};

// Re-export CMBS metrics
pub use cmbs::CmbsDscrCalculator;

// Re-export RMBS metrics
pub use rmbs::RmbsWalCalculator;
