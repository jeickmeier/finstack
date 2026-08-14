//! Deal-type specific metrics for structured credit.

pub(crate) mod abs;
pub(crate) mod cmbs;
pub(crate) mod rmbs;

pub use abs::{AbsChargeOffCalculator, AbsCreditEnhancementCalculator};

pub use cmbs::CmbsDscrCalculator;

pub use rmbs::RmbsWalCalculator;
